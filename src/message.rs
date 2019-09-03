use serde::Deserialize;

#[derive(Debug)]
/// Subset of the messages supported by the IPC protocol.
pub enum Message {
  GetOutputs,
  RunCommand(String),
}

impl Message {
  /// Returns the type (as in the protocol) of the message.
  pub fn what(&self) -> u32 {
    match &self {
      Self::GetOutputs => 3,
      Self::RunCommand(_) => 0,
    }
  }

  /// Returns the length of the payload.
  pub fn len(&self) -> u32 {
    match &self {
      Self::GetOutputs => 0,
      Self::RunCommand(data) => data.len() as u32,
    }
  }

  /// Returns the payload data.
  pub fn data(&self) -> Vec<u8> {
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
