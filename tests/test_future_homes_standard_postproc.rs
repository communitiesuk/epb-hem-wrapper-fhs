use csv::{ReaderBuilder, StringRecord};
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use indexmap::IndexMap;
use itertools::Itertools;
use rayon::prelude::*;
use serde_json::Value;
use std::borrow::Cow;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::{fmt, fs};

mod common;
use common::{InMemoryDirectoryOutputWriter, DEMO_FILES_DIR, FLOAT_THRESHOLD};
const PYTHON_POSTPROC_OUTPUT_DIR: &'static str = "./tests/e2e/expected_postproc_results/";

#[test]
fn test_fhs_postproc_result_files() {
    let demo_files = [
        "DESN-H-End-02-ESH-cMEV",
        "demo_FHS",
        "DESN-H-End-02-HP-iMEV-pre-heat",
        "DESN-H-End-02-HP-iMEV-wwhrs-storage-tank",
    ];
    let postproc_and_metric_differences: Vec<(Vec<Difference>, Vec<Difference>)> = demo_files
        .par_iter()
        .map(|demo_input_file_name| {
            let demo_input = demo_input(&demo_input_file_name);

            let output_writer = InMemoryDirectoryOutputWriter::new(demo_input_file_name);

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

            let rust_files = &output_writer.files();
            let differences = postproc_csv_results_differences(demo_input_file_name, rust_files);
            let metrics_differences =
                postproc_metrics_results_differences(demo_input_file_name, rust_files);

            (differences, metrics_differences)
        })
        .collect();

    let (differences, metric_differences) = postproc_and_metric_differences.into_iter().fold(
        (vec![], vec![]),
        |(mut diffs, mut metric_diffs), (diff, metric_diff)| {
            diffs.extend(diff);
            metric_diffs.extend(metric_diff);
            (diffs, metric_diffs)
        },
    );

    assert!(
        differences.is_empty() && metric_differences.is_empty(),
        "\n\nTotal postproc file differences: {}\n{}\n\nTotal metrics differences: {}\n{}\n\n",
        differences.len(),
        differences.iter().join("\n"),
        metric_differences.len(),
        metric_differences.iter().join("\n")
    );
}

// TODO: Consider deleting below test once python randomness match is confirmed
// test_fhs_postproc_result_files supersedes test_fhs_postproc_compliance_differences
#[test]
fn test_fhs_postproc_compliance_differences() {
    // even if we get different Target and Dwelling values
    // we may still get the correct compliance result (Target and Dwelling differences)
    let demo_files = [
        "DESN-H-End-02-ESH-cMEV",
        "demo_FHS",
        "DESN-H-End-02-HP-iMEV-pre-heat",
        "DESN-H-End-02-HP-iMEV-wwhrs-storage-tank",
    ];
    let differences: Vec<(String, usize)> = demo_files
        .par_iter()
        .map(|demo_input_file_name| {
            let demo_input = demo_input(&demo_input_file_name);

            let output_writer = InMemoryDirectoryOutputWriter::new(demo_input_file_name);

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

            let rust_files = &output_writer.files();
            let differences = postproc_csv_compliance_differences(demo_input_file_name, rust_files);

            (differences.iter().join("\n"), differences.len())
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
        "\n\nTotal compliance differences: {}\n{}\n\n",
        total_difference_count,
        differences.iter().join("\n"),
    );
}

fn demo_input(input_file_name: &&str) -> BufReader<File> {
    BufReader::new(
        File::open(Path::new(&format!(
            "{DEMO_FILES_DIR}{input_file_name}.json"
        )))
        .unwrap(),
    )
}

fn postproc_csv_results_differences(
    demo_input_file_name: &str,
    rust_files: &IndexMap<String, String>,
) -> Vec<Difference> {
    let postproc_file_suffixes = &[
        "__FHS__postproc_summary.csv",
        "__FHS_notional__postproc_summary.csv",
        "__FHS_FEE__postproc.csv",
        "__FHS_FEE_notional__postproc.csv",
    ];

    let mut differences = Vec::new();

    for suffix in postproc_file_suffixes {
        let mut file_differences =
            postproc_csv_file_differences(demo_input_file_name, suffix, rust_files);
        differences.append(&mut file_differences);
    }
    differences
}

fn postproc_csv_file_differences(
    file_name: &str,
    suffix: &str,
    rust_files: &IndexMap<String, String>,
) -> Vec<Difference> {
    let rust_postproc_file = postproc_file(file_name, Some(rust_files), suffix);
    let python_postproc_file = postproc_file(file_name, None, suffix);

    let mut file_differences: Vec<Difference> = vec![];

    for (rust_record, python_record) in rust_postproc_file.into_iter().zip(python_postproc_file) {
        let row_name = &rust_record.0;
        for (rust_value, python_value) in rust_record.1.iter().zip(python_record.1.iter()) {
            match (
                rust_value.parse::<f64>().ok(),
                python_value.parse::<f64>().ok(),
            ) {
                (Some(rust_f64), Some(python_f64)) => {
                    let numerical_difference = (rust_f64 - python_f64).abs();
                    if numerical_difference > FLOAT_THRESHOLD {
                        file_differences.push(Difference::Number {
                            rust: rust_f64,
                            python: python_f64,
                            file_name: format!("{file_name}__{suffix}"),
                            location: row_name.into(),
                            numerical_difference,
                        });
                    }
                }
                _ => {
                    if rust_value.to_string() != python_value.to_string() {
                        file_differences.push(Difference::String {
                            rust: rust_value.to_string(),
                            python: python_value.to_string(),
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

fn postproc_csv_compliance_differences(
    demo_input_file_name: &str,
    rust_files: &IndexMap<String, String>,
) -> Vec<Difference> {
    let python_compliance_scores = get_compliance_scores(demo_input_file_name, None);
    let rust_compliance_scores = get_compliance_scores(demo_input_file_name, Some(rust_files));

    let mut differences = Vec::new();

    if python_compliance_scores.emission_rate_is_compliant
        != rust_compliance_scores.emission_rate_is_compliant
    {
        differences.push(Difference::String {
            rust: rust_compliance_scores
                .emission_rate_is_compliant
                .to_string(),
            python: python_compliance_scores
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
            rust: rust_compliance_scores
                .primary_energy_rate_is_compliant
                .to_string(),
            python: python_compliance_scores
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
            rust: rust_compliance_scores
                .fabric_energy_efficiency_is_compliant
                .to_string(),
            python: python_compliance_scores
                .fabric_energy_efficiency_is_compliant
                .to_string(),
            file_name: demo_input_file_name.to_string(),
            location: "Fabric Energy Efficiciency (Dwelling <= Notional)".to_string(),
        });
    }

    differences
}

fn postproc_file(
    filename: &str,
    rust_files: Option<&IndexMap<String, String>>,
    suffix: &str,
) -> IndexMap<String, StringRecord> {
    let filename_with_suffix = &format!("{filename}{suffix}");

    let bytes = match rust_files {
        Some(rust_files) => Cow::Borrowed(
            rust_files
                .get(filename_with_suffix)
                .unwrap_or_else(|| panic!("File not found: {filename_with_suffix}"))
                .as_bytes(),
        ),
        None => {
            let path =
                format!("{PYTHON_POSTPROC_OUTPUT_DIR}/{filename}__results/{filename_with_suffix}");
            Cow::Owned(fs::read(&path).unwrap_or_else(|_| panic!("File not found: {path}")))
        }
    };

    ReaderBuilder::new()
        .has_headers(false)
        .from_reader(bytes.as_ref())
        .records()
        .flatten()
        .filter_map(|rec| {
            let key = rec.get(0)?.to_string();
            Some((key, rec))
        })
        .collect()
}

fn field_value(key: &str, summary_file_map: &IndexMap<String, StringRecord>) -> f64 {
    summary_file_map
        .get(key)
        .unwrap_or_else(|| panic!("{key} not found in postproc summary"))
        .get(2)
        .unwrap_or_else(|| panic!("{key} field 3 not found in postproc summary"))
        .parse::<f64>()
        .unwrap_or_else(|_| panic!("Unable to parse {key} field 3"))
}

struct ComplianceScores {
    emission_rate_is_compliant: bool,
    primary_energy_rate_is_compliant: bool,
    fabric_energy_efficiency_is_compliant: bool,
}

fn get_compliance_scores(
    demo_input_file_name: &str,
    rust_files: Option<&IndexMap<String, String>>,
) -> ComplianceScores {
    let postproc_summary_file = postproc_file(
        demo_input_file_name,
        rust_files,
        "__FHS__postproc_summary.csv",
    );
    let dwelling_emission_rate = field_value("DER", &postproc_summary_file);
    let dwelling_primary_energy_rate = field_value("DPER", &postproc_summary_file);

    let notional_postproc_summary_file = postproc_file(
        demo_input_file_name,
        rust_files,
        "__FHS_notional__postproc_summary.csv",
    );
    let notional_emission_rate = field_value("TER", &notional_postproc_summary_file);
    let notional_primary_energy_rate = field_value("TPER", &notional_postproc_summary_file);

    let postproc_fee_summary_file =
        postproc_file(demo_input_file_name, rust_files, "__FHS_FEE__postproc.csv");
    let dwelling_fabric_energy_efficiency =
        field_value("Fabric Energy Efficiency", &postproc_fee_summary_file);

    let notional_postproc_fee_summary_file = postproc_file(
        demo_input_file_name,
        rust_files,
        "__FHS_FEE_notional__postproc.csv",
    );
    let notional_fabric_energy_efficiency = field_value(
        "Fabric Energy Efficiency",
        &notional_postproc_fee_summary_file,
    );

    ComplianceScores {
        emission_rate_is_compliant: dwelling_emission_rate <= notional_emission_rate,
        primary_energy_rate_is_compliant: dwelling_primary_energy_rate
            <= notional_primary_energy_rate,
        fabric_energy_efficiency_is_compliant: dwelling_fabric_energy_efficiency
            <= notional_fabric_energy_efficiency,
    }
}

#[derive(Debug, Clone)]
pub enum Difference {
    Number {
        rust: f64,
        python: f64,
        numerical_difference: f64,
        file_name: String,
        location: String,
    },
    String {
        rust: String,
        python: String,
        file_name: String,
        location: String,
    },
    Key {
        rust: String,
        python: String,
        file_name: String,
        description: String,
    },
}

impl fmt::Display for Difference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Difference::String {
                rust,
                python,
                file_name,
                location,
            } => {
                write!(
                    f,
                    "file {file_name} - {location}, 🦀: \"{rust}\", 🐍: \"{python}\""
                )
            }
            Difference::Number {
                rust,
                python,
                numerical_difference,
                file_name,
                location,
            } => {
                write!(
                    f,
                    "file {file_name} - {location}, 🦀: {rust}, 🐍: {python}, Diff: {numerical_difference}"
                )
            }
            Difference::Key {
                rust,
                python,
                file_name,
                description,
            } => {
                write!(
                    f,
                    "file {file_name} - {description}, 🦀: \"{rust}\", 🐍: \"{python}\""
                )
            }
        }
    }
}

fn postproc_metrics_results_differences(
    demo_input_file_name: &str,
    rust_files: &IndexMap<String, String>,
) -> Vec<Difference> {
    let metrics_files = &[
        "FHS_metrics.json",
        "FHS_notional_metrics.json",
        "FHS_FEE_metrics.json",
        "FHS_FEE_notional_metrics.json",
    ];

    let mut differences = Vec::new();
    for metrics_file_name in metrics_files {
        let mut file_differences =
            metrics_file_differences(demo_input_file_name, metrics_file_name, rust_files);
        differences.append(&mut file_differences);
    }

    differences
}

fn metrics_file_differences(
    demo_input_file_name: &str,
    metrics_file_name: &str,
    rust_files: &IndexMap<String, String>,
) -> Vec<Difference> {
    let mut differences = vec![];
    let rust_output = serde_json::from_str(
        rust_files
            .get(&format!("{demo_input_file_name}__{metrics_file_name}"))
            .unwrap()
            .as_str(),
    )
    .unwrap();
    let file_path = format!("{demo_input_file_name}__results/{metrics_file_name}");
    let python_output = serde_json::from_str(
        &fs::read_to_string(format!("{PYTHON_POSTPROC_OUTPUT_DIR}/{file_path}")).unwrap(),
    )
    .unwrap();

    differences.append(
        metric_output_differences(&rust_output, &python_output, file_path.as_str()).as_mut(),
    );

    differences
}

pub(crate) fn metric_output_differences(
    rust: &Value,
    python: &Value,
    file_path: &str,
) -> Vec<Difference> {
    let mut differences = vec![];

    let rust_metric = rust.get("eer").unwrap();
    let python_metric = python.get("eer").unwrap();

    let rust_description = rust_metric.get("description").unwrap();
    let rust_grade = rust_metric.get("grade").unwrap();
    let rust_units = rust_metric.get("units").unwrap();
    let rust_value = rust_metric.get("value").unwrap().as_f64().unwrap();

    let python_description = python_metric.get("description").unwrap();
    let python_grade = python_metric.get("grade").unwrap();
    let python_units = python_metric.get("units").unwrap();
    let python_value = python_metric.get("value").unwrap().as_f64().unwrap();

    if rust_description != python_description {
        differences.push(Difference::String {
            rust: rust_description.to_string(),
            python: python_description.to_string(),
            file_name: file_path.to_string(),
            location: "description".to_string(),
        })
    }

    if rust_grade != python_grade {
        differences.push(Difference::String {
            rust: rust_grade.to_string(),
            python: python_grade.to_string(),
            file_name: file_path.to_string(),
            location: "grade".to_string(),
        })
    }

    if rust_units != python_units {
        differences.push(Difference::String {
            rust: rust_units.to_string(),
            python: python_units.to_string(),
            file_name: file_path.to_string(),
            location: "units".to_string(),
        })
    }

    let numerical_difference = (rust_value - python_value).abs();
    if numerical_difference > FLOAT_THRESHOLD {
        differences.push(Difference::Number {
            rust: rust_value,
            python: python_value,
            numerical_difference,
            file_name: file_path.to_string(),
            location: "value".to_string(),
        })
    }
    differences
}
