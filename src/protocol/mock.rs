use super::common::*;
use crate::error::*;
use crate::conn::common::Connection;


pub struct MockProtocol {}

impl MockProtocol {
    pub fn new(_connection: Box<dyn Connection<'_> + '_>) -> MockProtocol { MockProtocol {}
    }
}

impl Protocol<'_> for MockProtocol {
    fn handle_message(self: &mut Self, _message: &CommandMessage) -> Result<()> {
        match _message {
            CommandMessage::ActuatorsCommand { fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block, use_cache } => {
                let _cache = use_cache;
                log::trace!("Received ActuatorsCommand: {:#?} {:#?} {:#?} {:#?}", fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block);
                let command_id = 0x24;   // Command Id for Control Byte to write blocks to transponder's RF blocks
                let mode = 0x01; // addressed
                let db_n = 0x01;
                let db_size = 0x04;
                let addr = 0x00;
                
                let mut data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![command_id, mode, 0, 1, 2, 3, 4, 5, 6, 7, addr, db_n, db_size];
                let mut num_bytes = 3;
                let mut cmd_op;
                let mut act_cnt8 = 5;
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
                            let mut last_int = 0;
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
                                } else {
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
                log::debug!("Send command: {:#?}", hex::encode(data));
                Ok(())
            },
            _ => {
                log::debug!("Mock ignoring: {:?}", _message);
                Ok(())
            }
        }

    }
}