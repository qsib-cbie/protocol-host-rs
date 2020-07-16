#[path = "vrp.rs"] mod vrp;

use serde::{Serialize, Deserialize};

pub struct NetworkContext {
    pub endpoint: String, 

    _ctx: zmq::Context,
    socket: zmq::Socket,
}

impl NetworkContext {

    pub fn get_endpoint(protocol: &str, hostname: &str, port: i16) -> String {
        String::from(format!("{}://{}:{}", protocol, hostname, port.to_string()))
    }

    pub fn new(endpoint: String, socket_type: zmq::SocketType) -> Result<NetworkContext, Box<dyn std::error::Error>> {
        let ctx = Self::_new(endpoint, socket_type);
        match ctx {
            Ok(ctx) => Ok(ctx),
            Err(err) => Err(err.into()),
        }
    }
    
    pub fn _new(endpoint: String, socket_type: zmq::SocketType) -> Result<NetworkContext, zmq::Error> {
        let ctx = zmq::Context::new();

        match socket_type {
            zmq::REP => {
                let socket = ctx.socket(zmq::REP)?;
                log::trace!("Created socket");

                socket.bind(endpoint.as_str())?;
                log::info!("Bound socket on {}", endpoint);

                Ok(NetworkContext {
                    endpoint,
                    _ctx: ctx,
                    socket
                })
            },
            zmq::REQ => {
                let socket = ctx.socket(zmq::REQ)?;
                log::trace!("Created socket");

                socket.connect(endpoint.as_str())?;
                log::info!("Connected to {}", endpoint);

                Ok(NetworkContext {
                    endpoint,
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

pub struct ServerContext {
    net_ctx: NetworkContext,
    usb_ctx: libusb::Context,
}

impl ServerContext {
    pub fn new(endpoint: String) -> Result<ServerContext, Box<dyn std::error::Error>> {
        Ok(ServerContext {
            net_ctx: NetworkContext::new(endpoint, zmq::REP)?,
            usb_ctx: libusb::Context::new()?,
        })
    }
}

pub struct Server<'a> {
    ctx: &'a ServerContext,
    conn: vrp::UsbConnection<'a>,
    fabrics: std::vec::Vec<vrp::Fabric>,
    shutting_down: bool,
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
            shutting_down: false
        })
    }

    pub fn serve(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Beginning serve() loop ...");
        loop {
            // 1. Read the type of the next message
            let mut message = zmq::Message::new();
            self.ctx.net_ctx.socket.recv(&mut message, 0)?;
            let command_info_message = message.as_str().unwrap();
            log::trace!("Received: {}", command_info_message);
            let command_info_type = command_info_message.parse();

            // 1.5 Send confirmation of command_info_type received
            self.ctx.net_ctx.socket.send("", 0)?;

            // 2. Read the next message
            let mut message = zmq::Message::new();
            self.ctx.net_ctx.socket.recv(&mut message, 0)?;
            let command_message = message.as_str().unwrap();

            match command_info_type {
                Ok(info_type) => {
                    // 3. Handle Command by delegating to command implementations
                    let command = Command::from_envelope(info_type, command_message);
                    let result = command?.info.visit(self);

                    let okay = Box::new(Okay { message: Some("OK".to_string()) });

                    // 3. Send the message type
                    let okay_type = okay.type_id().to_string();
                    self.ctx.net_ctx.socket.send(okay_type.as_bytes(), 0)?;
                    log::trace!("Sent: {}", okay_type);

                    // 3.5 Recv the confirmation of the okay_type
                    self.ctx.net_ctx.socket.recv(&mut message, 0)?;

                    // 4. Send the not okay
                    let okay_message = okay.to_string()?;
                    self.ctx.net_ctx.socket.send(okay_message.as_bytes(), 0)?;
                    log::trace!("Sent: {}", okay_message);

                    // 5. Maybe finish
                    if result.is_err() {
                        let err = result.unwrap_err();
                        log::error!("Leaving serve() due to err: {}", &err);
                        return Err(err);
                    }
                    if self.shutting_down {
                        log::info!("Leaving serve() gracefully ...");
                        return Ok(());
                    }
                },
                Err(_) => {
                    let not_okay = Box::new(NotOkay {
                        message: format!("invalid 'command_info_type': {}", command_info_message)
                    });

                    // 3. Send the message type
                    let not_okay_type = not_okay.type_id().to_string();
                    self.ctx.net_ctx.socket.send(not_okay_type.as_bytes(), 0)?;
                    log::debug!("Sent: {}", not_okay_type);

                    // 3.5 Recv the confirmation of the not_okay_type
                    self.ctx.net_ctx.socket.recv(&mut message, 0)?;

                    // 4. Send the not okay
                    let not_okay_message = not_okay.to_string()?;
                    self.ctx.net_ctx.socket.send(not_okay_message.as_bytes(), 0)?;
                    log::debug!("Sent: {}", not_okay_message);
                }
            }
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
            net_ctx: NetworkContext::new(endpoint, zmq::REQ)?,
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
            "Stop" => Ok(Command { info: Stop::from_string(ser_str)?, }),
            "SetRadioFreqPower" => Ok(Command { info: SetRadioFreqPower::from_string(ser_str)?, }),
            "SystemReset" => Ok(Command { info: SystemReset::from_string(ser_str)?, }),
            "CustomCommand" => Ok(Command { info: CustomCommand::from_string(ser_str)?, }),
            "ActuatorsCommand" => Ok(Command { info: vrp::ActuatorsCommand::from_string(ser_str)?, }),
            _ => {
                log::error!("Unregistered command type.");
                Err(std::io::Error::new(std::io::ErrorKind::Other, "Bad 'command' object. Check command definitions"))
            },
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
            5 => Ok(Command { info: Stop::from_string(command_info)?, }),
            6 => Ok(Command { info: SetRadioFreqPower::from_string(command_info)?, }),
            7 => Ok(Command { info: SystemReset::from_string(command_info)?, }),
            8 => Ok(Command { info: CustomCommand::from_string(command_info)?, }),
            9 => Ok(Command { info: vrp::ActuatorsCommand::from_string(command_info)?, }),
            _ => Err(std::io::Error::new(std::io::ErrorKind::Other, format!("Unexpected envelope info_type: {}", info_type))),
        }
    }
}

pub trait Visitor {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>>;
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Stop {
    pub message: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SetRadioFreqPower {
    pub power_level: u8,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct SystemReset { }

#[derive(Serialize, Deserialize, Debug)]
pub struct CustomCommand {
    pub control_byte: u8,
    pub data: String,
    pub device_required: bool,
 }

impl Visitor for Okay {
    fn visit(self: &Self, _: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Nothing to do for Okay command");
        Ok(())
    }
}
impl Visitor for NotOkay {
    fn visit(self: &Self, _: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Nothing to do for NotOkay command");
        Ok(())
    }
}

impl Visitor for AddFabric {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        match vrp::Fabric::new(&state.conn, self.fabric_name.as_str()) {
            Ok(fabric) => {
                state.fabrics.push(fabric);
                state.fabrics.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));
                log::info!("Added new fabric to command for AddFabric command");
                log::trace!("Active Fabrics: {:#?}", state.fabrics);
                Ok(())
            },
            Err(err) => {
                log::error!("Failed to create Fabric: {}", err);
                Err(err)
            }
        }
    }
}

impl Visitor for RemoveFabric {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        match state.fabrics.binary_search_by(|fabric| fabric.name.cmp(&self.fabric_name)) {
            Ok(position) => {
                state.fabrics.remove(position);
                log::info!("Removed existing fabric to command for AddFabric command");
                log::trace!("Active Fabrics:  {:#?}", state.fabrics);
                Ok(())
            },
            Err(err) => {
                let message = format!("No existing fabric to remove for RemoveFabric command: {}", err);
                log::error!("{}", message.as_str());
                Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
            }
        }
    }
}

impl Visitor for Stop {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Received Stop command with message {:?}. Shutting down ...", self.message);
        state.shutting_down = true;
        Ok(())
    }
}

impl Visitor for SetRadioFreqPower {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Received SetRadioFreqPower command for power_level {:?}.", self.power_level);
        if self.power_level == 0 || (self.power_level >= 2 && self.power_level <= 12) {
            state.conn.set_radio_freq_power(self.power_level) 
        } else {
            let message = format!("Value for power level ({}) is outside acceptable range Low Power (0) or [2,12].", self.power_level);
            log::error!("{}", message.as_str());
            Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, message.as_str())))
        }
    }
}

impl Visitor for SystemReset {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Received SystemReset.");
        state.conn.system_reset()
    }
}

impl Visitor for CustomCommand {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Received Custom command with control_byte {} and data {}", hex::encode(vec![self.control_byte]), hex::encode(self.data.as_bytes()));

        let decoded_data = hex::decode(&self.data)?;
        state.conn.custom_command(self.control_byte, decoded_data.as_slice(), self.device_required)
    }
}

impl Visitor for vrp::ActuatorsCommand {
    fn visit(self: &Self, state: &mut Server) -> Result<(), Box<dyn std::error::Error>> {
        log::info!("Received ActuatorsCommand: {:#?} ", self);

        let fabric_uid;
        match state.fabrics.binary_search_by(|fabric| fabric.name.cmp(&self.fabric_name)) {
            Ok(position) => {
                let transponders = &state.fabrics[position].transponders;
                log::trace!("Found transponders for actuator command: {:?}", transponders);

                if transponders.len() > 0 {
                    fabric_uid = &transponders[0].uid;
                } else {
                    let message = format!("No UID for matching fabric: {:#?}", state.fabrics[position]);
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

        state.conn.actuators_command(fabric_uid.as_slice(), self)
    }
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
                    "vr_actuators_cli::network::Stop" => 5,
                    "vr_actuators_cli::network::SetRadioFreqPower" => 6,
                    "vr_actuators_cli::network::SystemReset" => 7,
                    "vr_actuators_cli::network::CustomCommand" => 8,
                    "vr_actuators_cli::network::vrp::ActuatorsCommand" => 9,
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

impl_command_info!(Okay, NotOkay, AddFabric, RemoveFabric, Stop, SetRadioFreqPower, SystemReset, CustomCommand, vrp::ActuatorsCommand);