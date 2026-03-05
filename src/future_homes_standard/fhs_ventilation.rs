use crate::future_homes_standard::input::{json_error, InputForProcessing};
use anyhow::bail;
use home_energy_model_legacy::core::space_heat_demand::building_element::{
    pitch_class, HeatFlowDirection,
};
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::sync::Arc;

/// Returns an indexmap of mechanical ventilation objects with vent_type
/// "Decentralised continuous MEV", according to the following rules:
///     - Create one dMEV per wet room
///     - Assign dMEVs first to the smallest windows, then to the largest walls if needed
///     - Ensure total summed air flow rate equals the specified minimum
pub(crate) fn create_mechanical_ventilation(
    input: InputForProcessing,
    minimum_air_flow_rate: f64,
) -> anyhow::Result<IndexMap<Arc<str>, Value>> {
    let building_elements = input.all_building_element_values()?;

    // Position vents in smallest windows that aren't rooflights
    let mut windows_excluding_rooflights = Vec::new();

    for window in sorted_windows_by_area(&building_elements)? {
        let pitch = window
            .as_object()
            .and_then(|el| el.get("pitch"))
            .and_then(|pitch| pitch.as_f64())
            .ok_or_else(|| json_error("Pitch missing or invalid"))?;

        if pitch_class(pitch) == HeatFlowDirection::Horizontal {
            windows_excluding_rooflights.push(window);
        }
    }

    // TODO review against the python - number of wet rooms optional but treated as expected here
    let number_of_wet_rooms = input
        .number_of_wet_rooms()?
        .ok_or_else(|| json_error("Expected NumberOfWetRooms to be provided"))?;
    let mut vent_placements: Vec<Value> = windows_excluding_rooflights
        .into_iter()
        .take(number_of_wet_rooms)
        .collect();
    // If needed, position remaining vents in largest walls
    let mut num_remaining_vent_placements = number_of_wet_rooms - vent_placements.len();

    if num_remaining_vent_placements > 0 {
        let mut walls = sorted_walls_by_area(&building_elements)?;

        if walls.is_empty() {
            bail!("Unable to place {num_remaining_vent_placements} remaining vent(s). Dwelling lacks suitable walls.");
        }

        walls.reverse();

        let mut wall_index = 0;
        // keep looping through the walls until all vents are placed
        while num_remaining_vent_placements > 0 {
            vent_placements.push(walls[wall_index].clone());
            num_remaining_vent_placements -= 1;
            wall_index = (wall_index + 1) % walls.len();
        }
    }

    let ventilation_zone_base_height = input.ventilation_zone_base_height()?;
    let airflow_rate_per_vent = minimum_air_flow_rate / number_of_wet_rooms as f64;

    let mut dmevs = IndexMap::new();
    for (i, vent_placement) in vent_placements.iter().enumerate() {
        let vent_mid_height_airflow_path =
            calc_vent_mid_height_airflow_path(ventilation_zone_base_height, vent_placement)?;
        let (orientation, pitch) = vent_placement
            .as_object()
            .and_then(|el| {
                let orientation = el.get("orientation360")?.as_f64()?;
                let pitch = el.get("pitch")?.as_f64()?;

                Some((orientation, pitch))
            })
            .ok_or_else(|| json_error("Building element fields missing or invalid"))?;

        let dmev_key = format!("Decentralised_Continuous_MEV_{i}");
        let dmev_value = create_dmev(
            airflow_rate_per_vent,
            vent_mid_height_airflow_path,
            orientation,
            pitch,
        );
        dmevs.insert(dmev_key.into(), dmev_value);
    }

    Ok(dmevs) // todo return json value?
}

fn create_dmev(
    design_outdoor_air_flow_rate: f64,
    mid_height_air_flow_path: f64,
    orientation360: f64,
    pitch: f64,
) -> Value {
    let default_dmev_sfp = 0.15;

    json!({
        "sup_air_flw_ctrl": "ODA",
        "sup_air_temp_ctrl": "NO_CTRL",
        "vent_type": "Decentralised continuous MEV",
        "SFP": default_dmev_sfp,
        "EnergySupply": "mains elec",
        "design_outdoor_air_flow_rate": design_outdoor_air_flow_rate,
        "mid_height_air_flow_path": mid_height_air_flow_path,
        "orientation360": orientation360,
        "pitch": pitch,
    })
}

fn calc_vent_mid_height_airflow_path(
    ventilation_zone_base_height: f64,
    building_element: &Value,
) -> anyhow::Result<f64> {
    let (el_type, base_height, height, pitch) = building_element
        .as_object()
        .and_then(|el| {
            let el_type = el.get("type")?.as_str()?;
            let base_height = el.get("base_height")?.as_f64()?;
            let height = el.get("height")?.as_f64()?;
            let pitch = el.get("pitch")?.as_f64()?;

            Some((el_type, base_height, height, pitch))
        })
        .ok_or_else(|| json_error("Building element fields missing or invalid"))?;

    Ok(if el_type == "BuildingElementOpaque" {
        base_height + (height * pitch.to_radians().sin() / 2.) - ventilation_zone_base_height
    } else {
        base_height + height * pitch.to_radians().sin() - ventilation_zone_base_height
    })
}

fn sorted_windows_by_area(building_elements: &[&Value]) -> anyhow::Result<Vec<Value>> {
    let mut windows = Vec::new();

    for building_element in building_elements {
        let el_type = building_element
            .as_object()
            .ok_or_else(|| json_error("Building element was not an object"))?
            .get("type")
            .and_then(|el_type| el_type.as_str())
            .ok_or_else(|| json_error("Building element type missing or not a string"))?;

        if el_type == "BuildingElementTransparent" {
            let width = building_element
                .get("width")
                .and_then(|width| width.as_f64())
                .ok_or_else(|| json_error("Building element width missing or invalid"))?;
            let height = building_element
                .get("height")
                .and_then(|height| height.as_f64())
                .ok_or_else(|| json_error("Building element height missing or invalid"))?;
            windows.push((width * height, (*building_element).clone()));
        }
    }

    windows.sort_by(|a, b| a.0.total_cmp(&b.0));

    Ok(windows.into_iter().map(|(_, element)| element).collect())
}

fn sorted_walls_by_area(building_elements: &[&Value]) -> anyhow::Result<Vec<Value>> {
    let mut walls = Vec::new();

    for building_element in building_elements {
        let element = building_element
            .as_object()
            .ok_or_else(|| json_error("Building element was not an object"))?;
        let el_type = element
            .get("type")
            .and_then(|el_type| el_type.as_str())
            .ok_or_else(|| json_error("Building element type missing or not a string"))?;

        if el_type == "BuildingElementOpaque" {
            let pitch = element
                .get("pitch")
                .and_then(|pitch| pitch.as_f64())
                .ok_or_else(|| json_error("Building element pitch missing or invalid"))?;

            if pitch_class(pitch) == HeatFlowDirection::Horizontal {
                let is_external_door = element
                    .get("is_external_door")
                    .and_then(|v| v.as_bool())
                    .ok_or_else(|| {
                        json_error("Building element is_external_door missing or invalid")
                    })?;
                let area = element
                    .get("area")
                    .and_then(|area| area.as_f64())
                    .ok_or_else(|| json_error("Building element area missing or invalid"))?;

                if !is_external_door {
                    walls.push((area, (*building_element).clone()));
                }
            }
        }
    }
    walls.sort_by(|a, b| a.0.total_cmp(&b.0));

    Ok(walls.into_iter().map(|(_, element)| element).collect())
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::*;
    use serde_json::json;

    #[fixture]
    fn input() -> InputForProcessing {
        let input_json = json!({
            "NumberOfWetRooms": 2,
            "InfiltrationVentilation": {
                "ventilation_zone_base_height": 0,
                "MechanicalVentilation": {
                    "mechvent1": {
                        "sup_air_flw_ctrl": "ODA",
                        "sup_air_temp_ctrl": "NO_CTRL",
                        "vent_type": "Centralised continuous MEV",
                        "measured_fan_power": 12.26,
                        "measured_air_flow_rate": 37,
                        "EnergySupply": "mains elec",
                        "design_outdoor_air_flow_rate": 80,
                        "mid_height_air_flow_path": 1.5,
                        "orientation360": 90,
                        "pitch": 60,
                    }
                },
            },
            "Zone": {
                "whole dwelling": {
                    "BuildingElement": {
                        "window 1": {
                            "type": "BuildingElementTransparent",
                            "thermal_resistance_construction": 0.4,
                            "pitch": 90,
                            "orientation360": 270,
                            "g_value": 0.75,
                            "frame_area_fraction": 0.25,
                            "base_height": 1,
                            "height": 1.25,
                            "width": 4,
                            "free_area_height": 1.6,
                            "mid_height": 1.5,
                            "max_window_open_area": 3,
                            "security_risk": true,
                            "window_part_list": [{"mid_height_air_flow_path": 1.5}],
                            "shading": [
                                {"type": "overhang", "depth": 0.5, "distance": 0.5},
                                {"type": "sidefinleft", "depth": 0.25, "distance": 0.1},
                                {"type": "sidefinright", "depth": 0.25, "distance": 0.1},
                            ],
                        },
                        "window 2": {
                            "type": "BuildingElementTransparent",
                            "thermal_resistance_construction": 0.4,
                            "pitch": 90,
                            "orientation360": 90,
                            "g_value": 0.75,
                            "frame_area_fraction": 0.25,
                            "base_height": 1,
                            "height": 1.25,
                            "width": 3,
                            "free_area_height": 1.6,
                            "mid_height": 1.5,
                            "max_window_open_area": 3,
                            "security_risk": true,
                            "window_part_list": [{"mid_height_air_flow_path": 1.5}],
                            "shading": [
                                {"type": "overhang", "depth": 0.5, "distance": 0.5},
                                {"type": "sidefinleft", "depth": 0.25, "distance": 0.1},
                                {"type": "sidefinright", "depth": 0.25, "distance": 0.1},
                            ],
                        },
                        "wall 0": {
                            "type": "BuildingElementOpaque",
                            "thermal_resistance_construction": 0.71,
                            "areal_heat_capacity": "Very light",
                            "mass_distribution_class": "M: Mass concentrated inside",
                            "pitch": 90,
                            "is_external_door": false,
                            "orientation360": 270,
                            "base_height": 0,
                            "height": 2.5,
                            "width": 10,
                            "area": 25.0,
                        },
                        "wall 1": {
                            "type": "BuildingElementOpaque",
                            "colour": "Intermediate",
                            "thermal_resistance_construction": 0.72,
                            "areal_heat_capacity": "Very light",
                            "mass_distribution_class": "E: Mass concentrated at external side",
                            "pitch": 90,
                            "is_external_door": false,
                            "orientation360": 0,
                            "base_height": 0,
                            "height": 2.5,
                            "width": 8,
                            "area": 20.0,
                        },
                    }
                }
            },
        });

        InputForProcessing { input: input_json }
    }

    #[rstest]
    fn test_two_vents_assigned_to_windows(input: InputForProcessing) {
        // Given an input with a dwelling with two wet rooms, two windows
        // and a part f minimum air flow rate of 100
        let minimum_air_flow_rate = 100.;

        // When the new mechanical vents are created
        let results = create_mechanical_ventilation(input, minimum_air_flow_rate).unwrap();

        // Then there are two dMEVs created: one for each wet room
        // the total air flow rate of the dMEVs is equal to the part f minimum
        // the positions of the two vents are correctly taken from the windows
        let expected = json!({
            "Decentralised_Continuous_MEV_0": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Decentralised continuous MEV",
                "SFP": 0.15,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 50.,
                "mid_height_air_flow_path": 2.25,
                "orientation360": 90.,
                "pitch": 90.,
            },
            "Decentralised_Continuous_MEV_1": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Decentralised continuous MEV",
                "SFP": 0.15,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 50.,
                "mid_height_air_flow_path": 2.25,
                "orientation360": 270.,
                "pitch": 90.,
            },
        });

        assert_eq!(json!(results), expected);
    }

    #[rstest]
    fn test_two_vents_assigned_to_window_and_wall(mut input: InputForProcessing) {
        // Given an input with a dwelling with two wet rooms, one window, two walls
        // and a part f minimum air flow rate of 100
        input.input["Zone"]["whole dwelling"]["BuildingElement"]
            .as_object_mut()
            .unwrap()
            .remove("window 1");
        let minimum_air_flow_rate = 100.;

        // When the mechanical vents are created
        let results = create_mechanical_ventilation(input, minimum_air_flow_rate).unwrap();

        // Then there are two dMEVs created: one for each wet room
        // the total air flow rate of the dMEVs is equal to the part f minimum
        // the positions of the two vents are correctly taken from the window
        // and the largest wall
        let expected = json!({
            "Decentralised_Continuous_MEV_0": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Decentralised continuous MEV",
                "SFP": 0.15,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 50.0,
                "mid_height_air_flow_path": 2.25,
                "orientation360": 90.,
                "pitch": 90.,
            },
            "Decentralised_Continuous_MEV_1": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Decentralised continuous MEV",
                "SFP": 0.15,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 50.0,
                "mid_height_air_flow_path": 1.25,
                "orientation360": 270.,
                "pitch": 90.,
            },
        });

        assert_eq!(json!(results), expected);
    }
}
