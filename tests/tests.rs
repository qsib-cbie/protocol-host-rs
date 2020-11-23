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
    e2e_pulsing()
}

#[test]
fn e2e_pulsing() -> Result<()> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = protocol_host_lib::protocol::haptic::v0::OpModeBlock {
        act_cnt8: 0x02, // ceil(36.0 / 32.0) = 2
        cmd_op: 0x00,
        command: 0x02,
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let hf_duty_cycle = 30;
    let hf_period = 150;
    let actuator_mode_blocks = protocol_host_lib::protocol::haptic::v0::ActuatorModeBlocks {
        block0_31: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x0F, // Enable all 8 acutators
            b1: 0x00, // Enable all 8 acutators
            b2: 0x00, // Enable all 8 acutators
            b3: 0x00, // Enable all 8 acutators
        }),
        block32_63: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00, // Enable last 4 actuators
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block64_95: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block96_127: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    let timer_mode_blocks = protocol_host_lib::protocol::haptic::v0::TimerModeBlocks {
        /*
         * In single pulse operation, the variable t_pulse[ms] (16 bits) determines the time the pulse will remain on.
         * The high frequency signal (carrier) timing is given by block 6 (0x18).
         *
         * In gestures, t_pulse[ms] (16 bits) and t_pause[ms] (16 bits) control the the on and pause timing
         */
        single_pulse_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
        }),

        /*
         * This block gives the timing condition for the high frequency signal option used in single pulse or continuous mode.
         * It is given by the period T_high(ms) [16 bit] and duty cycle ton_high(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        hf_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: (hf_duty_cycle & 0x00FF) as u8,
            b1: ((hf_duty_cycle & 0xFF00) >> 8) as u8,
            b2: (hf_period & 0x00FF) as u8,
        }),

        /*
         * In continuous mode, there is an option for continuous pulsed mode. This block gives the timing condition for the low frequency signal.
         * It is given by the period T_low(ms) [16 bit] and duty cycle ton_low(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        lf_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: 0xFF, // ((4000 & 0x00FF)) as u8,
            b1: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
            b2: 0xFF, // ((4000 & 0x00FF)) as u8,
        }),
    };
    let timer_mode_blocks = serde_json::to_string(&timer_mode_blocks)?;

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
                r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_blocks": {} }} }}"#,
                op_mode_block, actuator_mode_blocks, timer_mode_blocks
            )),
            // String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {} }} }}"#, op_mode_block)),
        ],
    )
}

#[test]
fn send_all_off() -> Result<()> {
    // Issues commands to enable all actuators in the single fabric of 36 in continuous 50 ms on 50 ms off
    let op_mode_block = protocol_host_lib::protocol::haptic::v0::OpModeBlock {
        act_cnt8: 0x02, // ceil(36.0 / 32.0) = 2
        cmd_op: 0x00,
        command: 0x00,
    };
    let op_mode_block = serde_json::to_string(&op_mode_block)?;

    let hf_duty_cycle = 30;
    let hf_period = 150;
    let actuator_mode_blocks = protocol_host_lib::protocol::haptic::v0::ActuatorModeBlocks {
        block0_31: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00, // Enable all 8 acutators
            b1: 0x00, // Enable all 8 acutators
            b2: 0x00, // Enable all 8 acutators
            b3: 0x00, // Enable all 8 acutators
        }),
        block32_63: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00, // Enable last 4 actuators
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block64_95: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
        block96_127: Some(protocol_host_lib::protocol::haptic::v0::ActuatorModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
            b3: 0x00,
        }),
    };
    let actuator_mode_blocks = serde_json::to_string(&actuator_mode_blocks)?;

    let timer_mode_blocks = protocol_host_lib::protocol::haptic::v0::TimerModeBlocks {
        /*
         * In single pulse operation, the variable t_pulse[ms] (16 bits) determines the time the pulse will remain on.
         * The high frequency signal (carrier) timing is given by block 6 (0x18).
         *
         * In gestures, t_pulse[ms] (16 bits) and t_pause[ms] (16 bits) control the the on and pause timing
         */
        single_pulse_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: 0x00,
            b1: 0x00,
            b2: 0x00,
        }),

        /*
         * This block gives the timing condition for the high frequency signal option used in single pulse or continuous mode.
         * It is given by the period T_high(ms) [16 bit] and duty cycle ton_high(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        hf_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: (hf_duty_cycle & 0x00FF) as u8,
            b1: ((hf_duty_cycle & 0xFF00) >> 8) as u8,
            b2: (hf_period & 0x00FF) as u8,
        }),

        /*
         * In continuous mode, there is an option for continuous pulsed mode. This block gives the timing condition for the low frequency signal.
         * It is given by the period T_low(ms) [16 bit] and duty cycle ton_low(ms) [16 bits].
         * The duty cycle is given in time on instead of % of period to avoid calculations in the microcontroller.
         * If ton_high is equal to the T_high, this high frequency signal is overridden by software in the microcontroller.
         */
        lf_block: Some(protocol_host_lib::protocol::haptic::v0::TimerModeBlock {
            b0: 0xFF, // ((4000 & 0x00FF)) as u8,
            b1: 0xFF, // ((4000 & 0xFF00) >> 8) as u8,
            b2: 0xFF, // ((4000 & 0x00FF)) as u8,
        }),
    };
    let timer_mode_blocks = serde_json::to_string(&timer_mode_blocks)?;

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
                r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {}, "actuator_mode_blocks": {}, "timer_mode_blocks": {} }} }}"#,
                op_mode_block, actuator_mode_blocks, timer_mode_blocks
            )),
            // String::from(format!(r#"{{ "ActuatorsCommand": {{  "fabric_name": "Jacob's Test Actuator Block of 36", "op_mode_block": {} }} }}"#, op_mode_block)),
        ],
    )
}
