use super::common::*;
use crate::error::*;
use crate::conn::common::Connection;


pub struct MockProtocol {}

impl MockProtocol {
    pub fn new(_connection: Box<dyn Connection<'_> + '_>) -> MockProtocol { MockProtocol {}
    }
}

impl Protocol<'_> for MockProtocol {
    fn handle_message(self: &mut Self, _message: &CommandMessage) -> Result<()> {
        Ok(())
    }
}