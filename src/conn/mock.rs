use crate::conn::common::*;
use crate::obid::*;
use crate::error::*;

pub struct MockConnection {}

impl<'a> Connection<'a> for MockConnection {
    fn send_command(self: &mut Self, _serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost> {
        Err(InternalError::from("Not yet implemented"))
    }
}

impl MockConnection {
    pub fn new() -> MockConnection {
        MockConnection {}
    }
}

pub struct MockContext { }

impl<'a> Context<'a> for MockContext {
    type Conn = MockConnection;

    fn connection(self: &'a Self) -> Result<MockConnection> {
        Ok(MockConnection::new())
    }
}

impl MockContext {
    pub fn new() -> MockContext {
        MockContext {}
    }
}