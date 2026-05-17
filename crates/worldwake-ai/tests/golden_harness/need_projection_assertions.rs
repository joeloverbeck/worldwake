//! Reusable assertions for need-horizon (S126) goldens.
//!
//! These helpers compose over `IntentionFrame`, `DiscrepancyMemory`, and
//! `BlockerScope` so future assumption-driven goldens can read them at the
//! intended proof surface (`FrameAssumption` presence, typed `Discrepancy`
//! recording, suppression status) without duplicating destructuring boilerplate.

use worldwake_core::{
    BlockerScope, Discrepancy, DiscrepancyClearing, DiscrepancyMemory, EntityId, FrameAssumption,
    HomeostaticNeedId, IntentionFrame, Tick, World,
};

/// Returns `true` when the agent's active `IntentionFrame` contains a
/// `NeedSafeUntilTick` assumption for the named need.
#[must_use]
pub fn frame_contains_need_safe_until_tick(
    frame: &IntentionFrame,
    need: HomeostaticNeedId,
) -> Option<FrameAssumption> {
    frame
        .assumptions
        .iter()
        .find(|assumption| {
            matches!(
                assumption,
                FrameAssumption::NeedSafeUntilTick { need: a_need, .. } if *a_need == need,
            )
        })
        .copied()
}

/// Returns the first `DiscrepancyEntry` in `DiscrepancyMemory` whose payload
/// is `Discrepancy::NeedHorizonExceeded` for the named need, paired with its
/// `BlockerScope` and `expires_tick` for downstream TTL assertions.
#[must_use]
pub fn first_need_horizon_entry(
    world: &World,
    agent: EntityId,
    need: HomeostaticNeedId,
) -> Option<(BlockerScope, Tick, DiscrepancyClearing)> {
    let memory = world.get_component_discrepancy_memory(agent)?;
    memory.entries.iter().find_map(|(key, entry)| {
        if matches!(
            entry.discrepancy,
            Discrepancy::NeedHorizonExceeded { need: e_need, .. } if e_need == need,
        ) {
            Some((*key, entry.expires_tick, entry.clearing_condition))
        } else {
            None
        }
    })
}

/// Returns `true` when `DiscrepancyMemory` would suppress the supplied
/// `BlockerScope` at `current_tick`. Returns `false` if the memory component
/// is absent.
#[must_use]
pub fn blocker_is_suppressed(
    world: &World,
    agent: EntityId,
    key: &BlockerScope,
    current_tick: Tick,
) -> bool {
    world
        .get_component_discrepancy_memory(agent)
        .is_some_and(|memory: &DiscrepancyMemory| memory.is_suppressed(key, current_tick))
}
