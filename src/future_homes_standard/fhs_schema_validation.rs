use jsonschema::Validator;
use serde_json::Value;
use std::sync::LazyLock;
use thiserror::Error;

static FHS_SCHEMA_VALIDATOR: LazyLock<Validator> = LazyLock::new(|| {
    let schema = serde_json::from_str(include_str!("../../schema/input_fhs.schema.json")).unwrap();
    jsonschema::validator_for(&schema).unwrap()
});

#[derive(Debug, Error)]
#[error("Invalid JSON against the FHS schema: {errors}")]
pub struct SchemaValidationError {
    pub errors: String,
}

pub(crate) fn apply_schema_validation(input: &Value) -> Result<(), SchemaValidationError> {
    let evaluation = FHS_SCHEMA_VALIDATOR.evaluate(input);
    if evaluation.flag().valid {
        Ok(())
    } else {
        Err(SchemaValidationError {
            errors: evaluation
                .iter_errors()
                .map(|e| e.error.to_string())
                .collect::<Vec<_>>()
                .join("\n"),
        })
    }
}
