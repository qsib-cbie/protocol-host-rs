use crate::conn::common::{Connection};
use crate::error::*;
use crate::obid::*;
use crate::protocol::common::*;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

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
    pub act_cnt8: u8,
    pub cmd_op: u8,
    pub command: u8,
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

pub struct V0FabricState {
    pub state: ActuatorsCommand,
}

impl V0FabricState {
    pub fn new(fabric_name: &str) -> Self {
        Self {
            state: ActuatorsCommand {
                fabric_name: String::from(fabric_name),
                op_mode_block: Some(OpModeBlock { act_cnt8: 0, cmd_op: 0, command: 0 }),
                actuator_mode_blocks: Some(ActuatorModeBlocks {
                    block0_31: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block32_63: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block64_95: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                    block96_127: Some(ActuatorModeBlock { b0: 0, b1: 0, b2: 0, b3: 0}),
                }),
                timer_mode_blocks: Some(TimerModeBlocks {
                    single_pulse_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0}),
                    hf_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0}),
                    lf_block: Some(TimerModeBlock { b0: 0, b1: 0, b2: 0}),
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

pub struct V0Fabric {
    // A set of VR Actuator Blocks that are to be considered 1 unit
    pub name: String,
    pub transponders: smallvec::SmallVec<[ObidTransponder; 2]>,
}

impl std::fmt::Debug for V0Fabric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Fabric(name: '{}')", self.name)
    }
}

impl V0Fabric {//switch passed arg to protocol?
    pub fn new(name: &str, transponders: smallvec::SmallVec<[ObidTransponder; 2]>) -> V0Fabric {
        V0Fabric {
            name: String::from(name),
            transponders: transponders,
        }
    }
}

impl Fabric for V0Fabric {
    fn name(self: &Self) -> String {
        self.name.clone()
    }

    fn identifier(self: &Self) -> Result<std::vec::Vec<u8>> {
        match self.transponders.len() {
            1 => {
                return Ok(self.transponders[0].uid.as_slice().into());
            },
            2 => {
                return Ok(self.transponders[0].uid.as_slice().into());
            },
            _ => return Err(InternalError::from(format!("Cannot produce identifier with {:?} transponders: {:?}",self.transponders.len(), self.transponders)))
        }
    }
}


pub struct HapticV0Protocol<'a> {
    conn: Box<dyn Connection<'a> + 'a >,
    fabrics: HashMap<String, Box<dyn Fabric>>,
    states: HashMap<String, V0FabricState>,
}

impl<'a> HapticV0Protocol<'a> {
    pub fn new(connection: Box<dyn Connection<'a> + 'a>) -> HapticV0Protocol<'a> {
        HapticV0Protocol {
            conn: connection,
            fabrics: HashMap::new(),
            states: HashMap::new(),
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
        // log::debug!("Sleep for 30ms");
        // std::thread::sleep(std::time::Duration::from_millis(30));

        let status = Status::from(response.status);
        if status != Status::Ok {
            let error_message = format!("Command failed with status code: {:?}.", status);
            Err(InternalError::from(error_message))
        } else {
            Ok(())
        }
    }

    fn handle_actuators_command(self: &mut Self, fabric_name: &String, timer_mode_blocks: &Option<TimerModeBlocks>, actuator_mode_blocks: &Option<ActuatorModeBlocks>, op_mode_block: &Option<OpModeBlock>, use_cache: &Option<bool>) -> Result<()> {
        let fabric = match self.fabrics.get_mut(fabric_name) {
            Some(fabric) => {
                log::trace!("Found transponder for actuator command: {:?}", fabric.identifier());
                fabric
            },
            None => {
                let message = format!("No existing fabric to write actuator command: {:?}", self.fabrics);
                log::error!("{}", message);
                return Err(InternalError::from(message.as_str()));
            }
        };
        let state = self.states.get(fabric_name).ok_or(InternalError::from(format!("Missing fabric state for {}", fabric_name)))?;


        let fabric_id = fabric.identifier()?;
        let mut actuators_command = ActuatorsCommand {
            fabric_name: fabric_name.clone(),
            timer_mode_blocks: timer_mode_blocks.clone(),
            actuator_mode_blocks: actuator_mode_blocks.clone(),
            op_mode_block: op_mode_block.clone(),
            use_cache: use_cache.clone(),
        };

        let actuators_command = match use_cache {
            Some(flag)  => {
                if *flag {
                if state.state.use_cache.unwrap() {
                    actuators_command = state.diff(actuators_command);
                        log::trace!("Writing using cached diff: {:#?}", &actuators_command);
                    } else {
                        log::debug!("Skipping cached diff to warm cache");
                    }
                } else {
                    log::debug!("Command electing to bypass cache");
                }
                actuators_command
            },
            _ => {
                if state.state.use_cache.unwrap() {
                    actuators_command = state.diff(actuators_command);
                    log::trace!("Writing using cached diff: {:#?}", &actuators_command);
                } else {
                    log::debug!("Skipping cached diff to warm cache");
                }
                actuators_command
            }
        };

        let result = self.actuators_command(fabric_id.as_slice(), &actuators_command.timer_mode_blocks, &actuators_command.actuator_mode_blocks, &actuators_command.op_mode_block);
        if result.is_ok() {
            let state = self.states.get_mut(fabric_name).ok_or(InternalError::from("Missing fabric state"))?;
            state.apply(actuators_command);
        }
        result
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
        let db_n = 0x01;
        let db_size = 0x04;
        let addr = 0x00;
        
        let mut data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, uid[0], uid[1], uid[2], uid[3], uid[4], uid[5], uid[6], uid[7], addr, db_n, db_size];
        let mut num_bytes = 3;
        let mut cmd_op;
        let mut act_cnt8 = 0;
        let mut is_actuators = false;
        let mut mem_blk1 = vec![0,0];
        let mut mem_blk2:Vec<u8> = vec![];
        let mut mem_blk3:Vec<u8> = vec![];

        let command;
        if op_mode_block.is_some(){
            let bl = op_mode_block.as_ref().unwrap();
            log::debug!("Num act blks: {:#?}, cmd_op: {:#?}, command: {:#?} ",bl.act_cnt8,bl.cmd_op,bl.command);
            command = bl.command;
            cmd_op = bl.cmd_op;
            if command != 0 {
                //Not all off command
                if actuator_mode_blocks.is_some() && cmd_op != 0 {
                    let bl = actuator_mode_blocks.as_ref().unwrap();
                    let mut blks = vec![];
                    if bl.block0_31.is_some(){ 
                        mem_blk1.append(&mut vec![0,bl.block0_31.as_ref().unwrap().b0]); 
                        blks.extend([bl.block0_31.as_ref().unwrap().b1, bl.block0_31.as_ref().unwrap().b2, bl.block0_31.as_ref().unwrap().b3].iter().copied());
                    }
                    if bl.block32_63.is_some(){ blks.extend([bl.block32_63.as_ref().unwrap().b0, bl.block32_63.as_ref().unwrap().b1, bl.block32_63.as_ref().unwrap().b2, bl.block32_63.as_ref().unwrap().b3].iter().copied());}
                    if bl.block64_95.is_some(){ blks.extend([bl.block64_95.as_ref().unwrap().b0, bl.block64_95.as_ref().unwrap().b1, bl.block64_95.as_ref().unwrap().b2, bl.block64_95.as_ref().unwrap().b3].iter().copied());}
                    if bl.block96_127.is_some(){ blks.extend([bl.block96_127.as_ref().unwrap().b0, bl.block96_127.as_ref().unwrap().b1, bl.block96_127.as_ref().unwrap().b2, bl.block96_127.as_ref().unwrap().b3].iter().copied());}
                    if blks.len() != 0 {
                        is_actuators = true;
                        let mut last_int = 0;
                        let mut cnt = 0;
                        for blk in blks.iter_mut() { //find last relavant byte
                            cnt += 1;
                            if *blk != 0 { last_int = cnt; }
                        }
                        blks.truncate(last_int); //remove unneeded bytes (trailing zeros)
                        for chunk in blks.chunks(db_size as usize) { //populate other memory blocks if possible
                            if mem_blk2.len() == 0 {mem_blk2.extend(chunk)}
                            else if mem_blk3.len() == 0 {mem_blk3.extend(chunk)}
                        }
                        act_cnt8 = 1 + blks.len() as u8;
                        cmd_op = 2; //Command without timing config. Overwritten if timing is added.
                    } else {
                        act_cnt8 = 0;
                    }
                }
                if timer_mode_blocks.is_some() {
                    let bl = timer_mode_blocks.as_ref().unwrap();
                    let mut blks = vec![];
                    if bl.single_pulse_block.is_some() {blks.extend([bl.single_pulse_block.as_ref().unwrap().b0,bl.single_pulse_block.as_ref().unwrap().b1,bl.single_pulse_block.as_ref().unwrap().b2].iter().copied());}
                    if bl.hf_block.is_some() {blks.extend([bl.hf_block.as_ref().unwrap().b0,bl.hf_block.as_ref().unwrap().b1,bl.hf_block.as_ref().unwrap().b2].iter().copied());}
                    if bl.lf_block.is_some() {blks.extend([bl.lf_block.as_ref().unwrap().b0,bl.lf_block.as_ref().unwrap().b1,bl.lf_block.as_ref().unwrap().b2].iter().copied());}
                    if blks.len() != 0 {
                        if is_actuators { 
                            cmd_op = 3; //Actuator command with timing config
                            //Fill memory blocks with space
                            while mem_blk2.len() < 4 { 
                                mem_blk2.push(blks.remove(0));
                            }
                            while mem_blk3.len() < 4 {
                                mem_blk3.push(blks.remove(0));
                            }
                        } else { //Only setting timing blocks
                            for _ in 0..2 { mem_blk1.push(blks.remove(0)); } //push first two bytes into mem_blk1
                            for chunk in blks.chunks(db_size as usize) { //populate other memory blocks if possible
                                if mem_blk2.len() == 0 {
                                    mem_blk2.extend(chunk)
                                } else if mem_blk3.len() == 0 {
                                    mem_blk3.extend(chunk)
                                }
                            }
                        }
                    }
                }                        
                
                let op_mode = cmd_op << 5 | act_cnt8;
                mem_blk1[1] = op_mode;

                if is_actuators {mem_blk1[2] = command;} //command not used when only setting timing

                num_bytes = mem_blk1.len()+mem_blk2.len()+mem_blk3.len();
                mem_blk1[0] = num_bytes as u8;
                data[11] = ((num_bytes as f32/4f32).ceil()) as u8; //calcuate number of memeory blocks to write to
                
                mem_blk1.reverse();
                mem_blk2.reverse();
                mem_blk3.reverse();
                data.extend_from_slice(mem_blk1.as_slice());
                data.extend_from_slice(mem_blk2.as_slice());
                data.extend_from_slice(mem_blk3.as_slice());
            } else {
                //all off
                act_cnt8 = bl.act_cnt8;
                let op_mode = cmd_op << 5 | act_cnt8;
                // LSB first
                data.push(0x00); //first byte is empty when cmd_op is 1
                data.push(command);
                data.push(op_mode);
                data.push(num_bytes as u8); //num_bytes
            }    
        }   
        match self.custom_command(control_byte, data.as_slice(), true) {
            Ok(_) => { },
            Err(err) => {
                log::error!("Failed to write actuators command: {}", err);
                return Err(err);
            }
        }
        Ok(())        
    }
}


impl<'a> Protocol<'a> for HapticV0Protocol<'a> {
    fn handle_message(self: &mut Self, message: &CommandMessage) -> Result<()> {
        match message {
            CommandMessage::AddFabric { fabric_name } => {
                let uid = match self.get_inventory(true) {
                    Ok(uid) => uid,
                    Err(err) => return Err(err)
                };
                let fabric: Box<dyn Fabric> = Box::new(V0Fabric::new(fabric_name.as_str(), uid));
                self.fabrics.insert(fabric_name.clone(), fabric);
                self.states.insert(fabric_name.clone(), V0FabricState::new(fabric_name.as_str()));
                log::info!("Added new fabric to command for AddFabric command");
                log::trace!("Active Fabrics: {:#?}", self.fabrics);
                Ok(())
            },
            CommandMessage::RemoveFabric { fabric_name } => {
                match self.fabrics.remove(fabric_name) {
                    Some(fabric) => {
                        log::info!("Removed existing fabric to command for AddFabric command");
                        log::trace!("Active Fabrics:  {:#?}", self.fabrics);
                        log::trace!("Removed Fabric:  {:#?}", fabric);
                        Ok(())
                    },
                    None => {
                        let message = format!("No existing fabric to remove for RemoveFabric command");
                        log::error!("{}", message.as_str());
                        Err(InternalError::from(message.as_str()))
                    }
                }
            },
            CommandMessage::SetRadioFreqPower { power_level } => {
                log::debug!("Received SetRadioFreqPower command for power_level {:?}.", power_level);
                match power_level {
                    pl if *pl == 0 || (*pl >= 2 && *pl <= 12) => {
                        self.set_radio_freq_power(*pl)
                    },
                    _ => {
                        let message = format!("Value for power level ({}) is outside acceptable range Low Power (0) or [2,12].", power_level);
                        log::error!("{}", message.as_str());
                        Err(InternalError::from(message.as_str()))
                    }
                }
            },
            CommandMessage::CustomCommand { control_byte, data, device_required } => {
                log::trace!("Received Custom command with control_byte {} and data {}", hex::encode(vec![control_byte.clone()]), hex::encode(data.as_bytes()));
                let decoded_data = hex::decode(&data)?;
                self.custom_command(control_byte.clone(), decoded_data.as_slice(), device_required.clone())
            },
            CommandMessage::ActuatorsCommand { fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block, use_cache } => {
                log::trace!("Received ActuatorsCommand: {:#?} {:#?} {:#?} {:#?}", fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block);
                self.handle_actuators_command(fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block, use_cache)
            },
            _ => {
                log::debug!("Haptic V0 ignoring: {:?}", message);
                Ok(())
            }
        }
    }
}