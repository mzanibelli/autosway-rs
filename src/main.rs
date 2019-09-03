use autosway::Action;
use std::env;

fn main() {
  let action = match first_cli_argument().as_ref() {
    Some(arg) if arg == "auto" => Action::Auto,
    Some(arg) if arg == "save" => Action::Save,
    Some(arg) if arg == "list" => Action::List,
    None => Action::Auto,
    _ => panic!("usage: autosway [auto|save|list]"),
  };

  match autosway::run(required_env("SWAYSOCK"), required_env("AUTOSWAY"), action) {
    Ok(output) => println!("{}", output),
    Err(error) => eprintln!("autosway: error: {}", error),
  }
}

/// The action to be performed, as string.
fn first_cli_argument() -> Option<String> {
  env::args().into_iter().skip(1).next() // $0 is the binary base name
}

/// Panics if the environment variable is unset.
fn required_env(name: &str) -> String {
  env::var(name).expect(&format!("${} is unset.", name))
}
