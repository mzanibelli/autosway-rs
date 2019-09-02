mod ipc;
mod layout;
mod message;
mod repository;

use ipc::Ipc;
use layout::Layout;
use message::{Message, Response};
use repository::Repository;
use std::error::Error;

pub use ipc::Action;

// TODO: implement tests
pub fn run(socket_path: String, fs_root: String, action: Action) -> Result<String, Box<dyn Error>> {
    let (mut ipc, repo) = (Ipc::connect(socket_path)?, Repository::new(fs_root));
    let layout = Layout::from_json(&ipc.roundtrip(Message::GetOutputs)?);
    match action {
        Action::Auto => Ok(configure(repo, ipc, layout).map(|_| String::new())?),
        Action::Save => Ok(repo.save(&layout).map(|_| String::new())?),
        Action::List => Ok(format!("{}", layout)),
    }
}

fn configure(repository: Repository, ipc: Ipc, layout: Layout) -> Result<(), String> {
    repository
        .load(layout)
        .map_err(|err| err.to_string())?
        .outputs
        .iter()
        .map(|o| format!("{}", o))
        .map(Message::RunCommand)
        .map(|m| (ipc.spawn().unwrap(), m))
        .map(command_stream)
        .collect::<Result<Vec<()>, String>>()
        .map(|_| ())
}

fn command_stream((mut ipc, m): (Ipc, Message)) -> Result<(), String> {
    match ipc
        .roundtrip(m)
        .map_err(|err| err.to_string())
        .map(|data| Response::from_json(&data).iter().all(|r| r.success))?
    {
        true => Ok(()),
        false => Err("command execution failed".to_owned()),
    }
}
