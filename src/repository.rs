use crate::layout::Layout;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

/// Responsible for saving and loading layouts to/from the filesystem.
pub struct Repository(String);

impl Repository {
    /// Returns a new Repository that gets data from a given folder.
    pub fn new(fs_root: &str) -> Self {
        Repository(String::from(fs_root))
    }

    /// Returns the filepath for a given layout.
    fn path(&self, layout: &Layout) -> PathBuf {
        Path::new(&self.0).join(layout.fingerprint())
    }

    /// Writes a file containing layout data in JSON.
    pub fn save(&self, layout: &Layout) -> Result<String, Box<dyn Error>> {
        match self.path(&layout).to_str() {
            Some(path) => {
                let content = serde_json::to_string(&layout.outputs)?;
                fs::write(path, content.as_bytes())?;

                Ok(String::new())
            }
            None => panic!("invalid filepath!"),
        }
    }

    /// Reads data into a given layout.
    pub fn load(&self, layout: &mut Layout) -> Result<String, Box<dyn Error>> {
        match self.path(&layout).to_str() {
            Some(path) => {
                let data = fs::read_to_string(path)?;
                layout.outputs = serde_json::from_str(&data)?;

                Ok(String::new())
            }
            None => panic!("invalid filepath!"),
        }
    }
}
