use common::DEMO_FILES_DIR;
use home_energy_model::output_writer::FileOutputWriter;
use home_energy_model::OutputFormat;
use home_energy_model_wrapper_fhs::{run_wrappers, FhsFlags};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
mod common;

#[test]
fn test_fhs_actual_calculations_succeeds() {
    let demo_input_file_name = "DESN-H-End-02-ESH-cMEV";
    let demo_input_file = BufReader::new(
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
        demo_input_file,
        output_writer,
        None,
        None,
        &FhsFlags::FHS,
        false,
        false,
        false,
        &[OutputFormat::Json],
    );
    // let error = result.err().unwrap().to_string();
    // assert_eq!(error, "");
    assert!(result.is_ok());

    common::delete_temporary_output_directory(demo_input_file_name);
}
