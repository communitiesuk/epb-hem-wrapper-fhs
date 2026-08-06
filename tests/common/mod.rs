use home_energy_model::output_writer::OutputWriter;
use indexmap::IndexMap;
use parking_lot::{Mutex, RwLock};
use std::io::{BufWriter, Write};
use std::sync::Arc;

pub(crate) const DEMO_FILES_DIR: &'static str = "./examples/input/future_homes_standard/";
pub(crate) const FLOAT_THRESHOLD: f64 = 1e-6; // 0.000001

#[derive(Clone, Debug)]
struct FileWriter(Arc<RwLock<Vec<u8>>>);

impl FileWriter {
    fn new() -> Self {
        Self(Arc::new(RwLock::new(Vec::with_capacity(2usize.pow(14)))))
    }
}

impl Write for FileWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write().extend_from_slice(buf);

        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub(crate) struct InMemoryDirectoryOutputWriter {
    input_filename: String,
    files: Arc<Mutex<IndexMap<String, FileWriter>>>,
}

impl InMemoryDirectoryOutputWriter {
    pub(crate) fn new(input_filename: &str) -> Self {
        Self {
            input_filename: input_filename.split('.').next().unwrap().to_string(),
            files: Arc::new(Mutex::new(IndexMap::new())),
        }
    }

    fn output_file_index(&self, location_key: &str, file_extension: &str) -> String {
        format!(
            "{}__{}.{}",
            self.input_filename, location_key, file_extension
        )
    }

    pub fn files(&self) -> IndexMap<String, String> {
        self.files
            .lock()
            .iter()
            .map(|(k, v)| {
                let bytes = v.0.read();
                let string_content = String::from_utf8_lossy(&bytes).to_string();
                (k.clone(), string_content)
            })
            .collect()
    }
}

impl OutputWriter for InMemoryDirectoryOutputWriter {
    fn writer_for_location_key(
        &self,
        location_key: &str,
        file_extension: &str,
    ) -> anyhow::Result<impl Write> {
        let key = self.output_file_index(location_key, file_extension);

        let file_writer = self
            .files
            .lock()
            .entry(key)
            .or_insert_with(FileWriter::new)
            .clone();

        // BufWriter prevents acquiring the RwLock on every byte chunk
        Ok(BufWriter::with_capacity(2usize.pow(14), file_writer))
    }
}
