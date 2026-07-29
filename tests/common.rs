use std::fs;
use std::path::PathBuf;

pub fn create_temporary_output_directory(directory: &str, demo_file_name: &str) -> PathBuf {
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(format!("{directory}{demo_file_name}__results"));
    fs::create_dir_all(&temp_output_dir).unwrap();
    temp_output_dir
}

pub fn delete_temporary_output_directory(parent_directory: &str, sub_directory: PathBuf) {
    fs::remove_dir_all(&sub_directory).unwrap();
    let mut temp_output_dir = PathBuf::new();
    temp_output_dir.push(parent_directory);
    let _ = fs::remove_dir(temp_output_dir);
}
