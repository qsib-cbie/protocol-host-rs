use crate::vrp::vrp;
use crate::conn::common::*;
use crate::network::common::*;
use crate::error::*;

use std::collections::HashMap;


pub struct ServerContext {
    net_ctx: NetworkContext,
}

impl ServerContext {//Need ability to select connection type here?
    pub fn new(endpoint: String) -> Result<ServerContext> {
        Ok(ServerContext {
            net_ctx: NetworkContext::new(endpoint, "REP_DEALER")?,
        })
    }
}

pub struct Server<'a, 'b> {
    ctx:  &'a ServerContext,
    conn: &'b dyn Connection<'b>,
    protocol: vrp::HapticProtocol<'b>,
    fabrics: HashMap<String, vrp::Fabric>,
}


impl<'a, 'b> Server<'a, 'b> {//Need ability to select connection type here?
    pub fn new(ctx: &'a ServerContext, conn: &'b impl Connection<'b>) -> Result<Server<'a, 'b>> {
        Ok(Server {
            ctx,
            conn,
            protocol: vrp::HapticProtocol::new(),
            fabrics: HashMap::new(),
        })
    }

    pub fn serve(&mut self) -> Result<bool> {
        log::info!("Beginning serve() loop ...");

        assert_eq!(self.ctx.net_ctx.socket_type_name, "REP_DEALER");
        loop {
            // Receive a message
            let id = self.ctx.net_ctx.socket.recv_bytes(0)?; // Simulated REP: Connection Identity
            let _ = self.ctx.net_ctx.socket.recv_bytes(0)?;           // Simulated REP: Empty Frame
            let msg = self.ctx.net_ctx.socket.recv_bytes(0)?;// Simulated REP: Message Content

            // Handle the message
            let request_message = serde_json::from_slice(msg.as_slice())?;
            let result: Result<()> = match request_message {
                CommandMessage::Stop{} => {
                    log::debug!("Received Stop.");

                    let success = serde_json::to_string(&CommandMessage::Success {})?;
                    self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(success.as_bytes(), 0)?;

                    return Ok(false);
                },

                CommandMessage::SystemReset { } => {
                    log::debug!("Received SystemReset.");
                    let reset = self.protocol.system_reset();

                    log::info!("Waiting for Feig Reader to reboot after system reset ...");
                    std::thread::sleep(std::time::Duration::from_millis(1000));
                    log::info!("Done waiting for reboot. Trying to reset connection ...");

                    let message = if reset.is_ok() { CommandMessage::Success{} } else { CommandMessage::Failure { message: String::from("Failed system reset") } };
                    let message = serde_json::to_string(&message)?;
                    self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(message.as_bytes(), 0)?;

                    return Ok(true);
                },
                CommandMessage::SetRadioFreqPower { power_level } => {
                    log::debug!("Received SetRadioFreqPower command for power_level {:?}.", power_level);
                    match power_level {
                        pl if pl == 0 || (pl >= 2 && pl <= 12) => {
                            self.protocol.set_radio_freq_power(power_level)
                        },
                        _ => {
                            let message = format!("Value for power level ({}) is outside acceptable range Low Power (0) or [2,12].", power_level);
                            log::error!("{}", message.as_str());
                            Err(InternalError::from(message.as_str()))
                        }
                    }
                },
                CommandMessage::CustomCommand { control_byte, data, device_required } => {
                    log::debug!("Received Custom command with control_byte {} and data {}", hex::encode(vec![control_byte]), hex::encode(data.as_bytes()));
                    let decoded_data = hex::decode(&data)?;
                    self.protocol.custom_command(control_byte, decoded_data.as_slice(), device_required)
                },

                CommandMessage::AddFabric { fabric_name } => {
                    match vrp::Fabric::new(&mut self.protocol, fabric_name.as_str()) {
                        Ok(fabric) => {
                            self.fabrics.insert(fabric_name, fabric);

                            log::info!("Added new fabric to command for AddFabric command");
                            log::trace!("Active Fabrics: {:#?}", self.fabrics);
                            Ok(())
                        },
                        Err(err) => {
                            log::error!("Failed to create Fabric: {}", err);
                            Err(err)
                        }
                    }

                },
                CommandMessage::RemoveFabric { fabric_name } => {
                    match self.fabrics.remove(&fabric_name) {
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
                CommandMessage::ActuatorsCommand { fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block, use_cache } => {
                    log::debug!("Received ActuatorsCommand: {:#?} {:#?} {:#?} {:#?}", fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block);
                    self.handle_actuators_command(fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block, use_cache)
                },
                CommandMessage::Success { } => {
                    Ok(())
                }

                other => {
                    let failure_message = String::from(format!("Unhandled CommandMessage request: {:#?}", other));
                    log::error!("{}", failure_message.as_str());

                    return Err(InternalError::from("Unhandled CommandMessage"));
                }
            };

            // Send a response using the result of handling the request
            let response = match result {
                Ok(_) => {
                    serde_json::to_string(&CommandMessage::Success { })?
                },
                Err(err) => {
                    let failure_message = err.to_string();
                    serde_json::to_string(&CommandMessage::Failure { message: failure_message })?
                }
            };

            self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
            self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
            self.ctx.net_ctx.socket.send(response.as_bytes(), 0)?;
            log::trace!("Sent Response: {}", response);
        }
    }

    #[allow(dead_code)]
    pub fn get_last_endpoint(self: &Self) -> String {
        self.ctx.net_ctx.socket.get_last_endpoint().unwrap().unwrap()
    }

    fn handle_actuators_command(self: &mut Self, fabric_name: String, timer_mode_blocks: Option<vrp::TimerModeBlocks>, actuator_mode_blocks: Option<vrp::ActuatorModeBlocks>, op_mode_block: Option<vrp::OpModeBlock>, use_cache: Option<bool>) -> Result<()> {
        let fabric = match self.fabrics.get_mut(&fabric_name) {
            Some(fabric) => {
                log::trace!("Found transponder for actuator command: {:?}", fabric.transponders);
                fabric
            },
            None => {
                let message = format!("No existing fabric to write actuator command: {:?}", self.fabrics);
                log::error!("{}", message);
                return Err(InternalError::from(message.as_str()));
            }
        };

        let fabric_uid = match fabric.transponders.len() {
            t_idx if t_idx > 0 => {
                &fabric.transponders[0].uid
            },
            _ => {
                return Err(InternalError::from("No transponder UID found for fabric"));
            }
        };

        let mut actuators_command = vrp::ActuatorsCommand {
            fabric_name: fabric_name.clone(),
            timer_mode_blocks: timer_mode_blocks.clone(),
            actuator_mode_blocks: actuator_mode_blocks.clone(),
            op_mode_block: op_mode_block.clone(),
            use_cache: use_cache.clone(),
        };

        let actuators_command = match use_cache {
            Some(flag)  => {
                if flag {
                    if fabric.state.state.use_cache.unwrap() {
                        actuators_command = fabric.state.diff(actuators_command);
                        log::debug!("Writing using cached diff: {:#?}", &actuators_command);
                    } else {
                        log::debug!("Skipping cached diff to warm cache");
                    }
                } else {
                    log::debug!("Command electing to bypass cache");
                }
                actuators_command
            },
            _ => {
                if fabric.state.state.use_cache.unwrap() {
                    actuators_command = fabric.state.diff(actuators_command);
                    log::debug!("Writing using cached diff: {:#?}", &actuators_command);
                } else {
                    log::debug!("Skipping cached diff to warm cache");
                }
                actuators_command
            }
        };

        let result = self.protocol.actuators_command(fabric_uid.as_slice(), &actuators_command.timer_mode_blocks, &actuators_command.actuator_mode_blocks, &actuators_command.op_mode_block);
        if result.is_ok() {
            fabric.state.apply(actuators_command);
        }
        result
    }
}
