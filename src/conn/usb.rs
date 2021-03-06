use crate::conn::common::*;
use crate::error::*;
use crate::obid::*;

/// Feig-based Connections are documented here
/// http://www.sebeto.com/intranet/ftpscambio/RFID_FEIG/Readers/ID%20ISC%20LR2500/Documentation/H01112-0e-ID-B.pdf

pub struct UsbConnection<'a> {
    state: AntennaState,
    device_handle: libusb::DeviceHandle<'a>,
    response_message_buffer: std::vec::Vec<u8>,
}

impl<'a> Connection<'a> for UsbConnection<'a> {
    fn send_command(
        self: &mut Self,
        serial_message: advanced_protocol::HostToReader,
    ) -> Result<advanced_protocol::ReaderToHost> {
        // Marshal the serial command
        let mut serial_message = serial_message;
        let msg = serial_message.serialize();
        let mut attempts = 0;
        loop {
            // Documented not less than 5 milliseconds between messages
            std::thread::sleep(std::time::Duration::from_millis(6));

            // Send the command to the Feig reader
            match self.device_handle.write_bulk(
                2,
                msg.as_slice(),
                std::time::Duration::from_millis(50),
            ) {
                Ok(bytes_written) => {
                    log::debug!(
                        "Sent Serial Command with {} bytes: {}",
                        bytes_written,
                        hex::encode(&msg)
                    );
                }
                Err(err) => {
                    log::error!("Failed Serial Command Send: {}", err.to_string());
                    return Err(InternalError::from(err));
                }
            }

            // Read the response to the command
            attempts += 1;
            let response_message_size;
            match self.device_handle.read_bulk(
                129,
                self.response_message_buffer.as_mut_slice(),
                std::time::Duration::from_millis(5000),
            ) {
                Ok(bytes_read) => {
                    log::debug!(
                        "Received Response to Serial Command with {} bytes: {}",
                        bytes_read,
                        hex::encode(&self.response_message_buffer[..bytes_read])
                    );
                    response_message_size = bytes_read;
                }
                Err(err) => {
                    log::error!("Failed Serial Command Read: {}", err.to_string());
                    continue;
                }
            }

            // Interpret the response
            let response = advanced_protocol::ReaderToHost::deserialize(
                &self.response_message_buffer[..response_message_size],
            )?;
            log::trace!(
                "Interpretting response for attempt {}: {:#?}",
                attempts,
                response
            );

            // Check for errors
            let status = Status::from(response.status);
            if status == Status::RFWarning {
                /*
                 * A monitor is continusously checking the RF hardware and
                 * if an error occurs the Reader answers every command with
                 * the error code 0x84
                 */
                let error_message = String::from(
                    "Generic Antenna Error: RF hardware monitor error status code 0x84",
                );
                log::error!("{}", error_message);
                return Err(InternalError::from(error_message));
            } else if serial_message.device_required && status == Status::NoTransponder {
                log::error!(
                    "No devices found on attempt {} of {}",
                    attempts,
                    self.state.max_attempts
                );
                if attempts >= self.state.max_attempts {
                    return Err(InternalError::from(
                        "Failed to communicate with device in antenna",
                    ));
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

impl<'a> UsbConnection<'a> {
    pub fn new(ctx: &'a UsbContext<'a>) -> Result<UsbConnection<'a>> {
        for _ in 0..10 {
            for device in ctx.ctx.devices()?.iter() {
                let device_desc = device.device_descriptor()?;
                log::trace!(
                    "Found USB Device || Bus {:03} Device {:03} ID {} : {}",
                    device.bus_number(),
                    device.address(),
                    device_desc.vendor_id(),
                    device_desc.product_id()
                );

                if device_desc.vendor_id() == 2737 {
                    log::debug!(
                        "Found Obid/Feig USB Device || Bus {:03} Device {:03} ID {} : {}",
                        device.bus_number(),
                        device.address(),
                        device_desc.vendor_id(),
                        device_desc.product_id()
                    );

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
                            log::trace!(
                                "Interface Descriptor of {}: {:#?}",
                                interface_number,
                                interface_descriptor
                            );
                            for endpoint_descriptor in interface_descriptor.endpoint_descriptors() {
                                log::trace!(
                                    "Endpoint Descriptor of {}: {:#?}",
                                    interface_number,
                                    endpoint_descriptor
                                );
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

                            command: None,
                            cmd_op: None,
                            act_block_count: None,

                            max_attempts: 5,
                        },
                        response_message_buffer: vec![0; 1024 * 1024 * 64],
                    });
                }
            }

            log::error!("No matching USB device found ...");
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        return Err(InternalError::from("No matching USB device found"));
    }
}

pub struct UsbContext<'a> {
    pub ctx: &'a libusb::Context,
}

impl<'a> UsbContext<'a> {
    pub fn new(ctx: &'a libusb::Context) -> Result<UsbContext<'a>> {
        Ok(UsbContext { ctx })
    }
}

impl<'a> Context<'a> for UsbContext<'a> {
    fn connection(self: &'a Self) -> Result<Box<dyn Connection<'a> + 'a>> {
        Ok(Box::new(UsbConnection::new(self)?))
    }
}
