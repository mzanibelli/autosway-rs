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
    make_request(&mut self.0, m)
      .and_then(|()| read_response_headers(&self.0))
      .and_then(|size| read_n(&self.0, size))
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_should_generate_a_valid_standard_get_outputs_message() {
    let mut c = io::Cursor::new(Vec::new());
    let expected = vec![
      //                                     | size              | type
      105u8, 51u8, 45u8, 105u8, 112u8, 99u8, 0u8, 0u8, 0u8, 0u8, 3u8, 0u8, 0u8, 0u8,
    ];
    super::make_request(&mut c, Message::GetOutputs).unwrap();
    assert_eq!(&expected, c.get_ref());
  }

  #[test]
  fn it_should_generate_a_valid_run_command_message_with_payload() {
    let mut c = io::Cursor::new(Vec::new());
    let expected = vec![
      //                                     | size              | type              | payload
      105u8, 51u8, 45u8, 105u8, 112u8, 99u8, 3u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 102u8, 111u8,
      111u8,
    ];
    super::make_request(&mut c, Message::RunCommand(String::from("foo"))).unwrap();
    assert_eq!(&expected, c.get_ref());
  }

  #[test]
  fn it_should_read_the_expected_payload_size_from_the_headers() {
    let c = io::Cursor::new(vec![
      //                                     | size              | type              | payload
      105u8, 51u8, 45u8, 105u8, 112u8, 99u8, 3u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 0u8, 102u8, 111u8,
      111u8,
    ]);
    let actual = super::read_response_headers(c).unwrap();
    assert_eq!(3, actual);
  }

  #[test]
  #[should_panic]
  fn it_should_panic_if_the_headers_dont_start_with_magic_string() {
    let c = io::Cursor::new(vec![
      //                               |x    | size              | type
      105u8, 51u8, 45u8, 105u8, 112u8, 98u8, 0u8, 0u8, 0u8, 0u8, 3u8, 0u8, 0u8, 0u8,
    ]);
    super::read_response_headers(c).unwrap();
  }

  #[test]
  fn it_should_read_a_limited_number_of_bytes() {
    let c = io::Cursor::new(vec![105u8, 51u8, 45u8, 105u8]);
    let actual = super::read_n(c, 2).unwrap();
    assert_eq!(vec![105u8, 51u8], actual);
  }
}
