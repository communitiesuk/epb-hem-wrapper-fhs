#![allow(dead_code)]

use anyhow::{anyhow, bail};
use arcstr::ArcStr;
use serde_json::Value;

use crate::future_homes_standard::input::InputForProcessing;
use indexmap::IndexMap;

/// Validate the charging sources declared on each heat battery.
///
/// These are checks no schema can express, because each compares values held in different parts
/// of the input.
///
/// They run before the wrapper branches by mode, which is the only place they can: the notional
/// building replaces the wet heat sources wholesale, so a heat battery does not survive into the
/// notional modes, and a check made during preprocessing would refuse an input in the actual
/// modes while accepting it in the notional ones. One input file has to get one verdict.
fn validate_heat_battery_charging(input: InputForProcessing) -> anyhow::Result<()> {
    let heat_batteries = input.heat_batteries_with_charging_sources()?;
    for (battery_name, battery) in &heat_batteries {
        let charging_sources = input.heat_battery_charging_sources(battery_name.clone())?;
        validate_one_charging_source(&battery_name, &charging_sources)?;

        for (source_name, source) in charging_sources {
            if source.get("type").and_then(Value::as_str) == Some("HeatSourceWet") {
                validate_hydronic_source(&input, battery_name, &battery, &source_name, &source)?;
            }
        }
    }
    Ok(())
}

/// Validate that a heat battery declares at most one charging source.
///
/// Every source is given the same off-peak window, so two of them are active over the same
/// timesteps, and the core requires charging sources to have non-overlapping schedules. Sharing
/// a battery between sources would mean dividing the window between them, which is a methodology
/// question rather than a detail of this wrapper, so a second source is refused rather than
/// described in a way that fails part way through a simulation.
fn validate_one_charging_source(
    battery_name: &str,
    charging_sources: &IndexMap<ArcStr, Value>,
) -> anyhow::Result<()> {
    if charging_sources.len() > 1 {
        let mut charging_sources_names: Vec<&str> =
            charging_sources.keys().map(|name| name.as_str()).collect();
        charging_sources_names.sort_unstable();

        bail!(
            "Heat battery '{battery_name}' has {} charging sources ({}), but only one is \
             supported. All charging sources are given the same off-peak window, and the core \
             requires their schedules not to overlap.",
            charging_sources.len(),
            charging_sources_names.join(", ")
        );
    }
    Ok(())
}

/// Validate one hydronic charging source against the battery and the rest of the input.
///
/// The named heat source must exist and must not be another battery. The charging circuit
/// should also be able to take the battery past its phase change temperature, which is a warning
/// rather than an error: below that point the battery still stores sensible heat, so the
/// configuration is degraded rather than impossible, and refusing it would block a system
/// somebody may have installed.
fn validate_hydronic_source(
    input: &InputForProcessing,
    battery_name: &ArcStr,
    battery: &Value,
    source_name: &str,
    source: &Value,
) -> anyhow::Result<()> {
    let referenced = source.get("name").and_then(Value::as_str).ok_or_else(|| {
        anyhow!("Heat battery '{battery_name}' charging source '{source_name}' has no name")
    })?;
    let heat_source_wet = input.heat_source_wet()?;
    let referenced_source = heat_source_wet.get(referenced).ok_or_else(|| {
        anyhow!(
            "Heat battery '{battery_name}' charging source '{source_name}' names heat source \
             '{referenced}', which is not in HeatSourceWet."
        )
    })?;

    if referenced_source.get("type").and_then(Value::as_str) == Some("HeatBattery") {
        bail!(
            "Heat battery '{battery_name}' charging source '{source_name}' names heat source \
             '{referenced}', which is itself a heat battery."
        );
    }

    let phase_change_temp = input.heat_battery_phase_transition_temperature_upper(battery)?;
    let temp_flow_limit_upper = source
        .get("temp_flow_limit_upper")
        .and_then(Value::as_f64)
        .ok_or_else(|| {
            anyhow!(
                "Heat battery '{battery_name}' charging source '{source_name}' has no numeric \
                 temp_flow_limit_upper"
            )
        })?;

    if let Some(phase_change_temp) = phase_change_temp {
        if temp_flow_limit_upper < phase_change_temp {
            // TODO: Implement structured way to return warnings & warn user that:
            // Heat battery '{battery_name}' charging source '{source_name}' is limited to \
            // {temp_flow_limit_upper}°C, below the battery's phase change temperature of \
            // {phase_change_temp}°C, so it can store only sensible heat. This compares the \
            // stated circuit limit, not what the heat source can achieve in operation.
        }
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};

    /// A dwelling with one PCM heat battery and a heat pump available to charge it.
    fn input(mut battery_extra: Map<String, Value>) -> InputForProcessing {
        let mut battery: Map<String, Value> = json!({
            "type": "HeatBattery",
            "battery_type": "pcm",
            "phase_transition_temperature_upper": 59,
            "phase_transition_temperature_lower": 57,
        })
        .as_object_mut()
        .unwrap()
        .clone();
        battery.append(&mut battery_extra);

        InputForProcessing {
            input: json!({
                "HeatSourceWet": {
                    "hb1": battery,
                    "HeatPump1": {
                        "type": "HeatPump"
                    },
                    "hb2": {
                        "type": "HeatBattery",
                        "battery_type": "pcm",
                    }
                }
            }),
        }
    }

    fn hydronic(overrides: Value) -> Value {
        let mut hydronic = json!({
            "type": "HeatSourceWet",
            "name": "HeatPump1",
            "temp_flow_limit_upper": 65.
        });
        hydronic
            .as_object_mut()
            .unwrap()
            .extend(overrides.as_object().unwrap().clone());
        hydronic
    }

    #[test]
    fn test_does_not_error_for_one_hydronic_source() {
        // Given a battery charged by a heat pump whose circuit exceeds the phase change
        let input = input(
            json!( {"temp_min_useful": 40.0, "HeatSource": {"hp": hydronic(json!({}))} })
                .as_object()
                .unwrap()
                .clone(),
        );
        // When validation is called
        assert!(validate_heat_battery_charging(input).is_ok());
        // Then no error is raised
    }

    // test renamed from Python's test_does_not_error_for_one_hydronic_source as that's a duplicate
    // test name and not accurate
    #[test]
    fn test_does_not_error_for_no_hydronic_source() {
        // Given a battery charged by a direct electric element at a rated power
        let input = input(
            json!( {"rated_charge_power": 15.0} )
                .as_object()
                .unwrap()
                .clone(),
        );
        // When validation is called
        assert!(validate_heat_battery_charging(input).is_ok());
        // Then no error is raised, there being no charging sources to check
    }

    #[test]
    fn test_error_for_more_than_one_charging_source() {
        // Given a battery with two charging sources, which the schema accepts
        let input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp_charging": hydronic(json!({})),
                "immersion": {"type": "DirectElectric", "rated_charge_power": 6.0},
            }})
            .as_object()
            .unwrap()
            .clone(),
        );
        // When validation is called, then it is rejected: every source is given the same off-peak
        // window, so two are active over the same timesteps, and the core requires charging
        // sources not to overlap
        let result = validate_heat_battery_charging(input);
        let error = result.unwrap_err().to_string();
        assert_eq!(
            error,
            "Heat battery 'hb1' has 2 charging sources (hp_charging, immersion), but only one is \
            supported. All charging sources are given the same off-peak window, and the core \
            requires their schedules not to overlap."
        );
    }

    #[test]
    fn test_error_when_the_named_heat_source_is_absent() {
        // Given a charging source naming a heat source that is not in the input
        let input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp": hydronic(json!({"name": "NoSuchPump"})),
            }})
            .as_object()
            .unwrap()
            .clone(),
        );
        // When validation is called, then it is rejected: no schema can express a cross-reference
        let result = validate_heat_battery_charging(input);
        let error = result.unwrap_err().to_string();
        assert!(error.contains("names heat source 'NoSuchPump', which is not in HeatSourceWet"));
    }

    #[test]
    fn test_error_when_the_named_heat_source_is_another_battery() {
        // Given a charging source naming another heat battery rather than a wet heat source
        let input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp": hydronic(json!({"name": "hb2"})),
            }})
            .as_object()
            .unwrap()
            .clone(),
        );
        // When validation is called, then it is rejected: a battery cannot charge from a battery
        let result = validate_heat_battery_charging(input);
        let error = result.unwrap_err().to_string();
        assert!(error.contains("which is itself a heat battery"));
    }

    #[ignore = "Ignore until we have implemented a structured way to return warnings"]
    #[test]
    fn test_warns_when_the_circuit_cannot_reach_the_phase_change() {
        // Given a charging circuit limited below the battery's 59 C phase change temperature
        let input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp": hydronic(json!({"temp_flow_limit_upper": 50.})),
            }})
            .as_object()
            .unwrap()
            .clone(),
        );
        // When validation is called
        let _result = validate_heat_battery_charging(input);
        // Then it warns rather than rejecting: below the phase change the battery still stores
        // sensible heat, so the configuration is degraded rather than impossible
    }

    #[test]
    fn test_no_warning_when_the_circuit_exceeds_the_phase_change() {
        // Given a charging circuit above the phase change temperature
        let input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp": hydronic(json!({})),
            }})
            .as_object()
            .unwrap()
            .clone(),
        );
        // When validation is called
        let result = validate_heat_battery_charging(input);
        // Then nothing is warned about
        assert!(result.is_ok());
    }

    #[test]
    fn test_ignores_a_heat_battery_that_is_not_pcm() {
        // Given a dry core battery, which has no phase change temperature
        let mut input = input(
            json!({
            "temp_min_useful": 40.0,
            "HeatSource": {
                "hp": hydronic(json!({})),
            }})
            .as_object()
            .unwrap()
            .clone(),
        );

        input
            .heat_source_wet_by_key_mut("hb1")
            .unwrap()
            .insert("battery_type".into(), json!("dry_core"));
        input
            .heat_source_wet_by_key_mut("hb1")
            .unwrap()
            .remove("phase_transition_temperature_upper");

        // When validation is called
        let result = validate_heat_battery_charging(input);
        // Then no error is raised and nothing is warned about, there being no phase change to
        // compare the circuit against
        assert!(result.is_ok());
    }
}
