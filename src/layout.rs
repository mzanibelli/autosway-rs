use serde::Deserialize;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::clone::Clone;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
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

  /// Returns afinger print that is unique for a given layout.
  pub fn fingerprint(&self) -> String {
    let mut hasher = Sha256::new();
    hasher.input(self.serialize_ids().join("+++").as_bytes());
    format!("{:x}", hasher.result())
  }

  /// A vector containing Sway commands.
  pub fn serialize_commands(&self) -> Vec<String> {
    self
      .activate_only_output()
      .iter()
      .map(sway_output_command)
      .collect()
  }

  /// Merges the configuration of two layouts.
  pub fn merge(mut self, layout: Self) -> Self {
    for ref mut o in &mut (self.outputs) {
      let o2 = layout.find_by_id(unique_oem_identifier(&o));
      o.rect.x = o2.rect.x;
      o.rect.y = o2.rect.y;
      o.rect.width = o2.rect.width;
      o.rect.height = o2.rect.height;
      match &o2.transform {
        Some(t) => o.transform = Some(t.clone()),
        None => (),
      }
    }
    self
  }

  /// Panics if there is no matching output.
  fn find_by_id(&self, id: String) -> &Output {
    self
      .outputs
      .iter()
      .find(|o| unique_oem_identifier(&o) == id)
      .expect("merge: incompatible layout")
  }

  /// A vector with an unique string for each output.
  fn serialize_ids(&self) -> Vec<String> {
    let mut ids = self
      .outputs
      .iter()
      .map(unique_oem_identifier)
      .collect::<Vec<String>>();
    ids.sort();
    ids
  }

  /// Activates any single output.
  fn activate_only_output(&self) -> Vec<Output> {
    let mut result = Vec::new();
    for o in &self.outputs {
      result.push(o.clone());
    }
    if result.len() == 1 {
      result[0].active = true;
    }
    result
  }
}

impl Display for Layout {
  /// Renders each output's string template separated by a line feed.
  fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
    write!(f, "{}", self.serialize_commands().join("\n"))
  }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
/// Represents an output.
pub struct Output {
  name: String,
  make: String,
  model: String,
  serial: String,
  transform: Option<String>,
  rect: Rect,
  active: bool,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Clone)]
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
    true => format!(
      "output {} enable res {} pos {} transform {}",
      output.name,
      format!("{}x{}", output.rect.width, output.rect.height),
      format!("{} {}", output.rect.x, output.rect.y),
      output.transform.as_ref().unwrap_or(&String::from("normal"))
    ),
    false => format!("output {} disable", output.name),
  }
}

/// Writes an unique string for the output.
fn unique_oem_identifier(output: &Output) -> String {
  format!("{}|{}|{}", output.make, output.model, output.serial)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  pub fn it_should_generate_sway_commands_according_to_the_current_layout() {
    let expected = vec![String::from(
      "output eDP1 enable res 1920x1080 pos 0 0 transform normal",
    )];
    let actual = make_layout().serialize_commands();
    assert_eq!(expected, actual);
  }

  #[test]
  pub fn if_transform_is_not_specified_the_generated_command_contains_normal() {
    let expected = vec![String::from(
      "output eDP1 enable res 1920x1080 pos 0 0 transform normal",
    )];
    let mut l = make_layout();
    l.outputs[0].transform = None;
    let actual = l.serialize_commands();
    assert_eq!(expected, actual);
  }

  #[test]
  pub fn it_should_handle_multiple_displays_with_disabled_outputs() {
    let expected = vec![
      String::from("output eDP1 enable res 1920x1080 pos 0 0 transform normal"),
      String::from("output HDMI-2 disable"),
    ];
    let mut l = make_multi_outputs_layout();
    l.outputs[0].transform = None;
    let actual = l.serialize_commands();
    assert_eq!(expected, actual);
  }

  #[test]
  pub fn it_should_activate_any_single_output() {
    let expected = vec![String::from(
      "output eDP1 enable res 1920x1080 pos 0 0 transform normal",
    )];
    let mut l = make_layout();
    l.outputs[0].active = false;
    let actual = l.serialize_commands();
    assert_eq!(expected, actual);
  }

  #[test]
  pub fn merge_should_override_transform() {
    let mut l1 = make_layout();
    let mut l2 = make_layout();
    l2.outputs[0].transform = Some(String::from("270"));
    l1 = l1.merge(l2);
    assert_eq!(Some(String::from("270")), l1.outputs[0].transform);
  }

  #[test]
  pub fn merge_should_override_rect() {
    let mut l1 = make_layout();
    let mut l2 = make_layout();
    l2.outputs[0].rect = super::Rect {
      x: 111,
      y: 222,
      width: 333,
      height: 444,
    };
    l1 = l1.merge(l2);
    assert_eq!(111, l1.outputs[0].rect.x);
    assert_eq!(222, l1.outputs[0].rect.y);
    assert_eq!(333, l1.outputs[0].rect.width);
    assert_eq!(444, l1.outputs[0].rect.height);
  }

  #[test]
  pub fn merge_should_not_override_name() {
    let mut l1 = make_layout();
    let mut l2 = make_layout();
    l2.outputs[0].name = String::from("HDMI-2");
    l1 = l1.merge(l2);
    assert_eq!(String::from("eDP1"), l1.outputs[0].name);
  }

  fn make_layout() -> super::Layout {
    Layout {
      outputs: vec![make_output()],
    }
  }

  fn make_multi_outputs_layout() -> super::Layout {
    let o1 = make_output();
    let mut o2 = make_output();
    o2.name = String::from("HDMI-2");
    o2.active = false;
    super::Layout {
      outputs: vec![o1, o2],
    }
  }

  fn make_output() -> super::Output {
    super::Output {
      name: String::from("eDP1"),
      make: String::from("Samsung"),
      model: String::from("XYZ"),
      serial: String::from("12345"),
      transform: Some(String::from("normal")),
      rect: super::Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
      },
      active: true,
    }
  }
}
