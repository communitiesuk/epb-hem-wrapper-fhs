use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use itertools::Itertools;
use serde_json::Value;
use std::fs;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};

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
    preprocessed_input_matches_expected(&actual_output, &expected_output, vec![]);
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

pub(crate) fn preprocessed_input_matches_expected(
    actual: &Value,
    expected: &Value,
    path_to_node: Vec<&str>,
) {
    let mut expected_keys = actual.as_object().unwrap().keys().collect_vec();
    expected_keys.sort();
    let mut actual_keys = expected.as_object().unwrap().keys().collect_vec();
    actual_keys.sort();
    let mut path_to_node = path_to_node;

    assert_eq!(actual_keys, expected_keys);
    for key in expected_keys {
        if key == "Events" {
            continue;
        }
        path_to_node.push(key);
        if expected[key].as_object().is_some() {
            preprocessed_input_matches_expected(&expected[key], &actual[key], path_to_node.clone());
        } else {
            assert_eq!(actual[key], expected[key], "{:?}", path_to_node);
        }
        path_to_node.pop();
    }
}
