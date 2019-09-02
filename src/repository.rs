use crate::layout::Layout;
use std::error;
use std::fmt;
use std::fs;
use std::path::Path;

/// Responsible for saving and loading layouts to/from the filesystem.
pub struct Repository(String);

impl Repository {
    /// Returns a new Repository that gets data from a given folder.
    pub fn new(fs_root: String) -> Self {
        Repository(fs_root)
    }

    /// Returns the filepath for a given layout.
    fn path(&self, layout: &Layout) -> Result<String, StorageError> {
        Path::new(&self.0)
            .join(layout.fingerprint())
            .to_str()
            .ok_or(StorageError::NoPath)
            .map(String::from)
    }

    /// Writes a file containing layout data in JSON.
    pub fn save(&self, layout: &Layout) -> Result<(), StorageError> {
        let data = serde_json::to_string(&layout.outputs).map_err(StorageError::Json)?;
        fs::write(&self.path(&layout)?, data.as_bytes()).map_err(StorageError::Io)?;
        Ok(())
    }

    /// Reads data into a given layout.
    pub fn load(&self, mut layout: Layout) -> Result<Layout, StorageError> {
        let data = fs::read_to_string(&self.path(&layout)?).map_err(StorageError::Io)?;
        layout.outputs = serde_json::from_str(&data).map_err(StorageError::Json)?;
        Ok(layout)
    }
}

#[derive(Debug)]
pub enum StorageError {
    Io(std::io::Error),
    Json(serde_json::error::Error),
    NoPath,
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StorageError::Io(ref err) => write!(f, "storage: io: {}", err),
            StorageError::Json(ref err) => write!(f, "storage: json: {}", err),
            StorageError::NoPath => write!(f, "storage: no path"),
        }
    }
}

impl error::Error for StorageError {
    fn description(&self) -> &str {
        match *self {
            StorageError::Io(ref err) => err.description(),
            StorageError::Json(ref err) => error::Error::description(err),
            StorageError::NoPath => "no path",
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            StorageError::Io(ref err) => Some(err),
            StorageError::Json(ref err) => Some(err),
            StorageError::NoPath => None,
        }
    }
}
