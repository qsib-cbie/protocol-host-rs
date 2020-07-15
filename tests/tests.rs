extern crate vr_actuators_cli;

use std::{sync::mpsc, thread, time::Duration};

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


fn connect_client_to_server(timeout: u64, client_commands: std::vec::Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    // Multiple tests may attempt to re-register the logger
    let _ = simple_logger::init_with_level(log::Level::Debug);

    panic_after(Duration::from_millis(timeout), move || {
        let shared_endpoint = std::sync::Mutex::new(String::from(""));
        let shared_endpoint = std::sync::Arc::new(shared_endpoint);

        let server_endpoint = shared_endpoint.clone();
        let client_endpoint = shared_endpoint.clone();
        let server_handle = std::thread::spawn(move || -> Result<(), ()>  {
            let endpoint = vr_actuators_cli::network::NetworkContext::get_endpoint("tcp", "*", 0);
            let server_context = vr_actuators_cli::network::ServerContext::new(endpoint);
            match server_context {
                Ok(_) => {}
                Err(_) => { return Err(()); }
            }
            let server_context = server_context.unwrap();
            let server = vr_actuators_cli::network::Server::new(&server_context);
            match &server {
                Ok(_) => {}
                Err(err) => { 
                    log::error!("Failed to initialize server: {}", err);
                    return Err(());
                }
            }
            let mut server = server.unwrap();

            let endpoint = server.get_last_endpoint();
            log::info!("Bound to endpoint: {}", endpoint); 
            {
                *server_endpoint.lock().unwrap() = endpoint;
            }

            match server.serve() {
                Ok(_) => {
                    log::info!("Finished serving with Ok result.");
                },
                Err(err) => {
                    log::error!("Failed to server: {}", err);
                    panic!(format!("Encountered error: {}", err));
                }
            }

            Ok(())
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

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let client = vr_actuators_cli::network::Client::new(endpoint);
            assert!(client.is_ok());
            let mut client = client.unwrap();

            let mut client_commands = client_commands;
            client_commands.push(String::from(r#"{ "command_type": "Stop", "command": { "message": "All Done" } }"#));
            for command in client_commands {
                let command_stream = serde_json::Deserializer::from_str(command.as_str()).into_iter::<serde_json::Value>();
                for command in command_stream {
                    let command = command.unwrap();
                    let command = client.parse_command(command).unwrap();
                    let result = client.request(command);
                    assert!(result.is_ok());
                }
            }

            ()
        });

        let client_result = client_handle.join();
        let server_result = server_handle.join();
        assert!(client_result.is_ok());
        assert!(server_result.is_ok());
    });

    Ok(())
}

#[test]
fn nop_serve_to_client() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(500, vec![])
}

#[test]
fn connect_to_fabric() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(500, vec![
        String::from(r#"{ "command_type": "AddFabric", "command": { "fabric_name": "Obid Feig LRM2500-B", "conn_type": "NFC" } }"#),
    ])
}

#[test]
fn set_the_power_level() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(500, vec![
        String::from(r#"{ "command_type": "SetRadioFreqPower", "command": { "power_level": 4 } }"#),
        String::from(r#"{ "command_type": "SystemReset", "command": { } }"#),
    ])
}

#[test]
fn set_the_power_level_low_power() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(1000, vec![
        String::from(r#"{ "command_type": "SetRadioFreqPower", "command": { "power_level": 0 } }"#),
        String::from(r#"{ "command_type": "SystemReset", "command": { } }"#),
    ])
}