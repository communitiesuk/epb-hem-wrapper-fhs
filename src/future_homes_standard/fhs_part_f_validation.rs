mod part_f {
    use home_energy_model::input::{InfiltrationVentilation, Vent};
    use indexmap::IndexMap;

    use crate::future_homes_standard::input::InputForProcessing;
    const MIN_KITCHEN_VENT_FLOW_RATE: f64 = 60.0; // the property needs one 60+ l/s fan for cooking events

    pub fn minimum_whole_dwelling_ventilation_rate_continuous(
        total_floor_area: f64,
        bedrooms: u32,
    ) -> f64 {
        todo!();
    }

    pub fn minimum_background_ventilation_area_continuous(habitable_rooms: u32) -> f64 {
        todo!();
    }

    pub fn minimum_whole_dwelling_ventilation_rate_intermittent(
        bathrooms: u32,
        utility_rooms: u32,
        sanitary_accommodations: u32,
    ) -> f64 {
        todo!();
    }

    pub fn minimum_background_ventilation_area_intermittent(
        habitable_rooms: u32,
        bathrooms: u32,
        storeys: u32,
    ) -> f64 {
        todo!();
    }

    pub fn sufficient_whole_dwelling_ventilation_rate_continuous(
        vents: Vec<IndexMap<String, Vent>>,
        total_floor_area: f64,
        bedrooms: u32,
    ) -> bool {
        todo!()
    }

    pub fn sufficient_whole_dwelling_ventilation_rate_intermittent(
        vents: Vec<IndexMap<String, Vent>>,
        bathrooms: u32,
        utility_rooms: u32,
        sanitary_accommodations: u32,
    ) -> bool {
        todo!()
    }

    pub fn sufficient_background_ventilation_area_continuous(
        vents: Vec<IndexMap<String, Vent>>,
        habitable_rooms: u32,
    ) -> bool {
        todo!();
    }

    pub fn sufficient_background_ventilation_area_intermittent(
        vents: Vec<IndexMap<String, Vent>>,
        habitable_rooms: u32,
        bathrooms: u32,
        storeys: u32,
    ) -> bool {
        todo!();
    }

    pub fn sufficient_imev_count(vents: Vec<IndexMap<String, Vent>>, wet_rooms: u32) -> bool {
        todo!();
    }

    pub fn sufficient_large_imev(vents: Vec<IndexMap<String, Vent>>) -> bool {
        todo!();
    }

    pub fn validate_dwelling_ventilation(
        ventilation: InfiltrationVentilation,
        total_floor_area: f64,
        bedrooms: u32,
        habitable_rooms: u32,
        wet_rooms: u32,
        bathrooms: u32,
        utility_rooms: u32,
        sanitary_accommodations: u32,
        storeys: u32,
    ) -> Result<(), String> {
        todo!();
    }

    pub fn minimum_background_ventilation(project_dict: InputForProcessing) -> () {
        // TODO set return type correctly
        todo!();
    }

    pub(crate) fn minumum_background_vent_count_continuous(bedrooms: i32) -> i32 {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use home_energy_model::input::InfiltrationVentilation;

    use crate::future_homes_standard::fhs_part_f_validation::part_f;

    #[test]
    #[ignore = "not yet implemented"]
    fn test_mwdvr_when_bedroom_based_value_is_greater() {
        // TFA criteria = 100 * 0.3 = 30 l/s
        // Bedroom criteria = 13 + 4 * 6 = 37 l/s

        let mwdvr = part_f::minimum_whole_dwelling_ventilation_rate_continuous(100.0, 4);
        assert_eq!(mwdvr, 133.2) // 37 * 3.6 m3/hr
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_mwdvr_when_floor_area_based_value_is_greater() {
        // TFA criteria = 150 * 0.3 = 45 l/s
        // Bedroom criteria = 13 + 4 * 6 = 37 l/s
        let mwdvr = part_f::minimum_whole_dwelling_ventilation_rate_continuous(150.0, 4);
        assert_eq!(mwdvr, 162.0) // 45 * 3.6 m3/hr
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_one_habitable_room() {
        let mbva = part_f::minimum_background_ventilation_area_continuous(1);
        assert_eq!(mbva, 40.0); // 1 * 40 = 40 cm2
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_five_habitable_rooms() {
        let mbva = part_f::minimum_background_ventilation_area_continuous(5);
        assert_eq!(mbva, 200.0); // 5 * 40 = 200 cm2
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_no_bedrooms() {
        let bedrooms = 0;
        let minimum_vent_count = part_f::minumum_background_vent_count_continuous(bedrooms);
        assert_eq!(2, minimum_vent_count);
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_five_bedrooms() {
        let bedrooms = 5;
        let minimum_vent_count = part_f::minumum_background_vent_count_continuous(bedrooms);
        assert_eq!(7, minimum_vent_count);
    }

    // test_does_not_raise_if_sufficient_continuous_MEV_and_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_cmev_and_background_vents() {
        // TODO match newest schema instead
        // note that in Python the JSON examples have less content
        //  because they don't need to passs schema validation prior to this

        let json = r#"{ 
    "Vents": {
        "vent1": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 100,
            "pressure_difference_ref": 20,
            "orientation360": 180,
            "pitch": 60
        },
        "vent2": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 100,
            "pressure_difference_ref": 20,
            "orientation360": 0,
            "pitch": 60
        }
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Centralised continuous MEV",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 133.2,
            "SFP":1.5
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;
        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();

        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    // test_raises_if_sufficient_continuous_MEV_but_insufficient_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_sufficient_cmev_but_insufficient_background_vents() {
        let json = r#"{ 
    "Vents": {},
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Centralised continuous MEV",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 133.2,
            "SFP": 1.5
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains(
                "Dwelling lacks sufficient background ventilator area for continuous ventilation"
            ));
        });
    }

    // TODO test_raises_if_sufficient_continuous_MEV_but_insufficient_background_vent_count

    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_no_mechanical_vents() {
        let json = r#"{ 
    "Vents": {
        "vent1": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 100,
            "pressure_difference_ref": 20,
            "orientation360": 180,
            "pitch": 60
        },
        "vent2": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 100,
            "pressure_difference_ref": 20,
            "orientation360": 0,
            "pitch": 60
        }
    },
    "MechanicalVentilation": {
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks any mechanical vents"));
        });
    }

    // test_raises_if_neither_background_nor_continuous_MEV_ventilation_sufficient
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_neither_background_nor_cmev_ventilation_sufficient() {
        let json = r#"{ 
    "Vents": {
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Centralised continuous MEV",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 50,
            "SFP": 1.5
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        // "design_outdoor_air_flow_rate": 50,  # not enough for 100 floor area/4 beds

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains(
                "Dwelling lacks sufficient background ventilator area for continuous ventilation"
            ));
            assert!(e.contains("Dwelling lacks sufficient continuous mechanical extract rate"));
        });
    }

    // test_does_not_raise_if_sufficient_MVHR_and_no_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    #[ignore = "depends on position_exhaust which is in newer schema version"]
    fn test_does_not_raise_if_sufficient_mvhr_and_no_background_vents() {
        let json = r#"{ 
    "Vents": {
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "MVHR",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 133.2,
            "mvhr_eff": 0.0,
            "mvhr_location": "inside",
            "position_intake": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "position_exhaust": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "ductwork": [
                {
                    "cross_section_shape": "circular",
                    "external_diameter_mm": 160,
                    "insulation_thermal_conductivity": 0.04,
                    "insulation_thickness_mm": 25,
                    "internal_diameter_mm": 150,
                    "length": 5.0,
                    "reflective": False,
                    "duct_type": "supply",
                }
            ],
            "SFP": 1.5,
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "not yet implemented"]
    #[ignore = "depends on position_exhaust which is in newer schema version"]
    fn test_raises_if_insufficient_mvhr() {
        let json = r#"{ 
    "Vents": {
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "MVHR",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 10,
            "mvhr_eff": 0.0,
            "mvhr_location": "inside",
            "position_intake": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "position_exhaust": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "ductwork": [
                {
                    "cross_section_shape": "circular",
                    "external_diameter_mm": 160,
                    "insulation_thermal_conductivity": 0.04,
                    "insulation_thickness_mm": 25,
                    "internal_diameter_mm": 150,
                    "length": 5.0,
                    "reflective": False,
                    "duct_type": "supply",
                }
            ],
            "SFP": 1.5
        }
    }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        // "design_outdoor_air_flow_rate": 50,  # not enough for 100 floor area/4 beds

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient continuous mechanical extract rate."));
        });
    }

    // TODO test_does_not_raise_if_sufficient_MVHR_and_continuous_MEV_and_no_background_vents
    // TODO test_raises_if_sufficient_iMEV_but_insufficient_background_vents

    #[test]
    #[ignore = "not yet implemented"]
    #[ignore = "depends on position_exhaust which is in newer schema version"]
    fn test_raises_if_sufficient_mvhr_and_cmev_but_no_background_vents() {
        let json = r#"{ 
    "Vents": {
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "MVHR",
            "measured_fan_power": 12.26,
            "measured_air_flow_rate": 37,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 100,
            "mvhr_eff": 0.0,
            "mvhr_location": "inside",
            "position_intake": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "position_exhaust": {
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            },
            "ductwork": [
                {
                    "cross_section_shape": "circular",
                    "external_diameter_mm": 160,
                    "insulation_thermal_conductivity": 0.04,
                    "insulation_thickness_mm": 25,
                    "internal_diameter_mm": 150,
                    "length": 5.0,
                    "reflective": False,
                    "duct_type": "supply",
                }
            ],
            "SFP": 1.5
        },
        "mechvent2": {
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
            "SFP: 1.5
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains(
                "Dwelling lacks sufficient background ventilator area for continuous ventilation."
            ));
        });
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_imev_and_background_vents() {
        let json = r#"{ 
    "Vents": {
        "vent1": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 300,
            "pressure_difference_ref": 20,
            "orientation360": 180,
            "pitch": 60
        },
        "vent2": {
            "mid_height_air_flow_path": 1.5,
            "area_cm2": 300,
            "pressure_difference_ref": 20,
            "orientation360": 0,
            "pitch": 60
        }
    },
    "MechanicalVentilation": {
        "mechvent1": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Intermittent MEV",
            "SFP": 1.5,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 240
        },
        "mechvent2": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Intermittent MEV",
            "SFP": 1.5,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 60
        },
        "mechvent3": {
            "sup_air_flw_ctrl": "ODA",
            "sup_air_temp_ctrl": "NO_CTRL",
            "vent_type": "Intermittent MEV",
            "SFP": 1.5,
            "EnergySupply": "mains elec",
            "design_outdoor_air_flow_rate": 60
        }
    },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_sufficient_imev_but_insufficient_background_vents() {
        let json = r#"{
        "Vents": {},
        "MechanicalVentilation": {
            "mechvent1": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Intermittent MEV",
                "SFP": 1.5,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 240
            },
            "mechvent2": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Intermittent MEV",
                "SFP": 1.5,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 60
            },
            "mechvent3": {
                "sup_air_flw_ctrl": "ODA",
                "sup_air_temp_ctrl": "NO_CTRL",
                "vent_type": "Intermittent MEV",
                "SFP": 1.5,
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 60
            }
        },
    "Leaks" : {
        "ventilation_zone_height" : 6,
        "test_pressure": 50,
        "test_result": 1.2,
        "env_area":220
    },
    "CombustionAppliances":{
        "Fireplace":{
            "supply_situation":"room_air",
            "exhaust_situation": "into_separate_duct",
            "fuel_type": "wood",
            "appliance_type": "open_fireplace"
        }
    },
    "cross_vent_possible": true,
    "noise_nuisance" : true,
    "shield_class": "Normal",
    "terrain_class": "OpenField",
    "ventilation_zone_base_height": 2.5,
    "altitude": 30
}"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient background ventilator area for intermittent ventilation."));
        });
    }

    // test_raises_if_sufficient_background_vents_but_insufficient_iMEV
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_sufficient_background_vents_but_insufficient_imev() {
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,  # insufficient flow rate
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient intermittent mechanical extract rate."));
            assert!(e.contains(
                "Dwelling lacks a large enough intermittent mechanical vent for cooking events."
            ));
        });
    }

    // test_raises_if_iMEV_sufficient_but_lacking_a_large_enough_kitchen_non_external_vent
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_imev_sufficient_but_lacking_a_large_enough_kitchen_non_external_vent() {
        // Given a dwelling with sufficient iMEV vents and background vents
        // But no individual fan of sufficient size for a kitchen without ventilation to the
        // outside (216+ m3/hr)

        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent4": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent5": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 215,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 215,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 215,
                },
                "mechvent4": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 215,
                },
                "mechvent5": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 215,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains(
                "Dwelling lacks a large enough intermittent mechanical vent for cooking events."
            ));
        });
    }

    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_lacking_a_large_enough_kitchen_external_vent() {
        // Given a dwelling with sufficient iMEV vents and background vents
        // But no individual fan of sufficient size for a kitchen with ventilation to the
        // outside (108+ m3/hr)

        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent4": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 107,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 107,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 107,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains(
                "Dwelling lacks a large enough intermittent mechanical vent for cooking events."
            ));
        });
    }

    // test_raises_if_neither_background_nor_iMEV_ventilation_sufficient
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_neither_background_nor_imev_ventilation_sufficient() {
        // Given a dwelling that neither has enough background vents nor
        // intermittent mechanical vents

        let json = r#"{
            "Vents": {},
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,  # insufficient flow rate
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient background vent area for intermittent ventilation."));
            assert!(e.contains("Dwelling lacks sufficient intermittent mechanical extract rate."));
            assert!(e.contains("Dwelling lacks a large enough intermittent mechanical vent for cooking events."));
            assert!(e.contains("Dwelling lacks sufficient number of background vents for intermittent ventilation."));
        });
    }

    // test_raises_if_insufficient_number_of_iMEV
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_insufficient_number_of_imev() {
        // Given a dwelling with sufficient background vents but insufficient number of iMEV
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent4": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent5": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 600,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 600,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(
                e.contains("Dwelling lacks sufficient number of intermittent mechanical vents.")
            );
        });
    }

    // test_does_not_raise_if_sufficient_iMEV_and_continuous_MEV_and_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_imev_and_continuous_mev_and_background_vents() {
        // Given a dwelling with sufficient iMEV vents and continuous MEV vents and background vents
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent4": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent5": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 240,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 60,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 60,
                },
                "mechvent4": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Centralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 133.2,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    // test_does_not_raise_if_sufficient_continuous_MEV_but_insufficient_background_vents_for_iMEV
    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_continuous_mev_but_insufficient_background_vents_for_imev()
    {
        // Given a dwelling with sufficient continuous MEV ventilation but
        // insufficient background vents for intermittent ventilation.
        // The dwelling passes because only one route needs to pass.
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 240,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 60,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 60,
                },
                "mechvent4": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Centralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 133.2,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    // test_does_not_raise_if_sufficient_iMEV_and_background_vents_but_insufficient_continuous_MEV
    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_imev_and_background_vents_but_insufficient_continuous_mev()
    {
        // Given a dwelling with sufficient iMEV vents and background vents but insufficient
        // continuous MEV vents. The dwelling passes because only one route needs to pass.
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent4": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent5": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 300,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 600,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 200,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 200,
                },
                "mechvent4": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Centralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,  # insufficient flow rate
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    // test_raises_if_insufficient_iMEV_and_continuous_MEV_and_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_insufficient_imev_and_continuous_mev_and_background_vents() {
        // Given a dwelling with insufficient iMEV vents and continuousMEV vents and background vents
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,  # insufficient area
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                }
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 240,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 60,  # insufficient flow rate
                },
                "mechvent4": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Centralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 10,  # insufficient flow rate
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient background vent area for intermittent ventilation."));
            assert!(e.contains("Dwelling lacks sufficient intermittent mechanical extract rate."));
            assert!(e.contains("Dwelling lacks sufficient number of intermittent mechanical vents."));
            assert!(e.contains("Dwelling lacks sufficient number of background vents for intermittent ventilation."));
            assert!(e.contains("Dwelling lacks sufficient continuous mechanical extract rate."));
            assert!(e.contains("Dwelling lacks sufficient background vent area for continuous ventilation."));
            assert!(e.contains("Dwelling lacks sufficient number of background vents for continuous ventilation."));
        });
    }

    // test_does_not_raise_if_sufficient_decentralised_continuous_MEV_and_background_vents
    #[test]
    #[ignore = "not yet implemented"]
    fn test_does_not_raise_if_sufficient_decentralised_continuous_mev_and_background_vents() {
        // Given a dwelling with sufficient decentralised continuous MEV vents and background vents
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Decentralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 133.2,
                }
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_ok());
    }

    // test_raises_if_sufficient_iMEV_but_insufficient_background_vent_count
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_sufficient_imev_but_insufficient_background_vent_count() {
        // Given a dwelling with sufficient iMEV vents and background ventilator area, but
        // insufficient background ventilator count
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 1000,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                }
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 700,
                },
                "mechvent2": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 200,
                },
                "mechvent3": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Intermittent MEV",
                    "SFP": 1.5,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 200,
                },
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient number of background vents for intermittent ventilation."));
        });
    }

    // test_raises_if_insufficient_decentralised_continuous_MEV_count
    #[test]
    #[ignore = "not yet implemented"]
    fn test_raises_if_insufficient_decentralised_continuous_mev_count() {
        // Given a dwelling with insufficient decentralised continuous MEV vents
        let json = r#"{
            "Vents": {
                "vent1": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 180,
                    "pitch": 60,
                },
                "vent2": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
                "vent3": {
                    "mid_height_air_flow_path": 1.5,
                    "area_cm2": 100,
                    "pressure_difference_ref": 20,
                    "orientation360": 0,
                    "pitch": 60,
                },
            },
            "MechanicalVentilation": {
                "mechvent1": {
                    "sup_air_flw_ctrl": "ODA",
                    "sup_air_temp_ctrl": "NO_CTRL",
                    "vent_type": "Decentralised continuous MEV",
                    "measured_fan_power": 12.26,
                    "measured_air_flow_rate": 37,
                    "EnergySupply": "mains elec",
                    "design_outdoor_air_flow_rate": 133.2,
                }
            },
        }"#;

        let ventilation: InfiltrationVentilation = serde_json::from_str(json).unwrap();
        let result = part_f::validate_dwelling_ventilation(ventilation, 100., 4, 5, 3, 2, 0, 0, 2);

        assert!(result.is_err());
        let _ = result.inspect_err(|e| {
            assert!(e.contains("Dwelling lacks sufficient number of decentralised mechanical vents for continuous."));
        });
    }
}
