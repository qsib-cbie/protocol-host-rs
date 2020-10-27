use crate::obid::*;
use crate::error::*;

#[derive(Debug)]
pub struct AntennaState {
    /// A Usb Connection manipulates a Feig reader and an NFC antenna

    pub antenna_id: Option<String>,
    pub pulse_mode: Option<i32>,
    pub hf_mod: Option<i32>,
    pub lf_mod: Option<i32>,
    pub act_block_count: Option<i32>,

    pub op_mode: Option<String>,
    pub act_mode: Option<String>,

    pub max_attempts: i32,
}
pub trait Connection<'a> {
    fn send_command(self: &mut Self, serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost>;
    // fn reset(self: &mut Self) -> Result<(),()>;
}

pub trait Context<'a> {
    type Conn: Connection<'a>;

    fn connection(self: &'a Self) -> Result<Self::Conn>;
}