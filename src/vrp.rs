#![allow(dead_code)]

use serial::{ObidSerialReceivable, ObidSerialSendable};

#[path = "serial.rs"] mod serial;

#[derive(Debug)]
pub struct ObidTransponder {
    uid: smallvec::SmallVec<[u8; 8]>, // 8-byte serial number
    tr_type_rf_tec: u8,
    tr_type_type_no: u8,
    dsfid: u8
}

pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub transponders: smallvec::SmallVec<[ObidTransponder; 2]>,
}

impl std::fmt::Debug for Fabric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fabric(name: '{}')", self.name)
    }

}

impl Fabric {
    pub fn new(conn: &UsbConnection, name: &str) -> Result<Fabric, Box<dyn std::error::Error>> {
        let mut fabric = Fabric {
            name: String::from(name),
            transponders: smallvec::smallvec![],
        };

        fabric.transponders = conn.get_inventory()?;

        Ok(fabric)
    }
}

/// Feig-based Connections are documented here
/// http://www.sebeto.com/intranet/ftpscambio/RFID_FEIG/Readers/ID%20ISC%20LR2500/Documentation/H01112-0e-ID-B.pdf

pub struct UsbConnection<'a> {
    state: AntennaState,
    device_handle: libusb::DeviceHandle<'a>,

}

#[derive(Debug)]
pub struct AntennaState {
    /// A Usb Connection manipulates a Feig reader and an NFC antenna
    
    antenna_id: Option<String>,
    pulse_mode: Option<i32>,
    hf_mod: Option<i32>,
    lf_mod: Option<i32>,
    act_block_count: Option<i32>,

    op_mode: Option<String>,
    act_mode: Option<String>,

    max_attempts: i32,

}

impl<'a> UsbConnection<'a> {
    pub fn new(ctx: &'a libusb::Context) -> Result<UsbConnection<'a>, Box<dyn std::error::Error>> {
        for device in ctx.devices()?.iter() {
            let device_desc = device.device_descriptor()?;
            log::trace!("Found USB Device || Bus {:03} Device {:03} ID {} : {}",
            device.bus_number(),
            device.address(),
            device_desc.vendor_id(),
            device_desc.product_id());

            if device_desc.vendor_id() == 2737 {
                log::debug!("Found Obid/Feig USB Device || Bus {:03} Device {:03} ID {} : {}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id());
            
                let mut device_handle = device.open()?;
                device_handle.reset()?;
                for interface in device.active_config_descriptor()?.interfaces() {
                    let interface_number = interface.number();
                    if device_handle.kernel_driver_active(interface_number)? {
                        log::debug!("Detaching kernel from interface: {}", interface_number);
                        device_handle.detach_kernel_driver(interface_number)?;
                    }
                    log::debug!("Claiming interface: {}", interface_number);
                    device_handle.claim_interface(interface_number)?;
                    for interface_descriptor in interface.descriptors() {
                        log::trace!("Interface Descriptor of {}: {:#?}", interface_number, interface_descriptor);
                        for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                            log::trace!("Endpoint Descriptor of {}: {:#?}", interface_number, endpoint_descriptor);
                        }
                    }
                }
                
                return Ok(UsbConnection {
                    device_handle: device_handle,
                    state: AntennaState {
                        antenna_id: None,
                        pulse_mode: None,
                        hf_mod: None,
                        lf_mod: None,

                        op_mode: None,
                        act_mode: None,
                        act_block_count: None,

                        max_attempts: 5
                    }
                });
            }
        }

        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No matching USB device found")));
    }

   /**
     * This command reads the UID of all Transponders inside the antenna field.
     * If the Reader has detected a new Transponder, that Transponder will be 
     * automatically set in the quiet state by the Reader. In this state the 
     * Transponder does not send back a response until the next inventory command.
     * 
     * @return transponders in the array
     */
    pub fn get_inventory(self: & Self) -> Result<smallvec::SmallVec<[ObidTransponder; 2]>, Box<dyn std::error::Error>> {
        log::trace!("Requesting inventory ids ...");
        let inventory_request = serial::advanced_protocol::HostToReader::new(0, 0xFF, 0xB0, vec![0x01, 0x00].as_slice(), 0, false);
        let inventory_response = self.send_command(inventory_request)?;
        log::info!("Received inventory_response: {:#?}", inventory_response);

        if inventory_response.status == 0 && inventory_response.data.len() > 0 {
            let mut transponders = smallvec::smallvec![];
            let encoded_transponders = inventory_response.data[0];
            let bytes_per_transponder = 1 + 1 + 8; // tr_type, dsfid, uid
            if inventory_response.data.len() != 1 + (encoded_transponders as usize) * (bytes_per_transponder as usize) {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unexpected data format in response to inventory request")));
            }

            for i in 0..encoded_transponders {
                let begin = (1 + bytes_per_transponder * i) as usize;
                let end = begin + bytes_per_transponder as usize;
                let encoded_transponder_slice = &inventory_response.data[begin..end]; 
                let tr_type = encoded_transponder_slice[0];
                let tr_type_rf_tec = (tr_type & 0b1100_0000) >> 6;
                let tr_type_type_no = tr_type & 0b0000_1111;
                let dsfid = encoded_transponder_slice[1];
                let uid = &encoded_transponder_slice[2..];
                transponders.push(ObidTransponder {
                    uid: smallvec::SmallVec::from(uid),
                    tr_type_rf_tec,
                    tr_type_type_no,
                    dsfid
                });
            }
            Ok(transponders)
        } else {
            Ok(smallvec::smallvec![])
        }
    }

    /// Set the wattage for the RF power on the antenna
    pub fn set_radio_freq_power(self: &mut Self, rf_power: u8) ->  Result<(), Box<dyn std::error::Error>> {
        log::trace!("Requesting RF power set to {} ...", rf_power);
        /*
         * RF Power format: 0bX0111111
         * Supported Wattage is [Low Power] union [2W, 12W] in 0.25W steps
         * 
         * If X is 1, then 0b00111111 is interpretting as 1/4 Watts.
         * Using 1/4 W, the boundaries are
         *   - 0x04 -> Low Power
         *   - 0x08 -> 2 W
         *   - 0x00111111 -> 12 W
         *
         * If X is 0, then 2 is th minimum and 12 is the max as 1 W steps
         */
        let encoded_rf_power;
         if rf_power == 0 {
            encoded_rf_power = 0b1000_0000 | 0x04;
            // encoded_rf_power = 0;
         } else {
            encoded_rf_power = 0b1000_0000 | (0b0011_1111 & (rf_power * 4));
            // encoded_rf_power = rf_power;
         }

        // Confusing command from python writes to more than CFG3 by 16 bytes as all 0s
        // msg = [2,0,44,255,139,2,1,1,1,30,0,3,0,8,pow,128,0,0,0,0,0,0,0,0,0,128,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]

        let data = vec![
            0x2, // Device
            0x1, // Bank
            0x1, // Mode
            0x1, // CFG-N
            30, // Block Size  // IDK why we need the extra 16 zeros that don't map to the CFG block
            0x00, // MSB CFG-ADR
            0x03, // LSB CFG-ADR 
            0x00,             // CFG-Data :: CFG3 Byte 0 TAG-DRV
            0x09,             // CFG-Data :: CFG3 Byte 1 TAG-DRV
            encoded_rf_power, // CFG-Data :: CFG3 Byte 2 RF-POWER
            0x80,             // CFG-Data :: CFG3 Byte 3 EAS-LEVEL
            0,0,0,            // CFG-Data :: CFG3 Byte 4,5,6 0x00
            0,0,0,0,0,0,      // CFG-Data :: CFG3 Byte 7,8,9,10,11,12 0x00
            0b1000_0000       // CFG-Data :: CFG3 Byte 13 FU_COM,
            ,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0 // IDK WHY THIS IS REQUIRED
        ];



        let request = serial::advanced_protocol::HostToReader::new(0, 0xFF, 0x8B, data.as_slice(), 0, false);
        let response = self.send_command(request)?;
        log::info!("Received response: {:#?}", response);    
        if response.status == 0x11 {
            let error_message = "A reasonableness check failed while writing the RF power parameter to the reader" ;
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)))
        } else {
            Ok(())
        }
    }

    pub fn system_reset(self: &mut Self)  -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("Requesting System Reset of RF controller ...");
        let request = serial::advanced_protocol::HostToReader::new(0, 0xFF, 0x64, vec![0].as_slice(), 0, false);
        let response = self.send_command(request)?;
        if response.status != 0x00 {
            let error_message = format!("System reset failed with status code: {}. See Annex D of system manual for more information", response.status);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)))
        } else {
            Ok(())
        }
    }

    // /// Set operating mode (i.e. All off, turn on/off, single pulse, continous)
    // fn get_op_mode(self: &mut Self) -> Result<String, Box<dyn std::error::Error>> {
    //     // TODO: Fix how the modes are put together
    //     if self.state.pulse_mode.is_some() && self.state.hf_mod.is_some() && self.state.lf_mod.is_some() {
    //         let op_mode = self.state.pulse_mode.unwrap() + self.state.hf_mod.unwrap() + self.state.lf_mod.unwrap();
    //         Ok(String::from(format!("{:X}", op_mode)))
    //     } else {
    //         log::error!("Cannot set op mode for state: {:#?}", self.state);
    //         Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid state to set op mode")))
    //     }
    // }

    // /// Set actuation mode (i.e.Unipolar, bipolar)
    // fn get_act_mode(self: &mut Self, mode: bool) -> Result<String, Box<dyn std::error::Error>> {
    //     if mode {
    //         Ok(String::from(format!("{:X}", 1)))
    //     } else {
    //         Ok(String::from(format!("{:X}", 0)))
    //     }
    // }

    // /// Set the number of actuator blocks (32 actuators per block)
    // fn get_act_block_count(self: &mut Self, num_blocks: i32) -> Result<String, Box<dyn std::error::Error>> {
    //     Ok(String::from(format!("{:X}", num_blocks)))
    // }

    fn send_command(self: & Self, serial_message: serial::advanced_protocol::HostToReader) -> Result<serial::advanced_protocol::ReaderToHost, Box<dyn std::error::Error>> {
        // Marshal the serial command
        let mut serial_message = serial_message;
        let msg = serial_message.serialize();

        let mut attempts = 0;
        let mut response_message_buffer = vec![0; 1024 * 1024 * 64]; // Max message size
        loop {
            // Documented not less than 5 milliseconds
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Send the command to the Feig reader
            match self.device_handle.write_bulk(2, msg.as_slice(), std::time::Duration::from_millis(25)) {
                Ok(bytes_written) => {
                    log::debug!("Sent Serial Command with {} bytes: {}", bytes_written, hex::encode(&msg));
                },
                Err(err) => {
                    log::error!("Failed Serial Command Send: {}", err.to_string());
                    return Err(Box::new(err));
                }
            }

            // Read the response to the command
            attempts += 1;
            let response_message_size ;
            match self.device_handle.read_bulk(129, &mut response_message_buffer, std::time::Duration::from_millis(500)) {
                Ok(bytes_read) => {
                    log::debug!("Received Response to Serial Command with {} bytes: {}", bytes_read, hex::encode(&response_message_buffer[..bytes_read]));
                    response_message_size = bytes_read;
                },
                Err(err) => {
                    log::error!("Failed Serial Command Read: {}", err.to_string());
                    continue
                    // return Err(Box::new(err));
                }
            }

            // Interpret the response
            let response = serial::advanced_protocol::ReaderToHost::deserialize(&response_message_buffer[..response_message_size])?;
            log::trace!("Interpretting response for attempt {}: {:#?}", attempts, response);


            // Check for errors
            if response.status == 0x84 {
                /*
                 * A monitor is continusously checking the RF hardware and
                 * if an error occurs the Reader answers every command with
                 * the error code 0x84
                 */
                 let error_message = String::from("Generic Antenna Error: RF hardware monitor error status code 0x84");
                 log::error!("{}", error_message);
                 return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)));
            } else if serial_message.device_required && response.status == 0x01 {
                log::error!("No devices found on attempt {} of {}", attempts, self.state.max_attempts);
                if attempts > self.state.max_attempts {
                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to communicate with device in sensor")));
                } else {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
            }

            // All done
            return Ok(response);
        }
    }


}