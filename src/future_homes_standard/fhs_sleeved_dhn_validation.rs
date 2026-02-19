use crate::future_homes_standard::input::{json_error, InputForProcessing};
use anyhow::bail;

fn validate_sleeved_dhn(input: InputForProcessing) -> anyhow::Result<()> {
    // Sleeved DHNs must be used for space heating and DHW
    for (heat_source_wet_key, heat_source_wet) in input.heat_source_wet()? {
        let is_heat_network: bool = heat_source_wet
            .get("is_heat_network")
            .ok_or(json_error(
                "Heat source wet did not have an is_heat_network field",
            ))?
            .as_bool()
            .ok_or(json_error("is_heat_network field was not a boolean"))?;

        if is_heat_network {
            let heat_network_type = heat_source_wet
                .get("heat_network_type")
                .ok_or(json_error(
                    "Heat source wet did not have a heat_network_type field",
                ))?
                .as_str()
                .ok_or(json_error("heat_network_type field was not a string"))?;

            if heat_network_type == "sleeved DHN" {
                let mut used_for_space_heating = false;

                for space_heat_system in input.space_heat_system_keys()? {
                    if let Some(heat_source) =
                        input.heat_source_for_space_heat_system(&space_heat_system)?
                    {
                        let name = heat_source
                            .get("name")
                            .ok_or(json_error("Heat source did not have a name field"))?
                            .as_str()
                            .ok_or(json_error("Heat source name field was not a string"))?;

                        if name == heat_source_wet_key {
                            used_for_space_heating = true;

                            break;
                        }
                    }
                }

                let mut used_for_hot_water = false;

                for hot_water_source in input.hot_water_source()?.values() {
                    if let Some(heat_source_map) = hot_water_source
                        .get("HeatSource")
                        .and_then(|v| v.as_object())
                    {
                        for heat_source in heat_source_map.values() {
                            let name = heat_source
                                .get("name")
                                .ok_or(json_error("Heat source did not have a name field"))?
                                .as_str()
                                .ok_or(json_error("heat source name field was not a string"))?;

                            if name == heat_source_wet_key {
                                used_for_hot_water = true;

                                break;
                            }
                        }

                        if used_for_hot_water {
                            break;
                        }
                    }

                    if let Some(_heat_source_wet_map) = hot_water_source
                        .get("HeatSourceWet")
                        .and_then(|v| v.as_object())
                    {
                        todo!()
                    }
                }

                if !(used_for_space_heating && used_for_hot_water) {
                    bail!(format!("HeatSourceWet '{heat_source_wet_key}' is a sleeved DHN which must be used for both space heating and hot water"));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;
    use rstest::*;
    use serde_json::json;

    #[fixture]
    fn dhn_sleeved() -> InputForProcessing {
        let input_json = json!({
            "HeatSourceWet": {
                "sleeved_dhn": {"is_heat_network": true, "heat_network_type": "sleeved DHN"}
            },
            "HotWaterSource": {
                "hw cylinder": {"HeatSource": {"heat network": {"name": "sleeved_dhn"}}}
            },
            "SpaceHeatSystem": {"radiators": {"HeatSource": {"name": "sleeved_dhn"}}}
        });

        InputForProcessing { input: input_json }
    }

    #[rstest]
    fn test_sleeved_dhn_used_for_space_heating_and_dhw(dhn_sleeved: InputForProcessing) {
        // Given an input where a sleeved DHN is used for both HotWater and SpaceHeating
        // When validation is called then no error
        assert!(validate_sleeved_dhn(dhn_sleeved).is_ok());
    }

    #[rstest]
    fn test_sleeved_dhn_used_for_space_heating_only(mut dhn_sleeved: InputForProcessing) {
        // Given an input where a sleeved DHN is used for SpaceHeating only
        dhn_sleeved.input["HotWaterSource"]["hw cylinder"]["HeatSource"]["heat network"]["name"] =
            json!("blah");

        // When validation is called
        let result = validate_sleeved_dhn(dhn_sleeved);

        // Then validation error is raised that the sleeved DHN must be used for both
        assert_eq!(result.unwrap_err().to_string(), "HeatSourceWet 'sleeved_dhn' is a sleeved DHN which must be used for both space heating and hot water");
    }
}
