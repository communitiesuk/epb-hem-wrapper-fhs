use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const DEMO_FILES_DIR: &str = "examples/input/future_homes_standard";
const PY_FHS_REPO: &str = "https://dev.azure.com/Sustenic/Home%20Energy%20Model%20Reference/_git/Future%20Homes%20Standard%20wrapper";
const PY_FHS_TAG: &str = "1.0.0a7";
const PY_FHS_TARGET_DIR: &str = "py_fhs_wrapper";
const PY_FHS_ENTRYPOINT: &str = "src/bin/fhs.py";
const PY_OUTPUT_DIR: &str = "tests/e2e/expected_generated_results";

fn main() {
    let timer = Instant::now();
    println!(
        "Starting Python FHS output generation...\n\n⚠️ WARNING: This can take over 2 hours! ⚠️ \n\n"
    );
    python_fhs_repo();
    clear_output_directory();
    install_python_fhs_requirements();
    run_all_python_fhs_files();
    remove_unnecessary_files();

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
    let _ = fs::remove_dir_all(PY_OUTPUT_DIR);

    // create empty output directory
    fs::create_dir_all(PY_OUTPUT_DIR).unwrap();
}

fn install_python_fhs_requirements() {
    println!("\nInstalling required packages...");
    run_command(&format!("uv sync --project {}", PY_FHS_TARGET_DIR));
}

fn run_all_python_fhs_files() {
    let file_count = fs::read_dir(DEMO_FILES_DIR)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_file() && path.extension().is_some_and(|ext| ext == "json"))
        .collect::<Vec<PathBuf>>()
        .into_par_iter()
        .map(|path| {
            run_python_fhs_file(&path);
        })
        .count();

    println!("\n🐍  Ran preprocessing for {file_count} files in total");
}

fn run_python_fhs_file(demo_file_path: &Path) {
    let file_name = demo_file_path.file_name().unwrap();
    let copied_demo_file_path = Path::new(PY_OUTPUT_DIR).join(file_name);

    fs::copy(demo_file_path, &copied_demo_file_path).unwrap();

    let entrypoint = Path::new(PY_FHS_TARGET_DIR).join(PY_FHS_ENTRYPOINT);
    let run_python_fhs_preprocessing_cmd = format!(
        "uv run --project {} {:?} {:?} --preprocess-only",
        PY_FHS_TARGET_DIR, entrypoint, copied_demo_file_path
    );
    let run_python_fhs_postprocessing_cmd = format!(
        "uv run --project {} {:?} {:?}",
        PY_FHS_TARGET_DIR, entrypoint, copied_demo_file_path
    );

    run_command(&run_python_fhs_preprocessing_cmd);
    run_command(&run_python_fhs_postprocessing_cmd);
}

fn remove_unnecessary_files() {
    fn should_keep(file_name: &str) -> bool {
        matches!(
            file_name,
            "FHS_metrics.json"
                | "FHS_notional_metrics.json"
                | "FHS_FEE_metrics.json"
                | "FHS_FEE_notional_metrics.json"
        ) || file_name.ends_with("__FHS__preproc.json")
            || file_name.ends_with("__FHS_FEE__preproc.json")
            || file_name.ends_with("__FHS_notional__preproc.json")
            || file_name.ends_with("__FHS_FEE_notional__preproc.json")
            || file_name.ends_with("__FHS_notional__postproc_summary.csv")
            || file_name.ends_with("__FHS_FEE__postproc.csv")
            || file_name.ends_with("__FHS_FEE_notional__postproc.csv")
            || file_name.ends_with("__FHS__postproc_summary.csv")
    }

    for path in fs::read_dir(PY_OUTPUT_DIR)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.is_dir())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_str().unwrap_or("").ends_with("__results"))
        })
    {
        for file_path in fs::read_dir(&path)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.is_file())
        {
            let file_name = file_path.file_name().unwrap().to_string_lossy();
            if !should_keep(&file_name) {
                fs::remove_file(&file_path).unwrap_or_else(|err| {
                    panic!("Failed to remove {}: {}", file_path.display(), err)
                });
            }
        }
    }
    for path in fs::read_dir(PY_OUTPUT_DIR).unwrap() {
        let path = path.unwrap().path();
        if path.is_file() {
            fs::remove_file(&path)
                .unwrap_or_else(|err| panic!("Failed to remove {}: {}", path.display(), err));
        }
    }
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
