#[path = "vrp.rs"] mod vrp;

use serde::{Serialize, Deserialize};

pub struct NetworkContext {
    pub protocol: String,
    pub hostname: String,
    pub port: i16,

    _ctx: zmq::Context,
    socket: zmq::Socket,
}

impl NetworkContext {

    pub fn new(protocol: &str, hostname: &str, port: i16, socket_type: zmq::SocketType) -> Result<NetworkContext, Box<dyn std::error::Error>> {
        let ctx = Self::_new(protocol, hostname, port, socket_type);
        match ctx {
            Ok(ctx) => Ok(ctx),
            Err(err) => Err(err.into()),
        }
    }
    
    pub fn _new(protocol: &str, hostname: &str, port: i16, socket_type: zmq::SocketType) -> Result<NetworkContext, zmq::Error> {
        let ctx = zmq::Context::new();

        match socket_type {
            zmq::REP => {
                let socket = ctx.socket(zmq::REP)?;
                log::trace!("Created socket");

                let addr = format!("{}://{}:{}", protocol, hostname, port.to_string());
                socket.bind(addr.as_str())?;
                log::info!("Bound socket on {}", addr);

                Ok(NetworkContext {
                    protocol: String::from(protocol),
                    hostname: String::from(hostname),
                    port,
                    _ctx: ctx,
                    socket
                })
            },
            zmq::REQ => {
                let socket = ctx.socket(zmq::REQ)?;
                log::trace!("Created socket");

                let addr = format!("{}://{}:{}", protocol, hostname, port.to_string());
                socket.connect(addr.as_str())?;
                log::info!("Connected socket to {}", addr);

                Ok(NetworkContext {
                    protocol: String::from(protocol),
                    hostname: String::from(hostname),
                    port,
                    _ctx: ctx,
                    socket
                })
            },
            _ => {
                log::error!("Unsupported socket type: {:#?}", socket_type);
                Err(zmq::Error::EINVAL)
            }
        }
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
    pub fn new(protocol: &str, hostname: &str, port: i16) -> Result<Server, Box<dyn std::error::Error>> {
        Ok(Server {
            net_ctx: NetworkContext::new(protocol, hostname, port, zmq::REP)?,
            fabrics: std::vec::Vec::new(),
        })
    }

    pub fn serve(&mut self) -> Result<(), std::io::Error> {
        loop {
            // 1. Read the type of the next message
            let mut message = zmq::Message::new();
            self.net_ctx.socket.recv(&mut message, 0)?;
            let command_info_message = message.as_str().unwrap();
            log::trace!("Received: {}", command_info_message);
            let command_info_type = command_info_message.parse();

            // 1.5 Send confirmation of command_info_type received
            self.net_ctx.socket.send("", 0)?;

            // 2. Read the next message
            let mut message = zmq::Message::new();
            self.net_ctx.socket.recv(&mut message, 0)?;
            let command_message = message.as_str().unwrap();

            match command_info_type {
                Ok(info_type) => {
                    // 3. Handle Command by delegating to command implementations
                    let command = Command::from_envelope(info_type, command_message);
                    command?.info.visit(self);

                    let okay = Box::new(Okay { message: Some("OK".to_string()) });

                    // 3. Send the message type
                    let okay_type = okay.type_id().to_string();
                    self.net_ctx.socket.send(okay_type.as_bytes(), 0)?;
                    log::trace!("Sent: {}", okay_type);

                    // 3.5 Recv the confirmation of the okay_type
                    self.net_ctx.socket.recv(&mut message, 0)?;

                    // 4. Send the not okay
                    let okay_message = okay.to_string()?;
                    self.net_ctx.socket.send(okay_message.as_bytes(), 0)?;
                    log::trace!("Sent: {}", okay_message);
                },
                Err(_) => {
                    let not_okay = Box::new(NotOkay {
                        message: format!("invalid 'command_info_type': {}", command_info_message)
                    });

                    // 3. Send the message type
                    let not_okay_type = not_okay.type_id().to_string();
                    self.net_ctx.socket.send(not_okay_type.as_bytes(), 0)?;
                    log::debug!("Sent: {}", not_okay_type);

                    // 3.5 Recv the confirmation of the not_okay_type
                    self.net_ctx.socket.recv(&mut message, 0)?;

                    // 4. Send the not okay
                    let not_okay_message = not_okay.to_string()?;
                    self.net_ctx.socket.send(not_okay_message.as_bytes(), 0)?;
                    log::debug!("Sent: {}", not_okay_message);
                }
            }
        }
    }
}

impl Client {
    pub fn new(protocol: &str, hostname: &str, port: i16) -> Result<Client, Box<dyn std::error::Error>> {
        Ok(Client {
            net_ctx: NetworkContext::new(protocol, hostname, port, zmq::REQ)?,
        })
    }

    pub fn request(& mut self, command: Command) -> Result<(), std::io::Error> {
        // 1. Send the type of request
        let command_info = command.info;
        let message =  command_info.type_id().to_string();
        self.net_ctx.socket.send(message.as_bytes(), 0)?;
        log::trace!("Sent: {}", message);

        // 1.5 Receive confirmation of command_info_type sent
        let mut message = zmq::Message::new();
        self.net_ctx.socket.recv(&mut message, 0)?;

        // 2. Send the request
        let message = command_info.to_string().expect("Failed to serialize command");
        self.net_ctx.socket.send(message.as_bytes(), 0)?;
        log::debug!("Sent: {}", message);

        // 3. Receive the command request type
        let mut message = zmq::Message::new();
        self.net_ctx.socket.recv(&mut message, 0)?;
        let response_type = message.as_str().unwrap();
        log::trace!("Received: {}", response_type);

        // 3.5 Send confirmation
        self.net_ctx.socket.send("", 0)?;

        // 4. Receive the command response
        let mut message = zmq::Message::new();
        self.net_ctx.socket.recv(&mut message, 0)?;
        log::debug!("Received: {}", message.as_str().unwrap());

        match response_type.parse() {
            Ok(1) => {
                Okay::from_string(message.as_str().unwrap())?;
                Ok(())
            },
            _ => {
                Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unexpected response from server: {:?}", message)))
            }
        }
    }

    pub fn parse_command(& self, command: serde_json::Value) -> Result<Command, std::io::Error> {
        let fields = command.as_object().expect("Commands must have fields");
        let command_type = fields.get("command_type").expect("'command_type' missing");
        let command = fields.get("command").expect("'command' missing");
        let ser_command = serde_json::to_string(command).expect("Failed to reserialize 'command'");
        let ser_str = ser_command.as_str();
        match command_type.as_str().unwrap() {
            "Okay" => Ok(Command { info: Okay::from_string(ser_str)?, }),
            "NotOkay" => Ok(Command { info: NotOkay::from_string(ser_str)?, }),
            "AddFabric" => Ok(Command { info: AddFabric::from_string(ser_str)?, }),
            "RemoveFabric" => Ok(Command { info: RemoveFabric::from_string(ser_str)?, }),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, "Bad 'command' object. Check command definitions")),
        }
    }
}

pub struct Command {
    info: Box<dyn CommandInfo>,
}

impl Command {
    pub fn from_envelope(info_type: i16, command_info: & str) -> Result<Command, std::io::Error> {
        match info_type {
            0 => Err(std::io::Error::new(std::io::ErrorKind::Other, "Unexpected envelope info_type: 0")),
            1 => Ok(Command { info: Okay::from_string(command_info)?, }),
            2 => Ok(Command { info: NotOkay::from_string(command_info)?, }),
            3 => Ok(Command { info: AddFabric::from_string(command_info)?, }),
            4 => Ok(Command { info: RemoveFabric::from_string(command_info)?, }),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unexpected envelope info_type: {}", info_type))),
        }
    }
}

pub trait CommandInfo: Visitor {
    fn type_id(self: &Self) -> i16;

    fn to_string(self: &Self) -> Result<String, serde_json::error::Error>;

    fn from_string(ser_self: &str) -> Result<Box<dyn CommandInfo>, serde_json::error::Error> where Self: Sized;
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Okay {
    pub message: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct NotOkay {
    pub message: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AddFabric {
    pub fabric_name: String,
    pub conn_type: String, 
}

#[derive(Serialize, Deserialize, Debug)]
pub struct RemoveFabric {
    pub fabric_name: String,
    pub conn_type: String, 
}

macro_rules! impl_command_info {
    ($($t:ty),+) => {
        $(impl CommandInfo for $t {
            fn type_id(self: &Self) -> i16 {
                log::trace!("Choosing type_id enumeration for {}", std::any::type_name::<$t>());
                match std::any::type_name::<$t>() {
                    "vr_actuators_cli::network::Okay" => 1,
                    "vr_actuators_cli::network::NotOkay" => 2,
                    "vr_actuators_cli::network::AddFabric" => 3,
                    "vr_actuators_cli::network::RemoveFabric" => 4,
                    _ => 0
                }
            }

            fn to_string(self: &Self) -> Result<String, serde_json::error::Error> {
                serde_json::to_string(self)
            }

            fn from_string(ser_self: &str) -> Result<Box<dyn CommandInfo>, serde_json::error::Error> {
                let de_self: $t = serde_json::from_str(ser_self)?;
                Ok(Box::new(de_self))
            }
        })+
    }
}

impl_command_info!(Okay, NotOkay, AddFabric, RemoveFabric);

pub trait Visitor {
    fn visit(self: &Self, state: &mut Server);
}

impl Visitor for Okay {
    fn visit(self: &Self, _: &mut Server) {
        log::info!("Nothing to do for Okay command");
    }
}
impl Visitor for NotOkay {
    fn visit(self: &Self, _: &mut Server) {
        log::info!("Nothing to do for NotOkay command");
    }
}

impl Visitor for AddFabric {
    fn visit(self: &Self, state: &mut Server) {
        state.fabrics.push(vrp::Fabric::new(self.fabric_name.as_str()));
        log::info!("Added new fabric to command for AddFabric command: {:#?}", state.fabrics);
    }
}

impl Visitor for RemoveFabric {
    fn visit(self: &Self, state: &mut Server) {
        match state.fabrics.binary_search_by(|fabric| fabric.name.cmp(&self.fabric_name)) {
            Ok(position) => {
                state.fabrics.remove(position);
                log::info!("Removed existing fabric to command for AddFabric command: {:#?}", state.fabrics);
            },
            Err(_) => {
                log::info!("No existing fabric to remove for RemoveFabric command");
            }
        }
    }
}

