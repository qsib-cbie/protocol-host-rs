use crate::obid::*;

pub trait Connection<'a> {
    fn send_command(self: &mut Self, serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost, Box<dyn std::error::Error>>;
    // fn reset(self: &mut Self) -> Result<(),()>;
}