// Clippy lints scoped to genuine domain shapes — the protocol enums are
// large by design, node constructors take many fields by design. Drop
// `dead_code` from the crate-wide allow set; narrow to specific items.
#![allow(
    clippy::large_enum_variant,
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::question_mark
)]
//! migration-runtime — extracted from `android-sender`.

pub mod frame_pair;
pub mod media_bridge;
pub mod messages;
pub mod node_manager;
pub mod nodes;
pub mod protocol;
pub mod runtime;
pub mod whep_signaller_compat;

pub use frame_pair::FramePair;
pub use protocol::{
    Command, CommandResult, ControlMode, ControlPoint, DestinationFamily, DestinationInfo,
    MixerInfo, MixerSlotInfo, NodeInfo, ServerMessage, SourceInfo, State,
};
