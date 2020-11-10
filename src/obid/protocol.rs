#![allow(dead_code)]


pub trait ObidSerialSendable {
    fn serialize(self: &mut Self) -> std::vec::Vec<u8> where Self: Sized;

    fn deserialize(self: & Self, data: &[u8]) -> Result<Box<dyn ObidSerialReceivable>, Box<dyn std::error::Error>> where Self: Sized;

    fn _deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> where Self: Sized;
}

pub trait ObidSerialReceivable {
    fn _serialize(self: &mut Self) -> std::vec::Vec<u8> where Self: Sized;

    fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>> where Self: Sized;
}

fn calc_crc16(data: &[u8]) -> u16 {
    let crc_poly: u16 = 0x8408;
    let mut crc: u16 = 0xFFFF;

    for data_byte in data.iter() {
        crc = crc ^ (*data_byte as u16);
        for _ in 0..8 {
            if crc & 1 == 1 {
                crc = (crc >> 1) ^ crc_poly;
            } else {
                crc = crc >> 1;
            }
        }
    }

    crc
}

/// The reader will reply with standard protocol if sent standard protocol
/// except if the response from the reader would be too big for standard protocol
pub mod standard_protocol {
    pub struct HostToReader {
        length: u8,
        com_adr: u8,
        control_byte: u8,
        data: std::vec::Vec<u8>,
        crc16: u16,
    }

    pub struct ReaderToHost {
        length: u8,
        com_adr: u8,
        control_byte: u8,
        status: u8,
        data: std::vec::Vec<u8>,
        crc16: u16,
    }
}

/// The reader will reply with advanced protocol if sent advanced protocol
/// USE THIS ONE FOR EASE OF USE
pub mod advanced_protocol {
    use byteorder::{ByteOrder, BigEndian, LittleEndian};
    use super::{ObidSerialReceivable, ObidSerialSendable, calc_crc16};

    #[derive(Debug)]
    pub struct HostToReader {
        stx: std::marker::PhantomData<u8>, // Always 0x02
        pub alength: u16, // alength includes stx, alength, and crc16
        pub com_adr: u8, // [0,254] address of device in bus mode
        pub control_byte: u8, // defines the command which the reader should operate
        pub data: std::vec::Vec<u8>, // optional data, as MSB first
        pub crc16: u16, // CRC from bytes [1,n-2]

        pub device_required: bool, // Indicates the message isn't sent if no device observed it
    }

    impl HostToReader {
        pub fn new(alength: u16, com_adr: u8, control_byte: u8, data: &[u8], crc16: u16, device_required: bool) -> HostToReader {
            HostToReader {
                stx: std::marker::PhantomData,
                alength,
                com_adr,
                control_byte,
                data: data.to_vec(),
                crc16,
                device_required,
            }
        }
    }

    impl PartialEq for HostToReader {
        fn eq(&self, other: &Self) -> bool {
            self.alength == other.alength &&
                self.com_adr == other.com_adr &&
                self.control_byte == other.control_byte &&
                self.data == other.data &&
                self.crc16 == other.crc16
        }
    }

    impl ObidSerialSendable for HostToReader {
        fn serialize(self: &mut Self) -> std::vec::Vec<u8> {
            // Reserve a vector of exactly the correct size
            self.alength = 1 + 2 + 1 + 1 + (self.data.len() as u16) + 2;
            let mut msg = vec![0u8; self.alength as usize];

            // Encode the values up to crc16
            msg[0] = 0x02; // STX
            BigEndian::write_u16(&mut msg[1..=2], self.alength); // ALENGTH
            msg[3] = self.com_adr;
            msg[4] = self.control_byte;
            if self.data.len() > 0 {
                msg[5..(5 + self.data.len())].clone_from_slice(self.data.as_slice()); // DATA
            }

            // Compute and encode crc16
            let n_2 = (self.alength - 2) as usize;
            self.crc16 = calc_crc16(&msg[..n_2]);
            LittleEndian::write_u16(&mut msg[n_2..], self.crc16);

            log::trace!("Serialized Message: {:#X?}", msg);
            msg
        }

        fn deserialize(self: &Self, data: &[u8]) -> Result<Box<dyn ObidSerialReceivable>, Box<dyn std::error::Error>> where Self: Sized{
            Ok(Box::new(ReaderToHost::deserialize(data)?))
        }

        fn _deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
        where Self: Sized {
            if data.len() < 7 {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid (short) length for HostToReader message")));
            }

            let n_2 = data.len() - 2;
            let crc16 = LittleEndian::read_u16(&data[n_2..]);
            if calc_crc16(&data[..n_2]) != crc16 {
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Invalid CRC16 for HostToReader message")));
            }

            let alength = BigEndian::read_u16(&data[1..=2]);
            let com_adr = data[3];
            let control_byte = data[4];
            let data = &data[5..n_2];


            Ok(Self::new(
                alength,
                com_adr,
                control_byte,
                data,
                crc16,
                false,
            ))
        }
    }

    #[derive(Debug)]
    pub struct ReaderToHost {
        stx: std::marker::PhantomData<u8>,
        pub alength: u16,
        pub com_adr: u8,
        pub control_byte: u8,
        pub status: u8,
        pub data: std::vec::Vec<u8>,
        pub crc16: u16,
    }

    impl ReaderToHost {
        pub fn new(alength: u16, com_adr: u8, control_byte: u8, status: u8, data: &[u8], crc16: u16) -> ReaderToHost {
            ReaderToHost {
                stx: std::marker::PhantomData,
                alength,
                com_adr,
                control_byte,
                status,
                data: data.to_vec(),
                crc16,
            }
        }
    }

    impl PartialEq for ReaderToHost {
        fn eq(&self, other: &Self) -> bool {
            self.alength == other.alength &&
                self.com_adr == other.com_adr &&
                self.control_byte == other.control_byte &&
                self.status == other.status &&
                self.data == other.data &&
                self.crc16 == other.crc16
        }
    }

    impl ObidSerialReceivable for ReaderToHost {
        fn _serialize(self: &mut Self) -> std::vec::Vec<u8> {
            // Reserve a vector of exactly the correct size
            self.alength = 1 + 2 + 1 + 1 + 1 + (self.data.len() as u16) + 2;
            let mut msg = vec![0u8; self.alength as usize];

            // Encode the values up to crc16
            msg[0] = 0x02; // STX
            BigEndian::write_u16(&mut msg[1..=2], self.alength); // ALENGTH
            msg[3] = self.com_adr;
            msg[4] = self.control_byte;
            msg[5] = self.status;
            if self.data.len() > 0 {
                msg[6..(6 + self.data.len())].clone_from_slice(self.data.as_slice()); // DATA
            }

            // Compute and encode crc16
            let n_2 = (self.alength - 2) as usize;
            self.crc16 = calc_crc16(&msg[..n_2]);
            LittleEndian::write_u16(&mut msg[n_2..], self.crc16);

            msg
        }

        fn deserialize(data: &[u8]) -> Result<Self, Box<dyn std::error::Error>>
        where Self: Sized {
            if data.len() < 8 {
                let error_message = String::from("Invalid (short) length for ReaderToHost message");
                log::error!("Failed deserializing Obid message: {}", error_message);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)));
            }

            let n_2 = data.len() - 2;
            let crc16 = LittleEndian::read_u16(&data[n_2..]);
            if calc_crc16(&data[..n_2]) != crc16 {
                let error_message = String::from("Invalid CRC16 for ReaderToHost message");
                log::error!("Failed deserializing Obid message: {}", error_message);
                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)));
            }

            let alength = BigEndian::read_u16(&data[1..=2]);
            let com_adr = data[3];
            let control_byte = data[4];
            let status = data[5];
            let data = &data[6..n_2];


            Ok(Self::new(
                alength,
                com_adr,
                control_byte,
                status,
                data,
                crc16,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::advanced_protocol;
    use super::{ObidSerialSendable,ObidSerialReceivable, calc_crc16};

    #[test]
    fn crc16_works() {
        let mut inventory_request = advanced_protocol::HostToReader::new(0, 0xFF, 0xB0, vec![0x01, 0x00].as_slice(), 0, false);
        let inventory_request = inventory_request.serialize();
        assert_eq!(0x4318, calc_crc16(&inventory_request[..inventory_request.len() - 2]));
    }

    #[test]
    fn host_to_reader_no_data() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![];
        let mut expected_msg = advanced_protocol::HostToReader::new(0, 0, 0,  &data[..], 0, false);
        let marshalled_data = expected_msg.serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::HostToReader::_deserialize(marshalled_data.as_slice()).unwrap();
        assert!(expected_msg == msg);
    }

    #[test]
    fn host_to_reader_some_data() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![1; 10];
        let mut expected_msg = advanced_protocol::HostToReader::new(0, 0, 0,  &data[..], 0, false);
        let marshalled_data = expected_msg.serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::HostToReader::_deserialize(marshalled_data.as_slice()).unwrap();
        assert!(expected_msg == msg);
    }

    #[test]
    fn reader_to_host_no_data() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![];
        let mut expected_msg = advanced_protocol::ReaderToHost::new(8, 0, 0, 0, &data[..], 0);
        let marshalled_data = expected_msg._serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::ReaderToHost::deserialize(marshalled_data.as_slice()).unwrap();
        assert!(expected_msg == msg);
    }

    #[test]
    fn reader_to_host_some_data() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![1; 100];
        let mut expected_msg = advanced_protocol::ReaderToHost::new(8, 0, 0, 0, &data[..], 0);
        let marshalled_data = expected_msg._serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::ReaderToHost::deserialize(marshalled_data.as_slice()).unwrap();
        assert!(expected_msg == msg);
    }

    #[test]
    fn reader_to_host_alotta_data() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![1; 10000];
        let mut expected_msg = advanced_protocol::ReaderToHost::new(8, 0, 0, 0, &data[..], 0);
        let marshalled_data = expected_msg._serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::ReaderToHost::deserialize(marshalled_data.as_slice()).unwrap();
        assert!(expected_msg == msg)
    }

    #[test]
    fn reader_to_host_bad_crc() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![1; 100];
        let mut expected_msg = advanced_protocol::ReaderToHost::new(8, 0, 0, 0, &data[..], 0);
        let mut marshalled_data = expected_msg._serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        *marshalled_data.last_mut().unwrap() = *marshalled_data.last().unwrap() + 1;
        let msg = advanced_protocol::ReaderToHost::deserialize(marshalled_data.as_slice());
        assert!(msg.is_err());
    }

    #[test]
    fn reader_to_host_too_short() {
        let _ = simple_logger::init_with_level(log::Level::Debug);

        let data = vec![1; 100];
        let mut expected_msg = advanced_protocol::ReaderToHost::new(8, 0, 0, 0, &data[..], 0);
        let marshalled_data = expected_msg._serialize();
        log::trace!("Serial Message: {:#X?}", marshalled_data.as_slice());

        let msg = advanced_protocol::ReaderToHost::deserialize(&marshalled_data[0..7]);
        assert!(msg.is_err());
    }
}
