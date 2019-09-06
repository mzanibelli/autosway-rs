mod ipc;
mod layout;
mod message;
mod repository;

use ipc::Ipc;
use layout::Layout;
use message::{Message, Response};
use repository::Repository;

pub enum Action {
  /// Automatically configure layout.
  Auto,
  /// Record current layout for future detection.
  Save,
  /// List outputs of the current layout.
  List,
}

// TODO: discover Rust testing

/// Runs the program by executing the requested action and return contents for stdout and stderr.
pub fn run(socket_path: String, fs_root: String, action: Action) -> Result<String, String> {
  connect_to_sway(socket_path).and_then(move |mut ipc| {
    match (
      Repository::new(fs_root),
      request_active_layout(&mut ipc),
      action,
    ) {
      (repo, Ok(layout), Action::Auto) => silently_configure_layout(repo, ipc, layout),
      (repo, Ok(layout), Action::Save) => silently_save_layout(repo, layout),
      (_, Ok(layout), _) => Ok(layout.to_string()),
      (_, Err(error), _) => Err(error),
    }
  })
}

/// Returns a handy IPC instance.
fn connect_to_sway(socket_path: String) -> Result<Ipc, String> {
  match Ipc::connect(socket_path) {
    Ok(ipc) => Ok(ipc),
    Err(error) => Err(error.to_string()),
  }
}

/// Ask Sway what the current layout is.
fn request_active_layout(ipc: &mut Ipc) -> Result<Layout, String> {
  match ipc.roundtrip(Message::GetOutputs) {
    Ok(data) => unserialize_layout(data),
    Err(error) => Err(error.to_string()),
  }
}

/// Unserialize layout from JSON.
fn unserialize_layout(data: Vec<u8>) -> Result<Layout, String> {
  match Layout::from_json(data) {
    Ok(layout) => Ok(layout),
    Err(error) => Err(error.to_string()),
  }
}

/// Apply configuration without producing stdout content.
fn silently_configure_layout(repo: Repository, ipc: Ipc, layout: Layout) -> Result<String, String> {
  match apply_configuration(repo, ipc, layout) {
    Ok(_) => Ok(String::new()),
    Err(error) => Err(error.to_string()),
  }
}

/// Persist layout without producing stdout content.
fn silently_save_layout(repo: Repository, layout: Layout) -> Result<String, String> {
  match repo.save(&layout) {
    Ok(_) => Ok(String::new()),
    Err(error) => Err(error.to_string()),
  }
}

/// Translate layout to a set of declarative commands and execute them.
fn apply_configuration(repo: Repository, ipc: Ipc, layout: Layout) -> Result<(), String> {
  repo
    .load(&layout)
    .map_err(|e| e.to_string())
    .map(move |l| layout.merge(l))?
    .serialize_commands()
    .drain(..)
    .map(Message::RunCommand)
    .map(|m| (ipc.clone(), m))
    .map(run_output_command)
    .collect()
}

/// Execute a Sway command and ensure it is successful.
fn run_output_command((mut ipc, message): (Ipc, Message)) -> Result<(), String> {
  match ipc
    .roundtrip(message)
    .map_err(|e| e.to_string())
    .map(Response::bulk_scan)?
  {
    true => Ok(()),
    false => Err(String::from("could not configure output")),
  }
}
