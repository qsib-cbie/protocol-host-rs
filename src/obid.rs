
#[derive(Debug)]
#[repr(u8)]
pub enum Status {

    /// This enum represents an value that could not map to an Obid Status Code
    Invalid = 0xFF,

    // MARK: GENERAL


    /// Data / parameters have been read or stored without error
    /// Control command has been executed
    Ok = 0x00,

    /// The Reader is in full activity. The host should repeat the command later
    Busy = 0x0F,

    /// RFC not working properly
    /// Communication link between ACC and RFC not working properly
    HardwareWarning = 0xF1,

    /// ACC is initialized partly or completely with default values and Host Mode may be enabled
    InitializationWarning = 0xF2,



    // MARK: TRANSPONDER STATUS



    /// No Transponder is located within the detection range of the Reader.
    /// The Transponder in the detection range has been switched to mute.
    /// The communication between Reader and Transponder has been interfered and the Reader
    /// not able to read the Transponder anymore.
    NoTransponder = 0x01,

    /// CRC16 data error at received data.
    DataFalse = 0x02,

    /// Negative plausibility check of the written data:
    /// Attempt to write on a read-only storing-area.
    /// Too much distance between Transponder and Reader antenna.
    /// Attempt to write in a noise area.
    WriteError = 0x03,

    /// The required data are outside of the logical or physical Transponder-address area:
    /// The address is beyond the max. address space of the Transponder.
    /// The address is beyond the configured address space of the Transponder.
    AddressError = 0x04,

    /// This command is not applicable at the Transponder:
    /// Attempt to write on or read from a Transponder.
    /// A special command is not applicable to the Transponder.
    WrongTransponderType = 0x05,



    // MARK: PARAMETER STATUS



    /// The EEPROM of the Reader is not able to be written on.
    /// Before writing onto the EEPROM a faulty checksum of parameters has been detected. Parameter-Range-Error:
    /// The value range of the parameters was exceeded.
    EepromFailure = 0x10,
  
    /// The value range of the parameters was exceeded.
    ParameterRangeError = 0x11,

    /// Configuration access without having logged in to the Reader before.
    LoginRequest = 0x13,

    /// Login attempt with wrong password.
    LoginError = 0x14,

    /// The configuration block is reserved for future use.
    ReadProtect = 0x15,

    /// The configuration block is reserved for future use.
    WriteProtect = 0x16,

    /// The firmware must be activated first using ISOStart demo program and the command “Set Firmware Upgrade”. The update code must be ordered by Feig Electronic.
    /// 1. Read the Device-ID using the command [0x66] Firmware version (Mode 0x80)
    /// 2. Send the Device-ID and the serial number of the reader to Feig Electronic
    /// 3. Write the upgrade code into the reader using the command [0x5F] Set Firmware Update
    FirmwareActivationRequired = 0x17,

    /// Firmwareversion conflict between RFC and FPGA
    /// Conflict between the supported tagdrivers of RFC and FPGA
    /// Readertype is not supported by the FPGA
    /// Mismatch between RFC Firmware and Hardware
    WrongFirmware = 0x18,



    // MARK: INTERFACE STATUS

    

    /// The reader does not support the selected function
    UnknownCommand = 0x80,

    /// Protocol is too short or too long
    LengthError = 0x81,

    CommandNotAvailable = 0x82,

    /// This error indicates that there is an error in communication between the Transponder
    /// and the Reader. Reason for this can be:
    /// The collision handling algorithm was not continued until no collision is detected, reasons for the break:
    /// - TR-RESPOSE-TIME in CFG1: Interface is to short
    RFCommuncicationError = 0x83,

    /// Detailed status information can be read with the command 6.9. [0x6E] Reader Diagnostic.
    /// The antenna configuration isn‟t correct. Check the antenna cables and the antenna matching.
    /// The environment is too noisy.
    /// The RF power doesn‟t have the configured value.
    RFWarning = 0x84,
 
    /// There is no valid data in the Buffered Read Mode.
    /// There is no Transponder in the antenna field.
    /// The VALID-TIME1 hasn‟t elapsed for Transponders in the antenna field.
    NoValidData = 0x92,


    /// A data buffer overflow occurred.
    DataBufferOverflow = 0x93,

    /// There are more Transponder data sets requested than the response protocol can transfer at once.
    MoreData = 0x94,

    /// A Tag error code was sent from the transponder. The Tag error code is shown in the following byte.
    /// Tag Error for ISO15693 Transponder are listed below
    TagError = 0x95,
}

impl From<u8> for Status {
    fn from(code: u8) -> Self {
        match code {
            0x00 => { Status::Ok },
            0x0F => { Status::Busy },
            0xF1 => { Status::HardwareWarning },
            0xF2 => { Status::InitializationWarning },
            0x01 => { Status::NoTransponder },
            0x02 => { Status::DataFalse },
            0x03 => { Status::WriteError },
            0x04 => { Status::AddressError },
            0x05 => { Status::WrongTransponderType },
            0x10 => { Status::EepromFailure },
            0x11 => { Status::ParameterRangeError },
            0x13 => { Status::LoginRequest },
            0x14 => { Status::LoginError },
            0x15 => { Status::ReadProtect },
            0x16 => { Status::WriteProtect },
            0x17 => { Status::FirmwareActivationRequired },
            0x18 => { Status::WrongFirmware },
            0x80 => { Status::UnknownCommand },
            0x81 => { Status::LengthError },
            0x82 => { Status::CommandNotAvailable },
            0x83 => { Status::RFCommuncicationError },
            0x84 => { Status::RFWarning },
            0x92 => { Status::NoValidData },
            0x93 => { Status::DataBufferOverflow },
            0x94 => { Status::MoreData },
            0x95 => { Status::TagError },
            _ => { 
                log::error!("Invalid code for Obid Status codes");
                Status::Invalid 
            },
        }
    }
}


#[cfg(test)]
mod tests {
    use super::Status;

    #[test]
    fn status_code_valid_string() {
        let status: Status = Status::from(0x00);
        assert_eq!("Ok", format!("{:?}", status));
        assert_eq!("Ok", format!("{:?}", Status::Ok));
    }

    #[test]
    fn status_code_invalid_string() {
        let status: Status = Status::from(0xFF);
        assert_eq!("Invalid", format!("{:?}", status));
    }
}
