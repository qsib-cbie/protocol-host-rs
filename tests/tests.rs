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


fn connect_client_to_server(timeout: u64, client_commands: std::vec::Vec<String>) {
    panic_after(Duration::from_millis(timeout), || {
        // Multiple tests may attempt to re-register the logger
        let _ = simple_logger::init_with_level(log::Level::Debug);

        #[derive(Clone)]
        struct ThreadInfo<'a> {
            protocol: &'a str,
            hostname: &'a str,
            port: i16,
        };

        let server = vr_actuators_cli::network::Server::new("tcp", "*", 0);
        assert!(server.is_ok());
        let server = server.unwrap();
        let endpoint = server.get_last_endpoint();
        log::info!("Bound to endpoint: {}", endpoint); 

        let server_handle = std::thread::spawn(move || {
            let mut server = server;
            match server.serve() {
                Ok(_) => {
                    log::info!("Finished serving with Ok result.");
                },
                Err(err) => {
                    panic!(format!("Encountered error: {}", err));
                }
            }

            ()
        });

        let client_handle = std::thread::spawn(move || {
            let client = vr_actuators_cli::network::Client::from_endpoint(endpoint);
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
}

#[test]
fn nop_serve_to_client() {
    connect_client_to_server(50000, vec![]);
}

#[test]
fn connect_to_fabric() {
    connect_client_to_server(15000, vec![
        String::from(r#"{ "command_type": "AddFabric", "command": { "fabric_name": "Obid Feig LRM2500-B", "conn_type": "NFC" } }"#),
    ]);
}