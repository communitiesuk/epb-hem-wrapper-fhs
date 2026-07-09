use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const DEMO_FILE_PATH: &str = "examples/input/future_homes_standard/demo_FHS.json";
const DEMO_FILES_DIR_PATH: &str = "examples/input/future_homes_standard";
const FHS_PY_ENTRYPOINT_PATH: &str = "./../epb-py-fhs-wrapper/src/bin/fhs.py";
const FHS_PY_PATH: &str = "./../epb-py-fhs-wrapper";
const PYTHON_OUTPUT_DIR_PATH: &str = "./../epb-hem-wrapper-fhs/tests/e2e/generated_results";

fn main() {
    generate_python_outputs().expect("Failed to generate python outputs")
}

fn generate_python_outputs() -> anyhow::Result<()> {
    let _ = clear_output_directory();
    install_python_fhs_requirements();
    run_all_python_fhs_files()?;

    Ok(())
}

fn run_all_python_fhs_files() -> anyhow::Result<()> {
    for entry in fs::read_dir(DEMO_FILES_DIR_PATH)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        if !path.is_dir() && file_name.to_str().unwrap().ends_with(".json") {
            run_python_fhs(path.to_str().unwrap())?;
        }
    }

    Ok(())
}
fn clear_output_directory() -> anyhow::Result<()> {
    // delete output directory and its contents if it exists
    let _ = fs::remove_dir_all(PYTHON_OUTPUT_DIR_PATH); // ignore eror result when directory does not exist

    // create empty output directory
    fs::create_dir_all(PYTHON_OUTPUT_DIR_PATH)?;
    Ok(())
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

fn install_python_fhs_requirements() {
    println!("Installing required packages...");
    run_command(&format!("uv sync --project {}", FHS_PY_PATH));
}

fn run_python_fhs(demo_file_path: &str) -> Result<(), anyhow::Error> {
    let path = Path::new(demo_file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let copied_demo_file_path = [PYTHON_OUTPUT_DIR_PATH, file_name]
        .iter()
        .collect::<PathBuf>();
    let copied_demo_file_path = copied_demo_file_path
        .to_str()
        .expect("couldn't make output dir path a string");
    fs::copy(DEMO_FILE_PATH, copied_demo_file_path)?;

    println!("Running Python FHS..");
    let run_python_fhs_cmd = format!(
        "uv run --project {} {} {} --preprocess-only",
        FHS_PY_PATH, FHS_PY_ENTRYPOINT_PATH, copied_demo_file_path
    );
    run_command(&run_python_fhs_cmd);
    println!("\n");
    println!("Python FHS ran");

    Ok(())
}
