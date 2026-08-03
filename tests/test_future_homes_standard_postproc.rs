use csv::ReaderBuilder;
use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use itertools::Itertools;
use std::fmt;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::time::Instant;

mod common;
use crate::common::{
    create_temporary_output_directory, DEMO_FILES_DIR, FLOAT_THRESHOLD, TEMPORARY_OUTPUT_DIR,
};
pub const EXPECTED_POSTPROC_OUTPUT_DIR: &'static str = "./tests/e2e/expected_postproc_results/";

#[test]
fn test_fhs_postproc_result_files() {
    let timer = Instant::now();
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input = demo_input(&demo_input_file_name);

    let output_writer = FileOutputWriter::new(
        create_temporary_output_directory(demo_input_file_name),
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
    println!("RUN WRAPPERS FINISHED: {:?}", timer.elapsed());

    assert!(result.is_ok());

    let postproc_file_suffixes = &[
        "FHS__postproc_summary.csv",
        "FHS_notional__postproc_summary.csv",
        "FHS_FEE__postproc.csv",
        "FHS_FEE_notional__postproc.csv",
    ];

    let mut difference_count = 0;
    let mut differences = Vec::new();

    for suffix in postproc_file_suffixes {
        let mut file_differences = postproc_file_differences(demo_input_file_name, suffix);
        difference_count = difference_count + file_differences.len();
        differences.append(&mut file_differences);
    }
    common::delete_temporary_output_directory(demo_input_file_name);

    println!("TEST FINISHED: {:?}", timer.elapsed());

    assert_eq!(
        difference_count,
        0,
        "\n\nTotal differences: {}\n{}\n\n",
        difference_count,
        differences.iter().join("\n")
    );
}

fn demo_input(demo_input_file_name: &&str) -> BufReader<File> {
    BufReader::new(
        File::open(Path::new(&format!(
            "{DEMO_FILES_DIR}{demo_input_file_name}.json"
        )))
        .unwrap(),
    )
}

fn postproc_file_path(dir: &str, file_name: &str, suffix: &str) -> String {
    format!("{dir}/{file_name}__results/{file_name}__{suffix}")
}

#[derive(Debug, Clone)]
pub enum Difference {
    Number {
        actual: f64,
        expected: f64,
        numerical_difference: f64,
        file_name: String,
        row_name: String,
    },
    String {
        actual: String,
        expected: String,
        file_name: String,
        row_name: String,
    },
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // assumption here that actual is Rust and expected is Python
        match self {
            Difference::String {
                actual,
                expected,
                file_name,
                row_name,
            } => {
                write!(
                    f,
                    "file {file_name} - row: {row_name}, 🦀: \"{actual}\", 🐍: \"{expected}\""
                )
            }
            Difference::Number {
                actual,
                expected,
                numerical_difference,
                file_name,
                row_name,
            } => {
                write!(
                    f,
                    "file {file_name} - row: {row_name}, 🦀: {actual}, 🐍: {expected}, Diff: {numerical_difference}"
                )
            }
        }
    }
}

fn postproc_file_differences(file_name: &str, suffix: &str) -> Vec<Difference> {
    let mut actual_postproc_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(postproc_file_path(TEMPORARY_OUTPUT_DIR, file_name, suffix))
        .unwrap();

    let mut expected_postproc_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(postproc_file_path(
            EXPECTED_POSTPROC_OUTPUT_DIR,
            file_name,
            suffix,
        ))
        .unwrap();

    let mut file_differences: Vec<Difference> = vec![];

    for (actual_record, expected_record) in actual_postproc_file
        .records()
        .zip(expected_postproc_file.records())
    {
        let row_name = actual_record.as_ref().unwrap().get(0).unwrap();
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
                    let numerical_difference = (actual_f64 - expected_f64).abs();
                    if numerical_difference > FLOAT_THRESHOLD {
                        file_differences.push(Difference::Number {
                            actual: actual_f64,
                            expected: expected_f64,
                            file_name: format!("{file_name}__{suffix}"),
                            row_name: row_name.into(),
                            numerical_difference,
                        });
                    }
                }
                _ => {
                    if actual_value.to_string() != expected_value.to_string() {
                        file_differences.push(Difference::String {
                            actual: actual_value.to_string(),
                            expected: expected_value.to_string(),
                            file_name: format!("{file_name}__{suffix}"),
                            row_name: row_name.into(),
                        });
                    }
                }
            }
        }
    }
    file_differences
}
