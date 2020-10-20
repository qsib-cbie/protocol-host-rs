use crate::conn::common::*;
use crate::obid::*;

pub struct MockConnection {}

impl<'a> Connection<'a> for MockConnection {
    fn send_command(self: &mut Self, _serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost, Box<dyn std::error::Error>> {
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Not yet implemented")))
    }
}

impl MockConnection {
    pub fn new() -> MockConnection {
        MockConnection {}
    }
}