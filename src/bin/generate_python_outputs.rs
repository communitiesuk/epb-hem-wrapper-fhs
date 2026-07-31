use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEMO_FILES_DIR: &str = "examples/input/future_homes_standard";
const PY_FHS_REPO: &str = "https://dev.azure.com/Sustenic/Home%20Energy%20Model%20Reference/_git/Future%20Homes%20Standard%20wrapper";
const PY_FHS_TAG: &str = "1.0.0a7";
const PY_FHS_TARGET_DIR: &str = "py_fhs_wrapper";
const PY_FHS_ENTRYPOINT: &str = "src/bin/fhs.py";
const PY_PREPROCESS_OUTPUT_DIR: &str = "tests/e2e/expected_generated_results";

fn main() {
    let timer = Instant::now();

    python_fhs_repo();
    clear_output_directory();
    install_python_fhs_requirements();
    run_all_python_fhs_files();

    let duration = timer.elapsed();
    println!("Time taken to generate Python outputs: {:.2?}", duration);
}

fn python_fhs_repo() {
    if !Path::new(PY_FHS_TARGET_DIR).exists() {
        println!("Cloning Python FHS repo...");
        Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                "--branch",
                PY_FHS_TAG,
                PY_FHS_REPO,
                PY_FHS_TARGET_DIR,
            ])
            .status()
            .unwrap();
    } else {
        println!("\n⚡ Using existing repository at '{}'", PY_FHS_TARGET_DIR);
    }
}

fn clear_output_directory() {
    // delete output directory and its contents if it exists
    let _ = fs::remove_dir_all(PY_PREPROCESS_OUTPUT_DIR);

    // create empty output directory
    fs::create_dir_all(PY_PREPROCESS_OUTPUT_DIR).unwrap();
}

fn install_python_fhs_requirements() {
    println!("\nInstalling required packages...");
    run_command(&format!("uv sync --project {}", PY_FHS_TARGET_DIR));
}

fn run_all_python_fhs_files() {
    let mut file_count = 0;

    for entry in fs::read_dir(DEMO_FILES_DIR).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let file_name = entry.file_name();
        if !path.is_dir() && file_name.to_str().unwrap().ends_with(".json") {
            run_python_fhs_preprocessing(path.to_str().unwrap());
            file_count += 1;
        }
    }

    println!("\n🐍  Ran preprocessing for {} files in total", file_count);
}

fn run_python_fhs_preprocessing(demo_file_path: &str) {
    let path = Path::new(demo_file_path);
    let file_name = path.file_name().unwrap().to_str().unwrap();

    let copied_demo_file_path = [PY_PREPROCESS_OUTPUT_DIR, file_name]
        .iter()
        .collect::<PathBuf>();
    let copied_demo_file_path = copied_demo_file_path
        .to_str()
        .expect("couldn't make output dir path a string");
    fs::copy(demo_file_path, copied_demo_file_path).unwrap();

    println!("\nRunning Python FHS preprocessing...");
    let run_python_fhs_cmd = format!(
        "uv run --project {} {:?} {} --preprocess-only",
        PY_FHS_TARGET_DIR,
        Path::new(PY_FHS_TARGET_DIR).join(PY_FHS_ENTRYPOINT),
        copied_demo_file_path
    );
    run_command(&run_python_fhs_cmd);
    println!("Python FHS preprocessing ran");
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
