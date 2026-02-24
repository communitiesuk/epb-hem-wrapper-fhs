//! Functionality to support looking up information from the project input to help interpret returns
//! from the core

use home_energy_model::input::{FuelType, Input, InputForCalcHtcHlp};
use home_energy_model::output::OutputSummary;
use indexmap::IndexMap;
use std::sync::Arc;

#[derive(thiserror::Error, Debug)]
enum ProjectLookupError {
    #[error("No unit prices for fuel type {0}")]
    NoUnitPricesForFuel(FuelType),
}

fn unit_price_lookup(fuel: FuelType) -> Result<Option<f64>, ProjectLookupError> {
    // From SAP 10.2 (17-03-2022) Table 12
    // Units are p/kWh
    Ok(match fuel {
        FuelType::Electricity => 16.49.into(),
        FuelType::MainsGas => 3.64.into(),
        FuelType::Custom => None,
        FuelType::LpgBulk => 6.74.into(),
        FuelType::LpgBottled => 9.46.into(),
        FuelType::LpgCondition11F => 3.46.into(),
        _ => return Err(ProjectLookupError::NoUnitPricesForFuel(fuel)),
        // additional values here for fuel types not captured:
        // "gas": 9.46
        // coal: 5.58
        // oil: 4.94
        // wood: 5.12
    })
}

fn standing_charge_lookup(fuel: FuelType) -> Option<u32> {
    match fuel {
        FuelType::MainsGas => 92.into(),
        FuelType::LpgBulk => 62.into(),
        FuelType::LpgCondition11F => 92.into(),
        FuelType::Custom => None,
        _ => 0.into(),
    }
}

fn select_eer_applicable_usage(project: &Input, usage: &IndexMap<Arc<str>, f64>) -> f64 {
    fn is_vent(source: &str, project: &Input) -> bool {
        project
            .infiltration_ventilation()
            .mechanical_ventilation()
            .contains_key(source)
    }

    fn is_space_heating(source: &str, project: &Input) -> bool {
        // The premise is that the source either is a space heat system itself, directly identifies
        // itself as space_heating or is an auxiliary pump
        project
            .space_heat_system()
            .is_some_and(|systems| systems.contains_key(source))
            || source.contains("space_heating")
            || source.contains("auxillary") // typo reported upstream to DESNZ, should be "auxiliary"
    }

    fn is_water_heating(source: &str, project: &Input) -> bool {
        // The premise is that any cylinder drawn water will always reference hw cylinder (the only
        // allowed name for a storage tank), or the source will be the heat source or heat source
        // wet of the cylinder or the source will directly state that it is water_heating
        source.contains("hw cylinder")
            || project
                .hot_water_source()
                .get("hw_cylinder")
                .is_some_and(|s| {
                    s.contains_heat_source(source) || s.contains_heat_source_wet_reference(source)
                })
            || source.contains("water_heating")
    }

    fn is_lighting(source: &str) -> bool {
        // Lighting is either exactly "lighting" or exactly "topup" (which refers to additional
        // non fixture based lighting)
        ["lighting", "topup"].contains(&source)
    }

    usage
        .iter()
        .filter(|(source, _)| {
            is_vent(source.as_ref(), project)
                || is_space_heating(source.as_ref(), project)
                || is_water_heating(source.as_ref(), project)
                || is_lighting(source.as_ref())
        })
        .map(|(_, &kwh)| kwh)
        .sum()
}

pub(crate) struct FuelOutput {
    fuel: FuelType,
    eer_energy: f64,
    unit_price: Option<f64>,
    standing_charge: Option<u32>,
}

pub(crate) fn by_fuel(
    project: &Input,
    output_summary: &OutputSummary,
) -> anyhow::Result<Vec<FuelOutput>> {
    output_summary
        .delivered_energy()
        .iter()
        .filter_map(|(supply, usage)| {
            if let Some(energy_supply) = project.energy_supply().get(supply.as_ref()) {
                let fuel = energy_supply.fuel;
                Some(Ok(FuelOutput {
                    fuel,
                    eer_energy: select_eer_applicable_usage(project, usage),
                    unit_price: match unit_price_lookup(fuel) {
                        Ok(price) => price,
                        Err(e) => {
                            return Some(Err(e.into()));
                        }
                    },
                    standing_charge: standing_charge_lookup(fuel),
                }))
            } else {
                None
            }
        })
        .collect()
}
