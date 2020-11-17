use crate::conn::common::*;
use crate::obid::*;
use crate::error::*;

use std::{sync::mpsc, thread, time::Duration, io::prelude::*};



/*Notes on Ethernet connection:
    Fieg Reader IP: 192.168.10.10, netmask: 255.255.0.0
    Need to match netmask when setting ip for linked computer (as long as ip address matches reader ip where subnet mask is 255 it should connect.
        Ex:192.168.1.1 since reader ip is 192.168.10.10 and mask is 255.255.0.0). 
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
            log::debug!("Sleep for 50ms");
            std::thread::sleep(std::time::Duration::from_millis(50));
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
}

impl EthernetConnection {
    pub fn new(addr: &str) -> Result<EthernetConnection> {
        log::debug!("Checking Ethernet Connection");
        match std::net::TcpStream::connect(addr) {
            Ok(stream) => {
                log::info!("Connected to the Fieg Reader!");
                return Ok(EthernetConnection {
                    state: AntennaState {
                            antenna_id: None,
                            pulse_mode: None,
                            hf_mod: None,
                            lf_mod: None,

                            command: None,
                            cmd_op: None,
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

pub struct EthernetContext<'a> {
    pub addr: &'a str,
}

impl<'a> EthernetContext<'a> {
    pub fn new(addr: &'a str) -> Result<EthernetContext<'a>> {
        Ok(EthernetContext { addr })
    }
}

impl<'a> Context<'a> for EthernetContext<'a> {
    fn connection(self: &'a Self) -> Result<Box<dyn Connection<'a> + 'a>> {
        Ok(Box::new(EthernetConnection::new(self.addr)?))
    }
}

#[cfg(feature = "ethernet")]
#[test]
fn check_ethernet_connection() -> Result<()> {
    let _ = simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Debug).init();
    _panic_after(Duration::from_millis(5000), move || -> (){
        let work = move || -> Result<()>{
            let context = Box::new(EthernetContext::new("192.168.10.10:10001")?);
            let _connection = Box::new(context.connection()?);
            Ok(())
        };
        match work() {
            Err(err) => panic!("Panicked with error {}", err),
            _ => {}
        }
    });

    Ok(())
}
fn _panic_after<T, F>(d: Duration, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("Unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(_) => handle.join().expect("Thread panicked"),
        Err(_) => panic!("Thread took too long"),
    }
}
