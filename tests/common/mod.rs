use std::fs;
use std::path::PathBuf;

pub(crate) const TEMPORARY_OUTPUT_DIR: &'static str =
    "./tests/e2e/test_future_homes_standard_outputs/";
pub(crate) const DEMO_FILES_DIR: &'static str = "./examples/input/future_homes_standard/";
pub(crate) const FLOAT_THRESHOLD: f64 = 1e-6; // 0.000001

pub fn create_temporary_output_directory(input_file_name: &str) -> PathBuf {
    let temp_output_dir =
        PathBuf::from(format!("{TEMPORARY_OUTPUT_DIR}{input_file_name}__results"));
    fs::create_dir_all(&temp_output_dir).unwrap();
    temp_output_dir
}

pub fn delete_temporary_output_directory(input_file_name: &str) {
    let temp_output_dir = PathBuf::from(TEMPORARY_OUTPUT_DIR);
    let temporary_output_sub_dir =
        PathBuf::from(format!("{TEMPORARY_OUTPUT_DIR}{input_file_name}__results"));

    fs::remove_dir_all(&temporary_output_sub_dir).unwrap();
    let _ = fs::remove_dir(temp_output_dir);
}
