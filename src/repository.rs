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
      .ok_or(StorageError::None)
      .map(String::from)
  }

  /// Writes a file containing layout data in JSON.
  pub fn save(&self, layout: Layout) -> Result<(), StorageError> {
    serde_json::to_string(&layout.outputs)
      .map_err(StorageError::Json)
      .and_then(
        move |data| match fs::write(&self.path(&layout)?, data.as_bytes()) {
          Ok(()) => Ok(()),
          Err(error) => Err(StorageError::Io(error)),
        },
      )
  }

  /// Reads data into a given layout.
  pub fn load(&self, mut layout: Layout) -> Result<Layout, StorageError> {
    fs::read_to_string(&self.path(&layout)?)
      .map_err(StorageError::Io)
      .and_then(move |data| match serde_json::from_str(&data) {
        Ok(outputs) => {
          // TODO: set rect, active and transform according to file
          // Lookup outputs using the OEM identifier string.
          // Right now, if outputs swapped names, the incorrect
          // configuration is applied.
          layout.outputs = outputs;
          Ok(layout)
        }
        Err(error) => Err(StorageError::Json(error)),
      })
  }
}

#[derive(Debug)]
pub enum StorageError {
  /// Results from a file operation error.
  Io(std::io::Error),
  /// Could not encode or decode to/from JSON.
  Json(serde_json::error::Error),
  /// We did not receive a value but expected one.
  None,
}

impl error::Error for StorageError {}

impl fmt::Display for StorageError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      StorageError::Io(ref err) => write!(f, "storage: io: {}", err),
      StorageError::Json(ref err) => write!(f, "storage: json: {}", err),
      StorageError::None => write!(f, "storage: no path"),
    }
  }
}
