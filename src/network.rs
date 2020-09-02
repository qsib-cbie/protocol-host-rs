#[path = "vrp.rs"] mod vrp;

use serde::{Serialize, Deserialize};

#[allow(dead_code)]
pub struct NetworkContext {
    pub endpoint: String, 

    _ctx: zmq::Context,
    socket: zmq::Socket,
    socket_type_name: String,

}

impl NetworkContext {

    pub fn get_endpoint(protocol: &str, hostname: &str, port: i16) -> String {
        String::from(format!("{}://{}:{}", protocol, hostname, port.to_string()))
    }

    pub fn new(endpoint: String, socket_type_name: &str) -> Result<NetworkContext, Box<dyn std::error::Error>> {
        let ctx = Self::_new(endpoint, socket_type_name);
        match ctx {
            Ok(ctx) => Ok(ctx),
            Err(err) => Err(err.into()),
        }
    }
    
    pub fn _new(endpoint: String, socket_type_name: &str) -> Result<NetworkContext, zmq::Error> {
        let ctx = zmq::Context::new();

        match socket_type_name {
            "REP_DEALER" => {
                let socket = ctx.socket(zmq::DEALER)?;
                log::trace!("Created socket DEALER to act as REP");

                socket.connect(endpoint.as_str())?;
                log::info!("Connected to {}", endpoint);


                Ok(NetworkContext {
                    endpoint,
                    _ctx: ctx,
                    socket,
                    socket_type_name: String::from(socket_type_name),

                })
            },
            "REQ_DEALER" => {
                let socket = ctx.socket(zmq::DEALER)?;
                log::trace!("Created socket DEALER to act as REQ");

                socket.connect(endpoint.as_str())?;
                log::info!("Connected to {}", endpoint);


                Ok(NetworkContext {
                    endpoint,
                    _ctx: ctx,
                    socket,
                    socket_type_name: String::from(socket_type_name),

                })
            },
            _ => {
                log::error!("Unsupported socket type: {:#?}", socket_type_name);
                Err(zmq::Error::EINVAL)
            }
        }
    }
}
pub struct ServerContext {
    net_ctx: NetworkContext,
    usb_ctx: libusb::Context,
}

impl ServerContext {
    pub fn new(endpoint: String) -> Result<ServerContext, Box<dyn std::error::Error>> {
        Ok(ServerContext {
            net_ctx: NetworkContext::new(endpoint, "REP_DEALER")?,
            usb_ctx: libusb::Context::new()?,
        })
    }
}

pub struct Server<'a> {
    ctx: &'a ServerContext,
    conn: vrp::UsbConnection<'a>,
    fabrics: std::vec::Vec<vrp::Fabric>,
}

pub struct Client {
    net_ctx: NetworkContext
}

impl<'a> Server<'a> {
    pub fn new(ctx: &'a ServerContext) -> Result<Server<'a>, Box<dyn std::error::Error>> {
        Ok(Server {
            ctx,
            conn: vrp::UsbConnection::new(&ctx.usb_ctx)?,
            fabrics: std::vec::Vec::new(),
        })
    }

    pub fn serve(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Beginning serve() loop ...");

        assert_eq!(self.ctx.net_ctx.socket_type_name, "REP_DEALER");
        loop {
            // Receive a message
            let id = self.ctx.net_ctx.socket.recv_bytes(0)?; // Simulated REP: Connection Identity
            let _ = self.ctx.net_ctx.socket.recv_bytes(0)?;           // Simulated REP: Empty Frame 
            let msg = self.ctx.net_ctx.socket.recv_bytes(0)?;// Simulated REP: Message Content

            // Handle the message
            let request_message = serde_json::from_slice(msg.as_slice())?;
            let result: Result<(), Box<dyn std::error::Error>> = match request_message {
                CommandMessage::Stop{} => {
                    log::info!("Received Stop.");

                    let success = serde_json::to_string(&CommandMessage::Success {})?;
                    self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(success.as_bytes(), 0)?;

                    return Ok(());
                },

                CommandMessage::SystemReset { } => {
                    log::info!("Received SystemReset.");
                    self.conn.system_reset()
                },
                CommandMessage::SetRadioFreqPower { power_level } => {
                    log::info!("Received SetRadioFreqPower command for power_level {:?}.", power_level);
                    match power_level {
                        pl if pl == 0 || (pl >= 2 && pl <= 12) => {
                            self.conn.set_radio_freq_power(power_level)
                        },
                        _ => {
                            let message = format!("Value for power level ({}) is outside acceptable range Low Power (0) or [2,12].", power_level);
                            log::error!("{}", message.as_str());
                            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
                        }
                    }
                },
                CommandMessage::CustomCommand { control_byte, data, device_required } => {
                    log::info!("Received Custom command with control_byte {} and data {}", hex::encode(vec![control_byte]), hex::encode(data.as_bytes()));
                    let decoded_data = hex::decode(&data)?;
                    self.conn.custom_command(control_byte, decoded_data.as_slice(), device_required)
                },
            
                CommandMessage::AddFabric { fabric_name } => {
                    match vrp::Fabric::new(&mut self.conn, fabric_name.as_str()) {
                        Ok(fabric) => {
                            self.fabrics.push(fabric);
                            self.fabrics.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
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
                    match self.fabrics.binary_search_by(|fabric| fabric.name.cmp(&fabric_name)) {
                        Ok(position) => {
                            self.fabrics.remove(position);
                            log::info!("Removed existing fabric to command for AddFabric command");
                            log::trace!("Active Fabrics:  {:#?}", self.fabrics);
                            Ok(())
                        },
                        Err(err) => {
                            let message = format!("No existing fabric to remove for RemoveFabric command: {}", err);
                            log::error!("{}", message.as_str());
                            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
                        }
                    }
                },
                CommandMessage::ActuatorsCommand { fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block } => {
                    log::info!("Received ActuatorsCommand: {:#?} {:#?} {:#?} {:#?}", fabric_name, timer_mode_blocks, actuator_mode_blocks, op_mode_block);

                    let fabric_uid;
                    match self.fabrics.binary_search_by(|fabric| fabric.name.cmp(&fabric_name)) {
                        Ok(position) => {
                            let transponders = &self.fabrics[position].transponders;
                            log::trace!("Found transponders for actuator command: {:?}", transponders);
            
                            if transponders.len() > 0 {
                                fabric_uid = &transponders[0].uid;
                            } else {
                                let message = format!("No UID for matching fabric: {:#?}", self.fabrics[position]);
                                log::error!("{}", message.as_str());
                                return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
                            }
                        },
                        Err(err) => {
                            let message = format!("No existing fabric to write actuator command: {}", err);
                            log::error!("{}", message.as_str());
                            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
                        }
                    }
            
                    self.conn.actuators_command(fabric_uid.as_slice(), timer_mode_blocks, actuator_mode_blocks, op_mode_block)
                },

                other => {
                    let failure_message = String::from(format!("Unhandled CommandMessage request: {:#?}", other));
                    log::error!("{}", failure_message.as_str());

                    return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, "Unhandled CommandMessage")));
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
}

impl Client {
    pub fn new(endpoint: String) -> Result<Client, Box<dyn std::error::Error>> {
        Ok(Client {
            net_ctx: NetworkContext::new(endpoint, "REQ_DEALER")?,
        })
    }

    pub fn request_message(&mut self, command_message: CommandMessage) -> Result<(), std::io::Error> {
        // Serialze the message
        let msg = match serde_json::to_string(&command_message) {
            Ok(msg) => msg,
            Err(err) => {
                log::error!("Failed to marshal: {:#?} with error: {:?}", &command_message, err);
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "Failed to marshal command_message"));
            }
        };

        // Send the message
        assert_eq!(self.net_ctx.socket_type_name, "REQ_DEALER");
        self.net_ctx.socket.send(vec![], zmq::SNDMORE)?; // Simulated REQ: Empty Frame
        self.net_ctx.socket.send(msg.as_bytes(), 0)?;    // Simulated REQ: Message Content

        // Receive Confirmation
        let _ = self.net_ctx.socket.recv_bytes(0)?;             // Simulated REQ: Empty Frame
        let resp = self.net_ctx.socket.recv_bytes(0)?; // Simulated REQ: Message Content 

        // Confirm Response
        let response_message = serde_json::from_slice(resp.as_slice())?;
        match response_message {
            CommandMessage::Failure { message } => {
                log::error!("Received Failure: {}", message);
                Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unexpected response from server: {:?}", message)))
            },
            other => {
                log::trace!("Received Response: {:#?}", other);
                Ok(())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub enum CommandMessage {
    Failure { message: String },
    Success { },

    Stop { },

    SystemReset { },
    SetRadioFreqPower { power_level: u8 },
    CustomCommand { control_byte: u8, data: String, device_required: bool },

    AddFabric { fabric_name: String },
    RemoveFabric { fabric_name: String },
    ActuatorsCommand { fabric_name: String, timer_mode_blocks: Option<vrp::TimerModeBlocks>, actuator_mode_blocks: Option<vrp::ActuatorModeBlocks>, op_mode_block: Option<vrp::OpModeBlock>},
    
}
