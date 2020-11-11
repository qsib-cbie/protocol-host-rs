

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

