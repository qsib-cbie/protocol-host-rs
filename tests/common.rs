
use protocol_host_lib::error::*;

use std::{sync::mpsc, thread, time::Duration};
use protocol_host_lib::conn::common::Context;

fn conn_type() -> &'static str {
    if cfg!(feature = "ethernet") {
        return "ethernet"
    } else if cfg!(feature = "usb") {
        return "usb"
    } else {
        return "mock"
    }
}

fn panic_after<T, F>(d: Duration, f: F) -> T
where
    T: Send + 'static,
    F: FnOnce() -> T,
    F: Send + 'static,
{
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let val = f();
        done_tx.send(()).expect("Unable to send completion signal");
        val
    });

    match done_rx.recv_timeout(d) {
        Ok(_) => handle.join().expect("Thread panicked"),
        Err(_) => panic!("Thread took too long"),
    }
}


pub fn connect_client_to_server(timeout: u64, client_commands: std::vec::Vec<String>) -> Result<()> {
    // Multiple tests may attempt to re-register the logger
    let _ = simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Debug).init();

    panic_after(Duration::from_millis(timeout), move || {
        let proxy_front_endpoint = std::sync::Mutex::new(String::from(""));
        let proxy_back_endpoint= std::sync::Mutex::new(String::from(""));

        let front_endpoint= std::sync::Arc::new(proxy_front_endpoint);
        let back_endpoint= std::sync::Arc::new(proxy_back_endpoint);

        let client_endpoint = front_endpoint.clone();
        let proxy_front = front_endpoint.clone();

        let server_endpoint = back_endpoint.clone();
        let proxy_back= back_endpoint.clone();

        let ctx = zmq::Context::new();
        let publisher = ctx.socket(zmq::PUB).unwrap();
        let any_local_endpoint = protocol_host_lib::network::common::NetworkContext::get_endpoint("tcp", "*", 0);
        publisher.bind(any_local_endpoint.as_str()).unwrap();
        let control_endpoint = publisher.get_last_endpoint().unwrap().unwrap();
        log::info!("Bound control to {:?}", control_endpoint);

        let proxy_handle = std::thread::spawn(move || -> () {
            log::info!("Starting proxy ...");

            let any_local_endpoint = protocol_host_lib::network::common::NetworkContext::get_endpoint("tcp", "*", 0);
            let ctx = zmq::Context::new();

            let mut front = ctx.socket(zmq::ROUTER).unwrap();
            let mut back = ctx.socket(zmq::DEALER).unwrap();
            let mut control = ctx.socket(zmq::SUB).unwrap();
            assert!(control.set_subscribe(&vec![]).is_ok());

            log::info!("Binding and connecting proxy sockets ...");

            {
                let mut front_guard = proxy_front.lock().unwrap();
                let mut back_guard= proxy_back.lock().unwrap();

                front.bind(any_local_endpoint.as_str()).unwrap();
                *front_guard = front.get_last_endpoint().unwrap().unwrap();
                log::info!("Bound front to {:?}", *front_guard);

                back.bind(any_local_endpoint.as_str()).unwrap();
                *back_guard = back.get_last_endpoint().unwrap().unwrap();
                log::info!("Bound back to {:?}", *back_guard);

                assert!(control.connect(control_endpoint.as_str()).is_ok());
                log::info!("Connected control to {:?}", control_endpoint);
            }

            assert!(zmq::proxy_steerable(&mut front, &mut back, &mut control).is_ok());
            ()
        });

        let server_handle = std::thread::spawn(move || -> ()  {
            let work = move || -> Result<()> {
                log::info!("Starting server ...");

                let endpoint;
                loop {
                    {
                        let guard = server_endpoint.lock().unwrap();
                        if !guard.is_empty() {
                            endpoint = guard.clone();
                            break;
                        }
                    }

                    std::thread::sleep(std::time::Duration::from_millis(50));
                }

                #[allow(unused_variables)]
                #[cfg(feature = "usb")]
                let libusb_context = libusb::Context::new()?;
                let server_context = protocol_host_lib::network::server::ServerContext::new((&endpoint).clone())?;

                loop {
                    let serve_again = match conn_type() {
                        "mock" => {
                            let context = Box::new(protocol_host_lib::conn::mock::MockContext::new());
                            let connection = context.connection()?;
                            start_server_with_connection(connection, &server_context)
                        },
                        #[cfg(feature = "usb")]
                        "usb" => {
                            let context = Box::new(protocol_host_lib::conn::usb::UsbContext::new(&libusb_context)?);
                            let connection = context.connection()?;
                            start_server_with_connection(connection, &server_context)
                        },
                        #[cfg(feature = "ethernet")]
                        "ethernet" => {
                            let context = Box::new(protocol_host_lib::conn::ethernet::EthernetContext::new("192.168.10.10:10001")?);
                            let connection = context.connection()?;
                            start_server_with_connection(connection, &server_context)
                        },
                        _ => return Err(InternalError::from("No conn_type")),
                    };

                    log::info!("Finished serving with {:?}", serve_again);
                    match serve_again {
                        Ok(true) => {
                            log::info!("Continuing to serve ...");
                        },
                        Ok(false) => return Ok(()),
                        Err(err) => {
                            log::error!("Stopping serve due to: {:?}", err);
                            return Err(err);
                        }
                    }
                }
            };
            match work() {
                Err(err) => panic!("{}",err),
                _ => ()
            }
        });

        let client_handle = std::thread::spawn(move || {
            let endpoint;
            loop {
                {
                    let guard = client_endpoint.lock().unwrap();
                    if !guard.is_empty() {
                        endpoint = guard.clone();
                        break;
                    }
                }

                std::thread::sleep(std::time::Duration::from_millis(50));
            }

            log::info!("Starting client ...");

            let client = protocol_host_lib::network::client::Client::new(endpoint);
            assert!(client.is_ok());
            let mut client = client.unwrap();

            let mut client_commands = client_commands;
            client_commands.push(String::from(r#"{ "Stop": {} }"#));

            log::info!("Running client commands:");
            for command in &client_commands {
                log::info!("{}", command);
            }

            for command in &client_commands {
                let command_stream = serde_json::Deserializer::from_str(command.as_str()).into_iter::<protocol_host_lib::protocol::common::CommandMessage>();
                for command in command_stream {
                    assert!(command.is_ok());
                    let result = client.request_message(command.unwrap());
                    assert!(result.is_ok());
                }
            }
            ()
        });

        let server_result = server_handle.join();
        log::info!("Server is finished.");
        let client_result = client_handle.join();
        log::info!("Client is finished.");
        assert!(client_result.is_ok());
        assert!(server_result.is_ok());

        log::info!("Signaling proxy to terminate");
        assert!(publisher.send("TERMINATE".to_ascii_uppercase().as_bytes(), 0).is_ok());

        log::info!("Waiting for proxy to finish ...");
        let proxy_result = proxy_handle.join();
        assert!(proxy_result.is_ok());
        log::info!("Success");
    });

    Ok(())
}

fn start_server_with_connection<'a, 'b>(connection: Box<dyn protocol_host_lib::conn::common::Connection<'a> + 'a>, server_context: &'b protocol_host_lib::network::server::ServerContext) -> Result<bool> {
    let mut server = protocol_host_lib::network::server::Server::new(server_context, connection).expect("Failed to initialize server");
    server.serve()
}