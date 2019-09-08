use serde::de::DeserializeOwned;
use serde::ser::Serialize;
use std::error;
use std::fmt;
use std::fs;
use std::io;
use std::path::Path;

/// Responsible for saving and loading layouts to/from the filesystem.
pub struct Repository(String);

impl Repository {
  /// Returns a new Repository that gets data from a given folder.
  pub fn new(fs_root: String) -> Self {
    Repository(fs_root)
  }

  /// Writes a file containing layout data in JSON.
  pub fn save<T>(&self, id: String, entity: T) -> Result<(), StorageError>
  where
    T: Serialize,
  {
    serde_json::to_string(&entity)
      .map_err(StorageError::Json)
      .map(|data| fs::write(&self.path(id), data.as_bytes()))?
      .map_err(StorageError::Io)
  }

  /// Reads data into a given layout.
  pub fn load<T>(&self, id: String) -> Result<T, StorageError>
  where
    T: DeserializeOwned,
  {
    fs::File::open(&self.path(id))
      .map_err(StorageError::Io)
      .map(|fd| serde_json::from_reader(fd))?
      .map_err(StorageError::Json)
  }

  /// Returns the filepath for a given layout.
  /// Panics if we can't build the path.
  fn path(&self, id: String) -> String {
    Path::new(&self.0)
      .join(id)
      .to_str()
      .map(String::from)
      .unwrap()
  }
}

#[derive(Debug)]
pub enum StorageError {
  /// Results from a file operation error.
  Io(io::Error),
  /// Could not encode or decode to/from JSON.
  Json(serde_json::error::Error),
}

impl error::Error for StorageError {}

impl fmt::Display for StorageError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      StorageError::Io(ref err) => write!(f, "storage: io: {}", err),
      StorageError::Json(ref err) => write!(f, "storage: json: {}", err),
    }
  }
}

impl From<io::Error> for StorageError {
  fn from(err: io::Error) -> StorageError {
    StorageError::Io(err)
  }
}

impl From<serde_json::error::Error> for StorageError {
  fn from(err: serde_json::error::Error) -> StorageError {
    StorageError::Json(err)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::panic;

  fn with_tmp_dir<T>(test: T) -> ()
  where
    T: FnOnce(String) -> () + panic::UnwindSafe,
  {
    let dir = tempfile::tempdir().unwrap();
    let result = panic::catch_unwind(|| test(dir.path().to_str().unwrap().to_string()));
    assert!(result.is_ok())
  }

  #[test]
  fn it_should_save_data_to_a_json_file() {
    with_tmp_dir(|root| {
      let (sut, path) = make_sut(root);
      let x: u32 = 42;
      sut.save(String::from("sut"), x).unwrap();
      let actual = fs::read_to_string(Path::new(&path)).unwrap();
      assert_eq!(String::from("42"), actual);
    });
  }

  #[test]
  fn it_should_retrieve_data_from_a_json_file() {
    with_tmp_dir(|root| {
      let (sut, path) = make_sut(root);
      fs::write(Path::new(&path), String::from("42").as_bytes()).unwrap();
      let actual: u32 = sut.load(String::from("sut")).unwrap();
      assert_eq!(42, actual);
    });
  }

  fn make_sut(root: String) -> (Repository, String) {
    (
      Repository::new(root.clone()),
      Path::new(&root)
        .join(String::from("sut"))
        .to_str()
        .map(String::from)
        .unwrap(),
    )
  }
}
