use crate::error::*;
use crate::protocol::haptic;
use core::fmt::Debug;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum CommandMessage {
    Failure {
        message: String,
    },
    Success {},

    Stop {},

    SystemReset {},
    SetRadioFreqPower {
        power_level: u8,
    },
    CustomCommand {
        control_byte: u8,
        data: String,
        device_required: bool,
    },

    RfFieldState {
        state: u8,
    },

    AddFabric {
        fabric_name: String,
    },
    RemoveFabric {
        fabric_name: String,
    },
    ActuatorsCommand {
        fabric_name: String,
        timer_mode_block: Option<haptic::v0::TimerModeBlocks>,
        actuator_mode_blocks: Option<haptic::v0::ActuatorModeBlocks>,
        op_mode_block: Option<haptic::v0::OpModeBlock>,
        use_cache: Option<bool>,
    },
}

pub trait Protocol<'a> {
    fn handle_message(self: &mut Self, message: &CommandMessage) -> Result<()>;
}

pub trait Fabric {
    fn name(self: &Self) -> String;
    fn identifier(self: &Self) -> Result<std::vec::Vec<u8>>;
}

impl Debug for dyn Fabric {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Fabric({},{:?})", self.name(), self.identifier())
    }
}
