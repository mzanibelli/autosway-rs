use std::clone::Clone;
use std::io;
use std::io::Read;
use std::io::Write;
use std::mem;
use std::os::unix::net::UnixStream;

const MAGIC_STRING: &'static str = "i3-ipc";

/// A message that can be sent to Sway.
pub trait Message {
  fn to_bytes(&self) -> Vec<u8>;
}

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
  pub fn roundtrip(&mut self, m: impl Message) -> Result<Vec<u8>, io::Error> {
    make_request(&mut self.0, m)
      .and_then(|()| read_response_headers(&self.0))
      .and_then(|size| read_n(&self.0, size))
  }
}

impl Clone for Ipc {
  fn clone(&self) -> Self {
    Ipc(self.0.try_clone().unwrap())
  }
}

/// Builds and write the predefined request to the socket.
fn make_request(mut stream: impl Write, mess: impl Message) -> Result<(), io::Error> {
  let mut request = Vec::<u8>::new();
  request.append(&mut MAGIC_STRING.as_bytes().to_vec());
  request.append(&mut mess.to_bytes());
  stream.write_all(&request)
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

  struct Foo;

  impl super::Message for Foo {
    fn to_bytes(&self) -> Vec<u8> {
      vec![0u8, 1u8, 2u8, 3u8]
    }
  }

  #[test]
  fn it_should_generate_a_valid_message() {
    let mut c = io::Cursor::new(Vec::new());
    let expected = vec![105u8, 51u8, 45u8, 105u8, 112u8, 99u8, 0u8, 1u8, 2u8, 3u8];
    super::make_request(&mut c, Foo).unwrap();
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
