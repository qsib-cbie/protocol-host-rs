pub struct NetworkContext {
    pub protocol: String,
    pub hostname: String,
    pub port: i16,

    ctx: zmq::Context,
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

    pub fn get_connected_socket(& self) -> Result<zmq::Socket, std::io::Error> {
        let socket = self.ctx.socket(zmq::REQ)?;
        log::trace!("Created socket");

        let addr = format!("{}://{}:{}", self.protocol, self.hostname, self.port.to_string());
        socket.connect(addr.as_str())?;
        log::info!("Connected socket to {}", addr);

        Ok(socket)
    }
}

pub struct Server {
    net_ctx: NetworkContext
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
            }
        }
    }

    pub fn serve(& self) -> Result<(), std::io::Error> {
        let socket = self.net_ctx.get_bound_socket()?;
        let mut message = zmq::Message::new();
        loop {
            socket.recv(&mut message, 0)?;
            let message = message.as_str().unwrap();
            log::debug!("Received: {}", message);

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
            }
        }
    }

    pub fn request(& self) -> Result<(), std::io::Error> {
        let socket = self.net_ctx.get_connected_socket()?;
        for i in 0..100 {
            let message = format!("Hello, World {}", i);
            socket.send(message.as_bytes(), 0)?;
            log::debug!("Sent: {}", message);

            let mut message = zmq::Message::new();
            socket.recv(&mut message, 0)?;
            log::debug!("Received: {}", message.as_str().unwrap());
        }

        Ok(())
    }
}
