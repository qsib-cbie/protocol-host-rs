use crate::conn::common::*;
use crate::obid::*;
use crate::error::*;

use std::io::prelude::*;


/*Notes on Ethernet connection: 
    Fieg Reader IP: 192.168.10.10, netmask: 255.255.0.0
    Need to match netmask when setting ip for linked computer (as long as ip address matches reader when subnet is 255 it should connect.Ex:192.168.1.1). 
        Connection succeed with IP set to 192.168.10.1 and netmask 255.255.0.0 via direct link with ethernet cable.
        Connection succeed with IP set to 192.168.10.1 and netmask 255.255.0.0 with reader and computer connected to seperate ethernet jacks in wall of bench.

    
*/


pub struct EthernetConnection {
    state: AntennaState,
    response_message_buffer: std::vec::Vec<u8>,
    stream: std::net::TcpStream,
}

impl<'a> Connection<'a> for EthernetConnection {
    fn send_command(self: &mut Self, serial_message: advanced_protocol::HostToReader) -> Result<advanced_protocol::ReaderToHost> {
        let mut serial_message = serial_message;
        let msg = serial_message.serialize();
        log::info!("{:?}",msg);
        let mut attempts = 0;
        loop {
            match self.stream.write(&msg) {
                Ok(bytes_written) => {
                    log::debug!("Sent TCP Command with {} bytes: {}", bytes_written, hex::encode(&msg));
                },
                Err(err) => {
                    log::error!("Failed TCP Command Send: {}", err.to_string());
                    return Err(InternalError::from(err));
                }
            }
        attempts += 1;
            let response_message_size ;
            match self.stream.read(self.response_message_buffer.as_mut_slice()) {
                Ok(bytes_read) => {
                    log::debug!("Received Response to Serial Command with {} bytes: {}", bytes_read, hex::encode(&self.response_message_buffer[..bytes_read]));
                    response_message_size = bytes_read;
                },
                Err(err) => {
                    log::error!("Failed Serial Command Read: {}", err.to_string());
                    continue
                }
            }

            // Interpret the response
            let response = advanced_protocol::ReaderToHost::deserialize(&self.response_message_buffer[..response_message_size])?;
            log::trace!("Interpretting response for attempt {}: {:#?}", attempts, response);


            // Check for errors
            let status  = Status::from(response.status);
            if status == Status::RFWarning {
                /*
                 * A monitor is continusously checking the RF hardware and
                 * if an error occurs the Reader answers every command with
                 * the error code 0x84
                 */
                 let error_message = String::from("Generic Antenna Error: RF hardware monitor error status code 0x84");
                 log::error!("{}", error_message);
                 return Err(InternalError::from(error_message));
            } else if serial_message.device_required && status == Status::NoTransponder {
                log::error!("No devices found on attempt {} of {}", attempts, self.state.max_attempts);
                if attempts >= self.state.max_attempts {
                    return Err(InternalError::from("Failed to communicate with device in antenna"));
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(8 * attempts as u64));
                    continue;
                }
            }

            // All done
            return Ok(response);
        }
    }
    // Err(InternalError::from("Not yet implemented"))
    // }
}

impl EthernetConnection {
    pub fn new(addr: &str) -> Result<EthernetConnection> {
        log::info!("Checking Ethernet Connection");
        match std::net::TcpStream::connect(addr) {
            Ok(stream) => {
                log::info!("Connected to the server!");
                return Ok(EthernetConnection {
                    state: AntennaState {
                            antenna_id: None,
                            pulse_mode: None,
                            hf_mod: None,
                            lf_mod: None,

                            op_mode: None,
                            act_mode: None,
                            act_block_count: None,

                            max_attempts: 5
                        },
                    stream,
                    response_message_buffer: vec![0; 1024 * 1024 * 64],
                })
            }
            Err(_) => {
                log::error!("Couldn't connect to server...");
                return Err(InternalError::from("Could not connect over ethernet"))
            }
        }
        
    }
}

// pub struct EthernetContext { }

// impl<'a> Context<'a> for EthernetContext {
//     type Conn = EthernetConnection;

//     fn connection(self: &'a Self) -> Result<EthernetConnection> {
//         Ok(EthernetConnection::new()?)
//     }
// }

// impl EthernetContext {
//     pub fn new() -> Result<EthernetContext> {
//         EthernetContext {}
//     }
// }