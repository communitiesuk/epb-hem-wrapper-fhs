use home_energy_model::output_writer::OutputWriter;
use home_energy_model::read_weather_file::{
    cibse_weather_data_to_external_conditions, ExternalConditions,
};
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use rayon::prelude::*;
use serde_json::{json, Number, Value};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock};
use std::{assert_eq, fs};

mod common;
use common::{DEMO_FILES_DIR, FLOAT_THRESHOLD};

pub(crate) const PROVIDED_EXPECTED_OUTPUT_DIR: &'static str =
    "./tests/e2e/expected_provided_results/future_homes_standard/";
const GENERATED_EXPECTED_OUTPUT_DIR: &'static str = "./tests/e2e/expected_generated_results/";
const ERRORS_TO_PRINT: usize = 10;

static MODE_OUTPUTS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    HashMap::from([
        ("actual", "FHS__preproc"),
        ("actual-FEE", "FHS_FEE__preproc"),
        ("notional", "FHS_notional__preproc"),
        ("notional-FEE", "FHS_FEE_notional__preproc"),
    ])
});

#[test]
fn test_fhs_preprocessing_output_against_provided_results() {
    let demo_file_names = [
        "DESN-H-End-02-ESH-cMEV",
        "DESN-H-End-02-HP-iMEV-pre-heat",
        "DESN-H-End-02-HP-iMEV-wwhrs-storage-tank",
        "demo_FHS",
    ];

    let mut total_difference_count = 0;
    let mut differences = vec![];

    for file_name in &demo_file_names {
        let external_conditions = get_external_conditions_for(file_name);
        let output_writer = InMemoryDirectoryOutputWriter::new(file_name);

        run_fhs_preprocessing(file_name, external_conditions, &output_writer);

        let files = output_writer.files();
        let file_differences =
            get_file_differences(PROVIDED_EXPECTED_OUTPUT_DIR, file_name, &files);

        if file_differences.len() > 0 {
            println!(
                "\nmismatches found for {file_name}: {}\n\n{:-^120}",
                file_differences.join(", "),
                ""
            )
        }

        differences.push(format!("{file_name}: {}", file_differences.join(", "),));
        total_difference_count += file_differences.len();
    }

    assert_eq!(
        total_difference_count,
        0,
        "\n\nTotal mismatches found against provided result files: {}\n{}\n\n",
        total_difference_count,
        differences.join("\n"),
    )
}

#[test]
fn test_fhs_preprocessing_output_against_generated_results() {
    let differences: Vec<(String, usize)> = fs::read_dir(DEMO_FILES_DIR)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<PathBuf>>()
        .into_par_iter()
        .map(|path| {
            let demo_input_file_name = path.clone();
            let demo_input_file_name = demo_input_file_name.file_stem().unwrap().to_str().unwrap();
            let output_writer = InMemoryDirectoryOutputWriter::new(demo_input_file_name);

            run_fhs_preprocessing(demo_input_file_name, None, &output_writer);

            let files = output_writer.files();
            let file_differences =
                get_file_differences(GENERATED_EXPECTED_OUTPUT_DIR, &demo_input_file_name, &files);

            if file_differences.len() > 0 {
                println!(
                    "\nmismatches found for {demo_input_file_name}: {}\n\n{:-^120}",
                    file_differences.join(", "),
                    ""
                )
            }
            (
                format!("{demo_input_file_name}: {}", file_differences.join(", "),),
                file_differences.len(),
            )
        })
        .collect();
    let (differences, total_difference_count) =
        differences
            .into_iter()
            .fold((Vec::new(), 0), |(mut names, total), (name, count)| {
                names.push(name);
                (names, total + count)
            });

    assert_eq!(
        total_difference_count,
        0,
        "\n\nTotal mismatches found against generated result files: {}\n{}\n\n",
        total_difference_count,
        differences.join("\n"),
    )
}

fn mode_differences(
    demo_input_file_name: &str,
    failing_modes: &mut Vec<String>,
    mode: &str,
    actual_files: &IndexMap<String, String>,
    expected_output_dir: &str,
) -> usize {
    let (actual_output, expected_output) = file_values(
        demo_input_file_name,
        mode,
        actual_files,
        expected_output_dir,
    );

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
    input_file_name: &str,
    external_conditions: Option<ExternalConditions>,
    output_writer: &impl OutputWriter,
) -> () {
    let input_file_path = &format!("{DEMO_FILES_DIR}{input_file_name}.json");
    let input_file_path = Path::new(input_file_path);
    let input = BufReader::new(File::open(input_file_path).unwrap());

    println!("\nStarting to run Rust FHS preprocessing for: {input_file_name}.json");
    let result = run_wrappers(
        input,
        output_writer,
        external_conditions,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        true,
        false,
        false,
        &[OutputFormat::Json],
    );
    assert!(
        result.is_ok(),
        "\nError running Rust FHS preprocessing for: {}.json",
        input_file_name
    );
    println!("Finished running Rust FHS preprocessing for: {input_file_name}.json");
}

fn file_values(
    file_name: &str,
    mode: &str,
    actual_files: &IndexMap<String, String>,
    expected_output_dir: &str,
) -> (Value, Value) {
    let suffix = MODE_OUTPUTS.get(mode).expect("Invalid mode");
    let filename_with_suffix = format!("{file_name}__{suffix}.json");

    let actual_str = actual_files.get(&filename_with_suffix).unwrap();
    let actual_output = serde_json::from_str(&actual_str).unwrap();

    let expected_path = format!("{expected_output_dir}{file_name}__results/{filename_with_suffix}");
    let expected_str = fs::read_to_string(&expected_path)
        .unwrap_or_else(|_| panic!("Output file not found at {expected_path}"));
    let expected_output = serde_json::from_str(&expected_str).unwrap();

    (actual_output, expected_output)
}

fn get_file_differences(
    expected_output_dir: &str,
    file_name: &&str,
    files: &IndexMap<String, String>,
) -> Vec<String> {
    let mut file_differences = vec![];
    for mode in ["actual", "actual-FEE", "notional", "notional-FEE"] {
        mode_differences(
            file_name,
            &mut file_differences,
            mode,
            files,
            expected_output_dir,
        );
    }

    file_differences.to_vec()
}

fn get_external_conditions_for(file_name: &&str) -> Option<ExternalConditions> {
    // In the Python the London weather file is only specified for demo_FHS,
    // the other cases use the default one
    let use_london_weather_file = *file_name == "demo_FHS";
    let external_conditions = use_london_weather_file.then_some(
        cibse_weather_data_to_external_conditions(BufReader::new(
            File::open("./examples/input/London_weather_CIBSE_format.csv").unwrap(),
        ))
        .unwrap(),
    );
    external_conditions
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
                // skip comparing "priority" due to know bug
                // TODO stop skipping "priority" when we migrate to alpha 8/9
                let keys_to_skip = ["priority".to_string()];
                if keys_to_skip.contains(key) {
                    continue;
                }
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

#[derive(Clone, Debug)]
struct FileWriter(Arc<RwLock<Vec<u8>>>);

impl FileWriter {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(Vec::with_capacity(2usize.pow(14)))))
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write().extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct InMemoryDirectoryOutputWriter {
    input_filename: String,
    files: Arc<Mutex<IndexMap<String, FileWriter>>>,
}

impl InMemoryDirectoryOutputWriter {
    fn new(input_filename: &str) -> Self {
        Self {
            input_filename: input_filename.split('.').next().unwrap().to_string(),
            files: Arc::new(Mutex::new(IndexMap::new())),
        }
    }

    fn output_file_index(&self, location_key: &str, file_extension: &str) -> String {
        format!(
            "{}__{}.{}",
            self.input_filename, location_key, file_extension
        )
    }

    pub fn files(&self) -> IndexMap<String, String> {
        self.files
            .lock()
            .iter()
            .map(|(k, v)| {
                let bytes = v.0.read();
                let string_content = String::from_utf8_lossy(&bytes).to_string();
                (k.clone(), string_content)
            })
            .collect()
    }
}

impl OutputWriter for InMemoryDirectoryOutputWriter {
    fn writer_for_location_key(
        &self,
        location_key: &str,
        file_extension: &str,
    ) -> anyhow::Result<impl Write> {
        let key = self.output_file_index(location_key, file_extension);

        let file_writer = self
            .files
            .lock()
            .entry(key)
            .or_insert_with(FileWriter::new)
            .clone();

        // BufWriter prevents acquiring the RwLock on every byte chunk
        Ok(BufWriter::with_capacity(2usize.pow(14), file_writer))
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
