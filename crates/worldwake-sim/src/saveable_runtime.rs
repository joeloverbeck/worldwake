use crate::save_load::SaveError;

/// Optional persistence surface for autonomous controller runtimes.
pub trait SaveableRuntime {
    fn save_runtime_state(&self) -> Result<Vec<u8>, SaveError>;
}
