#[path = "vrp.rs"] mod vrp;

use serde::{Serialize, Deserialize};

pub struct NetworkContext {
    pub protocol: String,
    pub hostname: String,
    pub port: i16,

    ctx: zmq::Context,
    socket: Option<zmq::Socket>,
}

impl NetworkContext {
    pub fn get_bound_socket(& self) -> Result<zmq::Socket, std::io::Error> {
        let socket = self.ctx.socket(zmq::REP)?;
        log::trace!("Created socket");

        let addr = format!("{}://{}:{}", self.protocol, self.hostname, self.port.to_string());
        socket.bind(addr.as_str())?;
        log::info!("Bound socket on {}", addr);

        Ok(socket)
    }

    pub fn get_connected_socket(& mut self) -> Result<&zmq::Socket, std::io::Error> {
        if self.socket.is_none() {
            let socket = self.ctx.socket(zmq::REQ)?;
            log::trace!("Created socket");

            let addr = format!("{}://{}:{}", self.protocol, self.hostname, self.port.to_string());
            socket.connect(addr.as_str())?;
            log::info!("Connected socket to {}", addr);

            self.socket = Some(socket);
        }

        let socket_ref = self.socket.as_ref().unwrap();
        Ok(socket_ref)
    }
}

pub struct Server {
    net_ctx: NetworkContext,
    fabrics: std::vec::Vec<vrp::Fabric>,
}

pub struct Client {
    net_ctx: NetworkContext
}

impl Server {
    pub fn new(protocol: String, hostname: String, port: i16) -> Server {
        Server {
            net_ctx: NetworkContext {
                protocol,
                hostname,
                port,
                ctx: zmq::Context::new(),
                socket: None,
            },
            fabrics: std::vec::Vec::new(),
        }
    }

    pub fn serve(& self) -> Result<(), std::io::Error> {
        let socket = self.net_ctx.get_bound_socket()?;
        let mut message = zmq::Message::new();
        loop {
            socket.recv(&mut message, 0)?;
            let message = message.as_str().unwrap();
            log::debug!("Received: {}", message);

            // TODO: Ser De for Json

            // TODO: Handle Command

            socket.send(message.as_bytes(), 0)?;
            log::debug!("Sent: {}", message);
        }
    }
}

impl Client {
    pub fn new(protocol: String, hostname: String, port: i16) -> Client {
        Client {
            net_ctx: NetworkContext {
                protocol,
                hostname,
                port,
                ctx: zmq::Context::new(),
                socket: None,
            }
        }
    }

    pub fn request(& mut self, command: Command) -> Result<(), std::io::Error> {
        let socket = self.net_ctx.get_connected_socket()?;

        let message = command.info.to_string().expect("Failed to serialize command");
        socket.send(message.as_bytes(), 0)?;
        log::debug!("Sent: {}", message);

        let mut message = zmq::Message::new();
        socket.recv(&mut message, 0)?;
        log::debug!("Received: {}", message.as_str().unwrap());

        
        Ok(())

        // if message.as_str().unwrap() == "OK" {
        //     Ok(())
        // } else {
        //     Err(std::io::Error::new(std::io::ErrorKind::Other, "NOT OK"))
        // }
    }

    pub fn parse_command(& self, command: serde_json::Value) -> Option<Command> {
        let fields = command.as_object().expect("Commands must have fields");
        let command_name = fields.get("command").expect("'command' missing");
        if command_name == "add-fabric" {
            let fabric_name = fields.get("fabric-name").expect("'fabric-name' missing");
            let fabric_name = fabric_name.as_str().expect("invalid fabric-name");
            let fabric_name = String::from(fabric_name);
            let conn_type= fields.get("conn-type").expect("'conn-type' missing");
            let conn_type = conn_type.as_str().expect("invalid conn-type");
            let conn_type = String::from(conn_type);


            Some(
                Command::new(
                Box::new(AddFabricInfo {
                    fabric_name,
                    conn_type,
                }))
            )
        } else {
            log::error!("Unexpected command: {:#?}", command);
            None 
        }
    }
}

pub struct Command {
    info: Box<dyn CommandInfo>
}

impl Command {
    pub fn new(info: Box<dyn CommandInfo>) -> Command {
        Command {
            info
        }
    }
}

pub trait CommandInfo {
    fn type_id() -> std::any::TypeId where Self: Sized;

    fn to_string(self: Box<Self>) -> Option<String>;

    fn from_string(ser_self: String) -> Box<Self> where Self: Sized;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFabricInfo {
    pub fabric_name: String,
    pub conn_type: String, 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveFabricInfo {
    pub fabric_name: String,
    pub conn_type: String, 
}

macro_rules! impl_command_info {
    ($($t:ty),+) => {
        $(impl CommandInfo for $t {
            fn type_id() -> std::any::TypeId {
                std::any::TypeId::of::<$t>()
            }

            fn to_string(self: Box<Self>) -> Option<String> {
                match serde_json::to_string(self.as_ref()) {
                    Ok(serialized_string) => {
                        Some(serialized_string)
                    },
                    Err(_) => {
                        log::error!("Failed to serialize: {:#?}", self);
                        None
                    }
                }
            }

            fn from_string(ser_self: String) -> Box<Self> {
                Box::new(serde_json::from_str(ser_self.as_str()).unwrap())
            }
        })+
    }
}

impl_command_info!(AddFabricInfo, RemoveFabricInfo);
