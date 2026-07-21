use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model::read_weather_file::cibse_weather_data_to_external_conditions;
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use rstest::rstest;
use serde_json::{json, Number, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const FLOAT_THRESHOLD: f64 = 1e-6; // 0.000001

const ERRORS_TO_PRINT: usize = 10;

static MODE_OUTPUTS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("actual", "FHS__preproc"),
        ("actual-FEE", "FHS_FEE__preproc"),
        ("notional", "FHS_notional__preproc"),
        ("notional-FEE", "FHS_FEE_notional__preproc"),
    ])
});

#[rstest]
#[case("DESN-H-End-02-ESH-cMEV", false)]
#[case("DESN-H-End-02-HP-iMEV-pre-heat", false)]
#[case("DESN-H-End-02-HP-iMEV-wwhrs-storage-tank", false)]
#[case("demo_FHS", true)] // expected results folder is called demo_fhs_with_weather_file in Python
fn test_fhs_preprocessing_output_against_expected_results(
    #[case] demo_input_file_name: &str,
    #[case] specify_weather_file: bool,
) {
    let demo_input_file = BufReader::new(
        File::open(Path::new(&format!(
            "./examples/input/future_homes_standard/{demo_input_file_name}.json"
        )))
        .unwrap(),
    );

    let weather_file = specify_weather_file.then_some(
        cibse_weather_data_to_external_conditions(
            File::open("./examples/input/London_weather_CIBSE_format.csv").unwrap(),
        )
        .unwrap(),
    );

    let temporary_output_dir = "./tests/e2e/test_future_homes_standard_outputs/";
    let temporary_output_sub_dir =
        create_temporary_output_directory(temporary_output_dir, demo_input_file_name);
    let output_writer = FileOutputWriter::new(
        temporary_output_sub_dir.clone(),
        format!("{demo_input_file_name}__{{}}.{{}}"),
    );

    let result = run_wrappers(
        demo_input_file,
        output_writer,
        weather_file,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        true,
        false,
        false,
        &[OutputFormat::Json],
    );
    assert!(result.is_ok());

    let mut difference_count = 0;
    let mut failing_modes = vec![];
    for mode in ["actual", "actual-FEE", "notional", "notional-FEE"] {
        let expected_output_dir = "./tests/e2e/expected_provided_results/future_homes_standard/";
        let expected_output = file_value(expected_output_dir, demo_input_file_name, mode);
        let actual_output = file_value(temporary_output_dir, demo_input_file_name, mode);
        let mut errors = vec![];
        let mode_difference_count = preprocessed_input_matches_expected(
            &actual_output,
            &expected_output,
            vec![],
            &mut errors,
        );
        if mode_difference_count > 0 {
            println!("\nMode '{mode}' for {demo_input_file_name}.json had {mode_difference_count} mismatches:\n");
            print_differences(&errors, Some(ERRORS_TO_PRINT));
            failing_modes.push(format!("{mode}: {mode_difference_count}"));
            difference_count += mode_difference_count;
        }
    }

    delete_temporary_output_directory(temporary_output_dir, temporary_output_sub_dir);

    assert_eq!(
        difference_count,
        0,
        "mismatches found for {demo_input_file_name}: {}\n",
        failing_modes.join(", ")
    );
}

#[test]
fn test_fhs_preprocessing_output_against_generated_results_from_python() {
    let demo_files_dir = Path::new("examples/input/future_homes_standard");
    let mut total_difference_count = 0;
    let mut differences = vec![];

    for entry in fs::read_dir(demo_files_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();

        if !path.is_dir() && entry.file_name().to_str().unwrap().ends_with(".json") {
            let demo_input_file_name = path.clone();
            let demo_input_file_name = demo_input_file_name.file_stem().unwrap().to_str().unwrap();

            let temporary_output_dir = "./tests/e2e/test_future_homes_standard_outputs/";
            let temporary_output_sub_dir =
                create_temporary_output_directory(temporary_output_dir, demo_input_file_name);

            run_fhs_preprocessing(path, demo_input_file_name, &temporary_output_sub_dir);

            let mut file_difference_count = 0;
            let mut file_differences = vec![];
            for mode in ["actual", "actual-FEE", "notional", "notional-FEE"] {
                file_difference_count += mode_differences(
                    demo_input_file_name,
                    temporary_output_dir,
                    &mut file_differences,
                    mode,
                );
            }
            if file_difference_count > 0 {
                println!(
                    "\nmismatches found for {demo_input_file_name}: {}\n\n{:-^120}",
                    file_differences.join(", "),
                    ""
                )
            }
            differences.push(format!(
                "{demo_input_file_name}: {}",
                file_differences.join(", "),
            ));
            total_difference_count += file_difference_count;
            delete_temporary_output_directory(temporary_output_dir, temporary_output_sub_dir);
        }
    }

    assert_eq!(
        total_difference_count,
        0,
        "\n\nTotal mismatches found: {}\n{}\n\n",
        total_difference_count,
        differences.join("\n"),
    )
}

fn mode_differences(
    demo_input_file_name: &str,
    temporary_output_dir: &str,
    failing_modes: &mut Vec<String>,
    mode: &str,
) -> usize {
    let expected_output_dir = "./tests/e2e/expected_generated_results/";
    let expected_output = file_value(expected_output_dir, demo_input_file_name, mode);
    let actual_output = file_value(temporary_output_dir, demo_input_file_name, mode);
    let mut errors = vec![];
    let mode_difference_count =
        preprocessed_input_matches_expected(&actual_output, &expected_output, vec![], &mut errors);
    if mode_difference_count > 0 {
        println!("\nMode '{mode}' for {demo_input_file_name}.json had {mode_difference_count} mismatches:\n");
        print_differences(&errors, Some(ERRORS_TO_PRINT));
        failing_modes.push(format!("{mode}: {mode_difference_count}"));
    }
    mode_difference_count
}

fn run_fhs_preprocessing(
    path: PathBuf,
    demo_input_file_name: &str,
    temporary_output_sub_dir: &PathBuf,
) -> () {
    let demo_input = BufReader::new(File::open(path).unwrap());

    let output_writer = FileOutputWriter::new(
        temporary_output_sub_dir.clone(),
        format!("{}__{{}}.{{}}", demo_input_file_name),
    );

    println!("\nStarting to run {demo_input_file_name}");
    let result = run_wrappers(
        demo_input,
        output_writer,
        None,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        true,
        false,
        false,
        &[OutputFormat::Json],
    );
    if result.is_err() {
        println!(
            "\nError running fhs preprocessing for: {}",
            demo_input_file_name
        );
    }
    assert!(result.is_ok());
    println!("Finished running {demo_input_file_name}");
}

fn file_value(directory: &str, file_name: &str, mode: &str) -> Value {
    let suffix = MODE_OUTPUTS.get(mode).expect("Invalid mode");
    let file_path = format!("{directory}{file_name}__results/{file_name}__{suffix}.json");
    let file =
        fs::read_to_string(&file_path).expect(&format!("Output file not found at {file_path}"));
    let output: Value = serde_json::from_str(&file).unwrap();
    output
}

fn create_temporary_output_directory(directory: &str, demo_file_name: &str) -> PathBuf {
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(format!("{directory}{demo_file_name}__results"));
    fs::create_dir_all(&temp_output_dir).unwrap();
    temp_output_dir
}

fn delete_temporary_output_directory(parent_directory: &str, sub_directory: PathBuf) {
    fs::remove_dir_all(&sub_directory).unwrap();
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(parent_directory);
    let _ = fs::remove_dir(temp_output_dir);
}

pub struct Location(Vec<String>);
impl Location {
    pub fn format(&self) -> String {
        if self.0.is_empty() {
            return "root".to_string();
        }
        self.0.join(" - ")
    }
}

pub enum MismatchType {
    Numerical {
        actual: f64,
        expected: f64,
        location: Location,
    },
    Boolean {
        actual: bool,
        expected: bool,
        location: Location,
    },
    String {
        actual: String,
        expected: String,
        location: Location,
    },
    ArrayLength {
        actual: usize,
        expected: usize,
        location: Location,
    },
    MissingKey {
        key: String,
        location: Location,
    },
    AdditionalKey {
        key: String,
        location: Location,
    },
    DifferentType {
        actual: String,
        expected: String,
        location: Location,
    },
}
pub(crate) fn preprocessed_input_matches_expected(
    actual: &Value,
    expected: &Value,
    path_to_node: Vec<String>,
    errors: &mut Vec<MismatchType>,
) -> usize {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            if !(a == b || numbers_match_as_floats(a, b)) {
                errors.push(MismatchType::Numerical {
                    actual: a.as_f64().unwrap(),
                    expected: b.as_f64().unwrap(),
                    location: Location(path_to_node.clone()),
                })
            };
        }
        (Value::Bool(a), Value::Bool(b)) => {
            if a != b {
                errors.push(MismatchType::Boolean {
                    actual: *a,
                    expected: *b,
                    location: Location(path_to_node.clone()),
                })
            };
        }
        (Value::Null, Value::Null) => (),
        (Value::String(a), Value::String(b)) => {
            if a != b {
                errors.push(MismatchType::String {
                    actual: a.to_string(),
                    expected: b.to_string(),
                    location: Location(path_to_node.clone()),
                });
            };
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                errors.push(MismatchType::ArrayLength {
                    actual: a.len(),
                    expected: b.len(),
                    location: Location(path_to_node.clone()),
                });
            } else {
                for (index, (actual_value, expected_value)) in a.iter().zip(b.iter()).enumerate() {
                    let mut path_to_node = path_to_node.clone();
                    path_to_node.push(format!("array_index_{}", index));
                    preprocessed_input_matches_expected(
                        actual_value,
                        expected_value,
                        path_to_node,
                        errors,
                    );
                }
            }
        }
        (Value::Object(actual_obj), Value::Object(expected_obj)) => {
            let mut actual_keys: Vec<&String> = actual_obj.keys().collect();
            let mut expected_keys: Vec<&String> = expected_obj.keys().collect();

            actual_keys.sort();
            expected_keys.sort();

            if actual_keys != expected_keys {
                let missing_keys: Vec<String> = expected_keys
                    .iter()
                    .filter(|key| !actual_keys.contains(key))
                    .map(|key| (*key).clone())
                    .collect();
                for key in missing_keys {
                    errors.push(MismatchType::MissingKey {
                        key,
                        location: Location(path_to_node.clone()),
                    });
                }
                let additional_keys: Vec<String> = actual_keys
                    .iter()
                    .filter(|key| !expected_keys.contains(key))
                    .map(|key| (*key).clone())
                    .collect();
                for key in additional_keys {
                    let actual_value = actual_obj.get(&key).unwrap_or(&Value::Null);

                    // don't record 'additional key' difference if its value is empty object
                    if let Some(actual) = actual_value.as_object() {
                        if actual.len() == 0 {
                            continue;
                        }
                    }
                    errors.push(MismatchType::AdditionalKey {
                        key,
                        location: Location(path_to_node.clone()),
                    });
                }
            }
            for key in expected_keys {
                let actual_value = actual_obj.get(key).unwrap_or(&Value::Null);
                let expected_value = expected_obj.get(key).unwrap_or(&Value::Null);
                if actual_value.is_null() {
                    continue;
                }
                let mut path_to_node = path_to_node.clone();
                path_to_node.push(key.clone());
                preprocessed_input_matches_expected(
                    actual_value,
                    expected_value,
                    path_to_node,
                    errors,
                );
            }
        }
        _ => {
            errors.push(MismatchType::DifferentType {
                actual: format!("{:?}", actual),
                expected: format!("{:?}", expected),
                location: Location(path_to_node.clone()),
            });
        }
    };
    errors.len()
}

fn numbers_match_as_floats(actual: &Number, expected: &Number) -> bool {
    let actual = actual.as_f64().unwrap();
    let expected = expected.as_f64().unwrap();
    (actual - expected).abs() < FLOAT_THRESHOLD
}

fn print_differences(differences: &[MismatchType], max_to_print: Option<usize>) {
    let iter = differences.iter().take(max_to_print.unwrap_or(usize::MAX));

    for mismatch in iter {
        match mismatch {
            MismatchType::Numerical {
                actual,
                expected,
                location,
            } => {
                println!(
                    "Numerical mismatch at {}: actual = {}, expected = {}",
                    location.format(),
                    actual,
                    expected
                );
            }
            MismatchType::Boolean {
                actual,
                expected,
                location,
            } => {
                println!(
                    "Boolean mismatch at {}: actual = {}, expected = {}",
                    location.format(),
                    actual,
                    expected
                );
            }
            MismatchType::String {
                actual,
                expected,
                location,
            } => {
                println!(
                    "String mismatch at {}: actual = {}, expected = {}",
                    location.format(),
                    actual,
                    expected
                );
            }
            MismatchType::ArrayLength {
                actual,
                expected,
                location,
            } => {
                println!(
                    "Array length mismatch at {}: actual = {}, expected = {}",
                    location.format(),
                    actual,
                    expected
                );
            }
            MismatchType::MissingKey { key, location } => {
                println!("Missing key '{}' at {}", key, location.format());
            }
            MismatchType::AdditionalKey { key, location } => {
                println!("Additional key '{}' at {}", key, location.format());
            }
            MismatchType::DifferentType {
                actual,
                expected,
                location,
            } => {
                println!(
                    "Different type at {}: actual = {}, expected = {}",
                    location.format(),
                    actual,
                    expected
                );
            }
        }
    }
}

mod test_preprocessed_input_matches_expected {
    use super::*;

    #[test]
    fn test_number_differences() {
        let actual = json!(1.0);
        let expected = json!(1.0);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(1.0);
        let expected = json!(1);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(1.3);
        let expected = json!(1.0);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_boolean_differences() {
        let actual = json!(false);
        let expected = json!(false);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!(true);
        let expected = json!(false);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_string_differences() {
        let actual = json!("test");
        let expected = json!("test");
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!("t3st");
        let expected = json!("test");
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_array_differences() {
        let actual = json!([1, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!([1.0, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!([2, 2, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!([3, 3, 3]);
        let expected = json!([1, 2, 3]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 2);

        let actual = json!([1, 2, 3]);
        let expected = json!([1, 2]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_array_vs_non_array() {
        let actual = json!([1, 2, 3]);
        let expected = json!(1);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!(1);
        let expected = json!([1, 2, 3]);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_null_differences() {
        let actual = json!(null);
        let expected = json!(null);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);
    }

    #[test]
    fn test_null_vs_non_null() {
        let actual = json!(null);
        let expected = json!(1);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!(1);
        let expected = json!(null);
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_object_different_number_of_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": 1});
        let expected = json!({"a": 1, "b": 2, "c": 3});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 2);
    }

    #[test]
    fn test_object_same_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "b": 2});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"b": 2, "a": 1});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "b": 3});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": 1, "b": 1, "c": 1});
        let expected = json!({"a": 2, "b": 2, "c": 2});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 3);
    }

    #[test]
    fn test_object_different_keys() {
        let actual = json!({"a": 1, "b": 2});
        let expected = json!({"a": 1, "c": 2});
        let errors = &mut vec![];
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], errors);
        print!("Errors: {:?}", errors.len());
        assert_eq!(difference_count, 2);
    }

    #[test]
    fn test_nested_structures() {
        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 3}]});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 4}]});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2]});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);

        let actual = json!({"a": [1, 2, {"b": 3}]});
        let expected = json!({"a": [1, 2, {"b": 5}]});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }

    #[test]
    fn test_events_key_is_skipped() {
        let actual = json!({"Events": 1});
        let expected = json!({"Events": 2});
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);
    }

    #[test]
    fn test_whole_file() {
        let file = fs::read_to_string("tests/e2e/expected_provided_results/future_homes_standard/demo_fhs__results/demo_FHS__FHS__preproc.json").expect("Output file not found");
        let actual: Value = serde_json::from_str(&file).unwrap();
        let expected = serde_json::from_str(&file).unwrap();
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 0);

        let mut actual: Value = serde_json::from_str(&file).unwrap();
        if let Value::Object(object) = &mut actual {
            object.remove("temp_internal_air_static_calcs").unwrap();
        }
        let difference_count =
            preprocessed_input_matches_expected(&actual, &expected, vec![], &mut vec![]);
        assert_eq!(difference_count, 1);
    }
}
