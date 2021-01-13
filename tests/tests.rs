use protocol_host_lib::error::*;

mod common;

use common::*;

#[test]
fn nop_serve_to_client() -> Result<()> {
    connect_client_to_server(2000, vec![])
}

#[test]
fn system_reset() -> Result<()> {
    #[allow(unused_variables)]
    let timeout = 10000;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 10000;
    connect_client_to_server(timeout, vec![String::from(r#"{ "SystemReset": { } }"#)])
}

#[test]
fn connect_to_fabric() -> Result<()> {
    #[allow(unused_variables)]
    let timeout = 1000;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 10000;
    connect_client_to_server(
        timeout,
        vec![String::from(
            r#"{ "AddFabric": { "fabric_name": "Obid Feig LRM2500-B" } }"#,
        )],
    )
}

#[test]
fn set_the_power_level() -> Result<()> {
    #[allow(unused_variables)]
    let timeout = 500;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 10000;
    connect_client_to_server(
        timeout,
        vec![
            String::from(r#"{ "SetRadioFreqPower": { "power_level": 4 } }"#),
            String::from(r#"{ "SystemReset": { } }"#),
        ],
    )
}

#[test]
fn set_the_power_level_low_power() -> Result<()> {
    #[allow(unused_variables)]
    let timeout = 500;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 10000;
    connect_client_to_server(
        timeout,
        vec![
            String::from(r#"{ "SetRadioFreqPower": { "power_level": 0 } }"#),
            String::from(r#"{ "SystemReset": { } }"#),
        ],
    )
}

#[test]
fn e2e_pulsing_after_antenna_reset() -> Result<()> {
    #[allow(unused_variables)]
    let timeout = 500;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 10000;
    // Reset the conditions of the antenna
    connect_client_to_server(
        timeout,
        vec![
            String::from(r#"{ "SetRadioFreqPower": { "power_level": 2 } }"#),
            String::from(r#"{ "SystemReset": { } }"#),
        ],
    )?;

    // Run the end to end pulsing routine
    activation_example()
}

#[test]
fn activation_example() -> Result<()> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = protocol_host_lib::protocol::haptic::v0::OpModeBlock {
        act_cnt8: 0x05, // ceil(36.0 / 8) = 5
        cmd_op: 0x03,
        command: 0x02,
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let actuator_mode_blocks = protocol_host_lib::protocol::haptic::v0::ActuatorModeBlocks {
        block0_31: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x11, // Enable all 2 acutators
            b1: 0x22, // Enable all 2 acutators
            b2: 0x44, // Enable all 2 acutators
            b3: 0x88, // Enable all 2 acutators
        }),
        block32_63: Some(Default::default()),
        block64_95: Some(Default::default()),
        block96_127: Some(Default::default()),
        block128_159: Some(Default::default()),
        block160_191: Some(Default::default()),
        block192_223: Some(Default::default()),
        block224_255: Some(Default::default()),
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    // 50% duty cycle on 100ms pulsing
    let mut timer_mode_block: Option<protocol_host_lib::protocol::haptic::v0::TimerModeBlock> =
        Some(Default::default());
    let mut timer_mode_block = timer_mode_block.as_mut().unwrap();
    timer_mode_block.ton_high = 0x0fff & 50; // 50 ms in 12 bits of a u16
    timer_mode_block.tperiod_high = 0x0fff & 100; // 100 ms in 12 bits of a u16

    let timer_mode_block = serde_json::to_string(&timer_mode_block)?;

    #[allow(unused_variables)]
    let timeout = 500;
    #[cfg(any(feature = "usb", feature = "ethernet"))]
    let timeout = 30000;
    connect_client_to_server(
        timeout,
        vec![
            String::from(
                r#"{ "AddFabric": { "fabric_name": "Jacob's Test Actuator Block of 36" } }"#,
            ),
            String::from(format!(
                r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_block": {} }} }}"#,
                op_mode_block, actuator_mode_blocks, timer_mode_block
            )),
        ],
    )
}
