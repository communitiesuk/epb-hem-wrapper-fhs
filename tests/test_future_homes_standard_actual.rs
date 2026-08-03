use approx::assert_relative_eq;
use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
mod common;
use crate::common::{DEMO_FILES_DIR, PROVIDED_EXPECTED_OUTPUT_DIR, TEMPORARY_OUTPUT_DIR};

#[test]
fn test_fhs_actual_calculations_succeeds() {
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input_file = BufReader::new(
        File::open(Path::new(&format!(
            "{DEMO_FILES_DIR}{demo_input_file_name}.json"
        )))
        .unwrap(),
    );

    let temporary_output_sub_dir = common::create_temporary_output_directory(demo_input_file_name);
    let output_writer = FileOutputWriter::new(
        temporary_output_sub_dir.clone(),
        format!("{demo_input_file_name}__{{}}.{{}}"),
    );

    let result = run_wrappers(
        demo_input_file,
        output_writer,
        None,
        None,
        &FhsFlags::FHS,
        false,
        false,
        false,
        &[],
    );

    assert!(result.is_ok());
    postproc_file_matches_expected(demo_input_file_name);
    common::delete_temporary_output_directory(demo_input_file_name);
}

fn postproc_file_path(dir: &str, file_name: &str) -> String {
    format!("{dir}/{file_name}__results/{file_name}__FHS__postproc_summary.csv")
}

fn postproc_file_matches_expected(file_name: &str) {
    let mut actual_postproc_file =
        csv::Reader::from_path(postproc_file_path(TEMPORARY_OUTPUT_DIR, file_name)).unwrap();
    let mut expected_postproc_file =
        csv::Reader::from_path(postproc_file_path(PROVIDED_EXPECTED_OUTPUT_DIR, file_name))
            .unwrap();

    for (actual_record, expected_record) in actual_postproc_file
        .records()
        .zip(expected_postproc_file.records())
    {
        for (actual_value, expected_value) in actual_record
            .as_ref()
            .unwrap()
            .iter()
            .zip(expected_record.unwrap().iter())
        {
            match (
                actual_value.parse::<f64>().ok(),
                expected_value.parse::<f64>().ok(),
            ) {
                (Some(actual_f64), Some(expected_f64)) => {
                    assert_relative_eq!(
                        actual_f64,
                        expected_f64,
                        max_relative = common::FLOAT_THRESHOLD
                    );
                }
                _ => assert_eq!(actual_value, expected_value),
            }
        }
    }
}
