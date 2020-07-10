#![allow(dead_code)]

use byteorder::{ByteOrder, LittleEndian};


pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub uuid: uuid::Uuid,

    conn: Box<dyn FabricConnection>,
}

impl std::fmt::Debug for Fabric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fabric(name: '{}')", self.name)
    }

}

impl Fabric {
    pub fn new(name: &str) -> Result<Fabric, Box<dyn std::error::Error>> {
        let fabric = Fabric {
            name: String::from(name),
            uuid: uuid::Uuid::new_v4(),
            conn: Box::new(UsbConnection::new()?),
        };

        fabric.conn.test_connection()?;
        Ok(fabric)
    }
}

pub trait FabricConnection { 
    fn test_connection(self: & Self) -> Result<(), Box<dyn std::error::Error>>;

    fn send(self: & Self, message: &str) -> Result<(), Box<dyn std::error::Error>>;
}

pub struct UsbConnection {
    ctx: libusb::Context,

    state: AntennaState
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

impl FabricConnection for UsbConnection {
    fn test_connection(self: & Self) -> Result<(), Box<dyn std::error::Error>> {
        let inventory_uid = self.get_inventory_id()?;
        log::info!("Connected to reader with id: {}", hex::encode(inventory_uid.as_ref()));
        Ok(())
    }

    fn send(self: & Self, message: &str) -> Result<(), Box<dyn std::error::Error>> {
        log::trace!("Sending Command: {}", message);
        Ok(())
    }
}

impl UsbConnection {
    pub fn new() -> Result<UsbConnection, Box<dyn std::error::Error>> {
        Ok(UsbConnection {
            ctx: libusb::Context::new()?,
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
        })
    }

    pub fn get_device_handle<'a>(self: &'a Self) -> Result<libusb::DeviceHandle<'a>, Box<dyn std::error::Error>> {
        match self._get_device_handle() {
            Ok(ok) => Ok(ok),
            Err(err) => {
                log::error!("Failed to prepare a device handle");
                Err(err)
            }
        }
    }

    pub fn _get_device_handle<'a>(self: &'a Self) -> Result<libusb::DeviceHandle<'a>, Box<dyn std::error::Error>> {
        for device in self.ctx.devices()?.iter() {
            let device_desc = device.device_descriptor()?;

            log::trace!("Found USB Device || Bus {:03} Device {:03} ID {} : {}",
                device.bus_number(),
                device.address(),
                device_desc.vendor_id(),
                device_desc.product_id());

            if device_desc.vendor_id() == 2737 {
                log::debug!("Found USB Device || Bus {:03} Device {:03} ID {} : {}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id());
              
                let mut handle = device.open()?;
                handle.reset()?;
                for interface in device.active_config_descriptor()?.interfaces() {
                    let interface_number = interface.number();
                    if handle.kernel_driver_active(interface_number)? {
                        log::debug!("Detaching kernel from interface: {}", interface_number);
                        handle.detach_kernel_driver(interface_number)?;
                    }
                    log::debug!("Claiming interface: {}", interface_number);
                    handle.claim_interface(interface_number)?;
                }
                
                return Ok(handle);
            }
        }

        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No matching USB device found")))
    }

    /// ???
    /// Get the UUID of the Feig array
    fn get_inventory_id(self: & Self) -> Result<Box<std::vec::Vec<u8>>, Box<dyn std::error::Error>> {
        let mut inventory_id_request = vec![2,0,9,255,176,1,0];
        log::trace!("Requesting inventory id with command: {}", hex::encode(&inventory_id_request));
        let device_handle = self.get_device_handle()?;
        self.send_command(&device_handle, &mut inventory_id_request)
    }

    /// Set the wattage for the RF power on the antenna
    fn set_radio_freq_power(self: & Self, _rf_power: i16) ->  Result<(), Box<dyn std::error::Error>> {
        log::error!("Not yet implemented!");
        Ok(())
    }

    /// Set operating mode (i.e. All off, turn on/off, single pulse, continous)
    fn get_op_mode(self: &mut Self) -> Result<String, Box<dyn std::error::Error>> {
        // TODO: Fix how the modes are put together
        if self.state.pulse_mode.is_some() && self.state.hf_mod.is_some() && self.state.lf_mod.is_some() {
            let op_mode = self.state.pulse_mode.unwrap() + self.state.hf_mod.unwrap() + self.state.lf_mod.unwrap();
            Ok(String::from(format!("{:X}", op_mode)))
        } else {
            log::error!("Cannot set op mode for state: {:#?}", self.state);
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid state to set op mode")))
        }
    }

    /// Set actuation mode (i.e.Unipolar, bipolar)
    fn get_act_mode(self: &mut Self, mode: bool) -> Result<String, Box<dyn std::error::Error>> {
        if mode {
            Ok(String::from(format!("{:X}", 1)))
        } else {
            Ok(String::from(format!("{:X}", 0)))
        }
    }

    /// Set the number of actuator blocks (32 actuators per block)
    fn get_act_block_count(self: &mut Self, num_blocks: i32) -> Result<String, Box<dyn std::error::Error>> {
        Ok(String::from(format!("{:X}", num_blocks)))
    }

    /// CRC16 encodes the message by extension
    fn encode_crc16(msg: &mut std::vec::Vec<u8>) {
        let crc_poly: u16 = 0x8408;
        let mut crc: u16 = 0xFFFF;
        for msg_byte in msg.iter() {
            crc = crc ^ (*msg_byte as u16);
            for _ in 0..8 {
                if crc & 1 == 1 {
                    crc = (crc >> 1) ^ crc_poly;
                } else {
                    crc = crc >> 1;
                }
            }
        }

        LittleEndian::write_u16(msg, crc);
    }

    fn send_command<'a>(self: & Self, device_handle: & libusb::DeviceHandle<'a>, msg: &mut std::vec::Vec<u8>) -> Result<Box<std::vec::Vec<u8>>, Box<dyn std::error::Error>> {
        // Serial expects CRC 16 encoded communications
        UsbConnection::encode_crc16(msg);

        let mut attempts = 0;
        loop {
            // Send the command to the Feig reader
            match device_handle.write_bulk(2, &msg, std::time::Duration::from_millis(1000)) {
                Ok(bytes_written) => {
                    log::debug!("Sent Serial Command with {} bytes: {}", bytes_written, hex::encode(&msg));
                },
                Err(err) => {
                    log::error!("Failed Serial Command Send: {}", err.to_string());
                    return Err(Box::new(err));
                }
            }

            // Read the response to the command
            let mut resp_msg: std::vec::Vec<_> = vec![];
            match device_handle.read_bulk(129, &mut resp_msg, std::time::Duration::from_millis(1000)) {
                Ok(bytes_read) => {
                    log::debug!("Received Response to Serial Command with {} bytes: {}", bytes_read, hex::encode(&resp_msg));
                },
                Err(err) => {
                    log::error!("Failed Serial Command Read: {}", err.to_string());
                    return Err(Box::new(err));
                }
            }
        
            // Check for errors
            if resp_msg.len() >= 6 && (resp_msg[2] == 8 || resp_msg[2] == 9) {
                if resp_msg[5] == 132 {
                    log::error!("Antenna Warning!");
                } else if resp_msg[5] == 1 {
                    attempts += 1;
                    log::error!("No devices found on attempt {} of {}", attempts, self.state.max_attempts);
                    if attempts > self.state.max_attempts {
                        return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Failed to communicate with device in sensor")));
                    } else {
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        continue;
                    }
                }
            }

            // All done
            return Ok(Box::new(resp_msg));
        }
    }


}