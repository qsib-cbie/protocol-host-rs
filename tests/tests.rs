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
    let _ = simple_logger::init_with_level(log::Level::Info);

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
    connect_client_to_server(30000, vec![
        String::from(r#"{ "command_type": "AddFabric", "command": { "fabric_name": "Obid Feig LRM2500-B", "conn_type": "NFC" } }"#),
    ])
}

#[test]
fn set_the_power_level() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "command_type": "SetRadioFreqPower", "command": { "power_level": 2 } }"#),
        String::from(r#"{ "command_type": "SystemReset", "command": { } }"#),
    ])
}

#[test]
fn set_the_power_level_low_power() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "command_type": "SetRadioFreqPower", "command": { "power_level": 0 } }"#),
        String::from(r#"{ "command_type": "SystemReset", "command": { } }"#),
    ])
}

#[test]
fn e2e_pulsing_after_antenna_reset() -> Result<(), Box<dyn std::error::Error>> {
    // Reset the conditions of the antenna
    connect_client_to_server(5000, vec![
        String::from(r#"{ "command_type": "SetRadioFreqPower", "command": { "power_level": 4 } }"#),
        String::from(r#"{ "command_type": "SystemReset", "command": { } }"#),
    ])?;

    // Allow the antenna time to come back online
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Run the end to end pulsing routine
    e2e_pulsing()
}

#[test]
fn e2e_pulsing() -> Result<(), Box<dyn std::error::Error>> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = vr_actuators_cli::vrp::OpModeBlock {
        act_cnt32: 0x02, // ceil(36.0 / 32.0) = 2
        act_mode: 0x00,
        op_mode: 0x86, 
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let hf_duty_cycle = 30;
    let hf_period = 150;
    let actuator_mode_blocks = vr_actuators_cli::vrp::ActuatorModeBlocks {
        block0_31: vr_actuators_cli::vrp::ActuatorModeBlock {
            b0: 0x01, // Enable all 8 acutators
            b1: 0x00, // Enable all 8 acutators
            b2: 0x00, // Enable all 8 acutators
            b3: 0x00, // Enable all 8 acutators
        },
        block32_63: vr_actuators_cli::vrp::ActuatorModeBlock {
            b0: 0x00, // Enable last 4 actuators
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        },
        block64_95: vr_actuators_cli::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        },
        block96_127: vr_actuators_cli::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        },
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    let timer_mode_blocks = vr_actuators_cli::vrp::TimerModeBlocks {
       /*
        * In single pulse operation, the variable t_pulse[ms] (16 bits) determines the time the pulse will remain on. 
        * The high frequency signal (carrier) timing is given by block 6 (0x18).
        * 
        * In gestures, t_pulse[ms] (16 bits) and t_pause[ms] (16 bits) control the the on and pause timing 
        */
        single_pulse_block: vr_actuators_cli::vrp::TimerModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        },

        /*
         * This block gives the timing condition for the high frequency signal option used in single pulse or continuous mode. 
         * It is given by the period T_high(ms) [16 bit] and duty cycle ton_high(ms) [16 bits]. 
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller. 
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
        */
        hf_block: vr_actuators_cli::vrp::TimerModeBlock {
            b0: ((hf_duty_cycle & 0x00FF)) as u8,
            b1: ((hf_duty_cycle & 0xFF00) >> 8) as u8,
            b2: ((hf_period & 0x00FF)) as u8,
            b3: ((hf_period & 0xFF00) >> 8) as u8,
        },

        /*
         * In continuous mode, there is an option for continuous pulsed mode. This block gives the timing condition for the low frequency signal. 
         * It is given by the period T_low(ms) [16 bit] and duty cycle ton_low(ms) [16 bits]. 
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller. 
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        lf_block: vr_actuators_cli::vrp::TimerModeBlock {
            b0: 0xFF, // ((4000 & 0x00FF)) as u8,
            b1: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
            b2: 0xFF, // ((4000 & 0x00FF)) as u8,
            b3: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
        }
    };
    let timer_mode_blocks= serde_json::to_string(&timer_mode_blocks)?;


    connect_client_to_server(150000, vec![
        String::from(r#"{ "command_type": "AddFabric", "command": { "fabric_name": "Jacob's Test Actuator Block of 36", "conn_type": "NFC" } }"#),
        // String::from(format!(r#"{{ "command_type": "ActuatorsCommand", "command": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_blocks": {} }} }}"#, op_mode_block, actuator_mode_blocks, timer_mode_blocks)),
        String::from(format!(r#"{{ "command_type": "ActuatorsCommand", "command": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {} }} }}"#, op_mode_block)),
    ])?;

    Ok(())
}