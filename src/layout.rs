use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
/// The currently available outputs.
pub struct Layout {
  pub outputs: Vec<Output>,
}

impl Layout {
  /// Returns a new instance from a slice of bytes containing JSON.
  pub fn from_json(input: Vec<u8>) -> Result<Self, serde_json::error::Error> {
    match serde_json::from_slice(&input) {
      Ok(outputs) => Ok(Layout { outputs }),
      Err(error) => Err(error),
    }
  }

  /// Returns a finger print that is unique for a given layout.
  pub fn fingerprint(&self) -> String {
    let mut hasher = Sha256::new();
    hasher.input(self.serialize_ids().join("+++").as_bytes());
    format!("{:x}", hasher.result())
  }

  /// A vector containing Sway commands.
  pub fn serialize_commands(&self) -> Vec<String> {
    self.outputs.iter().map(sway_output_command).collect()
  }

  /// A vector with an unique string for each output.
  fn serialize_ids(&self) -> Vec<String> {
    self.outputs.iter().map(unique_oem_identifier).collect()
  }
}

impl Display for Layout {
  /// Renders each output's string template separated by a line feed.
  fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
    write!(f, "{}", self.serialize_commands().join("\n"))
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
  fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
    write!(f, "{}", sway_output_command(&self))
  }
}

/// Writes the IPC command corresponding to the output.
fn sway_output_command(output: &Output) -> String {
  match output.active {
    // TODO: ensure one shot command works
    true => format!(
      "output {} enable res {} pos {} transform {}",
      output.name,
      format!("{}x{}", output.rect.width, output.rect.height),
      format!("{} {}", output.rect.x, output.rect.y),
      output.transform,
    ),
    false => format!("output {} disable", output.name),
  }
}

/// Writes an unique string for the output.
fn unique_oem_identifier(output: &Output) -> String {
  format!("{}|{}|{}", output.make, output.model, output.serial)
}
