use crate::message::Message;
use std::io;
use std::io::Read;
use std::io::Write;
use std::mem;
use std::os::unix::net::UnixStream;

const MAGIC_STRING: &'static str = "i3-ipc";

/// The connection to Sway.
pub struct Ipc(UnixStream);

impl Ipc {
  /// Connects to a given socket path.
  pub fn connect(path: String) -> Result<Self, io::Error> {
    match UnixStream::connect(path) {
      Ok(sway) => Ok(Ipc(sway)),
      Err(error) => Err(error),
    }
  }

  /// Sends a request and returns any valid response body as bytes.
  pub fn roundtrip(&mut self, m: Message) -> Result<Vec<u8>, io::Error> {
    match make_request(&mut self.0, m) {
      Ok(_) => read_response(&self.0),
      Err(error) => Err(error),
    }
  }

  /// Forks the connection.
  /// Mainly used as a way to obtain mutable reference in closure.
  /// Panics if we can't clone the underlying socket.
  /// This is probably a very bad idea but remember we're writing in the
  /// socket over a map() - potentially not sequential.
  pub fn clone(&self) -> Self {
    // TODO: actually understand why this seems to work...
    Ipc(self.0.try_clone().unwrap())
  }
}

/// Builds and write the predefined request to the socket.
fn make_request(mut stream: impl Write, mess: Message) -> Result<(), io::Error> {
  let mut request = Vec::<u8>::new();
  write_magic_string(&mut request);
  write_message(&mut request, mess);
  stream.write_all(&request)
}

/// Appends the static `i3-ipc` magic string to the given request.
fn write_magic_string(request: &mut Vec<u8>) {
  unsafe {
    // Unsafe because the compiler cannot guarantee the string is valid UTF-8.
    // This is one of the cases we know _more_ than the compiler.
    request.append(String::from(MAGIC_STRING).as_mut_vec());
  }
}

/// Writes a message to the given request.
fn write_message(request: &mut Vec<u8>, mess: Message) {
  let (l, t) = (mess.len().to_le_bytes(), mess.what().to_le_bytes());
  request.append(&mut l.to_vec());
  request.append(&mut t.to_vec());
  request.append(&mut mess.data().to_vec());
}

/// Returns the response body as a vector of bytes.
fn read_response(stream: &UnixStream) -> Result<Vec<u8>, io::Error> {
  match read_response_headers(stream) {
    Ok(size) => read_n(stream, size),
    Err(error) => Err(error),
  }
}

/// Returns the expected body length as announced by the server.
fn read_response_headers(stream: impl Read) -> Result<usize, io::Error> {
  let headers = read_n(stream, MAGIC_STRING.len() + 2 * mem::size_of::<u32>())?;
  guard_against_invalid_response(&headers);
  Ok(u32::from_le_bytes([headers[6], headers[7], headers[8], headers[9]]) as usize)
}

/// Returns a vector with the next N bytes read from stream.
fn read_n(stream: impl Read, n: usize) -> Result<Vec<u8>, io::Error> {
  let mut result = Vec::<u8>::with_capacity(n);
  stream.take(n as u64).read_to_end(&mut result)?;
  Ok(result)
}

/// Panics if the first six bytes of the reponse are not the magic string.
fn guard_against_invalid_response(headers: &[u8]) {
  assert!(headers.len() > 6);
  assert_eq!(
    String::from_utf8(headers[0..6].to_vec()).as_ref().unwrap(),
    MAGIC_STRING
  )
}
