use crate::obid::*;
use crate::error::*;

pub trait Connection<'a> {
    fn send_command(self: &mut Self, serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost>;
    // fn reset(self: &mut Self) -> Result<(),()>;
}

pub trait Context<'a> {
    type Conn: Connection<'a>;

    fn connection(self: &'a Self) -> Result<Self::Conn>;
}