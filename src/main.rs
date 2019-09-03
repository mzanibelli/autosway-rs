use autosway::Action;
use std::env;

fn main() {
  match autosway::run(
    required_env("SWAYSOCK"),
    required_env("AUTOSWAY"),
    action_from(first_cli_argument()),
  ) {
    Ok(ref output) if output.len() > 0 => println!("{}", output),
    Err(error) => eprintln!("error: {}", error),
    _ => (),
  }
}

/// Parses the action string to choose what to perform next.
fn action_from(action: Option<String>) -> Action {
  match action.as_ref() {
    Some(arg) if arg == "auto" => Action::Auto,
    Some(arg) if arg == "save" => Action::Save,
    Some(arg) if arg == "list" => Action::List,
    None => Action::Auto,
    _ => panic!("usage: autosway [auto|save|list]"),
  }
}

/// The action to be performed, as string.
fn first_cli_argument() -> Option<String> {
  env::args().into_iter().skip(1).next()
}

/// Panics if the environment variable is unset.
fn required_env(name: &str) -> String {
  env::var(name).expect(&format!("${} is unset.", name))
}
