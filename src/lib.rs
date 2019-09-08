mod ipc;
mod layout;
mod message;
mod repository;

use ipc::Ipc;
use layout::Layout;
use message::{Message, Response};
use repository::Repository;
use std::error;
use std::fmt;

pub enum Action {
  /// Automatically configure layout.
  Auto,
  /// Record current layout for future detection.
  Save,
  /// List outputs of the current layout.
  List,
}

/// Runs the program by executing the requested action and return contents for stdout and stderr.
pub fn run(socket_path: String, fs_root: String, action: Action) -> Result<String, Error> {
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
fn connect_to_sway(socket_path: String) -> Result<Ipc, Error> {
  Ipc::connect(socket_path).map_err(Error::Ipc)
}

/// Ask Sway what the current layout is.
fn request_active_layout(ipc: &mut Ipc) -> Result<Layout, Error> {
  ipc
    .roundtrip(Message::GetOutputs)
    .map_err(Error::Ipc)
    .map(|data| serde_json::from_slice(&data))?
    .map_err(Error::ActiveLayout)
}

/// Persist layout without producing stdout content.
fn silently_save_layout(repo: Repository, layout: Layout) -> Result<String, Error> {
  repo
    .save(layout.fingerprint(), &layout)
    .map_err(Error::Save)
    .map(|_| String::new())
}

/// Apply configuration without producing stdout content.
fn silently_configure_layout(repo: Repository, ipc: Ipc, layout: Layout) -> Result<String, Error> {
  apply_configuration(repo, ipc, layout).map(|_| String::new())
}

/// Translate layout to a set of declarative commands and execute them.
fn apply_configuration(repo: Repository, ipc: Ipc, layout: Layout) -> Result<(), Error> {
  merge_or_current(repo, layout)
    .serialize_commands()
    .drain(..)
    .map(Message::RunCommand)
    .map(|m| (ipc.clone(), m))
    .map(run_output_command)
    .collect()
}

/// Merges saved configuration if found, or returns the current layout.
fn merge_or_current(repo: Repository, layout: Layout) -> Layout {
  match repo.load(layout.fingerprint()) {
    Ok(l) => layout.merge(l),
    Err(_) => layout,
  }
}

/// Execute a Sway command and ensure it is successful.
fn run_output_command((mut ipc, message): (Ipc, Message)) -> Result<(), Error> {
  match ipc
    .roundtrip(message)
    .map_err(Error::Ipc)
    .map(Response::bulk_scan)?
  {
    true => Ok(()),
    false => Err(Error::Mutation),
  }
}

#[derive(Debug)]
pub enum Error {
  Ipc(std::io::Error),
  ActiveLayout(serde_json::error::Error),
  Save(repository::StorageError),
  Mutation,
}

impl fmt::Display for Error {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      Error::Ipc(ref err) => write!(f, "could not communicate with sway: {}", err),
      Error::ActiveLayout(ref err) => write!(f, "active layout request failed: {}", err),
      Error::Save(ref err) => write!(f, "could not persist layout: {}", err),
      Error::Mutation => write!(f, "error applying settings for output"),
    }
  }
}

impl error::Error for Error {
  fn description(&self) -> &str {
    match *self {
      Error::Ipc(ref err) => err.description(),
      Error::ActiveLayout(ref err) => err.description(),
      Error::Save(ref err) => err.description(),
      Error::Mutation => "",
    }
  }

  fn cause(&self) -> Option<&dyn error::Error> {
    match *self {
      Error::Ipc(ref err) => Some(err),
      Error::ActiveLayout(ref err) => Some(err),
      Error::Save(ref err) => Some(err),
      Error::Mutation => None,
    }
  }
}
