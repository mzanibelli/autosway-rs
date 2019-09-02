use autosway::Action;
use std::env;

fn main() {
    let action: Action = match env::args().into_iter().skip(1).next().as_ref() {
        Some(arg) if arg == "auto" => Action::Auto,
        Some(arg) if arg == "save" => Action::Save,
        Some(arg) if arg == "list" => Action::List,
        None => Action::Auto,
        _ => panic!("usage: autosway [auto|save|list]"),
    };

    // TODO: use XDG_CONFIG_HOME
    match autosway::run(env::var("SWAYSOCK").unwrap(), String::from("/tmp"), action) {
        Ok(output) => println!("{}", output),
        Err(error) => eprintln!("{:?}", error),
    }
}
