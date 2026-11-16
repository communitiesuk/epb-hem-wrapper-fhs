mod common;

use approx::assert_relative_eq;
use common::{InMemoryDirectoryOutputWriter, DEMO_FILES_DIR, FLOAT_THRESHOLD};
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::BufReader;
use std::path::{Path, PathBuf};

// The contents of this file is a port from the python `test_future_homes_standard_actual.py`.
// It compares the results from running two input files in `actual` mode, against the expected
// output files provided by the Python.

const PROVIDED_PYTHON_OUTPUT_DIR: &str =
    "tests/e2e/expected_provided_results/future_homes_standard";

// Fully simulated, and the only demo whose timestep results are compared. Its compliance
// rates are pinned here rather than on a demo of their own because this run happens anyway:
// a full-year simulation costs several minutes and dominates the suite.
const FHS_DEMO_FILE: &str = "DESN-H-End-02-ESH-cMEV";

// Fully simulated for its rating and compliance rates, but without its timestep results: those
// are megabytes that churn on every unrelated preprocessing change, where a rating and three
// summary rows are a few hundred bytes.
//
// The export-limited dwelling. It is rated on the flat electricity tariff, where the storage
// heater above is on Economy 7, so both tariff paths are exercised. It carries the only
// end-to-end cover for excluding curtailed generation from primary energy: it is the only demo
// that curtails anything, the dwelling above curtailing zero across all 17520 timesteps, and an
// export limit is describable, so an assessor can reach that path. The exclusion's arithmetic is
// hand-derived in unit tests; what this covers is the exclusion reaching the calculation at all.
//
// One dwelling, because a further full-year simulation does not fit inside the pipeline's
// 60-minute job limit, which cannot be raised. Its battery and export limit both reduce the
// export credit, so an unreduced credit is covered by the storage heater above rather than here.
const FHS_DEMO_FILE_WITHOUT_RESULTS_CSV: &str = "DESN-H-End-02-HP-cMEV-EB-DNO-limit";

fn expected_result_file(demo_input_file_name: &str, name: &str) -> PathBuf {
    Path::new(PROVIDED_PYTHON_OUTPUT_DIR)
        .join(format!("{demo_input_file_name}__results"))
        .join(name)
}

fn expected_output(demo_input_file_name: &str, suffix: &str) -> String {
    let path = expected_result_file(demo_input_file_name, suffix);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

fn demo_input(demo_input_file_name: &str) -> BufReader<File> {
    let path = Path::new(DEMO_FILES_DIR).join(format!("{demo_input_file_name}.json"));
    BufReader::new(
        File::open(&path).unwrap_or_else(|e| panic!("cannot open {}: {e}", path.display())),
    )
}

/// Run one demo file through the wrapper in actual mode, returning the files it wrote.
fn run_demo_actual(demo_input_file_name: &str) -> IndexMap<String, String> {
    let output_writer = InMemoryDirectoryOutputWriter::new(demo_input_file_name);

    let result = run_wrappers(
        demo_input(demo_input_file_name),
        &output_writer,
        None,
        None,
        &FhsFlags::FHS, // equivalent to running "actual" in Python
        false,
        false,
        false,
        &[OutputFormat::Csv],
    );
    result.unwrap_or_else(|e| panic!("run_wrappers failed for {demo_input_file_name}: {e}"));

    output_writer.files()
}

fn actual_output<'a>(
    files: &'a IndexMap<String, String>,
    demo_file_name: &str,
    suffix: &str,
) -> &'a str {
    let full_file_name = format!("{demo_file_name}{suffix}");
    files.get(&full_file_name).unwrap_or_else(|| {
        panic!(
            "{full_file_name} not written by the wrapper. Files written: {:?}",
            files.keys().collect::<Vec<_>>()
        )
    })
}

fn read_csv_records(content: &str, label: &str, flexible: bool) -> Vec<Vec<String>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(flexible)
        .from_reader(content.as_bytes())
        .records()
        .map(|record| {
            record
                .unwrap_or_else(|e| panic!("cannot parse {label}: {e}"))
                .iter()
                .map(str::to_owned)
                .collect()
        })
        .collect()
}

/// Reads a results CSV with its two header rows, parsing every body cell as f64
/// (an empty cell is NaN, as it is for pandas).
fn read_results_csv(content: &str, label: &str) -> (Vec<Vec<String>>, Vec<Vec<f64>>) {
    let mut records = read_csv_records(content, label, false);
    assert!(records.len() >= 2, "{label} has no header rows");
    let body = records.split_off(2);
    let values = body
        .iter()
        .enumerate()
        .map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .map(|(col, cell)| match cell.trim() {
                    "" => f64::NAN,
                    text => text.parse().unwrap_or_else(|_| {
                        panic!("{label}: row {row}, column {col}: {text:?} is not a float")
                    }),
                })
                .collect()
        })
        .collect();
    (records, values)
}

fn compare_results_files(actual: &str, expected: &str, file_name: &str) {
    let (actual_headers, actual_values) = read_results_csv(actual, "actual results");
    let (expected_headers, expected_values) = read_results_csv(expected, "expected results");

    assert_eq!(
        actual_headers, expected_headers,
        "{file_name}: column headers differ"
    );
    assert_eq!(
        actual_values.len(),
        expected_values.len(),
        "{file_name}: row counts differ"
    );
    for (row, (actual_row, expected_row)) in actual_values.iter().zip(&expected_values).enumerate()
    {
        assert_eq!(
            actual_row.len(),
            expected_row.len(),
            "{file_name}: column counts differ in row {row}"
        );
        for (col, (&actual, &expected)) in actual_row.iter().zip(expected_row).enumerate() {
            assert!(
                (actual - expected).abs() <= FLOAT_THRESHOLD,
                "{file_name}: row {row}, column {:?}: actual={actual} expected={expected}",
                expected_headers
                    .iter()
                    .map(|h| h[col].as_str())
                    .collect::<Vec<_>>(),
            );
        }
    }
}

/// Compare metrics.json files with a clear error message.
fn compare_metrics(actual: &str, expected: &str) {
    let actual: Value = serde_json::from_str(actual).unwrap();
    let expected: Value = serde_json::from_str(expected).unwrap();
    let actual_eer = &actual["eer"];
    let expected_eer = &expected["eer"];

    assert_eq!(
        actual_eer["description"], expected_eer["description"],
        "EER description mismatch: actual={:?} expected={:?}",
        actual_eer["description"], expected_eer["description"]
    );
    assert_eq!(
        actual_eer["grade"], expected_eer["grade"],
        "EER grade mismatch: actual={:?} expected={:?}",
        actual_eer["grade"], expected_eer["grade"]
    );
    assert_eq!(
        actual_eer["units"], expected_eer["units"],
        "EER units mismatch: actual={:?} expected={:?}",
        actual_eer["units"], expected_eer["units"]
    );
    assert_relative_eq!(
        actual_eer["value"].as_f64().unwrap(),
        expected_eer["value"].as_f64().unwrap(),
        max_relative = FLOAT_THRESHOLD
    );
}

/// Return {rate name: (units, value)} from a postprocessing summary file.
///
/// The first row is a header naming the value column; each row after it holds a rate name,
/// its units and its value.
fn read_postproc_summary(content: &str, label: &str) -> BTreeMap<String, (String, f64)> {
    read_csv_records(content, label, true)
        .into_iter()
        .skip(1)
        .map(|row| (row[0].clone(), (row[1].clone(), row[2].parse().unwrap())))
        .collect()
}

/// Compare postprocessing summaries, naming whichever compliance rate moved.
fn compare_postproc_summary(actual: &str, expected: &str) {
    let actual = read_postproc_summary(actual, "actual summary");
    let expected = read_postproc_summary(expected, "expected summary");

    assert!(
        actual.keys().eq(expected.keys()),
        "Summary rates differ: actual={:?} expected={:?}",
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
    );
    for (name, (expected_units, expected_value)) in &expected {
        let (actual_units, actual_value) = &actual[name];
        assert_eq!(
            actual_units, expected_units,
            "{name} units mismatch: actual={actual_units} expected={expected_units}"
        );
        assert_relative_eq!(
            *actual_value,
            *expected_value,
            max_relative = FLOAT_THRESHOLD
        );
    }
}

/// Compare one demo's DER and DPER against their fixture.
///
/// Only the summary is pinned, not the per-timestep postprocessing CSVs written beside it:
/// those are megabytes each and would churn on every unrelated preprocessing change.
///
/// The export-limited dwelling's DER is negative, which is expected rather than a fault: these
/// rates cover regulated energy only, and subtracting the unregulated total is what takes it
/// below zero. See the summary calculation in apply_fhs_postprocessing.
fn compare_summary(files: &IndexMap<String, String>, demo_input_file_name: &str) {
    let suffix = "__FHS__postproc_summary.csv";
    compare_postproc_summary(
        actual_output(files, demo_input_file_name, suffix),
        &expected_output(
            demo_input_file_name,
            &format!("{demo_input_file_name}{suffix}"),
        ),
    );
}

fn compare_demo_metrics(files: &IndexMap<String, String>, demo_input_file_name: &str) {
    compare_metrics(
        actual_output(files, demo_input_file_name, "__FHS_metrics.json"),
        &expected_output(demo_input_file_name, "FHS_metrics.json"),
    );
}

#[test]
fn test_demo_file() {
    let files = run_demo_actual(FHS_DEMO_FILE);
    let results_suffix = "__FHS__results.csv";
    compare_results_files(
        actual_output(&files, FHS_DEMO_FILE, results_suffix),
        &expected_output(FHS_DEMO_FILE, &format!("{FHS_DEMO_FILE}{results_suffix}")),
        &format!("{FHS_DEMO_FILE}{results_suffix}"),
    );
    compare_demo_metrics(&files, FHS_DEMO_FILE);
    compare_summary(&files, FHS_DEMO_FILE);
}

#[test]
fn test_demo_file_metrics_and_summary() {
    let files = run_demo_actual(FHS_DEMO_FILE_WITHOUT_RESULTS_CSV);
    compare_demo_metrics(&files, FHS_DEMO_FILE_WITHOUT_RESULTS_CSV);
    compare_summary(&files, FHS_DEMO_FILE_WITHOUT_RESULTS_CSV);
}
