#![allow(dead_code)]

use serde::{Serialize, Deserialize};
use crate::obid::*;
use crate::conn::{common::Connection, mock::MockConnection};
use crate::error::*;

#[derive(Debug)]
pub struct ObidTransponder {
    pub uid: smallvec::SmallVec<[u8; 8]>, // 8-byte serial number
    pub tr_type_rf_tec: u8,
    pub tr_type_type_no: u8,
    pub dsfid: u8
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomCommand {
    pub control_byte: u8,
    pub data: String,
    pub device_required: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct OpModeBlock {
    pub act_cnt32: u8,
    pub act_mode: u8,
    pub op_mode: u8,
}

#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct ActuatorModeBlock {
    pub b0: u8,
    pub b1: u8,
    pub b2: u8,
    pub b3: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct ActuatorModeBlocks {
    pub block0_31: Option<ActuatorModeBlock>,
    pub block32_63: Option<ActuatorModeBlock>,
    pub block64_95: Option<ActuatorModeBlock>,
    pub block96_127: Option<ActuatorModeBlock>,
}

#[derive(PartialEq, Clone, Serialize, Deserialize, Debug)]
pub struct TimerModeBlock {
    pub b0: u8,
    pub b1: u8,
    pub b2: u8,
    pub b3: u8,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TimerModeBlocks {
    pub single_pulse_block: Option<TimerModeBlock>,
    pub hf_block: Option<TimerModeBlock>,
    pub lf_block: Option<TimerModeBlock>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActuatorsCommand {
    pub fabric_name: String,
    pub op_mode_block: Option<OpModeBlock>,
    pub actuator_mode_blocks: Option<ActuatorModeBlocks>,
    pub timer_mode_blocks: Option<TimerModeBlocks>,
    pub use_cache: Option<bool>,
}

pub struct FabricState {
    pub state: ActuatorsCommand,
}

impl FabricState {
    pub fn new(fabric_name: &str) -> Self {
        Self {
            state: ActuatorsCommand {
                fabric_name: String::from(fabric_name),
                op_mode_block: Some(OpModeBlock { act_cnt32: 0, act_mode: 0, op_mode: 0 }),
                actuator_mode_blocks: Some(ActuatorModeBlocks {
                    block0_31: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block32_63: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block64_95: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block96_127: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                }),
                timer_mode_blocks: Some(TimerModeBlocks {
                    single_pulse_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    hf_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    lf_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                }),
                use_cache: Some(false),
            }
        }
    }

    // Return the minimum acutator command to update the state to the new actuators command
    pub fn diff(self: &Self, new_state: ActuatorsCommand) -> ActuatorsCommand {
        let new_actuator_blocks = &new_state.actuator_mode_blocks.unwrap_or(self.state.actuator_mode_blocks.clone().unwrap());
        let curr_actuator_blocks = self.state.actuator_mode_blocks.as_ref().unwrap();

        let new_timer_blocks = &new_state.timer_mode_blocks.unwrap_or(self.state.timer_mode_blocks.clone().unwrap());
        let curr_timer_blocks = self.state.timer_mode_blocks.as_ref().unwrap();

        ActuatorsCommand {
            fabric_name: self.state.fabric_name.clone(),
            op_mode_block: new_state.op_mode_block,
            actuator_mode_blocks: Some(ActuatorModeBlocks {
                block0_31: if new_actuator_blocks.block0_31.is_some() && curr_actuator_blocks.block0_31 != new_actuator_blocks.block0_31 { new_actuator_blocks.block0_31.clone() } else { None },
                block32_63: if new_actuator_blocks.block32_63.is_some() && curr_actuator_blocks.block32_63 != new_actuator_blocks.block32_63 { new_actuator_blocks.block32_63.clone() } else { None },
                block64_95: if new_actuator_blocks.block64_95.is_some() && curr_actuator_blocks.block64_95 != new_actuator_blocks.block64_95 { new_actuator_blocks.block64_95.clone() } else { None },
                block96_127: if new_actuator_blocks.block96_127.is_some() && curr_actuator_blocks.block96_127 != new_actuator_blocks.block96_127 { new_actuator_blocks.block96_127.clone() } else { None },
            }),
            timer_mode_blocks: Some(TimerModeBlocks {
                single_pulse_block: if new_timer_blocks.single_pulse_block.is_some() && curr_timer_blocks.single_pulse_block != new_timer_blocks.single_pulse_block { new_timer_blocks.single_pulse_block.clone() } else { None },
                hf_block: if new_timer_blocks.hf_block.is_some() && curr_timer_blocks.hf_block != new_timer_blocks.hf_block { new_timer_blocks.hf_block.clone() } else { None },
                lf_block: if new_timer_blocks.lf_block.is_some() && curr_timer_blocks.lf_block != new_timer_blocks.lf_block { new_timer_blocks.lf_block.clone() } else { None },
            }),
            use_cache: self.state.use_cache,
        }
    }

    // Update the state of the fabric with the new state
    pub fn apply(self: &mut Self, new_state: ActuatorsCommand) {
        let diff = self.diff(new_state);

        let new_actuator_blocks = &diff.actuator_mode_blocks.unwrap_or(self.state.actuator_mode_blocks.clone().unwrap());
        let curr_actuator_blocks = self.state.actuator_mode_blocks.as_ref().unwrap();

        let new_timer_blocks = &diff.timer_mode_blocks.unwrap_or(self.state.timer_mode_blocks.clone().unwrap());
        let curr_timer_blocks = self.state.timer_mode_blocks.as_ref().unwrap();


        self.state = ActuatorsCommand {
            fabric_name: self.state.fabric_name.clone(),
            op_mode_block: diff.op_mode_block,
            actuator_mode_blocks: Some(ActuatorModeBlocks {
                block0_31: if new_actuator_blocks.block0_31.is_some() { new_actuator_blocks.block0_31.clone() } else { curr_actuator_blocks.block0_31.clone() },
                block32_63: if new_actuator_blocks.block32_63.is_some() { new_actuator_blocks.block32_63.clone() } else { curr_actuator_blocks.block32_63.clone() },
                block64_95: if new_actuator_blocks.block64_95.is_some() { new_actuator_blocks.block64_95.clone() } else { curr_actuator_blocks.block64_95.clone() },
                block96_127: if new_actuator_blocks.block96_127.is_some() { new_actuator_blocks.block96_127.clone() } else { curr_actuator_blocks.block96_127.clone() },
            }),
            timer_mode_blocks: Some(TimerModeBlocks {
                single_pulse_block: if new_timer_blocks.single_pulse_block.is_some() { new_timer_blocks.single_pulse_block.clone() } else { curr_timer_blocks.single_pulse_block.clone() },
                hf_block: if new_timer_blocks.hf_block.is_some() { new_timer_blocks.hf_block.clone() } else { curr_timer_blocks.hf_block.clone() },
                lf_block: if new_timer_blocks.lf_block.is_some() { new_timer_blocks.lf_block.clone() } else { curr_timer_blocks.lf_block.clone() },
            }),
            use_cache: Some(true), // Use the cache once the cache starts applying on top of its own state
        };
    }
}

pub struct Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub transponders: smallvec::SmallVec<[ObidTransponder; 2]>,
    pub state: FabricState,
}

impl std::fmt::Debug for Fabric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fabric(name: '{}')", self.name)
    }
}

impl Fabric {//switch passed arg to protocol?
    pub fn new(protocol: &mut HapticProtocol, name: &str) -> Result<Fabric> {
        let mut fabric = Fabric {
            name: String::from(name),
            transponders: smallvec::smallvec![],
            state: FabricState::new(name),
        };

        fabric.transponders = protocol.get_inventory(true)?;

        Ok(fabric)
    }
}

pub struct HapticProtocol<'a> {
    // conn: &'a mut dyn Connection<'a>,
    conn: Box<dyn Connection<'a>>
}
impl<'a> HapticProtocol<'a> {

    pub fn new(_conn: &'a impl Connection<'a>) -> HapticProtocol<'a> {
        HapticProtocol {
            conn: Box::new(MockConnection::new())
        }
    }

   /**
     * This command reads the UID of all Transponders inside the antenna field.
     * If the Reader has detected a new Transponder, that Transponder will be
     * automatically set in the quiet state by the Reader. In this state the
     * Transponder does not send back a response until the next inventory command.
     *
     * @return transponders in the array
     */
    pub fn get_inventory(self: &mut Self, expect_device: bool) -> Result<smallvec::SmallVec<[ObidTransponder; 2]>> {
        log::trace!("Requesting inventory ids ...");
        let inventory_request = advanced_protocol::HostToReader::new(0, 0xFF, 0xB0, vec![0x01, 0x00].as_slice(), 0, expect_device);
        let inventory_response = self.conn.send_command(inventory_request)?;
        log::debug!("Received inventory_response: {:#?}", inventory_response);

        if inventory_response.status == 0 && inventory_response.data.len() > 0 {
            let mut transponders = smallvec::smallvec![];
            let encoded_transponders = inventory_response.data[0];
            let bytes_per_transponder = 1 + 1 + 8; // tr_type, dsfid, uid
            if inventory_response.data.len() != 1 + (encoded_transponders as usize) * (bytes_per_transponder as usize) {
                return Err(InternalError::from("Unexpected data format in response to inventory request"));
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

            log::debug!("Found transponders: {:?}", transponders);

            Ok(transponders)
        } else {
            Ok(smallvec::smallvec![])
        }
    }

    /// Set the wattage for the RF power on the antenna
    pub fn set_radio_freq_power(self: &mut Self, rf_power: u8) ->  Result<()> {
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
            0x08,             // CFG-Data :: CFG3 Byte 1 TAG-DRV
            encoded_rf_power, // CFG-Data :: CFG3 Byte 2 RF-POWER
            0x80,             // CFG-Data :: CFG3 Byte 3 EAS-LEVEL
            0,0,0,            // CFG-Data :: CFG3 Byte 4,5,6 0x00
            0,0,0,0,0,0,      // CFG-Data :: CFG3 Byte 7,8,9,10,11,12 0x00
            0b1000_0001       // CFG-Data :: CFG3 Byte 13 FU_COM,
            ,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0 // IDK WHY THIS IS REQUIRED
        ];



        let request = advanced_protocol::HostToReader::new(0, 0xFF, 0x8B, data.as_slice(), 0, false);
        let response = self.conn.send_command(request)?;
        log::debug!("Received response: {:#?}", response);
        if response.status == 0x11 {
            let error_message = "A reasonableness check failed while writing the RF power parameter to the reader" ;
            Err(InternalError::from(error_message))
        } else {
            Ok(())
        }
    }

    pub fn system_reset(self: &mut Self)  -> Result<()> {
        log::trace!("Requesting System Reset of RF controller ...");
        let request = advanced_protocol::HostToReader::new(0, 0xFF, 0x64, vec![0].as_slice(), 0, false);
        let response = self.conn.send_command(request)?;

        let status = Status::from(response.status);
        if status != Status::Ok {
            let error_message = format!("System reset failed with status code: {:?}.", status);
            Err(InternalError::from(error_message))
        } else {
            Ok(())
        }
    }

    pub fn custom_command(self: &mut Self, control_byte: u8, data: &[u8], device_required: bool)  -> Result<()> {
        log::trace!("Requesting Custom Command with control_byte {:#X?} and data {:#X?} ...", control_byte, data);

        let request = advanced_protocol::HostToReader::new(0, 0xFF, control_byte, data, 0, device_required);
        let response = self.conn.send_command(request)?;

        let status = Status::from(response.status);
        if status != Status::Ok {
            let error_message = format!("Command failed with status code: {:?}.", status);
            Err(InternalError::from(error_message))
        } else {
            Ok(())
        }
    }

    pub fn actuators_command(self: &mut Self, uid: &[u8], timer_mode_blocks: &Option<TimerModeBlocks>, actuator_mode_blocks: &Option<ActuatorModeBlocks>, op_mode_block: &Option<OpModeBlock>)  -> Result<()> {
        log::trace!("Requesting write to actuators' configuration ...");

        if uid.len() != 8 {
            return Err(InternalError::from(format!("Expected UID, which is a serial number of 8 bytes, but found {} bytes", uid.len())));
        }

        // Construct the feig command
        let control_byte = 0xB0; // Control Byte for manipulating transponder
        let command_id = 0x24;   // Command Id for Control Byte to write blocks to transponder's RF blocks
        let mode = 0x01; // addressed
        let _bank = 0x00; // this option is not used ?!
        let db_n = 0x01;
        let db_size = 0x04;
        let mut wrote_block = false;

        // Set the timer blocks first if present
        if timer_mode_blocks.is_some() {
            let timer_mode_blocks = timer_mode_blocks.as_ref().unwrap();

            let addr = 0x09;
            let bl = &timer_mode_blocks.single_pulse_block;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting single_pulse_block of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write timer block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }

            let addr = 0x0A;
            let bl = &timer_mode_blocks.hf_block;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting hf_block of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write timer block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }

            let addr = 0x0B;
            let bl = &timer_mode_blocks.lf_block;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting lf_block of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write timer block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }
        }

        // Set the actuator blocks next if present
        if actuator_mode_blocks.is_some() {
            let actuator_mode_blocks = actuator_mode_blocks.as_ref().unwrap();

            let addr = 0x01;
            let bl = &actuator_mode_blocks.block0_31;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting block0_31 of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write actuators block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }

            let addr = 0x02;
            let bl = &actuator_mode_blocks.block32_63;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting block32_63 of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write actuators block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }

            let addr = 0x03;
            let bl = &actuator_mode_blocks.block64_95;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting block64_95 of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write actuators block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }

            let addr = 0x04;
            let bl = &actuator_mode_blocks.block96_127;
            if bl.is_some() {
                wrote_block = true;
                let bl = bl.as_ref().unwrap();
                let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, bl.b3, bl.b2, bl.b1, bl.b0];
                log::debug!("Setting block96_127 of {}: {:?}", hex::encode(uid), bl);

                match self.custom_command(control_byte, data.as_slice(), true) {
                    Ok(_) => { },
                    Err(err) => {
                        log::error!("Failed to write actuators block for actuators command: {}", err);
                        return Err(err);
                    }
                }
            }
        }

        // Set the mode block last
        if op_mode_block.is_some() && wrote_block {
            let addr = 0x00;
            let bl = op_mode_block.as_ref().unwrap();
            let data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size, 0x00, bl.act_cnt32, bl.act_mode, bl.op_mode];
            log::debug!("Setting op_mode_block of {}: {:?}", hex::encode(uid), bl);

            match self.custom_command(control_byte, data.as_slice(), true) {
                Ok(_) => { },
                Err(err) => {
                    log::error!("Failed to write op block for actuators command: {}", err);
                    return Err(err);
                }
            }
        }

        Ok(())
    }

}