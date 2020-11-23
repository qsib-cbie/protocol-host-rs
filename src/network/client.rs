use crate::network::common::*;
use crate::protocol::common::CommandMessage;

pub struct Client {
    net_ctx: NetworkContext,
}

impl Client {
    pub fn new(endpoint: String) -> Result<Client, Box<dyn std::error::Error>> {
        Ok(Client {
            net_ctx: NetworkContext::new(endpoint, "REQ_DEALER")?,
        })
    }

    pub fn request_message(
        &mut self,
        command_message: CommandMessage,
    ) -> Result<(), std::io::Error> {
        // Serialze the message
        let msg = match serde_json::to_string(&command_message) {
            Ok(msg) => msg,
            Err(err) => {
                log::error!(
                    "Failed to marshal: {:#?} with error: {:?}",
                    &command_message,
                    err
                );
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to marshal command_message",
                ));
            }
        };

        // Send the message
        assert_eq!(self.net_ctx.socket_type_name, "REQ_DEALER");
        self.net_ctx.socket.send(vec![], zmq::SNDMORE)?; // Simulated REQ: Empty Frame
        self.net_ctx.socket.send(msg.as_bytes(), 0)?; // Simulated REQ: Message Content

        // Receive Confirmation
        let _ = self.net_ctx.socket.recv_bytes(0)?; // Simulated REQ: Empty Frame
        let resp = self.net_ctx.socket.recv_bytes(0)?; // Simulated REQ: Message Content

        // Confirm Response
        let response_message = serde_json::from_slice(resp.as_slice())?;
        match response_message {
            CommandMessage::Failure { message } => {
                log::error!("Received Failure: {}", message);
                Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Unexpected response from server: {:?}", message),
                ))
            }
            other => {
                log::trace!("Received Response: {:#?}", other);
                Ok(())
            }
        }
    }
}
