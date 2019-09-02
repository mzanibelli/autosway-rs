use serde::Deserialize;
use serde::Serialize;
use serde_json;
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::fmt::Formatter;

/// The currently available outputs.
pub struct Layout {
    pub outputs: Vec<Output>,
}

impl Layout {
    /// Returns a new instance from a slice of bytes containing JSON.
    pub fn from_json(input: &[u8]) -> Self {
        Layout {
            outputs: serde_json::from_slice(input).unwrap(),
        }
    }

    /// Returns a finger print that is unique for a given layout.
    pub fn fingerprint(&self) -> String {
        let raw: Vec<String> = self.outputs.iter().map(Self::id).collect();
        let mut hasher = Sha256::new();
        hasher.input(raw.join("+++").as_bytes());

        format!("{:x}", hasher.result())
    }

    /// Creates a unique string by output.
    fn id(o: &Output) -> String {
        format!("{}|{}|{}", o.make, o.model, o.serial)
    }
}

impl Display for Layout {
    /// Renders each output's string template separated by a line feed.
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        Ok(self
            .outputs
            .iter()
            .for_each(|o| write!(f, "{}\n", o).unwrap()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// Represents an output.
pub struct Output {
    name: String,
    make: String,
    model: String,
    serial: String,
    transform: String,
    rect: Rect,
    active: bool,
}

#[derive(Serialize, Deserialize, Debug)]
/// Represents the position and size of an output.
struct Rect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl Display for Output {
    /// Writes the IPC command corresponding to the output.
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "output {} res {} pos {} transform {}",
            self.name,
            format!("{}x{}", self.rect.width, self.rect.height),
            format!("{} {}", self.rect.x, self.rect.y),
            self.transform
        )
    }
}
