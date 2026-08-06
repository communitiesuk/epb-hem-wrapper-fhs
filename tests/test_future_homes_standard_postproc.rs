use csv::ReaderBuilder;
use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use itertools::Itertools;
use serde_json::Value;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::{fmt, fs};

mod common;
use crate::common::{DEMO_FILES_DIR, FLOAT_THRESHOLD};
const TEMPORARY_OUTPUT_DIR: &'static str = "./tests/e2e/test_future_homes_standard_outputs/";
const EXPECTED_POSTPROC_OUTPUT_DIR: &'static str = "./tests/e2e/expected_postproc_results/";

#[test]
fn test_fhs_postproc_result_files() {
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input = demo_input(&demo_input_file_name);

    let output_writer = FileOutputWriter::new(
        create_temporary_output_directory(demo_input_file_name),
        format!("{demo_input_file_name}__{{}}.{{}}"),
    );

    let result = run_wrappers(
        demo_input,
        &output_writer,
        None,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        false,
        false,
        false,
        &[],
    );

    assert!(result.is_ok());

    let differences = postproc_csv_results_differences(demo_input_file_name);
    let metrics_differences = postproc_metrics_results_differences(demo_input_file_name);

    delete_temporary_output_directory(demo_input_file_name);

    assert!(
        differences.is_empty() && metrics_differences.is_empty(),
        "\n\nTotal postproc file differences: {}\n{}\n\nTotal metrics differences: {}\n{}\n\n",
        differences.len(),
        differences.iter().join("\n"),
        metrics_differences.len(),
        metrics_differences.iter().join("\n")
    );
}

#[test]
fn test_fhs_postproc_compliance_differences() {
    // even if we get different Target and Dwelling values
    // we may still get the correct compliance result (Target and Dwelling differences)

    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input = demo_input(&demo_input_file_name);

    let output_writer = FileOutputWriter::new(
        create_temporary_output_directory(demo_input_file_name),
        format!("{demo_input_file_name}__{{}}.{{}}"),
    );

    let result = run_wrappers(
        demo_input,
        &output_writer,
        None,
        None,
        &FhsFlags::FHS_COMPLIANCE,
        false,
        false,
        false,
        &[],
    );

    assert!(result.is_ok());

    let differences = postproc_csv_compliance_differences(demo_input_file_name);
    // let metrics_differences = postproc_metrics_compliance_differences(demo_input_file_name);

    delete_temporary_output_directory(demo_input_file_name);

    assert!(
        differences.is_empty(),
        "\n\nTotal postproc file differences: {}\n{}\n\n",
        differences.len(),
        differences.iter().join("\n"),
    );
}

fn create_temporary_output_directory(input_file_name: &str) -> PathBuf {
    let temp_output_dir =
        PathBuf::from(format!("{TEMPORARY_OUTPUT_DIR}{input_file_name}__results"));
    fs::create_dir_all(&temp_output_dir).unwrap();
    temp_output_dir
}

fn delete_temporary_output_directory(input_file_name: &str) {
    let temp_output_dir = PathBuf::from(TEMPORARY_OUTPUT_DIR);
    let temporary_output_sub_dir =
        PathBuf::from(format!("{TEMPORARY_OUTPUT_DIR}{input_file_name}__results"));

    fs::remove_dir_all(&temporary_output_sub_dir).unwrap();
    let _ = fs::remove_dir(temp_output_dir);
}

fn demo_input(input_file_name: &&str) -> BufReader<File> {
    BufReader::new(
        File::open(Path::new(&format!(
            "{DEMO_FILES_DIR}{input_file_name}.json"
        )))
        .unwrap(),
    )
}

fn postproc_csv_results_differences(demo_input_file_name: &str) -> Vec<Difference> {
    let postproc_file_suffixes = &[
        "__FHS__postproc_summary.csv",
        "__FHS_notional__postproc_summary.csv",
        "__FHS_FEE__postproc.csv",
        "__FHS_FEE_notional__postproc.csv",
    ];

    let mut differences = Vec::new();

    for suffix in postproc_file_suffixes {
        let mut file_differences = postproc_csv_file_differences(demo_input_file_name, suffix);
        differences.append(&mut file_differences);
    }
    differences
}

fn postproc_csv_file_differences(file_name: &str, suffix: &str) -> Vec<Difference> {
    let mut actual_postproc_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(result_file_path(TEMPORARY_OUTPUT_DIR, file_name, suffix))
        .unwrap();

    let mut expected_postproc_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(result_file_path(
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
                            location: row_name.into(),
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
                            location: row_name.into(),
                        });
                    }
                }
            }
        }
    }
    file_differences
}

fn postproc_csv_compliance_differences(demo_input_file_name: &str) -> Vec<Difference> {
    let python_compliance_scores =
        get_compliance_scores(demo_input_file_name, EXPECTED_POSTPROC_OUTPUT_DIR);
    let rust_compliance_scores = get_compliance_scores(demo_input_file_name, TEMPORARY_OUTPUT_DIR);

    let mut differences = Vec::new();

    if python_compliance_scores.emission_rate_is_compliant
        != rust_compliance_scores.emission_rate_is_compliant
    {
        differences.push(Difference::String {
            actual: rust_compliance_scores
                .emission_rate_is_compliant
                .to_string(),
            expected: python_compliance_scores
                .emission_rate_is_compliant
                .to_string(),
            file_name: demo_input_file_name.to_string(),
            location: "Emission Rate (DER <= TER)".to_string(),
        });
    }

    if python_compliance_scores.primary_energy_rate_is_compliant
        != rust_compliance_scores.primary_energy_rate_is_compliant
    {
        differences.push(Difference::String {
            actual: rust_compliance_scores
                .primary_energy_rate_is_compliant
                .to_string(),
            expected: python_compliance_scores
                .primary_energy_rate_is_compliant
                .to_string(),
            file_name: demo_input_file_name.to_string(),
            location: "Primary Energy Rate (DPER <= TPER)".to_string(),
        });
    }

    if python_compliance_scores.fabric_energy_efficiency_is_compliant
        != rust_compliance_scores.fabric_energy_efficiency_is_compliant
    {
        differences.push(Difference::String {
            actual: rust_compliance_scores
                .fabric_energy_efficiency_is_compliant
                .to_string(),
            expected: python_compliance_scores
                .fabric_energy_efficiency_is_compliant
                .to_string(),
            file_name: demo_input_file_name.to_string(),
            location: "Fabric Energy Efficiciency (Dwelling <= Notional)".to_string(),
        });
    }

    differences
}

struct ComplianceScores {
    emission_rate_is_compliant: bool,
    primary_energy_rate_is_compliant: bool,
    fabric_energy_efficiency_is_compliant: bool,
}

fn get_compliance_scores(demo_input_file_name: &str, output_directory: &str) -> ComplianceScores {
    let mut postproc_summary_file = ReaderBuilder::new()
        .has_headers(true)
        .from_path(result_file_path(
            output_directory,
            demo_input_file_name,
            "__FHS__postproc_summary.csv",
        ))
        .unwrap();

    // TODO improve readability. Can we
    // get the third field in the first record

    let dwelling_emission_rate = postproc_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .iter()
        .nth(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    let dwelling_primary_energy_rate = postproc_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .iter()
        .nth(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();

    let mut notional_postproc_summary_file = ReaderBuilder::new()
        .has_headers(true)
        .from_path(result_file_path(
            output_directory,
            demo_input_file_name,
            "__FHS_notional__postproc_summary.csv",
        ))
        .unwrap();

    let notional_emission_rate = notional_postproc_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .get(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();
    let notional_primary_energy_rate = notional_postproc_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .get(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();

    let mut postproc_fee_summary_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(result_file_path(
            output_directory,
            demo_input_file_name,
            "__FHS_FEE__postproc.csv",
        ))
        .unwrap();

    let dwelling_fabric_energy_efficiency = postproc_fee_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .get(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();

    let mut notional_postproc_fee_summary_file = ReaderBuilder::new()
        .has_headers(false)
        .from_path(result_file_path(
            output_directory,
            demo_input_file_name,
            "__FHS_FEE_notional__postproc.csv",
        ))
        .unwrap();

    let notional_fabric_energy_efficiency = notional_postproc_fee_summary_file
        .records()
        .next()
        .unwrap()
        .unwrap()
        .get(2)
        .unwrap()
        .parse::<f64>()
        .unwrap();

    ComplianceScores {
        emission_rate_is_compliant: dwelling_emission_rate <= notional_emission_rate,
        primary_energy_rate_is_compliant: dwelling_primary_energy_rate
            <= notional_primary_energy_rate,
        fabric_energy_efficiency_is_compliant: dwelling_fabric_energy_efficiency
            <= notional_fabric_energy_efficiency,
    }
}

fn result_file_path(dir: &str, file_name: &str, suffix: &str) -> String {
    format!("{dir}/{file_name}__results/{file_name}{suffix}")
}

#[derive(Debug, Clone)]
pub enum Difference {
    Number {
        actual: f64,
        expected: f64,
        numerical_difference: f64,
        file_name: String,
        location: String,
    },
    String {
        actual: String,
        expected: String,
        file_name: String,
        location: String,
    },
    Key {
        actual: String,
        expected: String,
        file_name: String,
        description: String,
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
                location,
            } => {
                write!(
                    f,
                    "file {file_name} - {location}, 🦀: \"{actual}\", 🐍: \"{expected}\""
                )
            }
            Difference::Number {
                actual,
                expected,
                numerical_difference,
                file_name,
                location,
            } => {
                write!(
                    f,
                    "file {file_name} - {location}, 🦀: {actual}, 🐍: {expected}, Diff: {numerical_difference}"
                )
            }
            Difference::Key {
                actual,
                expected,
                file_name,
                description,
            } => {
                write!(
                    f,
                    "file {file_name} - {description}, 🦀: \"{actual}\", 🐍: \"{expected}\""
                )
            }
        }
    }
}

fn postproc_metrics_results_differences(demo_input_file_name: &str) -> Vec<Difference> {
    let metrics_files = &[
        "FHS_metrics.json",
        "FHS_notional_metrics.json",
        "FHS_FEE_metrics.json",
        "FHS_FEE_notional_metrics.json",
    ];

    let mut differences = Vec::new();
    for metrics_file_name in metrics_files {
        let mut file_differences =
            metrics_file_differences(demo_input_file_name, metrics_file_name);
        differences.append(&mut file_differences);
    }

    differences
}

fn metrics_file_value(directory: &str, input_file_name: &str, metrics_file_name: &str) -> Value {
    let file_path = format!("{directory}/{input_file_name}__results/{metrics_file_name}");
    let file =
        fs::read_to_string(&file_path).expect(&format!("Output file not found at {file_path}"));
    serde_json::from_str(&file).unwrap()
}

fn metrics_file_differences(
    demo_input_file_name: &str,
    metrics_file_name: &str,
) -> Vec<Difference> {
    let mut differences = vec![];
    let actual_output = metrics_file_value(
        TEMPORARY_OUTPUT_DIR,
        demo_input_file_name,
        metrics_file_name,
    );
    let expected_output = metrics_file_value(
        EXPECTED_POSTPROC_OUTPUT_DIR,
        demo_input_file_name,
        metrics_file_name,
    );
    let file_path = format!("{demo_input_file_name}__results/{metrics_file_name}");
    differences.append(
        metric_output_differences(&actual_output, &expected_output, file_path.as_str()).as_mut(),
    );

    differences
}

pub(crate) fn metric_output_differences(
    actual: &Value,
    expected: &Value,
    file_path: &str,
) -> Vec<Difference> {
    let mut differences = vec![];

    let actual_metric = actual.get("eer").unwrap();
    let expected_metric = expected.get("eer").unwrap();

    let actual_description = actual_metric.get("description").unwrap();
    let actual_grade = actual_metric.get("grade").unwrap();
    let actual_units = actual_metric.get("units").unwrap();
    let actual_value = actual_metric.get("value").unwrap().as_f64().unwrap();

    let expected_description = expected_metric.get("description").unwrap();
    let expected_grade = expected_metric.get("grade").unwrap();
    let expected_units = expected_metric.get("units").unwrap();
    let expected_value = expected_metric.get("value").unwrap().as_f64().unwrap();

    if actual_description != expected_description {
        differences.push(Difference::String {
            actual: actual_description.to_string(),
            expected: expected_description.to_string(),
            file_name: file_path.to_string(),
            location: "description".to_string(),
        })
    }

    if actual_grade != expected_grade {
        differences.push(Difference::String {
            actual: actual_grade.to_string(),
            expected: expected_grade.to_string(),
            file_name: file_path.to_string(),
            location: "grade".to_string(),
        })
    }

    if actual_units != expected_units {
        differences.push(Difference::String {
            actual: actual_units.to_string(),
            expected: expected_units.to_string(),
            file_name: file_path.to_string(),
            location: "units".to_string(),
        })
    }

    let numerical_difference = (actual_value - expected_value).abs();
    if numerical_difference > FLOAT_THRESHOLD {
        differences.push(Difference::Number {
            actual: actual_value,
            expected: expected_value,
            numerical_difference,
            file_name: file_path.to_string(),
            location: "value".to_string(),
        })
    }
    differences
}
