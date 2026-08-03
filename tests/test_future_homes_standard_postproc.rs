use approx::assert_relative_eq;
use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
mod common;
use crate::common::{DEMO_FILES_DIR, TEMPORARY_OUTPUT_DIR};
pub const EXPECTED_POSTPROC_OUTPUT_DIR: &'static str =
    "./tests/e2e/expected_postproc_results/";
#[test]
fn test_fhs_postproc_result_files() {
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input = BufReader::new(
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
        demo_input,
        output_writer,
        None,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        false,
        false,
        false,
        &[],
    );

    assert!(result.is_ok());
    
    let postproc_file_suffixes = &[
        "FHS__postproc_summary.csv",
        "FHS_notional__postproc_summary.csv",
        "FHS_FEE__postproc.csv",
        "FHS_FEE_notional__postproc.csv",
    ];
    for suffix in postproc_file_suffixes {
        postproc_file_matches_expected(demo_input_file_name, suffix);
    }
    common::delete_temporary_output_directory(demo_input_file_name);
}

fn postproc_file_path(dir: &str, file_name: &str, suffix: &str) -> String {
    format!("{dir}/{file_name}__results/{file_name}__{suffix}")
}

fn postproc_file_matches_expected(file_name: &str, suffix: &str) {
    let mut actual_postproc_file =
        csv::Reader::from_path(postproc_file_path(TEMPORARY_OUTPUT_DIR, file_name, suffix))
            .unwrap();
    let mut expected_postproc_file = csv::Reader::from_path(postproc_file_path(
        EXPECTED_POSTPROC_OUTPUT_DIR,
        file_name,
        suffix,
    ))
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
