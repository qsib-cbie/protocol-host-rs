
use crate::vrp;

use serde::{Serialize, Deserialize};

#[allow(dead_code)]
pub struct NetworkContext {
    pub endpoint: String,

    pub _ctx: zmq::Context,
    pub socket: zmq::Socket,
    pub socket_type_name: String,
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
    ActuatorsCommand { fabric_name: String, timer_mode_blocks: Option<vrp::vrp::TimerModeBlocks>, actuator_mode_blocks: Option<vrp::vrp::ActuatorModeBlocks>, op_mode_block: Option<vrp::vrp::OpModeBlock>, use_cache: Option<bool>},

}
