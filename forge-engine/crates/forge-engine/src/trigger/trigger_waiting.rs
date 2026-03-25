use crate::event::{RunParams, TriggerType};

/// Mirrors Java's TriggerWaiting data object.
#[derive(Debug, Clone)]
pub struct TriggerWaiting {
    pub mode: TriggerType,
    pub params: RunParams,
}
