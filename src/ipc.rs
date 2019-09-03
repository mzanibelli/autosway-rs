use crate::message::Message;
use std::error::Error;
use std::io::Read;
use std::io::Write;
use std::os::unix::net::UnixStream;

const MAGIC_STRING: &'static str = "i3-ipc";

/// Represents all the possible actions.
pub enum Action {
  Auto,
  Save,
  List,
}

/// The connection to Sway.
pub struct Ipc(UnixStream);

impl Ipc {
  /// Connects to a given socket path.
  pub fn connect(path: String) -> Result<Self, Box<dyn Error>> {
    Ok(Ipc(UnixStream::connect(path)?))
  }

  /// Sends a request and returns any valid response body as bytes.
  pub fn roundtrip(&mut self, m: Message) -> Result<Vec<u8>, Box<dyn Error>> {
    make_request(&mut (self.0), m)?;
    Ok(read_response(&self.0)?)
  }

  /// Forks the connection.
  /// Mainly used as a way to obtain mutable reference in closure.
  pub fn spawn(&self) -> Result<Self, Box<dyn Error>> {
    Ok(Ipc(self.0.try_clone()?))
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
  let (l, t) = (mess.len().to_le_bytes(), mess.what().to_le_bytes());
  request.append(&mut l.to_vec());
  request.append(&mut t.to_vec());
  request.append(&mut mess.data().to_vec());
}

/// Returns the response body as a vector of bytes.
fn read_response(stream: &UnixStream) -> Result<Vec<u8>, std::io::Error> {
  let size = read_response_headers(stream)?;
  Ok(read_n(stream, size)?)
}

/// Returns the expected body length as announced by the server.
fn read_response_headers(stream: impl Read) -> Result<usize, std::io::Error> {
  let headers = read_n(stream, 6 + 4 + 4)?; // "i3-ipc" + u32 + u32
  guard_against_invalid_response(&headers);
  Ok(u32::from_le_bytes([headers[6], headers[7], headers[8], headers[9]]) as usize)
}

/// Returns a vector with the next N bytes read from stream.
fn read_n(stream: impl Read, n: usize) -> Result<Vec<u8>, std::io::Error> {
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
