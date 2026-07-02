use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use serde_json::{json, Value};
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

const FLOAT_THRESHOLD: f64 = 1e-6; // 0.000001

#[test]
fn test_demo_file_preprocessing_output() {
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input_file = BufReader::new(
        File::open(Path::new(&format!(
            "./examples/input/future_homes_standard/{demo_input_file_name}.json"
        )))
        .unwrap(),
    );

    let temporary_output_dir = "./tests/e2e/test_future_homes_standard_outputs/";
    let temporary_output_dir_path =
        create_temporary_output_directory(temporary_output_dir, demo_input_file_name);
    let output_writer = FileOutputWriter::new(
        temporary_output_dir_path.clone(),
        format!("{demo_input_file_name}__{{}}.{{}}"),
    );

    let result = run_wrappers(
        demo_input_file,
        output_writer,
        None,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        true,
        false,
        false,
        &[OutputFormat::Json],
    );
    assert!(result.is_ok());

    let expected_output_dir = "./tests/e2e/expected_results/future_homes_standard/";
    let expected_output = file_value(expected_output_dir, demo_input_file_name);
    let actual_output = file_value(temporary_output_dir, demo_input_file_name);
    delete_temporary_output_directory(temporary_output_dir);
    let difference_count =
        preprocessed_input_matches_expected(&actual_output, &expected_output, vec![]);
    assert_eq!(difference_count, 0);
}

fn file_value(directory: &str, file_name: &str) -> Value {
    let file_path = format!("{directory}/{file_name}__results/{file_name}__FHS__preproc.json");
    let file = fs::read_to_string(&file_path).expect("Output file not found");
    let output: Value = serde_json::from_str(&file).unwrap();
    output
}

fn create_temporary_output_directory(directory: &str, demo_file_name: &str) -> PathBuf {
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(format!("{directory}/{demo_file_name}__results"));
    fs::create_dir_all(&temp_output_dir).unwrap();
    temp_output_dir
}

fn delete_temporary_output_directory(directory: &str) {
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(format!("{directory}"));
    fs::remove_dir_all(temp_output_dir).unwrap();
}

mod test_preprocessed_input_matches_expected {
    use super::*;
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_number_differences() {
        let actual = json!(1.0);
        let expected = json!(1.0);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(1.0);
        let expected = json!(1);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(1.3);
        let expected = json!(1.0);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_boolean_differences() {
        let actual = json!(false);
        let expected = json!(false);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(true);
        let expected = json!(false);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }

    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_string_differences() {
        let actual = json!("test");
        let expected = json!("test");
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!("t3st");
        let expected = json!("test");
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_array_differences() {
        let actual = json!([1, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!([1.0, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!([2, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!([3, 3, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 2);

        let actual = json!([1, 2, 3]);
        let expected = json!([1, 2]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_array_vs_non_array() {
        let actual = json!([1, 2, 3]);
        let expected = json!(1);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!(1);
        let expected = json!([1, 2, 3]);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_null_differences() {
        let actual = json!(null);
        let expected = json!(null);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_null_vs_non_null() {
        let actual = json!(null);
        let expected = json!(1);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!(1);
        let expected = json!(null);
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_object_different_number_of_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": 1});
        let expected = json!({"a": 1, "b": 2, "c": 3});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_object_same_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "b": 2});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"b": 2, "a": 1});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "b": 3});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": 1, "b": 1, "c": 1});
        let expected = json!({"a": 2, "b": 2, "c": 2});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 3);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_object_different_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "c": 2});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
    #[ignore = "Ignored to reduce noise"]
    #[test]
    fn test_nested_structures() {
        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 3}]});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 4}]});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2]});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 5}]});
        let difference_count = preprocessed_input_matches_expected(&actual, &expected, vec![]);
        assert_eq!(difference_count, 1);
    }
}
pub(crate) fn preprocessed_input_matches_expected(
    actual: &Value,
    expected: &Value,
    path_to_node: Vec<String>,
) -> usize {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            if a == b || values_match_as_numbers(actual, expected) {
                0
            } else {
                println!(
                    "Number values do not match at path: {:?}. Actual value: {}, Expected value: {}, difference: {}",
                    path_to_node, actual, expected, actual.as_f64().unwrap() - expected.as_f64().unwrap()
                );
                1
            }
        }
        (Value::Bool(a), Value::Bool(b)) => {
            if a == b {
                0
            } else {
                println!(
                    "Boolean values do not match at path: {:?}. Actual value: {}, Expected value: {}",
                    path_to_node, a, b
                );
                1
            }
        }
        (Value::Null, Value::Null) => 0,
        (Value::String(a), Value::String(b)) => {
            if a == b {
                0
            } else {
                println!(
                    "String values do not match at path: {:?}. Actual value: {}, Expected value: {}",
                    path_to_node, a, b
                );
                1
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            let mut difference_count = 0;
            if a.len() != b.len() {
                println!(
                    "Array lengths do not match at path: {:?}. Actual length: {}, Expected length: {}",
                    path_to_node,
                    a.len(),
                    b.len()
                );
                return 1;
            }
            for (index, (actual_value, expected_value)) in a.iter().zip(b.iter()).enumerate() {
                let mut path_to_node = path_to_node.clone();
                path_to_node.push(format!("array_index_{}", index));
                difference_count +=
                    preprocessed_input_matches_expected(actual_value, expected_value, path_to_node);
            }
            difference_count
        }
        (Value::Object(a), Value::Object(b)) => {
            let mut a_keys: Vec<&String> = a.keys().collect();
            let mut b_keys: Vec<&String> = b.keys().collect();

            a_keys.sort();
            b_keys.sort();

            if a_keys != b_keys {
                println!(
                    "Object keys do not match at path: {:?}. Actual keys: {:?}, Expected keys: {:?}",
                    path_to_node, a_keys, b_keys
                );
                return 1;
            }
            let mut difference_count = 0;
            for key in a_keys {
                let actual_value = a.get(key).unwrap_or(&Value::Null);
                let expected_value = b.get(key).unwrap_or(&Value::Null);
                let mut path_to_node = path_to_node.clone();
                path_to_node.push(key.clone());
                difference_count +=
                    preprocessed_input_matches_expected(actual_value, expected_value, path_to_node);
            }
            difference_count
        }
        _ => 1,
    }
}

fn values_match_as_numbers(actual_value: &Value, expected_value: &Value) -> bool {
    let actual_value = actual_value.as_f64();
    let expected_value = expected_value.as_f64();
    let values_are_numbers = actual_value.is_some() && expected_value.is_some();

    if values_are_numbers
        && (actual_value.unwrap() - expected_value.unwrap()).abs() < FLOAT_THRESHOLD
    {
        return true;
    }
    false
}
