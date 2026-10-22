#![allow(dead_code)]

use arcstr::ArcStr;
use serde_json::Value;

use crate::future_homes_standard::input::InputForProcessing;
use std::collections::HashMap;

/// Return the dwelling's smart air brick floors, by building element name.
///
/// The conditions are the ones the core applies when it honours a smart air brick control, so
/// that the wrapper and the core cannot disagree about which elements the feature is for. The
/// schema confines the flag to a suspended ground floor already, which makes the type checks
/// redundant against a valid input and load-bearing against anything else: without them a
/// dwelling could be refused over elements the core would ignore.
fn smart_air_brick_floors(input: InputForProcessing) -> anyhow::Result<HashMap<ArcStr, Value>> {
    let mut result = HashMap::new();
    for (name, element) in input.all_smart_air_brick_building_elements()? {
        if element
            .get("floor_type")
            .and_then(|v| v.as_str())
            .is_some_and(|floor_type| floor_type == "Suspended_floor")
        {
            result.insert(name, element.clone());
        }
    }
    Ok(result)
}

/// Validate that at most one floor of the dwelling has smart air bricks.
///
/// The core models one smart air brick system per zone and the schema allows a single zone, so
/// two flagged floors cannot be simulated. Both the schema and the core's own input model
/// accept them, and the run then fails part way through, so the input is refused here instead.
///
/// This runs before the wrapper branches by mode, which is the only place it can: the notional
/// building has fixed air bricks, so notional preprocessing strips the flag before any
/// mode-specific step could count it, and a check made later would refuse the input in the
/// actual modes while accepting it in the notional ones. One input file has to get one verdict.
fn validate_smart_air_brick_floors(input: InputForProcessing) -> anyhow::Result<()> {
    let mut declared_on: Vec<_> = smart_air_brick_floors(input)?.keys().cloned().collect();
    if declared_on.len() > 1 {
        anyhow::bail!(
            "Only one floor of the dwelling may have smart air bricks, but they are declared on {}.",
            {
                declared_on.sort();
                declared_on.join(", ")
            }
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Map, Value};

    /// A dwelling whose single zone holds the given ground floors, plus a wall.
    fn project(ground_floors: Vec<Value>) -> InputForProcessing {
        let mut elements = Map::new();
        for (index, floor) in ground_floors.into_iter().enumerate() {
            elements.insert(format!("ground {}", index + 1), floor);
        }
        elements.insert(
            "wall 0".to_string(),
            json!({"type": "BuildingElementOpaque"}),
        );

        InputForProcessing {
            input: json!({
                "Zone": {
                    "zone 1": {
                        "BuildingElement": elements
                    }
                }
            }),
        }
    }

    fn suspended_floor(overrides: Value) -> Value {
        let mut floor = json!({
            "type": "BuildingElementGround",
            "floor_type": "Suspended_floor",
            "area_per_perimeter_vent": 0.003
        });
        floor
            .as_object_mut()
            .expect("floor fixture is a JSON object")
            .extend(
                overrides
                    .as_object()
                    .expect("floor overrides are a JSON object")
                    .clone(),
            );
        floor
    }

    #[test]
    fn does_not_error_for_one_smart_air_brick_floor() {
        // Given a dwelling whose single suspended floor has smart air bricks
        let project = project(vec![suspended_floor(json!({"smart_air_bricks": true}))]);

        // When validation is called
        assert!(validate_smart_air_brick_floors(project).is_ok());
        // Then no error is raised
    }

    #[test]
    fn does_not_error_without_smart_air_bricks() {
        // Given two suspended floors, neither with smart air bricks
        let project = project(vec![suspended_floor(json!({})), suspended_floor(json!({}))]);

        // When validation is called
        assert!(validate_smart_air_brick_floors(project).is_ok());
        // Then no error is raised, the limit being on smart air bricks and not on floors
    }

    #[test]
    fn error_for_more_than_one_smart_air_brick_floor() {
        // Given two suspended floors both declaring smart air bricks, which the schema accepts
        let project = project(vec![
            suspended_floor(json!({"smart_air_bricks": true})),
            suspended_floor(json!({"smart_air_bricks": true})),
        ]);

        // When validation is called
        let error = validate_smart_air_brick_floors(project).unwrap_err();

        // Then an error is raised naming the floors. The core models one smart air brick system
        // per zone and the schema allows one zone, so this would otherwise be caught part way
        // through a simulation, after both the schema and the core's own model accepted it
        assert_eq!(
            error.to_string(),
            "Only one floor of the dwelling may have smart air bricks, but they are declared on ground 1, ground 2."
        );
    }

    #[test]
    fn flag_ignored_on_a_floor_the_core_would_not_honour() {
        // Given the flag on a solid floor as well as a suspended one, which the schema does not
        // allow but validation should not depend on that
        let project = project(vec![
            suspended_floor(json!({"smart_air_bricks": true})),
            suspended_floor(json!({
                "floor_type": "Slab_no_edge_insulation",
                "smart_air_bricks": true
            })),
        ]);

        // When validation is called
        assert!(validate_smart_air_brick_floors(project).is_ok());

        // Then no error is raised: only the suspended floor counts, matching the conditions the
        // core applies when it honours the control, so an input is never refused over elements
        // the core would ignore
    }
}
