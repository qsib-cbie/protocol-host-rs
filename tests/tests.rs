extern crate vr_actuators_cli;

use std::{sync::mpsc, thread, time::Duration};
use vr_actuators_cli::conn::common::Context;

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
        let any_local_endpoint = vr_actuators_cli::network::common::NetworkContext::get_endpoint("tcp", "*", 0);
        publisher.bind(any_local_endpoint.as_str()).unwrap();
        let control_endpoint = publisher.get_last_endpoint().unwrap().unwrap();
        log::info!("Bound control to {:?}", control_endpoint);

        let proxy_handle = std::thread::spawn(move || -> Result<(), ()> {
            log::info!("Starting proxy ...");

            let any_local_endpoint = vr_actuators_cli::network::common::NetworkContext::get_endpoint("tcp", "*", 0);
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
            Ok(())
        });

        let server_handle = std::thread::spawn(move || -> Result<(), ()>  {
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

                std::thread::sleep(std::time::Duration::from_millis(100));
            }

            let server_context = vr_actuators_cli::network::server::ServerContext::new(endpoint);
            match server_context {
                Ok(_) => {}
                Err(_) => { return Err(()); }
            }
            let server_context = server_context.unwrap();
            let context = Box::new(vr_actuators_cli::conn::mock::MockContext::new());
            let connection= context.connection();
            match connection {
                Ok(_) => {}
                Err(_) => {return Err(());}
            }
            let connection = Box::new(connection.unwrap());
            let server = vr_actuators_cli::network::server::Server::new(&server_context,connection);
            match &server {
                Ok(_) => {}
                Err(err) => {
                    log::error!("Failed to initialize server: {}", err);
                    return Err(());
                }
            }
            let mut server = server.unwrap();

            let endpoint = server.get_last_endpoint();
            log::info!("Server connected to: {}", endpoint);
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

            log::info!("Starting client ...");

            let client = vr_actuators_cli::network::client::Client::new(endpoint);
            assert!(client.is_ok());
            let mut client = client.unwrap();

            let mut client_commands = client_commands;
            client_commands.push(String::from(r#"{ "Stop": {} }"#));

            log::info!("Running client commands:");
            for command in &client_commands {
                log::info!("{}", command);
            }

            for command in &client_commands {
                let command_stream = serde_json::Deserializer::from_str(command.as_str()).into_iter::<vr_actuators_cli::network::common::CommandMessage>();
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

#[test]
fn nop_serve_to_client() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(2000, vec![])
}

#[test]
fn system_reset() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "SystemReset": { } }"#),
    ])
}

#[test]
fn connect_to_fabric() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "AddFabric": { "fabric_name": "Obid Feig LRM2500-B" } }"#),
    ])
}

#[test]
fn set_the_power_level() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "SetRadioFreqPower": { "power_level": 4 } }"#),
        String::from(r#"{ "SystemReset": { } }"#),
    ])
}

#[test]
fn set_the_power_level_low_power() -> Result<(), Box<dyn std::error::Error>> {
    connect_client_to_server(5000, vec![
        String::from(r#"{ "SetRadioFreqPower": { "power_level": 0 } }"#),
        String::from(r#"{ "SystemReset": { } }"#),
    ])
}

#[test]
fn e2e_pulsing_after_antenna_reset() -> Result<(), Box<dyn std::error::Error>> {
    // Reset the conditions of the antenna
    connect_client_to_server(5000, vec![
        String::from(r#"{ "SetRadioFreqPower": { "power_level": 2 } }"#),
        String::from(r#"{ "SystemReset": { } }"#),
    ])?;

    // Allow the antenna time to come back online
    std::thread::sleep(std::time::Duration::from_secs(5));

    // Run the end to end pulsing routine
    e2e_pulsing()
}

#[test]
fn e2e_pulsing() -> Result<(), Box<dyn std::error::Error>> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = vr_actuators_cli::vrp::vrp::OpModeBlock {
        act_cnt32: 0x02, // ceil(36.0 / 32.0) = 2
        act_mode: 0x00,
        op_mode: 0x02,
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let hf_duty_cycle = 30;
    let hf_period = 150;
    let actuator_mode_blocks = vr_actuators_cli::vrp::vrp::ActuatorModeBlocks {
        block0_31: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x0F, // Enable all 8 acutators
            b1: 0x00, // Enable all 8 acutators
            b2: 0x00, // Enable all 8 acutators
            b3: 0x00, // Enable all 8 acutators
        }),
        block32_63: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00, // Enable last 4 actuators
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block64_95: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block96_127: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    let timer_mode_blocks = vr_actuators_cli::vrp::vrp::TimerModeBlocks {
       /*
        * In single pulse operation, the variable t_pulse[ms] (16 bits) determines the time the pulse will remain on.
        * The high frequency signal (carrier) timing is given by block 6 (0x18).
        *
        * In gestures, t_pulse[ms] (16 bits) and t_pause[ms] (16 bits) control the the on and pause timing
        */
        single_pulse_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),

        /*
         * This block gives the timing condition for the high frequency signal option used in single pulse or continuous mode.
         * It is given by the period T_high(ms) [16 bit] and duty cycle ton_high(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
        */
        hf_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: ((hf_duty_cycle & 0x00FF)) as u8,
            b1: ((hf_duty_cycle & 0xFF00) >> 8) as u8,
            b2: ((hf_period & 0x00FF)) as u8,
            b3: ((hf_period & 0xFF00) >> 8) as u8,
        }),

        /*
         * In continuous mode, there is an option for continuous pulsed mode. This block gives the timing condition for the low frequency signal.
         * It is given by the period T_low(ms) [16 bit] and duty cycle ton_low(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        lf_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: 0xFF, // ((4000 & 0x00FF)) as u8,
            b1: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
            b2: 0xFF, // ((4000 & 0x00FF)) as u8,
            b3: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
        })
    };
    let timer_mode_blocks= serde_json::to_string(&timer_mode_blocks)?;


    connect_client_to_server(150000, vec![
        String::from(r#"{ "AddFabric": { "fabric_name": "Jacob's Test Actuator Block of 36" } }"#),
        String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_blocks": {} }} }}"#, op_mode_block, actuator_mode_blocks, timer_mode_blocks)),
        // String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {} }} }}"#, op_mode_block)),
    ])?;

    Ok(())
}

#[test]
fn send_all_off() -> Result<(), Box<dyn std::error::Error>> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = vr_actuators_cli::vrp::vrp::OpModeBlock {
        act_cnt32: 0x02, // ceil(36.0 / 32.0) = 2
        act_mode: 0x00,
        op_mode: 0x00,
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let hf_duty_cycle = 30;
    let hf_period = 150;
    let actuator_mode_blocks = vr_actuators_cli::vrp::vrp::ActuatorModeBlocks {
        block0_31: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00, // Enable all 8 acutators
            b1: 0x00, // Enable all 8 acutators
            b2: 0x00, // Enable all 8 acutators
            b3: 0x00, // Enable all 8 acutators
        }),
        block32_63: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00, // Enable last 4 actuators
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block64_95: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block96_127: Some(vr_actuators_cli::vrp::vrp::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    let timer_mode_blocks = vr_actuators_cli::vrp::vrp::TimerModeBlocks {
       /*
        * In single pulse operation, the variable t_pulse[ms] (16 bits) determines the time the pulse will remain on.
        * The high frequency signal (carrier) timing is given by block 6 (0x18).
        *
        * In gestures, t_pulse[ms] (16 bits) and t_pause[ms] (16 bits) control the the on and pause timing
        */
        single_pulse_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),

        /*
         * This block gives the timing condition for the high frequency signal option used in single pulse or continuous mode.
         * It is given by the period T_high(ms) [16 bit] and duty cycle ton_high(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
        */
        hf_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: ((hf_duty_cycle & 0x00FF)) as u8,
            b1: ((hf_duty_cycle & 0xFF00) >> 8) as u8,
            b2: ((hf_period & 0x00FF)) as u8,
            b3: ((hf_period & 0xFF00) >> 8) as u8,
        }),

        /*
         * In continuous mode, there is an option for continuous pulsed mode. This block gives the timing condition for the low frequency signal.
         * It is given by the period T_low(ms) [16 bit] and duty cycle ton_low(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        lf_block: Some(vr_actuators_cli::vrp::vrp::TimerModeBlock {
            b0: 0xFF, // ((4000 & 0x00FF)) as u8,
            b1: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
            b2: 0xFF, // ((4000 & 0x00FF)) as u8,
            b3: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
        })
    };
    let timer_mode_blocks= serde_json::to_string(&timer_mode_blocks)?;


    connect_client_to_server(150000, vec![
        String::from(r#"{ "AddFabric": { "fabric_name": "Jacob's Test Actuator Block of 36" } }"#),
        String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_blocks": {} }} }}"#, op_mode_block, actuator_mode_blocks, timer_mode_blocks)),
        // String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {} }} }}"#, op_mode_block)),
    ])?;

    Ok(())
}