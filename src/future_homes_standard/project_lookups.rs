//! Functionality to support looking up information from the project input to help interpret returns
//! from the core
#![allow(clippy::all)]
use anyhow::bail;
use fsum::FSum;
use home_energy_model::input::{FuelType, Input, InputForCalcHtcHlp};
use home_energy_model::output::{OutputCore, OutputSummary};
use indexmap::IndexMap;
use std::sync::Arc;

const OFFPEAK_END_HOUR: f64 = 7.0;

#[derive(thiserror::Error, Debug)]
enum ProjectLookupError {
    #[error("No unit prices for fuel type {0}")]
    NoUnitPricesForFuel(String),
}

fn tariff_lookup(fuel: &str) -> Result<Option<(f64, f64)>, ProjectLookupError> {
    // From SAP 10.2 (17-03-2022) Table 12
    // Each entry is (unit price in p/kWh, standing charge in £/yr).
    Ok(match fuel {
        "electricity" => Some((16.49, 0.0)),
        "mains_gas" => Some((3.64, 92.0)),
        "LPG_bulk" => Some((6.74, 62.0)),
        "LPG_bottled" => Some((9.46, 0.0)),
        "LPG_condition_11F" => Some((3.64, 92.0)),
        "gas" => Some((9.46, 0.0)),
        "coal" => Some((5.58, 0.0)),
        "oil" => Some((4.94, 0.0)),
        "wood" => Some((5.12, 0.0)),
        "custom" => None,
        // SAP 10.2 Table 12 prices electricity sold to the grid by PV at 5.59 p/kWh, with no standing
        // charge. It is a credit, so it is deducted from costs, emissions, or primary energy.
        "electricity_exported_pv" => Some((5.59, 0.0)),
        _ => return Err(ProjectLookupError::NoUnitPricesForFuel(fuel.into())),
    })
}

fn electricity_economy7_tarriff_lookup() -> [(f64, f64); 2] {
    // Economy 7 tiers are off-peak (00:00-07:00) then on-peak (07:00-24:00).
    // The off-peak standing charge is extra to the standard domestic electricity charge.
    // Regulated-energy calculations omit the standard charge but include this extra charge.
    [(9.40, 7.0), (19.60, 0.0)]
}

fn is_eer_applicable_source(project: &Input, source: &str) -> bool {
    let is_vent = project
        .infiltration_ventilation()
        .mechanical_ventilation()
        .contains_key(source);

    // The premise is that the source either is a space heat system itself, directly identifies
    // itself as space_heating or is an auxiliary pump.
    let is_space_heating = project
        .space_heat_system()
        .is_some_and(|systems| systems.contains_key(source))
        || source.contains("space_heating")
        || source.contains("auxiliary");

    // The premise is that any cylinder drawn water will always reference hw cylinder (the only
    // allowed name for a storage tank), or the source will be the heat source or heat source
    // wet of the cylinder or the source will directly state that it is water_heating.
    let is_water_heating = source.contains("hw cylinder")
        || project
            .hot_water_source()
            .get("hw cylinder")
            .is_some_and(|s| {
                s.contains_heat_source(source) || s.contains_heat_source_wet_reference(source)
            })
        || source.contains("water_heating");

    // Lighting is either exactly "lighting" or exactly "topup" (which refers to additional
    // non fixture based lighting).
    let is_lighting = ["lighting", "topup"].contains(&source);

    is_vent || is_space_heating || is_water_heating || is_lighting
}

fn select_eer_applicable_usage(project: &Input, usage: &IndexMap<Arc<str>, f64>) -> f64 {
    usage
        .iter()
        .filter(|(source, _)| is_eer_applicable_source(project, source.as_ref()))
        .map(|(_, &kwh)| kwh)
        .sum()
}

/// Return (net_off_peak_kwh, net_on_peak_kwh) for EER-applicable electricity.
///
/// SAP 10.2 splits Economy 7 costs into off-peak (00:00-07:00) and on-peak (07:00-24:00)
/// tiers per end-use using a fixed high/low rate fraction from Table 12a. FHS uses actual
/// half-hourly timestep data for a more precise split.
///
/// The self-consumed PV offset is also split by time of day: PV
/// displaces grid imports at the import tariff applicable at the time of generation.
///
/// The export credit is independent of time of day and
/// is handled separately.
///
/// Arguments:
/// * `project` - parsed FHS input, used for end-use applicability checks.
/// * `usage_by_end_use` - results_end_user[supply], per-end-use, per-timestep kWh.
/// * `generated_consumed_ts` - energy_generated_consumed[supply], self-consumed PV per
///   timestep (positive values). This is distinct from results_end_user negative entries,
///   which represent total generation (self-consumed + exported combined). Pass an empty
///   slice when no PV is present.
/// * `timestep_array` - simulation timestep start times in hours from start of year.
fn eer_energy_by_period(
    input: &Input,
    usage_by_end_use: &IndexMap<Arc<str>, Vec<f64>>,
    generated_consumed_ts: &[f64],
    timestep_array: &[f64],
) -> anyhow::Result<(f64, f64)> {
    let sim_start = input.simulation_time().iter().current_iteration().time;
    // FHS always starts at midnight Jan 1st (sim_start=0) but HEM core allows a
    // flexible start time, so we anchor to sim_start rather than assuming t=0 is midnight
    let is_offpeak = |t: f64| (t - sim_start).rem_euclid(24.) < OFFPEAK_END_HOUR;

    let mut consumption_off_peak: Vec<f64> = vec![];
    let mut consumption_on_peak: Vec<f64> = vec![];
    let mut generation_off_peak: Vec<f64> = vec![];
    let mut generation_on_peak: Vec<f64> = vec![];

    for (source, values) in usage_by_end_use.iter() {
        if !is_eer_applicable_source(input, source.as_ref()) {
            continue;
        }
        // replicate python has strict=True
        if values.len() != timestep_array.len() {
            bail!(
                "Usage for end use {source} has {} timesteps but expected {}",
                values.len(),
                timestep_array.len()
            );
        }
        for (&t, &kwh) in timestep_array.iter().zip(values.iter()) {
            if is_offpeak(t) {
                consumption_off_peak.push(kwh);
            } else {
                consumption_on_peak.push(kwh);
            }
        }
    }

    // Self-consumed PV offset: credited at the applicable import tariff rate,
    // split by time of day. generated_consumed_ts comes from output_core.energy_generated_consumed
    // which separates self-consumed from exported generation.
    if !generated_consumed_ts.is_empty() {
        if generated_consumed_ts.len() != timestep_array.len() {
            bail!(
                "Self-consumed generation has {} timesteps but expected {}",
                generated_consumed_ts.len(),
                timestep_array.len()
            );
        }
        for (&t, &kwh) in timestep_array.iter().zip(generated_consumed_ts.iter()) {
            if is_offpeak(t) {
                generation_off_peak.push(kwh);
            } else {
                generation_on_peak.push(kwh);
            }
        }
    }

    Ok((
        FSum::with_all(consumption_off_peak).value() - FSum::with_all(generation_off_peak).value(),
        FSum::with_all(consumption_on_peak).value() - FSum::with_all(generation_on_peak).value(),
    ))
}

/// Return the credit for electricity sold to the grid, or None if nothing was exported.
///
/// The credit is flat, at the SAP 10.2 Table 12 rate, and independent of time of day, so it
/// is the same on the standard and Economy 7 tariffs.
///
/// The credit is derived from generation to grid, which is generation surplus alone, so
/// discharge from a battery to the grid is excluded. The emissions and primary energy
/// calculations use total export, which includes it, so the two differ.
///
/// That difference is inert at present: the wrapper enables no battery grid export, so the two
/// quantities are equal in every run. Whether it is right is unsettled, and this split should
/// not be read as a settled position - the grid does not distinguish exported energy by its
/// source, and the Table 12 row is labelled for photovoltaic export rather than stating an
/// eligibility restriction.
///
/// Args:
///     generation_to_grid: output_core.generation_to_grid[supply], per timestep. Negative by
///         the core convention that export is negative demand, and all zeros for a supply
///         that exports nothing.
fn export_credit_tariff(generation_to_grid: &[f64]) -> anyhow::Result<Option<Tariff>> {
    let exported = -FSum::with_all(generation_to_grid.iter().copied()).value();
    if exported > 0.0 {
        let (export_price, export_sc) = match tariff_lookup("electricity_exported_pv") {
            Ok(Some((export_price, export_sc))) => (export_price, export_sc),
            _ => unreachable!("electricity_exported_pv tariff should always be present"),
        };
        Ok(Some(Tariff {
            eer_energy: -exported,
            unit_price: Some(export_price),
            standing_charge: Some(export_sc),
        }))
    } else {
        Ok(None)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Tariff {
    pub(crate) eer_energy: f64,
    pub(crate) unit_price: Option<f64>,
    pub(crate) standing_charge: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct FuelOutput {
    pub(crate) fuel: String,
    pub(crate) tariffs: Vec<Tariff>,
}

pub(crate) fn by_fuel(
    project: &Input,
    output_summary: &OutputSummary,
    output_core: &OutputCore,
    offpeak_electricity: bool,
) -> anyhow::Result<Vec<FuelOutput>> {
    // EER cost formula (based on SAP 10.2):
    //
    //   total cost = sum of (delivered_energy * unit_price / 100)  [SAP rows 240-250]
    //              + standing_charges                               [SAP row 251]
    //              - self_consumed_pv * import_tariff / 100        [SAP row 252/233a]
    //              - exported_pv * 5.59 / 100                      [SAP row 252/233b, Table 12]
    //
    // The delivered energy figures are gross: they include electricity
    // supplied by PV as well as by the grid. The deduction at the import tariff cancels
    // the PV-supplied portion. The deduction at 5.59 p/kWh
    // credits electricity sold to the grid. Both credits are represented as negative eer_energy
    // in the Tariff model so the standard cost formula (standing_charge + unit_price/100 *
    // eer_energy) produces a deduction.
    //
    // Comparison with the other FHS metrics (DER/carbon/PE in apply_fhs_postprocessing):
    //   total = energy_import * factor + energy_export * factor + energy_generated * gen_factor
    //         - energy_unregulated * factor
    // Those metrics use energy_import (actual grid imports, already net of self-consumed PV)
    // rather than gross delivered_energy. EER cannot do this directly because the EER-applicable
    // end-use filter requires per-end-use data that is only available in delivered_energy and
    // results_end_user, not in the aggregated energy_import array. The DER formula also applies
    // a separate generation factor (from the emissions/PE factors CSV) giving PV an additional
    // credit beyond just the displaced grid import. EER has no equivalent, PV electricity has
    // zero fuel cost, so there is no generation factor term here.
    let mut result = vec![];
    for (supply, usage) in output_summary.delivered_energy().iter() {
        let Some(energy_supply) = project.energy_supply().get(supply.as_ref()) else {
            // Excludes anything not from one of the provided EnergySupply's
            // e.g. _energy_from_environment
            continue;
        };
        let fuel = energy_supply.fuel;
        if offpeak_electricity && fuel == FuelType::Electricity {
            // Economy 7 path: split import cost into off-peak and on-peak tiers using timestep
            // data.
            let rates = electricity_economy7_tarriff_lookup();
            let usage_by_end_use = output_core
                .results_end_user
                .get(supply)
                .cloned()
                .unwrap_or_default();
            // energy_generated_consumed[supply] is self-consumed PV per timestep (positive
            // values). This is separate from results_end_user negative entries, which are total
            // generation (self-consumed + exported). We need only self-consumed here.
            let generated_consumed_ts = output_core
                .energy_generated_consumed
                .get(supply)
                .cloned()
                .unwrap_or_default();
            let energies = eer_energy_by_period(
                project,
                &usage_by_end_use,
                &generated_consumed_ts,
                &output_core.timestep_array,
            )?;
            let mut tariffs = rates
                .iter()
                .zip([energies.0, energies.1])
                .map(|(&(u, sc), e)| Tariff {
                    eer_energy: e,
                    unit_price: Some(u),
                    standing_charge: Some(sc),
                })
                .collect::<Vec<_>>();
            let export_credit = export_credit_tariff(
                &output_core
                    .generation_to_grid
                    .get(supply)
                    .cloned()
                    .unwrap_or_default(),
            )?;
            if let Some(export_credit) = export_credit {
                tariffs.push(export_credit);
            }
            result.push(FuelOutput {
                fuel: "electricity_economy7".into(),
                tariffs,
            });
        } else {
            let fuel = fuel.to_string();
            // None for a custom fuel: no price defined means EER not applicable
            let rates = [tariff_lookup(&fuel)?];
            let energy = select_eer_applicable_usage(project, usage);
            // Self-consumed PV offset: credits self-consumed PV at the import
            // tariff, cancelling the overstatement in the gross delivered_energy figure.
            // output_core.energy_generated_consumed separates self-consumed from exported
            // generation, avoiding over-crediting exported electricity at the full import rate.
            let self_consumed = FSum::with_all(
                output_core
                    .energy_generated_consumed
                    .get(supply)
                    .cloned()
                    .unwrap_or_default(),
            )
            .value();
            let mut tariffs = rates
                .iter()
                .map(|rate| Tariff {
                    eer_energy: energy - self_consumed,
                    unit_price: rate.map(|(u, _)| u),
                    standing_charge: rate.map(|(_, sc)| sc),
                })
                .collect::<Vec<_>>();
            // Export credit: only applicable to electricity supplies with PV.
            if fuel == "electricity" {
                let export_credit = export_credit_tariff(
                    &output_core
                        .generation_to_grid
                        .get(supply)
                        .cloned()
                        .unwrap_or_default(),
                )?;
                if let Some(export_credit) = export_credit {
                    tariffs.push(export_credit);
                }
            }
            result.push(FuelOutput { fuel, tariffs });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::future_homes_standard::future_homes_standard::{
        final_preprocessing, initial_preprocessing,
    };
    use crate::future_homes_standard::input::InputForProcessing;
    use crate::future_homes_standard::metrics::energy_efficiency_rating;
    use home_energy_model::input::ExternalConditionsInput;
    use home_energy_model::{load_weather_data, WeatherFileType};
    use rstest::*;
    use serde_json::{json, Value};
    use std::io::Cursor;

    #[fixture]
    fn instant_elec_project() -> Input {
        // Thing that matters here is that there are two EnergySupply entries
        // One called "mains elec" with fuel = electricity
        // One called "mains gas" with fuel = mains_gas
        // Two mechanical vents "mechvent1", "mechvent2"
        // One SpaceHeatSystem called "main"
        // A "hw cylinder" with a heat source called "immersion"

        let mut project = serde_json::from_str::<Value>(include_str!(
            "../../examples/input/future_homes_standard/demo_FHS.json"
        ))
        .unwrap();
        let weather = ExternalConditionsInput::from(
            load_weather_data(
                Cursor::new(include_str!("./RAF_Bedford_01.epw")),
                WeatherFileType::Epw,
            )
            .unwrap(),
        );
        let shading_segments = project["ExternalConditions"]["shading_segments"].clone();
        project["ExternalConditions"] = serde_json::to_value(weather).unwrap();
        project["ExternalConditions"]["shading_segments"] = shading_segments;

        let mut project = InputForProcessing { input: project };
        initial_preprocessing(&mut project).unwrap();
        final_preprocessing(&mut project).unwrap();

        serde_json::from_value(project.input).unwrap()
    }

    #[fixture]
    fn instant_elec_output_summary() -> OutputSummary {
        // delivered energy matters here

        // make OutputSummary derive Deserialize, and uncomment
        serde_json::from_value(json!({
            "total_floor_area": 100,
            "space_heat_demand_total": 1000,
            "space_cool_demand_total": -100,
            "electricity_peak_consumption": {
                "peak": 10,
                "index": 1,
                "month": 1,
                "day": 1,
                "hour": 1,
            },
            "energy_supply": {
                "mains elec": {
                    "generation": 0,
                    "consumption": 1100,
                    "generation_to_consumption": 0,
                    "generation_to_grid": 0,
                    "generation_to_diverter": 0,
                    "generation_to_storage": 0,
                    "grid_to_consumption": 1100,
                    "grid_to_storage": 0,
                    "storage_to_consumption": 0,
                    "storage_efficiency": null,
                    "net_import": 1100,
                    "total_gross_import": 1100,
                    "total_gross_export": 0,
                },
            },
            "delivered_energy": {
                "mains elec": {
                    "mechvent1": 100,  // vent
                    "mechvent3": 200,  // not a vent
                    "lighting": 300,  // counts as lighting
                    "something random": 400,  // not relevant for EER
                    "hw cylinder": 500,  // counts as water heating
                    "immersion": 600,  // counts as water heating
                    "something_water_heating": 700,  // counts as water heating
                },
                "mains gas": {
                    "main": 1000,  // heating system counts
                    "Hobs": 100,  // not relevant for EER,
                    "something_space_heating": 200,  // counts as space heating
                    "auxiliary_pump_thing": 300,  // counts as a space heating system pump
                }
            },
            "hot_water_demand_daily_75th_percentile": {
                "hw cylinder": 1000,
            }
        }))
        .unwrap()
    }

    #[fixture]
    fn heat_network_project() -> Input {
        // Thing that matters here is that there is two EnergySupply's
        // One called "mains elec" with fuel = electricity
        // One called "custom_heat_network_supply" with fuel = custom
        // One mechanical vent "cMEV"
        // One SpaceHeatSystem called "SpaceHeatSystem1"
        // A "hw cylinder" with a heat source called "heat network"

        let mut project = serde_json::from_str::<Value>(include_str!(
            "../../examples/input/future_homes_standard/DESN-H-End-02-HN-cMEV.json"
        ))
        .unwrap();
        let weather = ExternalConditionsInput::from(
            load_weather_data(
                Cursor::new(include_str!("./RAF_Bedford_01.epw")),
                WeatherFileType::Epw,
            )
            .unwrap(),
        );
        let shading_segments = project["ExternalConditions"]["shading_segments"].clone();
        project["ExternalConditions"] = serde_json::to_value(weather).unwrap();
        project["ExternalConditions"]["shading_segments"] = shading_segments;

        let mut project = InputForProcessing { input: project };
        initial_preprocessing(&mut project).unwrap();
        final_preprocessing(&mut project).unwrap();

        serde_json::from_value(project.input).unwrap()
    }

    #[fixture]
    fn heat_network_output_summary() -> OutputSummary {
        // delivered energy matters here

        // uncomment below once OutputSummary derives Deserialize
        serde_json::from_value(json!({
            "total_floor_area": 100,
            "space_heat_demand_total": 1000,
            "space_cool_demand_total": -100,
            "electricity_peak_consumption": {
                "peak": 10,
                "index": 1,
                "month": 1,
                "day": 1,
                "hour": 1,
            },
            "energy_supply": {
                "mains elec": {
                    "generation": 0,
                    "consumption": 1100,
                    "generation_to_consumption": 0,
                    "generation_to_grid": 0,
                    "generation_to_diverter": 0,
                    "generation_to_storage": 0,
                    "grid_to_consumption": 1100,
                    "grid_to_storage": 0,
                    "storage_to_consumption": 0,
                    "storage_efficiency": null,
                    "net_import": 1100,
                    "total_gross_import": 1100,
                    "total_gross_export": 0,
                }
            },
            "delivered_energy": {
                "mains elec": {
                    "cMEV": 100,  // vent
                    "mechvent3": 200,  // not a vent
                    "topup": 300,  // counts as lighting
                    "something random": 400,  // not relevant for EER
                    "hw cylinder": 500,  // counts as water heating
                    "heat network": 600,  // counts as water heating
                    "something_water_heating": 700,  // counts as water heating
                },
                "custom_heat_network_supply": {
                    "SpaceHeatSystem1": 1000,  // heating system counts
                    "Hobs": 100,  // not relevant for EER,
                    "something_space_heating": 200,  // counts as space heating
                    "auxiliary_pump_thing": 300,  // counts as a space heating system pump
                }
            },
            "hot_water_demand_daily_75th_percentile": {
                "hw cylinder": 1000
            }
        }))
        .unwrap()
    }

    #[fixture]
    fn electricity_and_gas_project() -> Input {
        // Thing that matters here is that there are three EnergySupply objects
        // One called "mains elec" with fuel = electricity
        // One called "mains gas" with fuel = mains_gas
        // One called "LPG_bulk" with fuel = LPG_bulk
        // One mechanical vent "cMEV"
        // One SpaceHeatSystem called "SpaceHeatSystem1"
        // A "hw cylinder" with a heat source called "boiler"

        let mut project = serde_json::from_str::<Value>(include_str!(
            "../../examples/input/future_homes_standard/DESN-H-End-02-Blr-cMEV-Combi-lpg-bulk.json"
        ))
        .unwrap();
        let weather = ExternalConditionsInput::from(
            load_weather_data(
                Cursor::new(include_str!("./RAF_Bedford_01.epw")),
                WeatherFileType::Epw,
            )
            .unwrap(),
        );
        let shading_segments = project["ExternalConditions"]["shading_segments"].clone();
        project["ExternalConditions"] = serde_json::to_value(weather).unwrap();
        project["ExternalConditions"]["shading_segments"] = shading_segments;
        // TODO complete when functions are implemented during migration to 1.0.0a4
        // initial_preprocessing(&mut project);
        // final_preprocessing(&mut project);

        let mut project = InputForProcessing { input: project };

        initial_preprocessing(&mut project).expect("");
        final_preprocessing(&mut project).expect("");

        project.finalize().unwrap()
    }

    #[fixture]
    fn electricity_and_gas_output_summary() -> OutputSummary {
        // delivered energy matters here

        // uncomment below once OutputSummary derives Deserialize
        serde_json::from_value(json!({
            "total_floor_area": 100,
            "space_heat_demand_total": 1000,
            "space_cool_demand_total": -100,
            "electricity_peak_consumption": {
                "peak": 10,
                "index": 1,
                "month": 1,
                "day": 1,
                "hour": 1,
            },
            "energy_supply": {
                "mains elec": {
                    "generation": 0,
                    "consumption": 1100,
                    "generation_to_consumption": 0,
                    "generation_to_grid": 0,
                    "generation_to_diverter": 0,
                    "generation_to_storage": 0,
                    "grid_to_consumption": 1100,
                    "grid_to_storage": 0,
                    "storage_to_consumption": 0,
                    "storage_efficiency": None::<f64>,
                    "net_import": 1100,
                    "total_gross_import": 1100,
                    "total_gross_export": 0,
                }
            },
            "delivered_energy": {
                "mains elec": {
                    "cMEV": 100,  // vent
                    "lighting": 300,  // counts as lighting
                    "something random": 400,  // not relevant for EER
                    "hw cylinder": 20,  // counts as water heating
                },
                "LPG_bulk": {
                    "boiler": 600,  // counts as water heating
                    "Hobs": 100,  // not relevant for EER,
                    "something_space_heating": 200,  // counts as space heating
                },
                "mains gas": {},
            },
            "hot_water_demand_daily_75th_percentile": {
                "hw cylinder": 1000
            }
        }))
        .unwrap()
    }

    #[fixture]
    fn no_pv_output_core() -> OutputCore {
        output_core(json!([]), json!({}), json!({}), json!({}))
    }

    fn output_core(
        timestep_array: Value,
        results_end_user: Value,
        energy_generated_consumed: Value,
        generation_to_grid: Value,
    ) -> OutputCore {
        serde_json::from_value(json!({
            "timestep_array": timestep_array,
            "results_totals": {},
            "results_end_user": results_end_user,
            "energy_import": {},
            "energy_export": {},
            "grid_to_consumption": {},
            "generation_to_grid": generation_to_grid,
            "energy_generated_consumed": energy_generated_consumed,
            "energy_to_storage": {},
            "energy_from_storage": {},
            "storage_from_grid": {},
            "battery_state_of_charge": {},
            "energy_diverted": {},
            "beta_factor": {},
            "zone_list": [],
            "zone_data": {
                "internal_gains": {},
                "solar_gains": {},
                "operative_temp": {},
                "internal_air_temp": {},
                "space_heat_demand": {},
                "space_cool_demand": {},
            },
            "heating_cooling_system": {
                "heating_system_output": {},
                "cooling_system_output": {},
            },
            "cop": {
                "space_heating_system": {},
                "space_cooling_system": {},
                "hot_water_system": {},
            },
            "ductwork_gains": [],
            "heat_balance_all": {},
            "heat_source_wet_results": {},
            "heat_source_wet_results_annual": {},
            "hot_water_source_results": {},
            "emitters": {},
            "electric_storage_heaters": {},
            "ventilation": [],
            "hot_water_systems": {
                "demand": {},
                "energy_demand_at_hot_water_source": {},
                "energy_demand_at_tapping_points": {},
                "duration": {},
                "events_count": {},
                "losses_pipework": {},
                "losses_primary_pipework": {},
                "losses_storage": {},
            },
        }))
        .unwrap()
    }

    fn output_summary_with_electricity_usage(delivered_energy: Value) -> OutputSummary {
        serde_json::from_value(json!({
            "total_floor_area": 100,
            "space_heat_demand_total": 1000,
            "space_cool_demand_total": -100,
            "electricity_peak_consumption": {
                "peak": 10,
                "index": 1,
                "month": 1,
                "day": 1,
                "hour": 1,
            },
            "energy_supply": {
                "mains elec": {
                    "generation": 0,
                    "consumption": 0,
                    "generation_to_consumption": 0,
                    "generation_to_grid": 0,
                    "generation_to_diverter": 0,
                    "generation_curtailed": 0,
                    "generation_to_storage": 0,
                    "grid_to_consumption": 0,
                    "grid_to_storage": 0,
                    "storage_to_consumption": 0,
                    "storage_to_grid": 0,
                    "storage_efficiency": null,
                    "net_import": 0,
                    "total_gross_import": 0,
                    "total_gross_export": 0,
                }
            },
            "delivered_energy": {
                "mains elec": delivered_energy,
            },
            "hot_water_demand_daily_75th_percentile": {
                "hw cylinder": 1000,
            }
        }))
        .unwrap()
    }

    fn economy7_output_core(
        timestep_array: &[f64],
        sources: &[&str],
        usage: &[f64],
        generated_consumed: Option<&[f64]>,
        generation_to_grid: Option<&[f64]>,
    ) -> OutputCore {
        let mut end_user = serde_json::Map::new();
        for source in sources {
            end_user.insert((*source).to_owned(), json!(usage));
        }

        output_core(
            json!(timestep_array),
            json!({"mains elec": end_user}),
            generated_consumed
                .map(|values| json!({"mains elec": values}))
                .unwrap_or_else(|| json!({})),
            generation_to_grid
                .map(|values| json!({"mains elec": values}))
                .unwrap_or_else(|| json!({})),
        )
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_without_custom_fuel(
        instant_elec_project: Input,
        instant_elec_output_summary: OutputSummary,
        no_pv_output_core: OutputCore,
    ) {
        // Given a by fuel lookup is called
        let fuels = by_fuel(
            &instant_elec_project,
            &instant_elec_output_summary,
            &no_pv_output_core,
            false,
        )
        .unwrap();
        // then the total delivered energy is calculated based on the floor area normalised
        // calculated properties
        assert_eq!(
            fuels,
            vec![
                FuelOutput {
                    fuel: "electricity".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 2200.,
                        unit_price: Some(16.49),
                        standing_charge: Some(0.),
                    }],
                },
                FuelOutput {
                    fuel: "mains_gas".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 1500.,
                        unit_price: Some(3.64),
                        standing_charge: Some(92.),
                    }],
                }
            ]
        );
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]

    fn test_with_custom_fuel(
        heat_network_project: Input,
        heat_network_output_summary: OutputSummary,
        no_pv_output_core: OutputCore,
    ) {
        // Given a by fuel lookup is called
        let fuels = by_fuel(
            &heat_network_project,
            &heat_network_output_summary,
            &no_pv_output_core,
            false,
        )
        .unwrap();
        // then the total delivered energy is calculated based on the floor area normalised
        // calculated properties
        assert_eq!(
            fuels,
            vec![
                FuelOutput {
                    fuel: "electricity".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 2200.,
                        unit_price: Some(16.49),
                        standing_charge: Some(0.),
                    }],
                },
                FuelOutput {
                    fuel: "custom".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 1500.,
                        unit_price: None,
                        standing_charge: None,
                    }],
                }
            ]
        );
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_electricity_and_gas(
        electricity_and_gas_project: Input,
        electricity_and_gas_output_summary: OutputSummary,
        no_pv_output_core: OutputCore,
    ) {
        // Given a project that has electricity, LPG_bulk and mains_gas energy supplies
        // When a by_fuel lookup is called
        let fuels = by_fuel(
            &electricity_and_gas_project,
            &electricity_and_gas_output_summary,
            &no_pv_output_core,
            false,
        )
        .unwrap();
        // Then the total delivered energy is calculated from the
        // floor-area-normalised results for all three energy supplies.
        // Note that nothing uses the mains_gas supply, so it has zero delivered energy.
        assert_eq!(
            fuels,
            vec![
                FuelOutput {
                    fuel: "electricity".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 420.,
                        unit_price: Some(16.49),
                        standing_charge: Some(0.),
                    }],
                },
                FuelOutput {
                    fuel: "LPG_bulk".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 800.,
                        unit_price: Some(6.74),
                        standing_charge: Some(62.),
                    }],
                },
                FuelOutput {
                    fuel: "mains_gas".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 0.,
                        unit_price: Some(3.64),
                        standing_charge: Some(92.),
                    }],
                }
            ]
        );
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_economy7_splits_electricity_by_timestep(instant_elec_project: Input) {
        // Given a project on Economy 7 with electricity consumption spread across on- and off-peak
        // timesteps. Two full days of half-hourly data (96 timesteps, step = 0.5h, start = 0.0h).
        // EER-applicable end-uses: "mechvent1" (vent), "lighting", "hw cylinder", "immersion",
        // "something_water_heating". Non-applicable: "mechvent3", "something random".
        // Off-peak slots: t % 24 < 7.0, i.e. t in {0.0, 0.5, ..., 6.5}: 14 slots per day.
        // On-peak slots: remaining 34 slots per day.
        // Each off-peak slot contributes 1 kWh per applicable end-use; each on-peak slot 2 kWh.
        let n_days = 2;
        let n_slots = 48 * n_days;
        let timestep_array = (0..n_slots)
            .map(|index| index as f64 * 0.5)
            .collect::<Vec<_>>();
        let usage = timestep_array
            .iter()
            .map(|time| if time % 24. < 7. { 1. } else { 2. })
            .collect::<Vec<_>>();
        let sources = [
            "mechvent1",
            "lighting",
            "hw cylinder",
            "immersion",
            "something_water_heating",
            "mechvent3",
            "something random",
        ];
        // No PV.
        // No export.
        let output_core = economy7_output_core(&timestep_array, &sources, &usage, None, None);
        let output_summary = output_summary_with_electricity_usage(json!({
            "mechvent1": 0,
            "mechvent3": 0,
            "lighting": 0,
            "something random": 0,
            "hw cylinder": 0,
            "immersion": 0,
            "something_water_heating": 0,
        }));

        // When by_fuel is called with offpeak_electricity=true
        let fuels = by_fuel(&instant_elec_project, &output_summary, &output_core, true).unwrap();

        // Then electricity appears as a single FuelOutput with two tariff tiers.
        // 5 applicable sources x 14 off-peak slots x 2 days x 1 kWh = 140 kWh off-peak.
        // 5 applicable sources x 34 on-peak slots x 2 days x 2 kWh = 680 kWh on-peak.
        assert_eq!(
            fuels,
            vec![FuelOutput {
                fuel: "electricity_economy7".into(),
                tariffs: vec![
                    Tariff {
                        eer_energy: 140.,
                        unit_price: Some(9.40),
                        standing_charge: Some(7.),
                    },
                    Tariff {
                        eer_energy: 680.,
                        unit_price: Some(19.60),
                        standing_charge: Some(0.),
                    },
                ],
            }]
        );
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_economy7_pv_offset_split_by_time_of_day(instant_elec_project: Input) {
        // Given an Economy 7 dwelling with PV self-consumption and export.
        // Two full days of half-hourly data (96 timesteps, step = 0.5h, start = 0.0h).
        // EER-applicable consumption: same as economy7 test (5 sources, 1 kWh off-peak /
        // 2 kWh on-peak per slot).
        // Self-consumed PV (energy_generated_consumed): 0.5 kWh off-peak, 1.0 kWh on-peak.
        // Exported PV (generation_to_grid): 0.25 kWh on-peak only (PV only exports on-peak).
        let n_days = 2;
        let n_slots = 48 * n_days;
        let timestep_array = (0..n_slots)
            .map(|index| index as f64 * 0.5)
            .collect::<Vec<_>>();
        let usage = timestep_array
            .iter()
            .map(|time| if time % 24. < 7. { 1. } else { 2. })
            .collect::<Vec<_>>();
        // Self-consumed PV from output_core.energy_generated_consumed (positive values).
        let generated_consumed = timestep_array
            .iter()
            .map(|time| if time % 24. < 7. { 0.5 } else { 1. })
            .collect::<Vec<_>>();
        // Exported PV from output_core.generation_to_grid - on-peak only (daylight).
        // Negative because the core reports export as negative demand.
        let exported = timestep_array
            .iter()
            .map(|time| if time % 24. < 7. { 0. } else { -0.25 })
            .collect::<Vec<_>>();
        let sources = [
            "mechvent1",
            "lighting",
            "hw cylinder",
            "immersion",
            "something_water_heating",
            "mechvent3",
            "something random",
        ];
        let output_core = economy7_output_core(
            &timestep_array,
            &sources,
            &usage,
            Some(&generated_consumed),
            Some(&exported),
        );
        let output_summary = output_summary_with_electricity_usage(json!({
            "mechvent1": 0,
            "mechvent3": 0,
            "lighting": 0,
            "something random": 0,
            "hw cylinder": 0,
            "immersion": 0,
            "something_water_heating": 0,
        }));

        // The self-consumed PV offset is split by time of day; the export credit is flat.
        let fuels = by_fuel(&instant_elec_project, &output_summary, &output_core, true).unwrap();

        // Consumption (5 applicable sources):
        //   off-peak: 5 x 14 slots x 2 days x 1 kWh = 140 kWh
        //   on-peak:  5 x 34 slots x 2 days x 2 kWh = 680 kWh
        // Self-consumed PV (SAP row 233a, split by time of day):
        //   off-peak: 14 slots x 2 days x 0.5 kWh = 14 kWh
        //   on-peak:  34 slots x 2 days x 1.0 kWh = 68 kWh
        // Net off-peak = 140 - 14 = 126 kWh; net on-peak = 680 - 68 = 612 kWh
        // Exported PV (SAP row 233b, flat 5.59 p/kWh regardless of time of day):
        //   34 on-peak slots x 2 days x 0.25 kWh = 17 kWh; credit as negative eer_energy
        assert_eq!(
            fuels,
            vec![FuelOutput {
                fuel: "electricity_economy7".into(),
                tariffs: vec![
                    Tariff {
                        eer_energy: 126.,
                        unit_price: Some(9.40),
                        standing_charge: Some(7.),
                    },
                    Tariff {
                        eer_energy: 612.,
                        unit_price: Some(19.60),
                        standing_charge: Some(0.),
                    },
                    Tariff {
                        eer_energy: -17.,
                        unit_price: Some(5.59),
                        standing_charge: Some(0.),
                    },
                ],
            }]
        );
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_standard_electricity_pv_exceeds_eer_consumption(instant_elec_project: Input) {
        // Given an all-electric dwelling where self-consumed PV exceeds all EER-applicable
        // consumption. Net eer_energy should be negative and EER above 100.
        // EER-applicable gross delivered energy: mechvent1 (100) + lighting (300)
        //   + hw cylinder (500) + immersion (600) + something_water_heating (700) = 2200 kWh.
        // Self-consumed PV: 2500 kWh: net eer_energy = 2200 - 2500 = -300 kWh.
        let output_summary = output_summary_with_electricity_usage(json!({
            "mechvent1": 100,
            "mechvent3": 200,
            "lighting": 300,
            "something random": 400,
            "hw cylinder": 500,
            "immersion": 600,
            "something_water_heating": 700,
        }));
        // Self-consumed PV from output_core.energy_generated_consumed: Using annual total as a
        // single-element list for simplicity; fsum gives the same result as timestep arrays.
        let output_core = output_core(
            json!([]),
            json!({}),
            json!({"mains elec": [2500.]}),
            json!({}),
        );

        let fuels = by_fuel(&instant_elec_project, &output_summary, &output_core, false).unwrap();
        // Gross delivered electricity less self-consumed PV is 2200 - 2500 = -300 kWh.
        assert_eq!(
            fuels,
            vec![FuelOutput {
                fuel: "electricity".into(),
                tariffs: vec![Tariff {
                    eer_energy: -300.,
                    unit_price: Some(16.49),
                    standing_charge: Some(0.),
                }],
            }]
        );

        // total_cost = 0.0 + 16.49/100 x (-300) = -49.47
        // ecf = 0.36 x (-49.47) / (100 + 45) = -0.12282206896...
        // eer = 100 - 16.21 x (-0.12282206896...) = 101.99094573793103
        let eer = energy_efficiency_rating(100., &fuels);
        let eer = serde_json::to_value(eer).unwrap();
        assert!((eer["value"].as_f64().unwrap() - 101.99094573793103).abs() < 1e-12);
        assert_eq!(eer["grade"], "A");
    }

    #[rstest]
    #[ignore = "Ignored while migrating to 1.0.0a9"]
    fn test_standard_electricity_pv_offset(
        instant_elec_project: Input,
        instant_elec_output_summary: OutputSummary,
    ) {
        // Given a dwelling on standard (non-Economy-7) electricity with on-site PV.
        // EER-applicable delivered energy: same as instant_elec test (2200 kWh gross, including
        // PV-supplied consumption). Self-consumed PV reported via output_core: 200 kWh.
        // Exported PV reported via output_core: 50 kWh.
        // energy_generated_consumed: annual total as a single-element list for simplicity; fsum
        // gives the same result as timestep arrays.
        // generation_to_grid: exported PV is negative because the core reports export as negative
        // demand.
        let output_core = output_core(
            json!([]),
            json!({}),
            json!({"mains elec": [200.]}),
            json!({"mains elec": [-50.]}),
        );

        // When by_fuel is called with standard electricity and PV.
        let fuels = by_fuel(
            &instant_elec_project,
            &instant_elec_output_summary,
            &output_core,
            false,
        )
        .unwrap();
        // EER-applicable electricity (gross): 100 + 300 + 500 + 600 + 700 = 2200 kWh
        // Self-consumed PV: 200 kWh deducted at import tariff
        // Net import electricity: 2200 - 200 = 2000 kWh at 16.49 p/kWh
        // Exported PV (SAP row 233b): 50 kWh credit at 5.59 p/kWh, represented as
        // negative eer_energy so the cost formula gives a deduction
        // Gas: unchanged - no generation on a gas supply
        assert_eq!(
            fuels,
            vec![
                FuelOutput {
                    fuel: "electricity".into(),
                    tariffs: vec![
                        Tariff {
                            eer_energy: 2000.,
                            unit_price: Some(16.49),
                            standing_charge: Some(0.),
                        },
                        Tariff {
                            eer_energy: -50.,
                            unit_price: Some(5.59),
                            standing_charge: Some(0.),
                        },
                    ],
                },
                FuelOutput {
                    fuel: "mains_gas".into(),
                    tariffs: vec![Tariff {
                        eer_energy: 1500.,
                        unit_price: Some(3.64),
                        standing_charge: Some(92.),
                    }],
                }
            ]
        );
    }
}
