use serde::Deserialize;
use serde::Serialize;
use serde_json;
use sha2::{Digest, Sha256};
use std::env;
use std::error::Error;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};

const MAGIC_STRING: &'static str = "i3-ipc";

fn main() {
    let action: Action = match env::args().into_iter().skip(1).next().as_ref() {
        Some(arg) if arg == "auto" => Action::Auto,
        Some(arg) if arg == "save" => Action::Save,
        Some(arg) if arg == "list" => Action::List,
        None => Action::Auto,
        _ => panic!("usage: autosway [auto|save|list]"),
    };

    // TODO: use XDG_CONFIG_HOME
    match run(env::var("SWAYSOCK").unwrap(), String::from("/tmp"), action) {
        Ok(output) => println!("{}", output),
        Err(error) => eprintln!("{:?}", error),
    }
}

// TODO: extract to library
// TODO: implement tests
fn run(socket_path: String, fs_root: String, action: Action) -> Result<String, Box<dyn Error>> {
    let mut stream = UnixStream::connect(socket_path)?;

    make_request(&mut stream, Message::GetOutputs)?;
    let data = read_response(&stream)?;

    let mut layout = Layout::from_json(&data);
    let repository = Repository::new(&fs_root);

    match action {
        Action::Auto => auto(&repository, &mut stream, &mut layout),
        Action::Save => repository.save(&layout),
        Action::List => Ok(format!("{}", layout)),
    }
}

/// Appends the static `i3-ipc` magic string to the given request.
fn write_magic_string(request: &mut Vec<u8>) {
    unsafe {
        // Unsafe because the compiler cannot guarantee the string is valid UTF-8.
        // This is one of the cases we know _more_ than the compiler.
        request.append(String::from(MAGIC_STRING).as_mut_vec());
    }
}

/// Builds and write the predefined request to the socket.
fn make_request(mut stream: impl Write, mess: Message) -> Result<(), std::io::Error> {
    let mut request = Vec::<u8>::new();
    write_magic_string(&mut request);
    write_message(&mut request, &mess);

    stream.write_all(&request)
}

/// Writes a `GET_OUTPUTS` message to the given request.
fn write_message(request: &mut Vec<u8>, mess: &Message) {
    // Write length (0) and message type (3) sequentially.
    let (l, t) = (mess.len().to_le_bytes(), mess.what().to_le_bytes());
    request.append(&mut l.to_vec());
    request.append(&mut t.to_vec());
    request.append(&mut mess.data().to_vec());
}

/// Returns the response body as a vector of bytes.
fn read_response(stream: &UnixStream) -> Result<Vec<u8>, Box<dyn Error>> {
    let size = read_response_headers(stream)?;

    Ok(read_n(stream, size)?)
}

/// Returns the expected body length as announced by the server.
fn read_response_headers(stream: impl Read) -> Result<usize, Box<dyn Error>> {
    let headers = read_n(stream, 6 + 4 + 4)?; // "i3-ipc" + u32 + u32
    guard_against_invalid_response(&headers);

    Ok(u32::from_le_bytes([headers[6], headers[7], headers[8], headers[9]]) as usize)
}

/// Returns a vector with the next N bytes read from stream.
fn read_n(stream: impl Read, n: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::<u8>::with_capacity(n);
    stream.take(n as u64).read_to_end(&mut result)?;

    Ok(result)
}

/// Panics if the first six bytes of the reponse are not the magic string.
fn guard_against_invalid_response(headers: &[u8]) {
    // TODO: should not panic
    assert!(headers.len() > 6);
    assert_eq!(
        String::from_utf8(headers[0..6].to_vec()).as_ref().unwrap(),
        MAGIC_STRING
    )
}

/// Configures the detected layout.
fn auto(
    repository: &Repository,
    stream: &mut UnixStream,
    layout: &mut Layout,
) -> Result<String, Box<dyn Error>> {
    repository.load(layout)?;

    layout
        .outputs
        .iter()
        .map(|o| format!("{}", o))
        .map(Message::RunCommand)
        .map(|m| make_checked_request(stream.try_clone()?, m))
        .for_each(|r| r.unwrap());

    Ok(String::new())
}

/// Panics in case of unsuccessful response.
fn make_checked_request(mut stream: UnixStream, m: Message) -> Result<(), Box<dyn Error>> {
    make_request(&mut stream, m)?;
    let data = read_response(&stream)?;
    for r in Response::from_json(&data).iter() {
        // TODO: should not panic
        assert!(r.success);
    }

    Ok(())
}

/// The currently available outputs.
struct Layout {
    outputs: Vec<Output>,
}

impl Layout {
    /// Returns a new instance from a slice of bytes containing JSON.
    fn from_json(input: &[u8]) -> Self {
        Layout {
            outputs: serde_json::from_slice(input).unwrap(),
        }
    }

    /// Returns a finger print that is unique for a given layout.
    fn fingerprint(&self) -> String {
        let raw: Vec<String> = self.outputs.iter().map(Self::id).collect();
        let mut hasher = Sha256::new();
        hasher.input(raw.join("+++").as_bytes());

        format!("{:x}", hasher.result())
    }

    /// Creates a unique string by output.
    fn id(o: &Output) -> String {
        format!("{}|{}|{}", o.make, o.model, o.serial)
    }
}

impl Display for Layout {
    /// Renders each output's string template separated by a line feed.
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        Ok(self
            .outputs
            .iter()
            .for_each(|o| write!(f, "{}\n", o).unwrap()))
    }
}

#[derive(Serialize, Deserialize, Debug)]
/// Represents an output.
struct Output {
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
    /// Writes the IPC command corresponding to the output.
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "output {} res {} pos {} transform {}",
            self.name,
            format!("{}x{}", self.rect.width, self.rect.height),
            format!("{} {}", self.rect.x, self.rect.y),
            self.transform
        )
    }
}

/// Represents all the possible actions.
enum Action {
    Auto,
    Save,
    List,
}

/// Subset of the messages supported by the IPC protocol.
enum Message {
    GetOutputs,
    RunCommand(String),
}

impl Message {
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

/// Responsible for saving and loading layouts to/from the filesystem.
struct Repository(String);

impl Repository {
    /// Returns a new Repository that gets data from a given folder.
    fn new(fs_root: &str) -> Self {
        Repository(String::from(fs_root))
    }

    /// Returns the filepath for a given layout.
    fn path(&self, layout: &Layout) -> PathBuf {
        Path::new(&self.0).join(layout.fingerprint())
    }

    /// Writes a file containing layout data in JSON.
    fn save(&self, layout: &Layout) -> Result<String, Box<dyn Error>> {
        match self.path(&layout).to_str() {
            Some(path) => {
                let content = serde_json::to_string(&layout.outputs)?;
                fs::write(path, content.as_bytes())?;

                Ok(String::new())
            }
            None => panic!("invalid filepath!"),
        }
    }

    /// Reads data into a given layout.
    fn load(&self, layout: &mut Layout) -> Result<String, Box<dyn Error>> {
        match self.path(&layout).to_str() {
            Some(path) => {
                let data = fs::read_to_string(path)?;
                layout.outputs = serde_json::from_str(&data)?;

                Ok(String::new())
            }
            None => panic!("invalid filepath!"),
        }
    }
}

#[derive(Debug, Deserialize)]
/// Represents the output of a RunCommand command.
struct Response {
    success: bool,
}

impl Response {
    /// Retuns an instance read from JSON as bytes.
    fn from_json(input: &[u8]) -> Vec<Self> {
        serde_json::from_slice(input).unwrap()
    }
}
