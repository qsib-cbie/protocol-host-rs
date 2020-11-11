use crate::conn::common::*;
use crate::obid::*;
use crate::error::*;

pub struct MockConnection {}

impl<'a> Connection<'a> for MockConnection {
    fn send_command(self: &mut Self, serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost> {
        let mut serial_message = serial_message;
        let msg = serial_message.serialize();
        log::debug!("Sent msg: {:?}",msg);
        let response = advanced_protocol::ReaderToHost::deserialize(&msg)?;
        log::debug!("Recieved response: {:?}",response);
        Ok(response)
    }
}

impl MockConnection {
    pub fn new() -> MockConnection {
        MockConnection {}
    }
}

pub struct MockContext { }

impl<'a> Context<'a> for MockContext {
    fn connection(self: &'a Self) -> Result<Box<dyn Connection<'a> + 'a>> {
        Ok(Box::new(MockConnection::new()))
    }
}

impl MockContext {
    pub fn new() -> MockContext {
        MockContext {}
    }
}