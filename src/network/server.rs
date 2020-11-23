use crate::conn::common::*;
use crate::error::*;
use crate::network::common::*;
use crate::protocol::common::*;
use crate::protocol::{haptic::v0::HapticV0Protocol, mock::MockProtocol};

pub struct ServerContext {
    net_ctx: NetworkContext,
}

impl ServerContext {
    //Need ability to select connection type here?
    pub fn new(endpoint: String) -> Result<ServerContext> {
        Ok(ServerContext {
            net_ctx: NetworkContext::new(endpoint, "REP_DEALER")?,
        })
    }
}

pub struct Server<'a, 'b> {
    ctx: &'a ServerContext,
    protocol: Box<dyn Protocol<'b> + 'b>,
}

impl<'a, 'b> Server<'a, 'b> {
    pub fn new(
        ctx: &'a ServerContext,
        conn: Box<dyn Connection<'b> + 'b>,
    ) -> Result<Server<'a, 'b>> {
        if cfg![feature = "haptic_v0"] {
            log::info!("Creating HapticV0Protocol instance ...");
            Ok(Server {
                ctx,
                protocol: Box::new(HapticV0Protocol::new(conn)),
            })
        } else {
            log::info!("Creating MockProtocol instance ...");
            Ok(Server {
                ctx,
                protocol: Box::new(MockProtocol::new(conn)),
            })
        }
    }

    pub fn serve(&mut self) -> Result<bool> {
        log::info!("Beginning serve() loop ...");

        assert_eq!(self.ctx.net_ctx.socket_type_name, "REP_DEALER");
        loop {
            // Receive a message
            let id = self.ctx.net_ctx.socket.recv_bytes(0)?; // Simulated REP: Connection Identity
            let _ = self.ctx.net_ctx.socket.recv_bytes(0)?; // Simulated REP: Empty Frame
            let msg = self.ctx.net_ctx.socket.recv_bytes(0)?; // Simulated REP: Message Content

            // Handle the message
            let request_message = serde_json::from_slice(msg.as_slice())?;
            let result: Result<()> = match request_message {
                CommandMessage::Stop {} => {
                    log::debug!("Received Stop.");

                    let success = serde_json::to_string(&CommandMessage::Success {})?;
                    self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(success.as_bytes(), 0)?;

                    return Ok(false);
                }
                CommandMessage::SystemReset {} => {
                    log::debug!("Received SystemReset.");
                    let reset = self.protocol.handle_message(&request_message);

                    log::info!("Waiting for Feig Reader to reboot after system reset ...");
                    let timeout = if cfg!(any(feature = "haptic_v0")) {
                        1000
                    } else {
                        1
                    };
                    std::thread::sleep(std::time::Duration::from_millis(timeout));
                    log::info!("Done waiting for reboot. Trying to reset connection ...");

                    let message = if reset.is_ok() {
                        CommandMessage::Success {}
                    } else {
                        CommandMessage::Failure {
                            message: String::from("Failed system reset"),
                        }
                    };
                    let message = serde_json::to_string(&message)?;
                    self.ctx.net_ctx.socket.send(id, zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(vec![], zmq::SNDMORE)?;
                    self.ctx.net_ctx.socket.send(message.as_bytes(), 0)?;

                    return Ok(true);
                }
                CommandMessage::Success {} => Ok(()),

                other => self.protocol.handle_message(&other),
            };

            // Send a response using the result of handling the request
            let response = match result {
                Ok(_) => serde_json::to_string(&CommandMessage::Success {})?,
                Err(err) => {
                    let failure_message = err.to_string();
                    serde_json::to_string(&CommandMessage::Failure {
                        message: failure_message,
                    })?
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
        self.ctx
            .net_ctx
            .socket
            .get_last_endpoint()
            .unwrap()
            .unwrap()
    }
}
