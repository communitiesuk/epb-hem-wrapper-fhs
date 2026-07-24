use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEMO_FILES_DIR_PATH: &str = "examples/input/future_homes_standard";
const FHS_PY_ENTRYPOINT_PATH: &str = "./../epb-py-fhs-wrapper/src/bin/fhs.py";
const FHS_PY_PATH: &str = "./../epb-py-fhs-wrapper";
const PYTHON_OUTPUT_DIR_PATH: &str = "tests/e2e/expected_generated_results";

fn main() {
    clear_output_directory();
    install_python_fhs_requirements();
    run_all_python_fhs_files();
}

fn clear_output_directory() {
    // delete output directory and its contents if it exists
    let _ = fs::remove_dir_all(PYTHON_OUTPUT_DIR_PATH);

    // create empty output directory
    fs::create_dir_all(PYTHON_OUTPUT_DIR_PATH).unwrap();
}

fn install_python_fhs_requirements() {
    println!("Installing required packages...");
    run_command(&format!("uv sync --project {}", FHS_PY_PATH));
}

fn run_all_python_fhs_files() {
    for entry in fs::read_dir(DEMO_FILES_DIR_PATH).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        if !path.is_dir() && file_name.to_str().unwrap().ends_with(".json") {
            run_python_fhs_preprocessing(path.to_str().unwrap());
        }
    }
}

fn run_python_fhs_preprocessing(demo_file_path: &str) {
    let path = Path::new(demo_file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let copied_demo_file_path = [PYTHON_OUTPUT_DIR_PATH, file_name]
        .iter()
        .collect::<PathBuf>();
    let copied_demo_file_path = copied_demo_file_path
        .to_str()
        .expect("couldn't make output dir path a string");
    fs::copy(demo_file_path, copied_demo_file_path).unwrap();

    println!("Running Python FHS..");
    let run_python_fhs_cmd = format!(
        "uv run --project {} {} {} --preprocess-only",
        FHS_PY_PATH, FHS_PY_ENTRYPOINT_PATH, copied_demo_file_path
    );
    run_command(&run_python_fhs_cmd);
    println!("\n");
    println!("Python FHS ran");
}

fn run_command(cmd: &str) -> String {
    println!("{cmd}");
    let output = Command::new("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .expect("Failed to run command");

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse stdout as a string");
    let stderr = String::from_utf8(output.stderr).expect("Failed to parse stderr as a string");

    println!("{stdout}");
    println!("{stderr}");
    stderr
}
