use super::common::*;
use crate::conn::common::Connection;
use crate::error::*;

pub struct MockProtocol {}

impl MockProtocol {
    pub fn new(_connection: Box<dyn Connection<'_> + '_>) -> MockProtocol {
        MockProtocol {}
    }
}

impl Protocol<'_> for MockProtocol {
    fn handle_message(self: &mut Self, _message: &CommandMessage) -> Result<()> {
        match _message {
            CommandMessage::ActuatorsCommand {
                fabric_name: _,
                timer_mode_block: _,
                actuator_mode_blocks: _,
                op_mode_block: _,
                use_cache: _,
            } => {
                log::debug!("Send command: {:#?}", hex::encode(vec![]));
                Ok(())
            }
            _ => {
                log::debug!("Mock ignoring: {:?}", _message);
                Ok(())
            }
        }
    }
}
