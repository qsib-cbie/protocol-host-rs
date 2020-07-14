#![allow(dead_code)]

use serial::{ObidSerialReceivable, ObidSerialSendable};

#[path = "serial.rs"] mod serial;


pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub uuid: uuid::Uuid,

    conn: Box<dyn FabricConnection + Send>,
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

/// Feig-based Connections are documented here
/// http://www.sebeto.com/intranet/ftpscambio/RFID_FEIG/Readers/ID%20ISC%20LR2500/Documentation/H01112-0e-ID-B.pdf
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

    /**
     * A function for getting a hanlde that has claimed an Obid/Feig reader and prints interface/endpoint info to trace
     *
     * 2020-07-13 12:15:50,835 DEBUG [vr_actuators_cli::network::vrp] Interface Descriptor of 0: InterfaceDescriptor {
     *    bLength: 9,
     *    bDescriptorType: 4,
     *    bInterfaceNumber: 0,
     *    bAlternateSetting: 0,
     *    bNumEndpoints: 2,
     *    bInterfaceClass: 255,
     *    bInterfaceSubClass: 255,
     *    bInterfaceProtocol: 0,
     *    iInterface: 6,
     *}
     *2020-07-13 12:15:50,835 DEBUG [vr_actuators_cli::network::vrp] Endpoint Descriptor of 0: EndpointDescriptor {
     *    bLength: 7,
     *    bDescriptorType: 5,
     *    bEndpointAddress: 129,
     *    bmAttributes: 2,
     *    wMaxPacketSize: 64,
     *    bInterval: 0,
     *}
     *2020-07-13 12:15:50,835 DEBUG [vr_actuators_cli::network::vrp] Endpoint Descriptor of 0: EndpointDescriptor {
     *    bLength: 7,
     *    bDescriptorType: 5,
     *    bEndpointAddress: 2,
     *    bmAttributes: 2,
     *    wMaxPacketSize: 64,
     *    bInterval: 0,
     *}
     */
    pub fn _get_device_handle<'a>(self: &'a Self) -> Result<libusb::DeviceHandle<'a>, Box<dyn std::error::Error>> {
        for device in self.ctx.devices()?.iter() {
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
                    for interface_descriptor in interface.descriptors() {
                        log::trace!("Interface Descriptor of {}: {:#?}", interface_number, interface_descriptor);
                        for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                            log::trace!("Endpoint Descriptor of {}: {:#?}", interface_number, endpoint_descriptor);
                        }
                    }
                }
                
                return Ok(handle);
            }
        }

        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No matching USB device found")))
    }

    /**
     * The Serial Data Format and Protocol Frames are well defined for the Feig reader.
     * 
     * If using a TCP/IP protocol, the data of the TCP/IP protocol frame is the Serial Data Format
     * and Protocol Frames data payload. There are additional parameters for TCP that are documented.
     *
     * For USB, we just need to worry about getting the correct framing in order to get a response.

     * Protocol frame: Standard Protocol-Length (up to 255 byte)
     * Protocol frame: Advanced Protocol-Length (up to 65535 byte)
     */
    fn frame_message() -> std::vec::Vec<u8> {


        vec![]
    }

    /// ???
    /// Get the UUID of the Feig array
    fn get_inventory_id(self: & Self) -> Result<Box<std::vec::Vec<u8>>, Box<dyn std::error::Error>> {
        log::trace!("Requesting inventory id");
        let device_handle = self.get_device_handle()?;
        // let mut inventory_id_request = vec![2,0,9,255,176,1,0];
        let inventory_request = serial::advanced_protocol::HostToReader::new(0, 0xFF, 0xB0, vec![0x01, 0x00].as_slice(), 0, false);
        let inventory_response = self.send_command(&device_handle, inventory_request)?;
        log::info!("Received inventory_response: {:#?}", inventory_response);

        Ok(Box::new(vec![]))
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

    fn send_command<'a>(self: & Self, device_handle: & libusb::DeviceHandle<'a>, serial_message: serial::advanced_protocol::HostToReader) -> Result<serial::advanced_protocol::ReaderToHost, Box<dyn std::error::Error>> {
        // Marshal the serial command
        let mut serial_message = serial_message;
        let msg = serial_message.serialize();

        let mut attempts = 0;
        let mut response_message_buffer = vec![0; 1024 * 1024 * 64]; // Max message size
        loop {
            // Documented not less than 5 milliseconds
            std::thread::sleep(std::time::Duration::from_millis(10));

            // Send the command to the Feig reader
            match device_handle.write_bulk(2, msg.as_slice(), std::time::Duration::from_millis(25)) {
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
            match device_handle.read_bulk(129, &mut response_message_buffer, std::time::Duration::from_millis(500)) {
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