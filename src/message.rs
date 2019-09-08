use serde::Deserialize;

#[derive(Debug)]
/// Subset of the messages supported by the IPC protocol.
pub enum Message {
  GetOutputs,
  RunCommand(String),
}

impl Message {
  /// Serialize message.
  pub fn to_bytes(&self) -> Vec<u8> {
    let mut result = Vec::new();
    result.append(&mut self.len().to_le_bytes().to_vec());
    result.append(&mut self.what().to_le_bytes().to_vec());
    result.append(&mut self.data());
    result
  }

  /// Returns the type (as in the protocol) of the message.
  fn what(&self) -> u32 {
    match &self {
      Self::GetOutputs => 3,
      Self::RunCommand(_) => 0,
    }
  }

  /// Returns the length of the payload.
  fn len(&self) -> u32 {
    match &self {
      Self::GetOutputs => 0,
      Self::RunCommand(data) => data.len() as u32,
    }
  }

  /// Returns the payload data.
  fn data(&self) -> Vec<u8> {
    match &self {
      Self::GetOutputs => Vec::<u8>::new(),
      Self::RunCommand(data) => data.as_bytes().to_vec(),
    }
  }
}

#[derive(Debug, Deserialize)]
/// Represents the output of a RunCommand command.
pub struct Response {
  pub success: bool,
}

impl Response {
  /// Retuns true if all responses are successful.
  pub fn bulk_scan(input: Vec<u8>) -> bool {
    match serde_json::from_slice::<Vec<Self>>(&input) {
      Ok(resp) => resp.iter().all(|r| r.success),
      _ => false,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_should_serialize_a_standard_get_outputs_message() {
    let expected = vec![0, 0, 0, 0, 3, 0, 0, 0];
    let actual = super::Message::GetOutputs.to_bytes();
    assert_eq!(expected, actual);
  }

  #[test]
  fn it_should_serialize_a_run_command_message_with_a_payload() {
    let expected = vec![3, 0, 0, 0, 0, 0, 0, 0, 102, 111, 111];
    let actual = super::Message::RunCommand(String::from("foo")).to_bytes();
    assert_eq!(expected, actual);
  }

  #[test]
  fn it_should_return_true_if_all_responses_are_successful() {
    let input = String::from(
      r#"
      [
        {"success": true},
        {"success": true},
        {"success": true}
      ]
    "#,
    )
    .as_bytes()
    .to_vec();
    assert!(Response::bulk_scan(input));
  }

  #[test]
  fn it_should_return_false_with_invalid_json() {
    let input = String::from(
      r#"
      [
        {"success": true}
        {"success": true}
        {"success": true}
      ]
    "#,
    )
    .as_bytes()
    .to_vec();
    assert!(!Response::bulk_scan(input));
  }

  #[test]
  fn it_should_return_false_if_one_response_is_unsuccessful() {
    let input = String::from(
      r#"
      [
        {"success": true},
        {"success": false},
        {"success": true}
      ]
    "#,
    )
    .as_bytes()
    .to_vec();
    assert!(!Response::bulk_scan(input));
  }
}
