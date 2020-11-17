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
                let mut data: smallvec::SmallVec<[u8; 32]> = smallvec::smallvec![0x24,0x01,0,1,2,3,4,5,6,7];
                let mut num_bytes = 3;
                let mut cmd_op = 1;
                let mut is_actuators = false;
                if op_mode_block.is_some(){
                    let bl = op_mode_block.as_ref().unwrap();
                    log::debug!("Num act blks: {:#?}, cmd_op: {:#?}, command: {:#?} ",bl.act_cnt8,bl.cmd_op,bl.command);
                    if bl.command != 0 {
                        //Not all off
                        if actuator_mode_blocks.is_some() {
                            is_actuators = true;
                            let mut last_int = 0;
                            let bl = actuator_mode_blocks.as_ref().unwrap();
                            let blks = vec![bl.block0_31.as_ref().unwrap().b0, bl.block0_31.as_ref().unwrap().b1, bl.block0_31.as_ref().unwrap().b2, bl.block0_31.as_ref().unwrap().b3,
                                                    bl.block32_63.as_ref().unwrap().b0,bl.block32_63.as_ref().unwrap().b1,bl.block32_63.as_ref().unwrap().b2,bl.block32_63.as_ref().unwrap().b3,
                                                    bl.block64_95.as_ref().unwrap().b0,bl.block64_95.as_ref().unwrap().b1,bl.block64_95.as_ref().unwrap().b2,bl.block64_95.as_ref().unwrap().b3,
                                                    bl.block96_127.as_ref().unwrap().b0,bl.block96_127.as_ref().unwrap().b1,bl.block96_127.as_ref().unwrap().b2,bl.block96_127.as_ref().unwrap().b3];
                            for blk in blks.into_iter() {
                                data.push(blk);
                                if blk != 0 { last_int = data.len(); }
                            }
                            data.truncate(last_int);
                            cmd_op = 2; //Command without timing config. Overwritten if timing is added.
                        }
                        if timer_mode_blocks.is_some() {
                            if is_actuators { 
                                cmd_op = 3; //Actuator command with timing config
                            } else {
                                cmd_op = 0; //Only update timing
                            }
                            let bl = timer_mode_blocks.as_ref().unwrap();
                            let blks = vec![bl.single_pulse_block.as_ref().unwrap().b0,bl.single_pulse_block.as_ref().unwrap().b1,bl.single_pulse_block.as_ref().unwrap().b2,
                                                    bl.hf_block.as_ref().unwrap().b0,bl.hf_block.as_ref().unwrap().b1,bl.hf_block.as_ref().unwrap().b2,
                                                    bl.lf_block.as_ref().unwrap().b0,bl.lf_block.as_ref().unwrap().b1,bl.lf_block.as_ref().unwrap().b2];
                            for blk in blks.into_iter() {
                                data.push(blk);
                            }
                        }
                        let op_mode = cmd_op << 5 | bl.act_cnt8;
                        data.insert(10, op_mode);
                        num_bytes = (data.len() as u8) - 9;
                        data.insert(10, num_bytes);
                    } else {
                        //all off
                        let op_mode = bl.act_cnt8;
                        data.push(num_bytes); //num_bytes
                        data.push(op_mode);
                        data.push(bl.command);
                    }
                    log::debug!("Sending command: {:#?}", hex::encode(data));
                }   
                
                Ok(())
            },
            _ => {
                log::debug!("Mock ignoring: {:?}", _message);
                Ok(())
            }
        }

    }
}