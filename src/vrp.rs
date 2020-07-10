

#![allow(dead_code)]

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

}

impl FabricConnection for UsbConnection {
    fn test_connection(self: & Self) -> Result<(), Box<dyn std::error::Error>> {
        self.get_device_handle()?;
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
            }
        })
    }

    pub fn get_device_handle<'a>(self: &'a Self) -> Result<libusb::DeviceHandle<'a>, Box<dyn std::error::Error>> {
        for device in self.ctx.devices()?.iter() {
            let device_desc = device.device_descriptor()?;

            log::trace!("Found USB Device || Bus {:03} Device {:03} ID {}:{}",
                device.bus_number(),
                device.address(),
                device_desc.vendor_id(),
                device_desc.product_id());

            if device_desc.vendor_id() == 2737 {
                log::debug!("Found USB Device || Bus {:03} Device {:03} ID {}:{}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id());
                
                return Ok(device.open()?);
            }
        }

        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "No matching USB device found")))
    }

    /// ???
    /// Get the UUID of the Feig array
    fn get_inventory(self: & Self) -> String {
        log::error!("Not yet implemented!");
        String::from("Not yet implemented!")
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


}