use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use serde_json::{json, Value};
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

fn test_number_differences() {
    let actual = json!(1.0);
    let expected = json!(1.0);
    let difference_cout = preprocessed_input_matches_expected(&actual, &expected, vec![]);
    assert_eq!(difference_cout, 0);

    let actual = json!(1.0);
    let expected = json!(1);
    let difference_cout = preprocessed_input_matches_expected(&actual, &expected, vec![]);
    assert_eq!(difference_cout, 0);

    let actual = json!(1.3);
    let expected = json!(1.0);
    let difference_cout = preprocessed_input_matches_expected(&actual, &expected, vec![]);
    assert_eq!(difference_cout, 1);
}

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

fn test_string_differences() {
    let actual = json!("test");
    let expected = json!("test");
    let difference_cout = preprocessed_input_matches_expected(&actual, &expected, vec![]);
    assert_eq!(difference_cout, 0);

    let actual = json!("t3st");
    let expected = json!("test");
    let difference_cout = preprocessed_input_matches_expected(&actual, &expected, vec![]);
    assert_eq!(difference_cout, 1);
}

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

#[test]
fn test_preprocessed_input_matches_expected() {
    test_number_differences();
    test_boolean_differences();
    test_string_differences();
    test_array_differences();
    test_array_vs_non_array();
}
pub(crate) fn preprocessed_input_matches_expected(
    actual: &Value,
    expected: &Value,
    mut path_to_node: Vec<String>,
) -> isize {
    if actual == expected {
        return 0;
    }

    let mut differences_in_array = |actual: &Value, expected: &Value| -> isize {
        let actual_as_array = actual.as_array();
        let expected_as_array = expected.as_array();

        let mut difference_count: isize = 0;

        match (actual_as_array, expected_as_array) {
            (Some(actual), Some(expected)) => {
                if actual.len() != expected.len() {
                    difference_count += 1;
                    println!(
                        "Expected array to have length of {:?}, but got {:?} at {:?} \n",
                        expected.len(),
                        actual.len(),
                        path_to_node
                    );
                }

                // } else {
                for (i, expected_item) in expected.iter().enumerate() {
                    path_to_node.push(i.to_string());
                    difference_count += preprocessed_input_matches_expected(
                        &actual[i.clone()],
                        expected_item,
                        path_to_node.clone(),
                    );
                    path_to_node.pop();
                }
                difference_count
            }
            (Some(_), None) | (None, Some(_)) => 1,
            (None, None) => -1,
        }
    };

    let array_differences: isize = differences_in_array(actual, expected);
    if array_differences == -1 {
        if values_match_as_numbers(actual, expected) {
            return 0;
        } else {
            return 1;
        }
    }
    array_differences

    //
    // let mut differences_in_object = |actual: &Value, expected: &Value| -> usize {
    //     let actual_as_object = actual.as_object();
    //     let expected_as_object = expected.as_object();
    //     match (actual_as_object, expected_as_object) {
    //         (Some(actual), Some(expected)) => {
    //             let mut expected_keys = actual.keys().collect_vec();
    //             expected_keys.sort();
    //             let mut actual_keys = expected.keys().collect_vec();
    //             actual_keys.sort();
    //             let mut path_to_node = path_to_node;
    //
    //
    //             assert_eq!(actual_keys, expected_keys);
    //
    //             for key in expected_keys {
    //                 if key == "Events" {
    //                     continue;
    //                 }
    //                 let actual_value = &actual[key];
    //                 let expected_value = &expected[key];
    //
    //                 difference_count += preprocessed_input_matches_expected(
    //                     actual_value,
    //                     expected_value,
    //                     path_to_node.clone(),
    //                     difference_count,
    //                 );
    //             }
    //         }
    //         _ => todo!(),
    //     }
    //
    //     difference_count
    // };
    //
    //
    // difference_count += differences_in_object(actual, expected);
    //
    // difference_count += 1;
    //
    // println!(
    //     "Expected {:?}, but got {:?} at {:?} \n",
    //     expected, actual, path_to_node
    // );

    // difference_count
}

fn values_match_as_numbers(actual_value: &Value, expected_value: &Value) -> bool {
    let actual_value = actual_value.as_f64();
    let expected_value = expected_value.as_f64();
    let values_are_numbers = actual_value.is_some() && expected_value.is_some();

    if values_are_numbers && actual_value.unwrap() == expected_value.unwrap() {
        return true;
    }
    false
}
