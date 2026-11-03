use jsonschema::error::ValidationErrorKind;
use jsonschema::{validator, Validator};
use serde_json::Value;
use std::fmt;
use std::sync::LazyLock;
use thiserror::Error;

#[validator(path = "./src/future_homes_standard/schema.json", methods = { validate = false, iter_errors = false })]
struct FhsSchema;

static FHS_SCHEMA_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let schema = serde_json::from_str(include_str!("schema.json")).unwrap();
    jsonschema::validator_for(&schema).unwrap()
});

#[derive(Debug, Error)]
#[error("FHS input validation failed: {errors}")]
pub struct SchemaValidationError {
    pub errors: String,
}

pub(crate) fn apply_schema_validation(input: &Value) -> Result<(), SchemaValidationError> {
    // use precompiled schema validator to check high-level validity as speed needed here
    // fall back to static validator on failure (precompiled validator does not support evaluate() at current time (jsonchema 0.58.3))
    if FhsSchema::is_valid(input) {
        return Ok(());
    }

    let validation_errors = collect_errors(input);

    // Drop parent's unevaluated properties errors explained by child's unevaluated properties error
    let reportable_errors: Vec<&ValidationError> = validation_errors
        .iter()
        .filter(|error| !error.is_unreliable_unevaluated_properties(&*validation_errors))
        .collect();

    // Suppression only ever removes errors explained by another, so it cannot empty a
    // non-empty set. Fall back to the full set regardless: an input must never pass
    // validation because its errors were filtered out.
    let reportable_errors = if reportable_errors.is_empty() {
        validation_errors.iter().collect()
    } else {
        reportable_errors
    };

    Err(SchemaValidationError {
        errors: reportable_errors
            .iter()
            .map(|error| error.to_string())
            .collect::<Vec<_>>()
            .join("\n"),
    })
}

/// A single validation error: where it occurred (JSON pointer) and what went wrong.
struct ValidationError {
    path: String,
    message: String,
    is_unevaluated_properties_error: bool,
}

impl ValidationError {
    fn is_below(&self, ancestor_path: &str) -> bool {
        self.path
            .strip_prefix(ancestor_path)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// Report whether an unevaluated-properties error is collateral from a deeper failure.
    ///
    /// An object's unevaluatedProperties check relies on annotations produced by the
    /// subschemas that matched it - here the type-specific branches selected by if/then.
    /// A subschema that fails produces no annotations, so every property it would have
    /// accounted for is reported as unexpected. The resulting message lists the object's
    /// whole contents and buries the error that actually caused it.
    ///
    /// A failure strictly below the object explains the cascade, so the object's own
    /// listing is dropped in favour of the deeper error. Where the object also carries a
    /// genuinely unrecognised property, dropping the listing defers the report of it
    /// rather than losing it: the input cannot pass while the deeper failure stands, and
    /// the property is named once that failure is fixed.

    /// A failure reported against the object itself is ambiguous. It may equally have
    /// caused the cascade, but the same error set also arises when a property really is
    /// unrecognised alongside an unrelated one - a shower that changes type and keeps its
    /// now-invalid flowrate, say. Those errors are therefore kept, at the cost of leaving
    /// some noisy messages, rather than risk discarding the only report of a bad property.
    ///
    /// Args:
    ///    error(self in Rust): The candidate validation error.
    ///    errors: All validation errors raised for the input.
    ///
    /// Returns:
    ///    True if the error is collateral damage and should not be reported.
    fn is_unreliable_unevaluated_properties(&self, errors: &[ValidationError]) -> bool {
        self.is_unevaluated_properties_error
            && errors.iter().any(|other| other.is_below(&self.path))
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message.replace('"', "'"))
    }
}

fn collect_errors(input: &Value) -> Vec<ValidationError> {
    FHS_SCHEMA_VALIDATOR
        .iter_errors(input)
        .map(|e| ValidationError {
            path: e.instance_path().to_string(),
            message: e.to_string(),
            is_unevaluated_properties_error: matches!(
                e.kind(),
                ValidationErrorKind::UnevaluatedProperties { .. }
            ),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde_json::json;
    fn validation_error(
        path: &str,
        message: &str,
        is_unevaluated_properties_error: bool,
    ) -> ValidationError {
        ValidationError {
            path: path.into(),
            message: message.into(),
            is_unevaluated_properties_error,
        }
    }

    #[test]
    // unit test for the Rust only
    fn test_is_unreliable_unevaluated_properties_for_multiple_child_errors() {
        let errors = vec![
            validation_error("/Appliances", "", true),
            validation_error("/Appliances/Fridge-Freezer", "", false),
            validation_error("/Appliances/Fridge", "", false),
        ];
        assert_eq!(
            errors[0].is_unreliable_unevaluated_properties(&errors),
            true
        );
        assert_eq!(
            errors[1].is_unreliable_unevaluated_properties(&errors),
            false
        );
        assert_eq!(
            errors[2].is_unreliable_unevaluated_properties(&errors),
            false
        );
    }

    #[test]
    // unit test for the Rust only
    fn test_is_unreliable_unevaluated_properties_returns_false_given_no_child_errors() {
        let errors = vec![validation_error("/Appliances", "", true)];
        assert_eq!(
            errors[0].is_unreliable_unevaluated_properties(&errors),
            false
        );
    }

    #[fixture]
    fn project() -> Value {
        serde_json::from_str(include_str!(
            "./test_assets/fixtures/minimal_FHS_input.json"
        ))
        .unwrap()
    }

    fn contains_expected_schema_validation_errors_exactly(
        result: Result<(), SchemaValidationError>,
        expected_error_messages: &mut [&str],
    ) {
        if let Err(SchemaValidationError { errors }) = result {
            let mut actual_error_messages = errors.split("\n").collect::<Vec<_>>();
            expected_error_messages.sort();
            actual_error_messages.sort();
            for (expected, actual) in expected_error_messages.iter().zip(actual_error_messages) {
                assert_eq!(actual, *expected);
            }
        } else {
            panic!("Expected validation error, got {:?}", result);
        }
    }

    fn contains_expected_schema_validation_errors_partially(
        result: Result<(), SchemaValidationError>,
        expected_error_messages: &[&str],
    ) {
        if let Err(SchemaValidationError { errors }) = result {
            for expected_error_message in expected_error_messages {
                assert!(
                    errors.contains(expected_error_message),
                    "Expected message:\n{}\nto be found within:\n{}",
                    *expected_error_message,
                    errors
                );
            }
        } else {
            panic!("Expected validation error, got {:?}", result);
        }
    }

    fn remove_key(base: &mut Value, key: &str) {
        base.as_object_mut().unwrap().remove(key);
    }

    fn insert_key_value(base: &mut Value, key: &str, value: Value) {
        base.as_object_mut().unwrap().insert(key.into(), value);
    }

    #[rstest]
    fn test_microwave_and_kettle_accepted_in_appliances(mut project: Value) {
        // Microwave and Kettle are supported by the runtime and may be
        // declared explicitly in the input alongside the other appliances.
        project["Appliances"]["Microwave"] = json!("Default");
        project["Appliances"]["Kettle"] = json!("Not Installed");
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_otherdevices_rejected_in_appliances(mut project: Value) {
        // Otherdevices is injected as a default during preprocessing but must
        // not be nameable in the input: it is an unavoidable background load
        // that an assessment cannot be allowed to pin or opt out of.
        project["Appliances"]["Otherdevices"] = json!("Default");

        let result = apply_schema_validation(&project);
        let expected_message =
            "/Appliances: Unevaluated properties are not allowed ('Otherdevices' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    fn test_appliance_name_containing_an_appliance_is_rejected(mut project: Value) {
        // The appliance pattern is anchored, so a name that merely contains a
        // recognised appliance name is not accepted and cannot reach
        // preprocessing as an unrecognised appliance.
        project["Appliances"]["Ovenette"] = json!("Default");

        let result = apply_schema_validation(&project);
        let expected_message =
            "/Appliances: Unevaluated properties are not allowed ('Ovenette' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    // test name in Python is test_string_instead_of_number_raises
    fn test_string_instead_of_number_errors(mut project: Value) {
        project["NumberOfBedrooms"] = json!("2");

        let result = apply_schema_validation(&project);
        let expected_message = "/NumberOfBedrooms: '2' is not of type 'integer'";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    // test name in Python is test_value_instead_of_array_raises
    fn test_value_instead_of_array_errors(mut project: Value) {
        project["ExternalConditions"]["shading_segments"] = json!({"start360": 0, "end360": 45});
        let result = apply_schema_validation(&project);
        let expected_message = "/ExternalConditions/shading_segments: {'start360':0,'end360':45} is not of type 'array'";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    // test name in Python is test_insufficient_array_length_raises
    fn test_insufficient_array_length_errors(mut project: Value) {
        project["ExternalConditions"]["shading_segments"] = json!([
            {"start360": 0, "end360": 45}
        ]);
        let result = apply_schema_validation(&project);
        let expected_message = "/ExternalConditions/shading_segments: [{'start360':0,'end360':45}] has less than 8 items";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    fn test_multiple_problems_all_included_in_message(mut project: Value) {
        project["NumberOfBedrooms"] = json!("2");
        project["ExternalConditions"]["shading_segments"] = json!(
            {"number": 1, "start360": 0, "end360": 45}
        );
        let result = apply_schema_validation(&project);
        let expected_messages = &mut [
            "/NumberOfBedrooms: '2' is not of type 'integer'",
            "/ExternalConditions/shading_segments: {'number':1,'start360':0,'end360':45} is not of type 'array'"];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_incorrect_enum_value_errors(mut project: Value) {
        project["HeatingControlType"] = json!("something invalid");
        let result = apply_schema_validation(&project);
        let expected_messages = &mut [
            "/HeatingControlType: 'something invalid' is not one of 'SeparateTempControl' or 'SeparateTimeAndTempControl'"
        ];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_missing_required_value_errors(mut project: Value) {
        remove_key(&mut project["ExternalConditions"], "shading_segments");
        let result = apply_schema_validation(&project);
        let expected_messages =
            &mut ["/ExternalConditions: 'shading_segments' is a required property"];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_missing_one_of_errors(mut project: Value) {
        remove_key(&mut project["ColdWaterSource"], "header tank");
        let result = apply_schema_validation(&project);
        let expected_messages =
            &mut ["/ColdWaterSource: {} is not valid under any of the schemas listed in the 'oneOf' keyword"];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_both_one_of_errors(mut project: Value) {
        // Given a BuildingElement with both u_value and
        // thermal_resistance_contruction provided as inputs, where only "oneOf" is
        // permitted by the schema
        project["Zone"]["zone 1"]["BuildingElement"]["window 0"]["u_value"] = json!(1.2);
        project["Zone"]["zone 1"]["BuildingElement"]["window 0"]
            ["thermal_resistance_construction"] = json!(0.4);
        let result = apply_schema_validation(&project);
        let expected_messages = &mut [
            "/Zone/zone 1/BuildingElement/window 0: {'type':'BuildingElementTransparent','thermal_resistance_construction':0.4,'pitch':90,'orientation360':90,'g_value':0.75,'frame_area_fraction':0.25,'base_height':1,'height':1.25,'width':4,'security_risk':true,'window_part_list':[{'free_area_height':1.6,'mid_height':1.5,'max_window_open_area':3}],'shading':[{'type':'overhang','depth':0.5,'distance':0.5},{'type':'sidefinleft','depth':0.25,'distance':0.1},{'type':'sidefinright','depth':0.25,'distance':0.1}],'treatment':[{'type':'blinds','controls':'manual','delta_r':0.05,'trans_red':0.5}],'u_value':1.2} is valid under more than one of the schemas listed in the 'oneOf' keyword"
        ];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_thermal_bridging_number_form_rejected(mut project: Value) {
        // The absolute-loss (bare number) form of ThermalBridging is no longer
        // accepted; only the per-junction object form is valid.
        project["Zone"]["zone 1"]["ThermalBridging"] = json!(0.5);
        let result = apply_schema_validation(&project);
        let expected_messages = &mut ["/Zone/zone 1/ThermalBridging: 0.5 is not of type 'object'"];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    fn smart_hot_water_tank_preheated_source() -> Value {
        // A SmartHotWaterTank pre-heated source fed by the header tank. temp_setpnt_max
        // is not a user input (the wrapper injects it), so it is absent here.
        json!({
            "preheated tank": {
                "type": "SmartHotWaterTank",
                "volume": 80.0,
                "daily_losses": 1.68,
                "init_temp": 35.0,
                "ColdWaterSource": "header tank",
                "EnergySupply_pump": "mains elec",
                "power_pump_kW": 5.0,
                "max_flow_rate_pump_l_per_min": 1000.0,
                "temp_usable": 40.0,
                "HeatSource": {
                    "immersion": {
                        "type": "ImmersionHeater",
                        "power": 3.0,
                        "EnergySupply": "mains elec",
                        "heater_position": 0.1,
                        }
                },
            }
        })
    }

    #[rstest]
    fn test_preheated_smart_hot_water_tank_validates(mut project: Value) {
        // A SmartHotWaterTank is now accepted as the pre-heated water source.
        project["PreHeatedWaterSource"] = smart_hot_water_tank_preheated_source();
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_preheated_smart_hot_water_tank_missing_pump_field_rejected(mut project: Value) {
        // The SmartHotWaterTank constraints are applied via the type discriminator, so a
        // smart pre-heated tank missing a required pump field fails validation.
        let mut preheated_source = smart_hot_water_tank_preheated_source();
        remove_key(&mut preheated_source["preheated tank"], "temp_usable");
        project["PreHeatedWaterSource"] = preheated_source;
        assert!(apply_schema_validation(&project).is_err());
    }

    #[rstest]
    fn test_preheated_tank_unexpected_property_rejected(mut project: Value) {
        // The pre-heated tank object rejects unexpected keys (unevaluatedProperties: false),
        // consistent with the hot water cylinder.
        let mut preheated_source = smart_hot_water_tank_preheated_source();
        preheated_source["preheated tank"]["not_a_real_field"] = json!(1);
        project["PreHeatedWaterSource"] = preheated_source;
        let result = apply_schema_validation(&project);
        contains_expected_schema_validation_errors_exactly(
            result,
            &mut ["/PreHeatedWaterSource/preheated tank: Unevaluated properties are not allowed ('not_a_real_field' was unexpected)"],
        );
    }

    #[rstest]
    fn test_if_then_conditions(mut project: Value) {
        // If a shower of type MixerShower then it must have a flow_rate property
        remove_key(
            &mut project["HotWaterDemand"]["Shower"]["mixer"],
            "flowrate",
        );
        let result = apply_schema_validation(&project);
        let expected_message = "/HotWaterDemand/Shower/mixer: 'flowrate' is a required property";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    fn test_if_then_conditions_negation(mut project: Value) {
        // If a shower is not of type MixerShower then it must not have a flow_rate property
        let mixer = &mut project["HotWaterDemand"]["Shower"]["mixer"];
        mixer["type"] = json!("InstantElecShower");
        remove_key(mixer, "allow_low_flowrate");
        let result = apply_schema_validation(&project);
        let expected_message = "/HotWaterDemand/Shower/mixer: Unevaluated properties are not allowed ('flowrate' was unexpected)";
        contains_expected_schema_validation_errors_partially(result, &[expected_message]);
    }

    #[rstest]
    fn test_flowrate_min_based_on_allow_low_flowrate_false(mut project: Value) {
        // Given a MixerShower with allow_low_flow_rate set to False
        project["HotWaterDemand"]["Shower"]["mixer"]["allow_low_flowrate"] = json!(false);
        // And a flowrate below the standard mininum of 8 l/min
        project["HotWaterDemand"]["Shower"]["mixer"]["flowrate"] = json!(5.0);
        let result = apply_schema_validation(&project);
        let expected_message =
            "/HotWaterDemand/Shower/mixer/flowrate: 5.0 is less than the minimum of 8";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message])
    }

    #[rstest]
    fn test_if_else_conditions(mut project: Value) {
        // If a shower is not of type MixerShower then it must have a `rated_power` property
        project["HotWaterDemand"]["Shower"]["other"] = json!({
            "type": "InstantElecShower",
            "EnergySupply": "something",
            "ColdWaterSource": "header tank",
        });
        let result = apply_schema_validation(&project);
        let expected_message = "/HotWaterDemand/Shower/other: 'rated_power' is a required property";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message])
    }

    #[rstest]
    fn test_all_of_conditions(mut project: Value) {
        // Within SpaceHeatSystem all systems much meet the requirements for their specific type
        // InstantElecHeater with a missing EnergySupply property
        project["SpaceHeatSystem"]["main 1"] = json!({
            "type": "InstantElecHeater",
            "rated_power": 6.0,
            "convective_type": "Wall heating, radiant ceiling panels, accumulation stoves",
        });
        // WetDistribution with missing dry_core_min_output and dry_core_max_output properties
        project["SpaceHeatSystem"]["main 2"] = json!({
            "type": "ElecStorageHeater",
            "pwr_in": 12,
            "rated_power_instant": 12,
            "storage_capacity": 12,
            "air_flow_type": "fan-assisted",
            "frac_convective": 12,
            "fan_pwr": 12,
            "n_units": 12,
            "EnergySupply": "thing",
            "Zone": "thing",
        });
        let result = apply_schema_validation(&project);
        let expected_messages = &mut [
            "/SpaceHeatSystem/main 1: Unevaluated properties are not allowed ('rated_power', 'convective_type' were unexpected)",
            "/SpaceHeatSystem/main 1: 'EnergySupply' is a required property",
            "/SpaceHeatSystem/main 2: Unevaluated properties are not allowed ('pwr_in', 'rated_power_instant', 'storage_capacity', 'air_flow_type', 'frac_convective', 'fan_pwr', 'n_units', 'EnergySupply', 'Zone' were unexpected)",
            "/SpaceHeatSystem/main 2: 'dry_core_min_output' is a required property",
            "/SpaceHeatSystem/main 2: 'dry_core_max_output' is a required property",
        ];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    #[rstest]
    fn test_any_of_conditions(mut project: Value) {
        // BuildingElementOpaque must have either `u_value` or `thermal_resistance_construction`
        remove_key(
            &mut project["Zone"]["zone 1"]["BuildingElement"]["roof 0"],
            "thermal_resistance_construction",
        );
        let result = apply_schema_validation(&project);
        let expected_message =
            "/Zone/zone 1/BuildingElement/roof 0: {'type':'BuildingElementOpaque','is_unheated_pitched_roof':true,'colour':'Intermediate','area':20.0,'areal_heat_capacity':'Very light','mass_distribution_class':'IE: Mass divided over internal and external side','pitch':45,'orientation360':90,'base_height':2.5,'height':2.5,'width':10} is not valid under any of the schemas listed in the 'oneOf' keyword";
        contains_expected_schema_validation_errors_exactly(result, &mut [expected_message]);
    }

    #[rstest]
    fn test_conditionally_required(mut project: Value) {
        // Given a unfilled_sealed party wall without party_wall_lining_type
        remove_key(
            &mut project["Zone"]["zone 1"]["BuildingElement"]["wall 0"],
            "party_wall_lining_type",
        );
        let result = apply_schema_validation(&project);
        let expected_message = &mut [
            "/Zone/zone 1/BuildingElement/wall 0: 'party_wall_lining_type' is a required property",
            "/Zone/zone 1/BuildingElement/wall 0: Unevaluated properties are not allowed ('areal_heat_capacity', 'mass_distribution_class', 'pitch', 'area', 'party_wall_cavity_type', 'u_value_whole_wall' were unexpected)",
        ];
        contains_expected_schema_validation_errors_exactly(result, expected_message);
    }

    #[rstest]
    fn test_nested_conditions(mut project: Value) {
        // SpaceHeatSystem->type=WetDistribution->emitters->wet_emitter_type=radiator
        // has different required properties to
        // `SpaceHeatSystem->type=WetDistribution->emitters->wet_emitter_type=ufh
        // Radiator wet emitter expects `n` as a property
        project["SpaceHeatSystem"]["main 1"] = json!({
            "type": "WetDistribution",
            "temp_diff_emit_dsgn": 12,
            "variable_flow": true,
            "HeatSource": {"name": "something"},
            "ecodesign_controller": {"ecodesign_control_class": 1},
            "design_flow_temp": 20,
            "Zone": "zone 1",
            "min_flow_rate": 12,
            "max_flow_rate": 12,
            "thermal_mass": 12,
            "emitters": [{"wet_emitter_type": "radiator", "frac_convective": 12, "c": 1}],
        });
        // Whereas ufh emitter expects `emitter_floor_area` as a property
        project["SpaceHeatSystem"]["main 2"] = json!({
            "type": "WetDistribution",
            "temp_diff_emit_dsgn": 12,
            "variable_flow": true,
            "HeatSource": {"name": "something"},
            "ecodesign_controller": {"ecodesign_control_class": 1},
            "design_flow_temp": 20,
            "Zone": "zone 1",
            "min_flow_rate": 12,
            "max_flow_rate": 12,
            "thermal_mass": 12,
            "emitters": [
                {
                    "wet_emitter_type": "ufh",
                    "frac_convective": 12,
                    "equivalent_specific_thermal_mass": 12,
                    "system_performance_factor": 12,
                }
            ],
        });
        let result = apply_schema_validation(&project);
        let expected_messages = &mut [
            "/SpaceHeatSystem/main 1/emitters/0: Unevaluated properties are not allowed ('c' was unexpected)",
            "/SpaceHeatSystem/main 1/emitters/0: 'n' is a required property",
            "/SpaceHeatSystem/main 2/emitters/0: Unevaluated properties are not allowed ('equivalent_specific_thermal_mass', 'system_performance_factor' were unexpected)",
            "/SpaceHeatSystem/main 2/emitters/0: 'emitter_floor_area' is a required property"
        ];
        contains_expected_schema_validation_errors_exactly(result, expected_messages);
    }

    fn wet_distribution(emitters: Value, thermal_mass: Option<f64>) -> Value {
        let mut wet_distribution = json!({
            "type": "WetDistribution",
            "temp_diff_emit_dsgn": 10,
            "variable_flow": false,
            "design_flow_rate": 0.5,
            "HeatSource": {"name": "heat_source"},
            "ecodesign_controller": {"ecodesign_control_class": 1},
            "design_flow_temp": 45,
            "Zone": "zone 1",
            "emitters": emitters
        });
        if let Some(thermal_mass) = thermal_mass {
            wet_distribution["thermal_mass"] = json!(thermal_mass);
        }
        wet_distribution
    }
    #[rstest]
    fn test_wet_distribution_fancoil_thermal_mass_rejected(mut project: Value) {
        // thermal_mass is not a valid field on a fan coil WetDistribution system
        let emitters = json!([{
            "wet_emitter_type": "fancoil",
            "frac_convective": 1.0,
            "n_units": 1,
            "fancoil_test_data": {
                "fan_power_W": [15],
                "fan_speed_data": [{"temperature_diff": 50.0, "power_output": [2.0]}],
            },
        }]);
        let thermal_mass = Some(1.0);
        project["SpaceHeatSystem"]["main 1"] = wet_distribution(emitters, thermal_mass);
        let result = apply_schema_validation(&project);
        // Note that the Python assertions are different because the jsonschema library
        // seems to report the false-schema error against the parent object.
        // Specifically, it seems to return a false subschema’s error before adding
        // the property to the path. The Rust crate puts the property in the path,
        // so the error is strictly below the parent and the parent error is dropped
        // by the filters.
        let error_messages =
            &mut ["/SpaceHeatSystem/main 1/thermal_mass: False schema does not allow 1.0"];
        contains_expected_schema_validation_errors_exactly(result, error_messages);
    }

    #[rstest]
    fn test_wet_distribution_requires_system_thermal_mass_when_any_radiator_lacks_own(
        mut project: Value,
    ) {
        // A system with multiple radiators where at least one has no per-emitter thermal_mass
        // requires system-level thermal_mass, even if other radiators specify their own
        let emitters = json!([
            {
                "wet_emitter_type": "radiator",
                "frac_convective": 0.5,
                "n": 1.34,
                "c": 0.05,
                "thermal_mass": 0.014,
            },
            {"wet_emitter_type": "radiator", "frac_convective": 0.5, "n": 1.34, "c": 0.05},
        ]);
        project["SpaceHeatSystem"]["main 1"] = wet_distribution(emitters, None);
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/SpaceHeatSystem/main 1: Unevaluated properties are not allowed",
            "/SpaceHeatSystem/main 1: 'thermal_mass' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages);
    }

    #[rstest]
    fn test_wet_distribution_thermal_mass_not_required_when_emitters_each_specify_own(
        mut project: Value,
    ) {
        // System-level thermal_mass is not required when every radiator emitter
        // provides its own (c-based: thermal_mass on the emitter)
        let emitters = json!([
            {
                "wet_emitter_type": "radiator",
                "frac_convective": 0.5,
                "n": 1.34,
                "c": 0.05,
                "thermal_mass": 0.014,
            },
        ]);
        project["SpaceHeatSystem"]["main 1"] = wet_distribution(emitters, None);
        let result = apply_schema_validation(&project);
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_wet_distribution_thermal_mass_per_m_not_required_when_emitters_each_specify_own(
        mut project: Value,
    ) {
        // System-level thermal_mass is not required when every radiator emitter
        // provides its own (length-based: thermal_mass_per_m on the emitter)
        let emitters = json!([
            {
                "wet_emitter_type": "radiator",
                "frac_convective": 0.5,
                "n": 1.34,
                "length": 1.0,
                "c_per_m": 0.05,
                "thermal_mass_per_m": 51.8,
            },
        ]);
        project["SpaceHeatSystem"]["main 1"] = wet_distribution(emitters, None);
        let result = apply_schema_validation(&project);
        assert!(result.is_ok());
    }

    fn merge_json_values(a: &mut Value, b: Value) {
        match (a, b) {
            (a @ &mut Value::Object(_), Value::Object(b)) => {
                let a = a.as_object_mut().unwrap();
                // clobber target object if item being merged is empty object
                if b.is_empty() {
                    a.clear();
                    return;
                }
                for (k, v) in b {
                    merge_json_values(a.entry(k).or_insert(Value::Null), v);
                }
            }
            (a, b) => *a = b,
        }
    }

    #[rstest]
    fn test_conditions_apply_to_multiple_values(mut project: Value) {
        // MechanicalVents must always have:
        project["InfiltrationVentilation"]["MechanicalVentilation"] = json!({
            "vent1": {
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 1,
            },
            "vent2": {
                "EnergySupply": "mains elec",
                "design_outdoor_air_flow_rate": 1,
            },
        });
        // Centralised continuous MEV and MVHR must have
        // measured_fan_power and measured_air_flow_rate, or SFP
        // This stacks with MVHR individual requirements of mvhr_eff, mvhr_location, ductwork,
        // position exhaust and location
        merge_json_values(
            &mut project["InfiltrationVentilation"]["MechanicalVentilation"]["vent1"],
            json!({
                "vent_type": "MVHR",
                "mvhr_eff": 1,
                "mvhr_location": "inside",
                "measured_air_flow_rate": 1,
                "position_intake": {"mid_height_air_flow_path": 1.5, "orientation360": 90, "pitch": 60},
                "position_exhaust": {
                    "mid_height_air_flow_path": 1.5,
                    "orientation360": 90,
                    "pitch": 60,
                },
            }),
        );
        // whereas Decentralised continuous MEV and Intermittent MEV
        // must provide SFP, mid_height_airflow_path, orientation360 and pitch directly
        merge_json_values(
            &mut project["InfiltrationVentilation"]["MechanicalVentilation"]["vent2"],
            json!({
                "vent_type": "Intermittent MEV",
                "mid_height_air_flow_path": 1.5,
                "orientation360": 90,
                "pitch": 60,
            }),
        );

        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/InfiltrationVentilation/MechanicalVentilation/vent1: Unevaluated properties are not allowed ",
            "/InfiltrationVentilation/MechanicalVentilation/vent1: 'ductwork' is a required property",
            "/InfiltrationVentilation/MechanicalVentilation/vent1: {'EnergySupply':'mains elec','design_outdoor_air_flow_rate':1,'vent_type':'MVHR','mvhr_eff':1,'mvhr_location':'inside','measured_air_flow_rate':1,'position_intake':{'mid_height_air_flow_path':1.5,'orientation360':90,'pitch':60},'position_exhaust':{'mid_height_air_flow_path':1.5,'orientation360':90,'pitch':60}} is not valid under any of the schemas listed in the 'oneOf' keyword",
            "/InfiltrationVentilation/MechanicalVentilation/vent2: 'SFP' is a required property"
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages);
    }

    #[rstest]
    fn test_value_out_of_range_errors(mut project: Value) {
        project["NumberOfBedrooms"] = json!(-1);
        let result = apply_schema_validation(&project);
        let error_message = "/NumberOfBedrooms: -1 is less than the minimum of 0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    // test name in Python is test_additional_top_level_property_raises
    fn test_additional_top_level_property_errors(mut project: Value) {
        project["SomeOtherTopLevelProperty"] = json!("a_value");
        let result = apply_schema_validation(&project);
        let error_message =
            "Unevaluated properties are not allowed ('SomeOtherTopLevelProperty' was unexpected)";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message]);
    }

    #[rstest]
    fn test_number_of_sanitary_accommodations_is_rejected(mut project: Value) {
        // NumberOfSanitaryAccommodations is no longer an FHS input: sanitary
        // accommodations may use purge ventilation under Part F table 1.1, so
        // they are excluded from the intermittent mechanical extract minimum.
        project["NumberOfSanitaryAccommodations"] = json!(0);
        let result = apply_schema_validation(&project);
        let error_message =
            "Unevaluated properties are not allowed ('NumberOfSanitaryAccommodations' was unexpected)";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message]);
    }

    #[rstest]
    fn test_storey_of_dwelling_is_rejected(mut project: Value) {
        // storey_of_dwelling is no longer an FHS input; it was unused by the
        // wrapper. A flat input that still carries it is rejected because the
        // General object allows no unevaluated properties.
        project["General"] = json!({
            "build_type": "flat",
            "storeys_in_dwelling": 1,
            "storeys_in_building": 1,
            "storey_of_dwelling": 1,
        });
        let result = apply_schema_validation(&project);
        let error_message =
            "/General: Unevaluated properties are not allowed ('storey_of_dwelling' was unexpected)";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message]);
    }

    #[rstest]
    fn test_flat_without_storey_of_dwelling_validates(mut project: Value) {
        // A flat needs only storeys_in_building (and the common storeys_in_dwelling
        // and build_type); storey_of_dwelling is no longer required or accepted.
        project["General"] = json!({
            "build_type": "flat",
            "storeys_in_dwelling": 1,
            "storeys_in_building": 1,
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    // Python name test_additional_property_lower_level_raises
    fn test_additional_property_lower_level_errors(mut project: Value) {
        project["ExternalConditions"]["SomeNewThing"] = json!("val");
        let result = apply_schema_validation(&project);
        let error_message =
            "/ExternalConditions: Unevaluated properties are not allowed ('SomeNewThing' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_exclusive_minimum_errors(mut project: Value) {
        // Given an electric battery with 0 charge/discharge efficiency
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "ElectricBattery": {
                "capacity": 2,
                "charge_discharge_efficiency_round_trip": 0,
                "minimum_charge_rate_one_way_trip": 0.001,
                "maximum_charge_rate_one_way_trip": 1.5,
                "maximum_discharge_rate_one_way_trip": 1.25,
                "battery_location": "inside",
            },
        });
        let result = apply_schema_validation(&project);
        let error_message =
            "/EnergySupply/mains elec/ElectricBattery/charge_discharge_efficiency_round_trip: 0 is less than or equal to the minimum of 0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_export_power_limit_accepted_on_an_export_capable_supply(mut project: Value) {
        // Given an electricity supply that is export capable and carries a DNO export limit
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "is_export_capable": true,
            "power_limit_export": 3.68,
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_export_power_limit_accepted_when_export_capability_is_left_unstated(
        mut project: Value,
    ) {
        // Given an electricity supply with a DNO export limit and no is_export_capable
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "power_limit_export": 3.68,
        });
        // When validation is run, then no error: the wrapper defaults an electricity
        // supply to export capable, so an unstated value does not conflict with the limit
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_export_power_limit_rejected_on_a_supply_that_cannot_export(mut project: Value) {
        // Given an electricity supply with a DNO export limit and no is_export_capable
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "is_export_capable": false,
            "power_limit_export": 3.68,
        });
        // When validation is run
        // Then it is rejected, as the core model rejects it: a limit on grid export is
        // meaningless where nothing can be exported
        let result = apply_schema_validation(&project);
        // Note that in Python the error reads "3.68 should not be valid under {}" instead
        let error_message =
            "/EnergySupply/mains elec/power_limit_export: {} is not allowed for 3.68";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_export_power_limit_in_watts_is_rejected(mut project: Value) {
        // Given a limit entered in watts rather than kW, the slip the upper bound exists
        // to catch: it would otherwise validate and then never bind, leaving the run to
        // proceed as though the supply were unlimited
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "is_export_capable": true,
            "power_limit_export": 3680,
        });
        // When validation is run
        // Then it is rejected as above the maximum
        let result = apply_schema_validation(&project);
        let error_message =
            "/EnergySupply/mains elec/power_limit_export: 3680 is greater than the maximum of 100";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_export_power_limit_at_the_upper_bound_is_accepted(mut project: Value) {
        // Given a limit at the bound, far above any domestic connection but still allowed
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "is_export_capable": true,
            "power_limit_export": 100,
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_export_power_limit_must_be_greater_than_zero(mut project: Value) {
        // Given an electricity supply with a zero DNO export limit
        project["EnergySupply"]["mains elec"] = json!({
            "fuel": "electricity",
            "is_export_capable": true,
            "power_limit_export": 0,
        });
        // When validation is run
        // Then it is rejected: a supply that cannot export at all is described by
        // is_export_capable, not by a limit of zero
        let result = apply_schema_validation(&project);
        let error_message =
            "/EnergySupply/mains elec/power_limit_export: 0 is less than or equal to the minimum of 0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_dependency_at_offset_levels(mut project: Value) {
        // Given PartO_active_cooling_required at the top level
        // But no SpaceCoolSystem defined within a zone
        project["PartO_active_cooling_required"] = json!(true);
        remove_key(&mut project["Zone"]["zone 1"], "SpaceCoolSystem");
        let result = apply_schema_validation(&project);
        let error_message = "/Zone/zone 1: 'SpaceCoolSystem' is a required property";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_mutually_absent_appliance_settings(mut project: Value) {
        // Given a specification with neither Fridge nor Fridge-Freezer installed
        project["Appliances"]["Fridge"] = json!("Not Installed");
        project["Appliances"]["Fridge-Freezer"] = json!("Not Installed");
        let result = apply_schema_validation(&project);
        let error_message = "/Appliances: {'required':['Fridge','Fridge-Freezer']";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message]);
    }

    #[rstest]
    fn test_omitting_both_cold_appliances_is_valid(mut project: Value) {
        // Given a specification that declares neither Fridge nor Fridge-Freezer
        // (both are injected as installed defaults during preprocessing)
        remove_key(&mut project["Appliances"], "Fridge");
        remove_key(&mut project["Appliances"], "Fridge-Freezer");
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_fridge_not_installed_without_fridge_freezer_is_valid(mut project: Value) {
        // Given Fridge explicitly "Not Installed" and Fridge-Freezer omitted
        // (Fridge-Freezer is injected as an installed default, so a cold appliance remains)
        project["Appliances"]["Fridge"] = json!("Not Installed");
        remove_key(&mut project["Appliances"], "Fridge-Freezer");
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_must_have_mains_elec(mut project: Value) {
        // Given input without a "mains elec" EnergySupply (which will be used for Appliances)
        remove_key(&mut project["EnergySupply"], "mains elec");
        let result = apply_schema_validation(&project);
        let error_message = "/EnergySupply: 'mains elec' is a required property";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_mains_elec_must_use_electricity(mut project: Value) {
        // Given a "main elec" EnergySupply that uses a different fuel
        project["EnergySupply"]["mains elec"]["fuel"] = json!("mains gas");
        assert!(apply_schema_validation(&project).is_err());

        let result = apply_schema_validation(&project);
        let error_message = "/EnergySupply/mains elec/fuel: 'electricity' was expected";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_heat_pump_backup_properties_not_allowed_without_backup_ctrl_type(mut project: Value) {
        // Given a HeatSourceWet of type HeatPump with backup_ctrl_type = None
        project["HeatSourceWet"] = json!({
            "heatpump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "EnergySupply": "mains elec",
                "source_type": "OutsideAir",
                "sink_type": "Water",
                "backup_ctrl_type": "None",
                "modulating_control": true,
                "min_modulation_rate_35": 0.2,
                "min_modulation_rate_55": 0.2,
                "temp_return_feed_max": 40,
                "temp_lower_operating_limit": -10,
                "min_temp_diff_flow_return_for_hp_to_operate": 5,
                "var_flow_temp_ctrl_during_test": false,
                "power_heating_circ_pump": 0.1,
                "power_source_circ_pump": 0.1,
                "power_standby": 0.01,
                "power_crankcase_heater": 0.01,
                "power_off": 0.0,
                "test_data_EN14825": [],
                "time_constant_onoff_operation": 1,
                "power_max_backup": 1,
            }
        });
        let result = apply_schema_validation(&project);
        let error_message = "/HeatSourceWet/heatpump: Unevaluated properties are not allowed ('power_max_backup' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_heat_pump_backup_properties_required_with_backup_ctrl_type(mut project: Value) {
        // Given a HeatSourceWet of type HeatPump with a numerical power_max_backup
        project["HeatSourceWet"] = json!({
            "heatpump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "EnergySupply": "mains elec",
                "source_type": "OutsideAir",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.2,
                "min_modulation_rate_55": 0.2,
                "temp_return_feed_max": 40,
                "temp_lower_operating_limit": -10,
                "min_temp_diff_flow_return_for_hp_to_operate": 5,
                "var_flow_temp_ctrl_during_test": false,
                "power_heating_circ_pump": 0.1,
                "power_source_circ_pump": 0.1,
                "power_standby": 0.01,
                "power_crankcase_heater": 0.01,
                "power_off": 0.0,
                "test_data_EN14825": [],
                "time_constant_onoff_operation": 1,
            }
        });
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/HeatSourceWet/heatpump: Unevaluated properties are not allowed",
            "/HeatSourceWet/heatpump: {'type':'HeatPump','is_heat_network':false,'EnergySupply':'mains elec','source_type':'OutsideAir','sink_type':'Water','backup_ctrl_type':'TopUp','modulating_control':true,'min_modulation_rate_35':0.2,'min_modulation_rate_55':0.2,'temp_return_feed_max':40,'temp_lower_operating_limit':-10,'min_temp_diff_flow_return_for_hp_to_operate':5,'var_flow_temp_ctrl_during_test':false,'power_heating_circ_pump':0.1,'power_source_circ_pump':0.1,'power_standby':0.01,'power_crankcase_heater':0.01,'power_off':0.0,'test_data_EN14825':[],'time_constant_onoff_operation':1} is not valid under any of the schemas listed in the 'oneOf' keyword",

            ];
        contains_expected_schema_validation_errors_partially(result, error_messages);
    }

    #[rstest]
    fn test_heat_network_energy_supply_may_be_custom(mut project: Value) {
        // Given a top level HeatSourceWet that is a heat network (so here the
        // "EnergySupply" of  "heat network" is the DHN generator site. It is not
        // the electrical power to the HIU)
        project["HeatSourceWet"] = json!({
            "heat network": {
                "type": "HIU",
                "is_heat_network": true,
                "heat_network_type": "sleeved DHN",
                "HIU_daily_loss": 1,
                "power_max": 1,
                "building_level_distribution_losses": 1,
                "EnergySupply": {
                    "name": "custom_heat_network_supply",
                    "factor": {
                        "Emissions Factor kgCO2e/kWh": 1,
                        "Emissions Factor kgCO2e/kWh including out-of-scope emissions": 1,
                        "Primary Energy Factor kWh/kWh delivered": 1,
                    },
                    "is_export_capable": false,
                },
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
        // And a reference to an existing EnergySupply is also fine
        project["HeatSourceWet"]["heat network"]["EnergySupply"] = json!("mains elec");
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_hiu_building_level_distribution_losses_is_optional(mut project: Value) {
        // Given an HIU heat source that omits building_level_distribution_losses
        // (the wrapper supplies the build-type-dependent default)
        project["HeatSourceWet"] = json!({
            "heat network": {
                "type": "HIU",
                 "is_heat_network": true,
                "heat_network_type": "sleeved DHN",
                "HIU_daily_loss": 1,
                "power_max": 1,
                "EnergySupply": "mains elec"
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_heat_pump_may_have_heat_network_energy_supply_that_is_custom(mut project: Value) {
        // Given a locally powered heat pump, sourcing energy from a heat network
        // i.e. a 5th gen heat pump
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "source_type": "HeatNetwork",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.35,
                "min_modulation_rate_55": 0.4,
                "time_constant_onoff_operation": 140,
                "temp_return_feed_max": 70.0,
                "temp_lower_operating_limit": -5.0,
                "min_temp_diff_flow_return_for_hp_to_operate": 0.0,
                "var_flow_temp_ctrl_during_test": true,
                "power_source_circ_pump": 0.010,
                "power_standby": 0.015,
                "power_crankcase_heater": 0.01,
                "power_off": 0.015,
                "power_max_backup": 3.0,
                "test_data_EN14825": [],
                "EnergySupply": "mains elec",
                "EnergySupply_heat_network": {
                    "name": "custom_heat_network_supply",
                    "factor": {
                        "Emissions Factor kgCO2e/kWh": 1,
                        "Emissions Factor kgCO2e/kWh including out-of-scope emissions": 1,
                        "Primary Energy Factor kWh/kWh delivered": 1,
                    },
                    "is_export_capable": false,
                },
                "temp_distribution_heat_network": 60,
            }
        });
        // When the validation is run there is no validation error
        // Because it is legitimate to have "EnergySupply_heat_network" using a custom fuel
        // as it refers to the heat network generator site
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_non_heat_network_may_not_have_custom_energy_supply(mut project: Value) {
        // Given a heat pump with a heat network source, with is_heat_network = False
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "source_type": "HeatNetwork",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.35,
                "min_modulation_rate_55": 0.4,
                "time_constant_onoff_operation": 140,
                "temp_return_feed_max": 70.0,
                "temp_lower_operating_limit": -5.0,
                "min_temp_diff_flow_return_for_hp_to_operate": 0.0,
                "var_flow_temp_ctrl_during_test": true,
                "power_source_circ_pump": 0.010,
                "power_standby": 0.015,
                "power_crankcase_heater": 0.01,
                "power_off": 0.015,
                "power_max_backup": 3.0,
                "test_data_EN14825": [],
                "EnergySupply": {
                    "name": "custom_heat_pump_supply",
                    "factor": {
                        "Emissions Factor kgCO2e/kWh": 1,
                        "Emissions Factor kgCO2e/kWh including out-of-scope emissions": 1,
                        "Primary Energy Factor kWh/kWh delivered": 1,
                    },
                    "is_export_capable": false,
                },
                "EnergySupply_heat_network": "mains gas",
                "temp_distribution_heat_network": 60,
            }
        });
        // Then the "EnergySupply" of the HeatPump may NOT be custom because it is not itself the
        // energy supply of the heat network (it's just to power the pump). Only when the
        // top level HeatSourceWet is a heat network (i.e. is_heat_network is true), should
        // a custom "EnergySupply" be permitted
        let result = apply_schema_validation(&project);
        let error_messages =
            "/HeatSourceWet/heat pump/EnergySupply: {'name':'custom_heat_pump_supply','factor':{'Emissions Factor kgCO2e/kWh':1,'Emissions Factor kgCO2e/kWh including out-of-scope emissions':1,'Primary Energy Factor kWh/kWh delivered':1},'is_export_capable':false} is not of type 'string'";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages]);
    }

    #[rstest]
    fn test_air_sourced_heat_pump_test_subschema(mut project: Value) {
        // Given a heat pump with outside air as its source
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "source_type": "OutsideAir",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.35,
                "min_modulation_rate_55": 0.4,
                "time_constant_onoff_operation": 140,
                "temp_return_feed_max": 70.0,
                "temp_lower_operating_limit": -5.0,
                "min_temp_diff_flow_return_for_hp_to_operate": 0.0,
                "var_flow_temp_ctrl_during_test": true,
                "power_source_circ_pump": 0.010,
                "power_standby": 0.015,
                "power_crankcase_heater": 0.01,
                "power_off": 0.015,
                "power_max_backup": 3.0,
                "EnergySupply": "mains elec",
                "eahp_mixed_max_temp": 60,
                "eahp_mixed_min_temp": 0,
                "test_data_EN14825": [
                {
                    "test_letter": "A",
                    "capacity": 8.4,
                    "cop": 4.6,
                    "design_flow_temp": 35,
                    "temp_outlet": 34,
                    "temp_source": 0,
                    "temp_test": -7,
                    "eahp_mixed_ext_air_ratio": 1,
                    "air_flow_rate": 1,
                }
                ],
            }
        });
        // When validation is run
        // Then it errors that test data must not have eahp_mixed_ext_air_ratio
        // And the heat pump itself must not have eahp_mixed_max_temp and eahp_mixed_min_temp
        // And because the subschema is invalid, none of the other properties are evaluated either
        let result = apply_schema_validation(&project);
        let error_messages =
            "/HeatSourceWet/heat pump/test_data_EN14825/0: Unevaluated properties are not allowed ('eahp_mixed_ext_air_ratio' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages]);
    }

    #[rstest]
    fn test_exhaust_air_source_heat_pump_subschema_valid(mut project: Value) {
        // Given a heat pump with mixed exhaust air as its source
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "source_type": "ExhaustAirMixed",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.35,
                "min_modulation_rate_55": 0.4,
                "time_constant_onoff_operation": 140,
                "temp_return_feed_max": 70.0,
                "temp_lower_operating_limit": -5.0,
                "min_temp_diff_flow_return_for_hp_to_operate": 0.0,
                "var_flow_temp_ctrl_during_test": true,
                "power_source_circ_pump": 0.010,
                "power_standby": 0.015,
                "power_crankcase_heater": 0.01,
                "power_off": 0.015,
                "power_max_backup": 3.0,
                "eahp_mixed_max_temp": 60,
                "eahp_mixed_min_temp": 50,
                "EnergySupply": "mains elec",
                "test_data_EN14825": [
                {
                    "air_flow_rate": 1,
                    "test_letter": "A",
                    "capacity": 8.4,
                    "cop": 4.6,
                    "design_flow_temp": 35,
                    "temp_outlet": 34,
                    "temp_source": 0,
                    "temp_test": -7,
                    "eahp_mixed_ext_air_ratio": 1,
                }
                ],
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_exhaust_air_source_heat_pump_subschema_invalid(mut project: Value) {
        // Given a heat pump with mixed exhaust air as its source
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "source_type": "ExhaustAirMixed",
                "sink_type": "Water",
                "backup_ctrl_type": "TopUp",
                "modulating_control": true,
                "min_modulation_rate_35": 0.35,
                "min_modulation_rate_55": 0.4,
                "time_constant_onoff_operation": 140,
                "temp_return_feed_max": 70.0,
                "temp_lower_operating_limit": -5.0,
                "min_temp_diff_flow_return_for_hp_to_operate": 0.0,
                "var_flow_temp_ctrl_during_test": true,
                "power_source_circ_pump": 0.010,
                "power_standby": 0.015,
                "power_crankcase_heater": 0.01,
                "power_off": 0.015,
                "power_max_backup": 3.0,
                "EnergySupply": "mains elec",
                "test_data_EN14825": [
                {
                    "test_letter": "A",
                    "capacity": 8.4,
                    "cop": 4.6,
                    "design_flow_temp": 35,
                    "temp_outlet": 34,
                    "temp_source": 0,
                    "temp_test": -7,
                }
                ],
            }
        });
        // Then it errors that test data must have eahp_mixed_ext_air_ratio and air_flow_rate
        // And the heat pump itself must have eahp_mixed_max_temp and eahp_mixed_min_temp
        // Because the subschema is invalid then none of the properties are evaluated
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/HeatSourceWet/heat pump: 'eahp_mixed_max_temp' is a required property",
            "/HeatSourceWet/heat pump: 'eahp_mixed_min_temp' is a required property",
            "/HeatSourceWet/heat pump/test_data_EN14825/0: 'air_flow_rate' is a required property",
            "/HeatSourceWet/heat pump/test_data_EN14825/0: 'eahp_mixed_ext_air_ratio' is a required property",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages);
    }

    #[rstest]
    fn test_if_not_anyof_condition_for_flat_opaque_elements(mut project: Value) {
        // If a BuildingElementOpaque is not either pitch 0 or pitch 180 then an orientation360
        // is required. (Else the wrapper will create an orientation360 = 180)
        project["Zone"]["zone 1"]["BuildingElement"]["floor 0"]["pitch"] = json!(180);
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/Zone/zone 1/BuildingElement/floor 0: Unevaluated properties are not allowed ('orientation360' was unexpected)",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages);
    }

    #[rstest]
    fn test_if_not_anyof_condition_for_flat_transparent_elements(mut project: Value) {
        // Given a BuildingElementTransparent with pitch 180, indicating a skylight
        project["Zone"]["zone 1"]["BuildingElement"]["window 0"]["pitch"] = json!(180);
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/Zone/zone 1/BuildingElement/window 0: Unevaluated properties are not allowed ('orientation360' was unexpected)",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages);
    }

    #[rstest]
    fn test_transparent_element_may_omit_window_part_list(mut project: Value) {
        // Given a fixed (non-openable) window with no openable sections, the
        // window_part_list is omitted entirely
        remove_key(
            &mut project["Zone"]["zone 1"]["BuildingElement"]["window 0"],
            "window_part_list",
        );
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_transparent_element_may_have_empty_window_part_list(mut project: Value) {
        // Given a fixed (non-openable) window with an explicitly empty section list
        project["Zone"]["zone 1"]["BuildingElement"]["window 0"]["window_part_list"] = json!([]);
        assert!(apply_schema_validation(&project).is_ok());
    }

    /// A PCM heat battery, as used by the charging configuration tests.
    fn pcm_heat_battery(
        temp_min_useful: Option<f64>,
        rated_charge_power: Option<f64>,
        heat_source: Value,
    ) -> Value {
        let mut battery = json!({
            "type": "HeatBattery",
            "battery_type": "pcm",
            "EnergySupply": "mains elec",
            "is_heat_network": false,
            "A": 174.33952,
            "B": -931.565,
            "electricity_circ_pump": 0.06,
            "electricity_standby": 0.0244,
            "flow_rate_l_per_min": 10,
            "heat_storage_kJ_per_K_above_Phase_transition": 381.5,
            "heat_storage_kJ_per_K_below_Phase_transition": 305.2,
            "heat_storage_kJ_per_K_during_Phase_transition": 12317,
            "inlet_diameter_mm": 6.5,
            "max_rated_losses": 0.22,
            "max_temperature": 80,
            "number_of_units": 1,
            "phase_transition_temperature_lower": 57,
            "phase_transition_temperature_upper": 59,
            "simultaneous_charging_and_discharging": false,
            "velocity_in_HEX_tube_at_1_l_per_min_m_per_s": 0.035,
        });
        if let Some(temp_min_useful) = temp_min_useful {
            insert_key_value(&mut battery, "temp_min_useful", json!(temp_min_useful));
        }

        if let Some(rated_charge_power) = rated_charge_power {
            insert_key_value(
                &mut battery,
                "rated_charge_power",
                json!(rated_charge_power),
            );
        }

        insert_key_value(&mut battery, "HeatSource", json!(heat_source));
        battery
    }

    fn hydronic_charging_source() -> Value {
        json!({
            "type": "HeatSourceWet",
            "name": "HeatPump1",
            "temp_flow_limit_upper": 65.0,
            "flow_rate_charging_l_per_min": 10,
            "A": 174.33952,
            "B": -931.565,
            "velocity_in_HEX_tube_at_1_l_per_min_m_per_s": 0.035,
            "inlet_diameter_mm": 6.5,
        })
    }

    #[rstest]
    fn test_heat_battery_charged_by_a_wet_heat_source_accepted(mut project: Value) {
        // Given a PCM heat battery charged by a heat pump rather than a direct electric element.
        let temp_min_useful = Some(40.);
        let heat_battery = pcm_heat_battery(
            temp_min_useful,
            None,
            json!({"hp_charging": hydronic_charging_source()}),
        );
        project["HeatSourceWet"] = json!({"hb1": heat_battery});
        // When validation is run, then no error: the charging circuit's own heat exchanger and
        // flow characteristics are given separately from the battery's, which may differ
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_heat_battery_cannot_combine_both_charging_configurations(mut project: Value) {
        // Given a PCM heat battery declaring charging sources and a rated charge power
        let temp_min_useful = Some(40.);
        let rated_charge_power = Some(15.);
        let heat_battery = pcm_heat_battery(
            temp_min_useful,
            rated_charge_power,
            json!({"hp_charging": hydronic_charging_source()}),
        );
        project["HeatSourceWet"] = json!({"hb1": heat_battery});
        // When validation is run
        // Then it is rejected here rather than by the core, which forbids the same combination.
        // The path and the offending property are asserted rather than the whole message, which
        // is jsonschema's wording and can change when it is upgraded
        let result = apply_schema_validation(&project);
        let error_message = "HeatSourceWet/hb1/rated_charge_power";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message])
    }

    #[rstest]
    fn test_heat_battery_charging_sources_require_the_discharge_reference(mut project: Value) {
        // Given charging sources but no temperature to treat as fully discharged, which the
        // state-of-charge calculation needs
        let heat_battery = pcm_heat_battery(
            None,
            None,
            json!({"hp_charging": hydronic_charging_source()}),
        );
        project["HeatSourceWet"] = json!({"hb1": heat_battery});
        let result = apply_schema_validation(&project);
        let error_message = "'temp_min_useful' is a required property";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message])
    }

    #[rstest]
    fn test_edge_insulation_subschema_valid(mut project: Value) {
        // Given a BuildingElementGround containing horizontal edge_insulation with width property
        project["Zone"]["zone 1"]["BuildingElement"]["ground"] = json!({
            "type": "BuildingElementGround",
            "total_area": 15.0,
            "area": 15.0,
            "u_value": 1.4,
            "thermal_resistance_floor_construction": 0.2,
            "areal_heat_capacity": "Very light",
            "mass_distribution_class": "D: Mass equally distributed",
            "floor_type": "Slab_edge_insulation",
            "thickness_walls": 0.2,
            "perimeter": 16.0,
            "psi_wall_floor_junc": 0.5,
            "edge_insulation": [{"edge_thermal_resistance": 10, "type": "horizontal", "width": 0.1}],
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    /// A suspended-floor ground element, as used by the smart air brick tests.
    fn suspended_floor(
        smart_air_bricks: Option<bool>,
        vents_open_during_airtightness_test: Option<bool>,
    ) -> Value {
        let mut floor = json!({
            "type": "BuildingElementGround",
            "total_area": 15.0,
            "area": 15.0,
            "u_value": 1.4,
            "thermal_resistance_floor_construction": 0.2,
            "areal_heat_capacity": "Very light",
            "mass_distribution_class": "D: Mass equally distributed",
            "floor_type": "Suspended_floor",
            "thickness_walls": 0.2,
            "perimeter": 16.0,
            "psi_wall_floor_junc": 0.5,
            "height_upper_surface": 0.3,
            "thermal_transm_walls": 1.5,
            "thermal_resist_insul": 0.5,
            "area_per_perimeter_vent": 0.003,
        });
        if let Some(smart_air_bricks) = smart_air_bricks {
            insert_key_value(&mut floor, "smart_air_bricks", json!(smart_air_bricks));
        };

        if let Some(vents_open_during_airtightness_test) = vents_open_during_airtightness_test {
            insert_key_value(
                &mut floor,
                "vents_open_during_airtightness_test",
                json!(vents_open_during_airtightness_test),
            );
        };
        floor
    }

    #[rstest]
    fn test_smart_air_bricks_require_the_airtightness_test_state(mut project: Value) {
        // Given a suspended floor with smart air bricks but no statement of whether the
        // underfloor vents were open during the pressurisation test
        project["Zone"]["zone 1"]["BuildingElement"]["ground"] = suspended_floor(Some(true), None);
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: the infiltration adjustment is applied relative to the tested
        // state, so it cannot be derived without knowing it. The sibling listing is present
        // because a missing property fails at the element rather than below it, so it is not
        // suppressed.
        let error_message = &mut [
            "/Zone/zone 1/BuildingElement/ground: Unevaluated properties are not allowed",
            "/Zone/zone 1/BuildingElement/ground: 'vents_open_during_airtightness_test' is a required property"
            ];
        contains_expected_schema_validation_errors_partially(result, error_message)
    }

    #[rstest]
    fn test_airtightness_test_state_rejected_without_smart_air_bricks(mut project: Value) {
        // Given a suspended floor stating the airtightness-test vent state but no smart air
        // bricks, where the state has nothing to adjust
        insert_key_value(
            &mut project["Zone"]["zone 1"]["BuildingElement"],
            "ground",
            suspended_floor(None, Some(true)),
        );
        // When validation is run
        let result = apply_schema_validation(&project);
        // validation message in Python is true should not be valid under {}
        let error_message = "/Zone/zone 1/BuildingElement/ground/vents_open_during_airtightness_test: {} is not allowed for true";
        contains_expected_schema_validation_errors_partially(result, &mut [error_message])
    }

    #[rstest]
    fn test_smart_air_bricks_with_the_airtightness_test_state_accepted(mut project: Value) {
        // Given a suspended floor stating the airtightness-test vent state but no smart air
        // bricks, where the state has nothing to adjust
        project["Zone"]["zone 1"]["BuildingElement"]["ground"] =
            suspended_floor(Some(true), Some(false));
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_edge_insulation_subschema_invalid(mut project: Value) {
        // Given a BuildingElementGround containing horizontal edge_insulation with depth property
        project["Zone"]["zone 1"]["BuildingElement"]["ground"] = json!({
            "type": "BuildingElementGround",
            "total_area": 15.0,
            "area": 15.0,
            "u_value": 1.4,
            "thermal_resistance_floor_construction": 0.2,
            "areal_heat_capacity": "Very light",
            "mass_distribution_class": "D: Mass equally distributed",
            "floor_type": "Slab_edge_insulation",
            "thickness_walls": 0.2,
            "perimeter": 16.0,
            "psi_wall_floor_junc": 0.5,
            "edge_insulation": [
            {"edge_thermal_resistance": 10, "type": "horizontal", "depth": 0.1}
            ],
        });
        let result = apply_schema_validation(&project);
        let error_message = &mut [
            "/Zone/zone 1/BuildingElement/ground/edge_insulation/0: Unevaluated properties are not allowed ('depth' was unexpected)",
            "/Zone/zone 1/BuildingElement/ground/edge_insulation/0: 'width' is a required property"
        ];
        contains_expected_schema_validation_errors_exactly(result, error_message)
    }

    #[rstest]
    fn test_energy_supply_with_priority(mut project: Value) {
        // Given an energy supply with a battery and diverter with a full set of properties,
        // including a priority array
        project["EnergySupply"] = json!({
            "mains elec": {
                "fuel": "electricity",
                "is_export_capable": false,
                "priority": ["ElectricBattery", "diverter"],
                "ElectricBattery": {
                    "capacity": 2,
                    "charge_discharge_efficiency_round_trip": 0.8,
                    "minimum_charge_rate_one_way_trip": 0.001,
                    "maximum_charge_rate_one_way_trip": 1.5,
                    "maximum_discharge_rate_one_way_trip": 1.25,
                    "battery_location": "inside",
                },
                "diverter": {"HeatSource": "immersion"},
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_heat_pump_without_modulating_control(mut project: Value) {
        // Given a heat pump without modulating_control
        project["HeatSourceWet"] = json!({
            "heat pump": {
                "type": "HeatPump",
                "is_heat_network": false,
                "EnergySupply": "mains elec",
                "source_type": "OutsideAir",
                "sink_type": "Air",
                "backup_ctrl_type": "None",
                "modulating_control": false,
                "temp_return_feed_max": 40,
                "temp_lower_operating_limit": -10,
                "min_temp_diff_flow_return_for_hp_to_operate": 5,
                "var_flow_temp_ctrl_during_test": false,
                "power_source_circ_pump": 0.1,
                "power_standby": 0.01,
                "power_crankcase_heater": 0.01,
                "power_off": 0.0,
                "test_data_EN14825": [],
                "time_constant_onoff_operation": 1,
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_heat_pump_hw_only_heat_exchanger(mut project: Value) {
        // Given a StorageTank fed by a HP for exclusive HW use
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({
            "hwo_hp": {
                "EnergySupply": "mains elec",
                "daily_losses_declared": 1.05,
                "heat_exchanger_surface_area_declared": 1.5,
                "heater_position": 0.1,
                "in_use_factor_mismatch": 0.6,
                "power_max": 5.0,
                "tank_volume_declared": 100.0,
                "test_data": {
                    "M": {
                        "cop_dhw": 2.5,
                        "energy_input_measured": 2.338,
                        "hw_tapping_prof_daily_total": 5.845,
                        "hw_vessel_loss_daily": 2.0,
                        "power_standby": 0.02,
                    }
                },
                "thermostat_position": 0.33,
                "type": "HeatPump_HWOnly",
            }
        });
        // Then it should raise an error indicating that a heat_exchanger_surface_area is required
        let result = apply_schema_validation(&project);
        let error_message = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_message)
    }

    #[rstest]
    fn test_smart_hot_water_tank_with_hw_only_heat_pump_needs_a_heat_exchanger_area(
        mut project: Value,
    ) {
        // Given a SmartHotWaterTank fed by a hot-water-only heat pump, with no
        // heat-exchanger surface area declared
        project["HotWaterSource"]["hw cylinder"] = json!({
            "type": "SmartHotWaterTank",
            "volume": 80.0,
            "daily_losses": 1.68,
            "ColdWaterSource": "header tank",
            "EnergySupply_pump": "mains elec",
            "max_flow_rate_pump_l_per_min": 1000,
            "power_pump_kW": 5.0,
            "temp_usable": 40,
            "HeatSource": {
                "hwo_hp": {
                    "type": "HeatPump_HWOnly",
                    "EnergySupply": "mains elec",
                    "power_max": 5.0,
                    "tank_volume_declared": 100.0,
                    "daily_losses_declared": 1.05,
                    "heat_exchanger_surface_area_declared": 1.5,
                    "in_use_factor_mismatch": 0.6,
                    "heater_position": 0.1,
                    "test_data": {
                        "M": {
                            "cop_dhw": 2.5,
                            "energy_input_measured": 2.338,
                            "hw_tapping_prof_daily_total": 5.845,
                            "hw_vessel_loss_daily": 2.0,
                            "power_standby": 0.02,
                        }
                    },
                }
            },
        });
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: a tank installed separately from the heat pump is heated
        // through a coil, whose surface area must be characterised
        let error_message = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_message)
    }

    #[rstest]
    fn test_smart_hot_water_tank_may_declare_a_heat_exchanger_area(mut project: Value) {
        // Given a SmartHotWaterTank fed by a hot-water-only heat pump that declares a
        // heat-exchanger surface area for its coil
        project["HotWaterSource"]["hw cylinder"] = json!({
            "type": "SmartHotWaterTank",
            "volume": 80.0,
            "daily_losses": 1.68,
            "heat_exchanger_surface_area": 3.0,
            "ColdWaterSource": "header tank",
            "EnergySupply_pump": "mains elec",
            "max_flow_rate_pump_l_per_min": 1000,
            "power_pump_kW": 5.0,
            "temp_usable": 40,
            "HeatSource": {
                "hwo_hp": {
                    "type": "HeatPump_HWOnly",
                    "EnergySupply": "mains elec",
                    "power_max": 5.0,
                    "tank_volume_declared": 100.0,
                    "daily_losses_declared": 1.05,
                    "heat_exchanger_surface_area_declared": 1.5,
                    "in_use_factor_mismatch": 0.6,
                    "heater_position": 0.1,
                    "test_data": {
                        "M": {
                            "cop_dhw": 2.5,
                            "energy_input_measured": 2.338,
                            "hw_tapping_prof_daily_total": 5.845,
                            "hw_vessel_loss_daily": 2.0,
                            "power_standby": 0.02,
                        }
                    },
                }
            },
        });
        // When validation is run, then no error: a separately-installed smart tank is
        // heated through a coil, characterised by its surface area
        assert!(apply_schema_validation(&project).is_ok());
    }

    /// A hot-water-only heat pump heat source, as used by the exchanger-area tests.
    fn hw_only_heat_pump(mut extra: Value) -> Value {
        let mut hp = json!({
            "type": "HeatPump_HWOnly",
            "EnergySupply": "mains elec",
            "power_max": 5.0,
            "tank_volume_declared": 100.0,
            "daily_losses_declared": 1.05,
            "heat_exchanger_surface_area_declared": 1.5,
            "in_use_factor_mismatch": 0.6,
            "heater_position": 0.1,
            "thermostat_position": 0.33,
            "test_data": {
                "M": {
                    "cop_dhw": 2.5,
                    "energy_input_measured": 2.338,
                    "hw_tapping_prof_daily_total": 5.845,
                    "hw_vessel_loss_daily": 2.0,
                    "power_standby": 0.02,
                }
            },
        });
        hp.as_object_mut()
            .unwrap()
            .append(extra.as_object_mut().unwrap());
        hp
    }

    /// A hot-water-only heat pump supplied as a single unit with its store.
    fn integral_hw_only_heat_pump() -> Value {
        hw_only_heat_pump(json!({"tank_is_integral": true}))
    }

    ///A smart hot water tank fed by the given heat sources.
    fn smart_hot_water_tank(heat_source: Value) -> Value {
        json!({
            "type": "SmartHotWaterTank",
            "volume": 80.0,
            "daily_losses": 1.68,
            "ColdWaterSource": "header tank",
            "EnergySupply_pump": "mains elec",
            "max_flow_rate_pump_l_per_min": 1000,
            "power_pump_kW": 5.0,
            "temp_usable": 40,
            "HeatSource": heat_source,
        })
    }

    #[rstest]
    fn test_storage_tank_needs_a_heat_exchanger_area_beside_another_heat_source(
        mut project: Value,
    ) {
        // Given a StorageTank whose HeatSource holds a hot-water-only heat pump alongside
        // the immersion heater, with no heat-exchanger surface area declared
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"]["HeatSource"],
            "hwo_hp",
            hw_only_heat_pump(json!({})),
        );

        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: the exchanger area is required whenever any heat source is a
        // hot-water-only heat pump, not only when it is the sole heat source
        let error_message = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_message)
    }

    #[rstest]
    fn test_smart_hot_water_tank_needs_a_heat_exchanger_area_beside_another_heat_source(
        mut project: Value,
    ) {
        // Given a SmartHotWaterTank whose HeatSource holds a hot-water-only heat pump
        // alongside an immersion heater, with no heat-exchanger surface area declared
        project["HotWaterSource"]["hw cylinder"] = json!({
            "type": "SmartHotWaterTank",
            "volume": 80.0,
            "daily_losses": 1.68,
            "ColdWaterSource": "header tank",
            "EnergySupply_pump": "mains elec",
            "max_flow_rate_pump_l_per_min": 1000,
            "power_pump_kW": 5.0,
            "temp_usable": 40,
            "HeatSource": {
                "hwo_hp": hw_only_heat_pump(json!({})),
                "immersion": {
                    "type": "ImmersionHeater",
                    "EnergySupply": "mains elec",
                    "power": 3.0,
                    "heater_position": 0.1,
                    "thermostat_position": 0.33,
                },
            },
        });

        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected for the same reason as the storage tank above
        let error_message = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_message)
    }

    #[rstest]
    fn test_storage_tank_integral_to_its_heat_pump_omits_the_exchanger_area(mut project: Value) {
        // Given a StorageTank supplied as a single unit with its hot-water-only heat pump,
        // declaring no exchanger area
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"]["HeatSource"],
            "hwo_hp",
            integral_hw_only_heat_pump(),
        );

        // When validation is run, then no error: the unit is tested with its store to
        // EN 16147, so there is no separately installed exchanger to compare against
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_storage_tank_integral_to_its_heat_pump_must_not_declare_an_exchanger_area(
        mut project: Value,
    ) {
        // Given an integral StorageTank that nonetheless declares an exchanger area
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] =
            json!({"hwo_hp": integral_hw_only_heat_pump()});
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"],
            "heat_exchanger_surface_area",
            json!(3.0),
        );

        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: no de-rating comparison applies to an integral unit, so
        // there is no exchanger area to declare
        // Python validation error message varies slightly: 3.0 should not be valid under {}
        let error_message =
            "/HotWaterSource/hw cylinder/heat_exchanger_surface_area: {} is not allowed for 3.0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message])
    }

    #[rstest]
    fn test_smart_hot_water_tank_integral_to_its_heat_pump_omits_the_exchanger_area(
        mut project: Value,
    ) {
        // Given a SmartHotWaterTank supplied as a single unit with its hot-water-only heat
        // pump, declaring no exchanger area
        project["HotWaterSource"]["hw cylinder"] =
            smart_hot_water_tank(json!({"hwo_hp": integral_hw_only_heat_pump()}));
        // When validation is run, then no error, for the same reason as the storage tank
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_smart_hot_water_tank_integral_to_its_heat_pump_must_not_declare_an_area(
        mut project: Value,
    ) {
        // Given an integral SmartHotWaterTank that nonetheless declares an exchanger area
        let mut cylinder = smart_hot_water_tank(json!({"hwo_hp": integral_hw_only_heat_pump()}));
        insert_key_value(&mut cylinder, "heat_exchanger_surface_area", json!(3.0));
        project["HotWaterSource"]["hw cylinder"] = cylinder;
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected, for the same reason as the storage tank
        // Python validation error message is varies slightly: 3.0 should not be valid under {}
        let error_message =
            "/HotWaterSource/hw cylinder/heat_exchanger_surface_area: {} is not allowed for 3.0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message])
    }

    #[rstest]
    fn test_tank_declared_separate_from_its_heat_pump_still_needs_an_exchanger_area(
        mut project: Value,
    ) {
        // Given a tank whose heat pump states tank_is_integral explicitly as false
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({
            "hwo_hp": hw_only_heat_pump(json!({})),
        });
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected just as when the property is absent: the core model defaults
        // tank_is_integral to false, so stating it changes nothing
        let error_messages = &mut [
            "HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages)
    }

    #[rstest]
    fn test_integral_heat_pump_beside_another_heat_source_omits_the_exchanger_area(
        mut project: Value,
    ) {
        // Given an integral hot-water-only heat pump alongside the immersion heater
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"]["HeatSource"],
            "hwo_hp",
            integral_hw_only_heat_pump(),
        );
        // When validation is run, then no error: the exchanger area is tied to how the heat
        // pump and store are supplied, not to how many heat sources the tank has
        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_a_tank_cannot_be_both_integral_and_separate_from_its_heat_pumps(mut project: Value) {
        // Given one integral and one separately-installed hot-water-only heat pump on the
        // same tank, which the core model rejects whether or not an area is declared
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({
            "integral_hp": integral_hw_only_heat_pump(),
            "separate_hp": hw_only_heat_pump(json!({})),
        });
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: the separately-installed pump requires an area that the
        // integral one forbids, so no input can satisfy both
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'heat_exchanger_surface_area' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages)
    }

    #[rstest]
    fn test_an_unrecognised_property_is_named_rather_than_its_siblings(mut project: Value) {
        // Given a heat source carrying a misspelled property, with the input otherwise valid
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({
            "hwo_hp": hw_only_heat_pump(json!({"tank_is_integrall": true})),
        });
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"],
            "heat_exchanger_surface_area",
            json!(3.0),
        );
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then the offending property is named on its own. The unknown property makes the
        // tank's subschema fail, which would otherwise leave every sibling of the tank
        // reported as unexpected and bury this message
        let error_messages =
            "/HotWaterSource/hw cylinder/HeatSource/hwo_hp: Unevaluated properties are not allowed ('tank_is_integrall' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages])
    }

    #[rstest]
    fn test_an_unrecognised_property_on_the_tank_itself_is_still_named(mut project: Value) {
        // Given a misspelled property on the tank, where nothing below it fails
        insert_key_value(
            &mut project["HotWaterSource"]["hw cylinder"],
            "daily_lossses",
            json!(1.0),
        );
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is reported precisely: suppressing the unreliable report must not
        // suppress the reliable one
        let error_messages =
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed ('daily_lossses' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages])
    }

    #[rstest]
    fn test_an_unrecognised_property_is_deferred_behind_a_deeper_failure(mut project: Value) {
        // Given a tank carrying both an unrecognised property and a deeper failure
        let cylinder = &mut project["HotWaterSource"]["hw cylinder"];
        insert_key_value(cylinder, "bogus_property", json!(1.0));
        cylinder["HeatSource"] = json!({"hwo_hp": hw_only_heat_pump(json!({"power_max": -5}))});
        insert_key_value(cylinder, "heat_exchanger_surface_area", json!(1.0));
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then only the deeper failure is reported. Suppressing the tank's listing defers
        // the report of bogus_property rather than losing it: the input cannot pass while
        // power_max stands, and the property is named once power_max is corrected.
        let error_messages =
            "/HotWaterSource/hw cylinder/HeatSource/hwo_hp/power_max: -5 is less than or equal to the minimum of 0";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages])
    }

    #[rstest]
    fn test_an_unrecognised_property_resurfaces_once_the_deeper_failure_is_fixed(
        mut project: Value,
    ) {
        // Given the same input with the deeper failure corrected
        let cylinder = &mut project["HotWaterSource"]["hw cylinder"];
        insert_key_value(cylinder, "bogus_property", json!(1.0));
        cylinder["HeatSource"] = json!({"hwo_hp": hw_only_heat_pump(json!({}))});
        insert_key_value(cylinder, "heat_exchanger_surface_area", json!(1.0));
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then the deferred property is reported
        let error_messages =
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed ('bogus_property' was unexpected)";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_messages])
    }

    #[rstest]
    fn test_an_unrecognised_property_survives_an_unrelated_failure_elsewhere(mut project: Value) {
        // Given an unrecognised property on the tank, and a failure in an unrelated
        // subtree that is deeper than the tank. The depth must be greater for this to
        // test anything: suppression only considers errors deeper than the object, so a
        // shallower failure never reaches the check that the paths share a prefix.
        let cylinder = &mut project["HotWaterSource"]["hw cylinder"];
        insert_key_value(cylinder, "bogus_property", json!(1.0));
        project["Zone"]["zone 1"]["BuildingElement"]["wall 0"]["pitch"] = json!(999);
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then the tank's report survives: a deeper failure elsewhere in the input does
        // not explain it, so only a failure beneath the tank may suppress it
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed ('bogus_property' was unexpected)",
            "/Zone/zone 1/BuildingElement/wall 0/pitch: 999 is greater than the maximum of 120.0",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages)
    }

    #[rstest]
    fn test_a_tank_without_a_heat_source_is_not_told_to_remove_the_exchanger_area(
        mut project: Value,
    ) {
        // Given a tank missing HeatSource entirely but declaring an exchanger area
        let cylinder = &mut project["HotWaterSource"]["hw cylinder"];
        remove_key(cylinder, "HeatSource");
        insert_key_value(cylinder, "heat_exchanger_surface_area", json!(1.0));
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then the exchanger area is not among the errors. Both branches are guarded on
        // HeatSource being present, so neither fires: telling the assessor to remove the
        // exchanger area would point at a property needed once a heat source is given.
        // The object's own listing remains, being ambiguous rather than misleading.
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder: 'HeatSource' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages)
    }

    #[rstest]
    fn test_hw_only_heat_pump_must_declare_the_database_exchanger_area(mut project: Value) {
        // Given an integral hot-water-only heat pump omitting the declared exchanger area,
        // which the core model requires even though it reads it only when de-rating applies
        let heat_pump = &mut integral_hw_only_heat_pump();
        remove_key(heat_pump, "heat_exchanger_surface_area_declared");
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({ "hwo_hp": heat_pump });
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected here, naming the property, rather than later in the core
        // where the same omission surfaces as a bare "Field required"
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder/HeatSource/hwo_hp: Unevaluated properties are not allowed",
            "/HotWaterSource/hw cylinder/HeatSource/hwo_hp: 'heat_exchanger_surface_area_declared' is a required property",
        ];
        contains_expected_schema_validation_errors_partially(result, error_messages)
    }

    #[rstest]
    fn test_integral_hw_only_heat_pump_may_declare_a_null_exchanger_area(mut project: Value) {
        // Given an integral hot-water-only heat pump declaring the property as null, the
        // store being tested with the pump so there is no database area to compare against
        let heat_pump = &mut integral_hw_only_heat_pump();
        insert_key_value(
            heat_pump,
            "heat_exchanger_surface_area_declared",
            json!(null),
        );
        project["HotWaterSource"]["hw cylinder"]["HeatSource"] = json!({ "hwo_hp": heat_pump });
        // When validation is run, then no error: the core model types the field as optional
        // and never reads it for an integral unit, so no figure need be invented
        let result = apply_schema_validation(&project);
        // Then it is rejected here, naming the property, rather than later in the core
        // where the same omission surfaces as a bare "Field required"
        assert!(result.is_ok(), "{}", result.unwrap_err());
    }

    #[rstest]
    fn test_separate_hw_only_heat_pump_must_not_declare_a_null_exchanger_area(mut project: Value) {
        // Given a separately-installed hot-water-only heat pump declaring the property as
        // null, its tank supplying the exchanger area the de-rating compares against
        let cylinder = &mut project["HotWaterSource"]["hw cylinder"];
        insert_key_value(cylinder, "heat_exchanger_surface_area", json!(1.2));
        let hp = &mut hw_only_heat_pump(json!({"heat_exchanger_surface_area_declared": null}));
        cylinder["HeatSource"] = json!({"hwo_hp": hp});
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it is rejected: here the core does read the value, comparing it against the
        // installed area, so null would leave the de-rating with nothing to compare
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder/HeatSource/hwo_hp/heat_exchanger_surface_area_declared: null is not of type 'number'",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages)
    }

    #[rstest]
    fn test_heat_source_of_storage_tank_must_have_thermostat_position(mut project: Value) {
        // Given a StorageTank whose HeatSource does not have a thermostat_position
        let immersion = &mut project["HotWaterSource"]["hw cylinder"]["HeatSource"]["immersion"];
        remove_key(immersion, "thermostat_position");
        let result = apply_schema_validation(&project);
        let error_messages = &mut [
            "/HotWaterSource/hw cylinder/HeatSource/immersion: 'thermostat_position' is a required property",
        ];
        contains_expected_schema_validation_errors_exactly(result, error_messages)
    }

    #[rstest]
    fn test_preheated_tanks_may_not_themselves_be_preheated(mut project: Value) {
        // Given a PreHeatedWaterSource, with a ColdWaterSource of itself
        project["PreHeatedWaterSource"] = json!({
            "preheated tank": {
                "type": "StorageTank",
                "volume": 80.0,
                "daily_losses": 1.68,
                "init_temp": 35.0,
                "ColdWaterSource": "preheated tank",
                "HeatSource": {
                    "immersion": {
                        "type": "ImmersionHeater",
                        "power": 3.0,
                        "EnergySupply": "mains elec",
                        "heater_position": 0.1,
                        "thermostat_position": 0.33,
                    }
                },
            }
        });
        // When validation is run
        let result = apply_schema_validation(&project);
        // Then it should raise an error indicating that thermostat_position is required
        let error_message_partials = &mut [
            "/PreHeatedWaterSource/preheated tank:",
            "{'properties':{'ColdWaterSource':{'const':'preheated tank'}}}",
            "{'type':'StorageTank','volume':80.0,'daily_losses':1.68,'init_temp':35.0,'ColdWaterSource':'preheated tank','HeatSource':{'immersion':{'type':'ImmersionHeater','power':3.0,'EnergySupply':'mains elec','heater_position':0.1,'thermostat_position':0.33}}}",
        ];
        contains_expected_schema_validation_errors_partially(result, error_message_partials)
    }

    #[rstest]
    fn test_storage_tanks_may_use_preheated_source(mut project: Value) {
        // Given a PreHeatedWaterSource, with a ColdWaterSource of itself
        project["PreHeatedWaterSource"] = json!({
            "preheated tank": {
                "type": "StorageTank",
                "volume": 80.0,
                "daily_losses": 1.68,
                "init_temp": 35.0,
                "ColdWaterSource": "mains water",
                "HeatSource": {
                    "immersion": {
                        "type": "ImmersionHeater",
                        "power": 3.0,
                        "EnergySupply": "mains elec",
                        "heater_position": 0.1,
                        "thermostat_position": 0.33,
                    }
                },
            }
        });

        project["HotWaterSource"]["hw cylinder"]["ColdWaterSource"] = json!("preheated tank");

        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_storage_tanks_may_use_wwhrs_source(mut project: Value) {
        // Given a WWHRS used by a StorageTank as its ColdWaterSource
        project["WWHRS"] = json!({
            "wwhrs system": {
                "type": "WWHRS_Instantaneous",
                "ColdWaterSource": "mains water",
                "flow_rates": [1],
                "system_a_efficiencies": [100],
                "system_a_utilisation_factor": 0.972,
            }
        });

        project["HotWaterSource"]["hw cylinder"]["ColdWaterSource"] = json!("wwhrs system");

        assert!(apply_schema_validation(&project).is_ok());
    }

    #[rstest]
    fn test_preheated_tanks_may_use_wwhrs_source(mut project: Value) {
        // Given a WWHRS used by a PreHeatedWaterSource as its ColdWaterSource
        project["WWHRS"] = json!({
            "wwhrs system": {
                "type": "WWHRS_Instantaneous",
                "ColdWaterSource": "mains water",
                "flow_rates": [1],
                "system_a_efficiencies": [100],
                "system_a_utilisation_factor": 0.972,
            }
        });
        project["PreHeatedWaterSource"] = json!({
            "preheated tank": {
                "type": "StorageTank",
                "volume": 80.0,
                "daily_losses": 1.68,
                "init_temp": 35.0,
                "ColdWaterSource": "wwhrs system",
                "HeatSource": {
                    "immersion": {
                        "type": "ImmersionHeater",
                        "power": 3.0,
                        "EnergySupply": "mains elec",
                        "heater_position": 0.1,
                        "thermostat_position": 0.33,
                    }
                },
            }
        });
        assert!(apply_schema_validation(&project).is_ok());
    }
    #[rstest]
    fn test_showers_may_not_use_preheated_source(mut project: Value) {
        // Given a preheated tank used by a mixer shower as its source
        project["PreHeatedWaterSource"] = json!({
            "preheated tank": {
                "volume": 80.0,
                "daily_losses": 1.68,
                "init_temp": 35.0,
                "ColdWaterSource": "mains water",
                "HeatSource": {
                    "immersion": {
                        "type": "ImmersionHeater",
                        "power": 3.0,
                        "EnergySupply": "mains elec",
                        "heater_position": 0.1,
                        "thermostat_position": 0.33,
                    }
                },
            }
        });
        project["HotWaterDemand"]["Shower"]["mixer"]["ColdWaterSource"] = json!("preheated tank");
        let result = apply_schema_validation(&project);
        let error_message = "/HotWaterDemand/Shower/mixer/ColdWaterSource: 'preheated tank' is not one of 'header tank' or 'mains water'";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }

    #[rstest]
    fn test_showers_may_not_use_wwhrs(mut project: Value) {
        // Given a WWHRS system, used by a IES for its ColdWaterSource
        project["WWHRS"] = json!({
            "wwhrs system": {
                "type": "WWHRS_Instantaneous",
                "ColdWaterSource": "mains water",
                "flow_rates": [1],
                "system_a_efficiencies": [100],
                "system_a_utilisation_factor": 0.972,
            }
        });
        project["HotWaterDemand"]["Shower"]["mixer"]["ColdWaterSource"] = json!("wwhrs system");
        let result = apply_schema_validation(&project);
        let error_message = "/HotWaterDemand/Shower/mixer/ColdWaterSource: 'wwhrs system' is not one of 'header tank' or 'mains water'";
        contains_expected_schema_validation_errors_exactly(result, &mut [error_message]);
    }
}
