//! Layer 0 golden for the S126 need-projection chain (ticket S126NEEPROTIM-004).
//!
//! Proves the end-to-end behavioral contract authored by spec
//! `specs/S126-need-projection-time-budget.md` D1–D8: an agent committing to a
//! plan whose completion tick exceeds its projected hunger-high crossing
//! recognises the breach via `populate_assumptions` →
//! `evaluate_assumptions` → `record_assumption_failure`, lands a typed
//! `Discrepancy::NeedHorizonExceeded` entry in `DiscrepancyMemory` with
//! `DiscrepancyClearing::TtlExpiry`, suppresses the original goal's
//! `BlockerKey` for `structural_block_ticks`, executes a different planned
//! goal under the suppression window, and clears the suppression at TTL
//! expiry.
//!
//! Scenario authority: `scenarios/survival-need-projection.ron`. The scenario
//! tightens `MetabolismProfile.hunger_rate` to 30/tick so the agent's
//! `hunger=600` projects breach against the default `DriveThresholds::hunger
//! .high()=750` in 5 ticks, well inside the only known apple-acquisition
//! plan (5-tick travel to Distant Orchard + 3-tick Harvest = ~8 ticks).
//! `cognitive_profile.structural_block_ticks=30` keeps the TTL-expiry phase
//! of the test short.
//!
//! Note on frame vs plan goals: when the `AcquireCommodity` goal is interrupted
//! by a higher-priority survival goal (e.g., consuming an acquired apple, or
//! sleeping), the existing intention frame transitions to
//! `FrameState::Suspended { reason: PriorityInterrupt }` and retains its
//! original `goal` field while the agent's runtime executes a different
//! `current_plan`. The test therefore checks the agent's executing
//! `runtime.current_plan.goal` for alternative-goal adoption rather than
//! `frame.goal`.

mod golden_harness;

use std::path::PathBuf;

use golden_harness::{
    GoldenHarness, blocker_is_suppressed, first_need_horizon_entry,
    frame_contains_need_safe_until_tick, seed_actor_world_beliefs,
};
use worldwake_ai::CommodityPurpose;
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    AcquisitionQuantity, BlockerKey, CommodityKind, DiscrepancyClearing, EntityId, FrameAssumption,
    GoalKey, GoalKind, HomeostaticNeedId, PerceptionSource, Tick,
};

const STRUCTURAL_BLOCK_TICKS: u32 = 30;
const TICK_BUDGET: u32 = STRUCTURAL_BLOCK_TICKS + 30;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-need-projection.ron")
}

fn load_harness() -> (GoldenHarness, EntityId) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("survival-need-projection scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival-need-projection scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness.enable_action_tracing();

    let agent = harness
        .world
        .query_name_and_agent_data()
        .find_map(|(entity, name, _)| (name.0 == "Surveyor").then_some(entity))
        .expect("scenario should spawn 'Surveyor' agent");

    // Seed the agent's beliefs about every non-self entity so the planner
    // immediately sees the remote orchard's apple source. The scenario
    // disables curiosity-driven exploration to keep ranking deterministic
    // after suppression, so the agent would otherwise never discover the
    // orchard within the test budget.
    seed_actor_world_beliefs(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        Tick(0),
        PerceptionSource::Inference,
    );

    (harness, agent)
}

fn original_apple_goal_key() -> GoalKey {
    GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Apple,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    })
}

#[test]
#[ignore = "CI-only: scenario-load golden; run via golden-survival workflow"]
fn golden_need_projection_chain() {
    let (mut h, agent) = load_harness();
    let original_goal_key = original_apple_goal_key();

    let mut saw_assumption_at: Option<(Tick, FrameAssumption)> = None;
    let mut saw_discrepancy: Option<(Tick, BlockerKey, Tick, DiscrepancyClearing)> = None;
    // Suppression must hold for the recorded BlockerKey at the discrepancy tick.
    let mut suppressed_at_disc_tick: Option<bool> = None;
    // The agent's runtime executing plan picks at least one goal whose key is
    // not the suppressed AcquireCommodity goal during the suppression window.
    let mut saw_alternative_plan: Option<(Tick, GoalKey)> = None;
    // After expires_tick, is_suppressed flips false for the same BlockerKey.
    let mut saw_ttl_expiry_at: Option<Tick> = None;

    for tick_num in 0..TICK_BUDGET {
        h.step_once();
        let current_tick = Tick(u64::from(tick_num));

        // 1. populate_assumptions adds a NeedSafeUntilTick assumption for hunger
        //    once the agent commits to the multi-step apple plan.
        if saw_assumption_at.is_none()
            && let Some(frame) = h.world.get_component_intention_frame(agent)
            && let Some(assumption) =
                frame_contains_need_safe_until_tick(frame, HomeostaticNeedId::Hunger)
        {
            saw_assumption_at = Some((current_tick, assumption));
        }

        // 2. record_assumption_failure lands the typed discrepancy entry once
        //    evaluate_assumptions returns CriticalFailure. Capture suppression
        //    status at this exact tick — later TTL expiry would otherwise
        //    remove the entry before a final assertion runs.
        if saw_discrepancy.is_none()
            && let Some((blocker, expires_tick, clearing)) =
                first_need_horizon_entry(&h.world, agent, HomeostaticNeedId::Hunger)
        {
            saw_discrepancy = Some((current_tick, blocker, expires_tick, clearing));
            suppressed_at_disc_tick = Some(blocker_is_suppressed(
                &h.world,
                agent,
                &blocker,
                current_tick,
            ));
        }

        // 3. After the discrepancy is recorded, the agent's executing plan
        //    eventually adopts a different goal (the original AcquireCommodity
        //    frame may persist as `Suspended { reason: PriorityInterrupt }` —
        //    the agent's actual behaviour is the active `current_plan`, not
        //    the suspended frame's goal).
        if saw_alternative_plan.is_none()
            && let Some((disc_tick, suppressed_key, _expires, _clearing)) = saw_discrepancy
            && current_tick > disc_tick
            && let Some(plan) = h
                .driver
                .runtime(agent)
                .and_then(|runtime| runtime.current_plan.as_ref())
            && plan.goal != suppressed_key.goal_key
        {
            saw_alternative_plan = Some((current_tick, plan.goal));
        }

        // 4. After TTL expires, is_suppressed flips to false for the BlockerKey.
        if saw_ttl_expiry_at.is_none()
            && let Some((_disc_tick, suppressed_key, expires_tick, _clearing)) = saw_discrepancy
            && current_tick >= expires_tick
            && !blocker_is_suppressed(&h.world, agent, &suppressed_key, current_tick)
        {
            saw_ttl_expiry_at = Some(current_tick);
        }

        if saw_assumption_at.is_some()
            && saw_discrepancy.is_some()
            && saw_alternative_plan.is_some()
            && saw_ttl_expiry_at.is_some()
        {
            break;
        }
    }

    // Milestone 1: NeedSafeUntilTick(Hunger) was populated into the active frame.
    let (assumption_tick, assumption) = saw_assumption_at.unwrap_or_else(|| {
        panic!(
            "S126: populate_assumptions must add NeedSafeUntilTick(Hunger) to the active frame \
             when the agent commits to a multi-step plan whose completion tick exceeds the \
             projected hunger-high crossing"
        )
    });
    let FrameAssumption::NeedSafeUntilTick { need, until_tick } = assumption else {
        panic!("expected NeedSafeUntilTick variant, got {assumption:?}");
    };
    assert_eq!(need, HomeostaticNeedId::Hunger);
    assert!(
        until_tick > assumption_tick,
        "S126: NeedSafeUntilTick.until_tick must exceed the tick on which the assumption was \
         populated (saw until_tick={until_tick:?}, populated at tick={assumption_tick:?})"
    );

    // Milestone 2: Discrepancy::NeedHorizonExceeded(Hunger) was recorded with TtlExpiry.
    let (disc_tick, suppressed_key, expires_tick, clearing) =
        saw_discrepancy.unwrap_or_else(|| {
            panic!(
                "S126: evaluate_assumptions + record_assumption_failure must land \
             Discrepancy::NeedHorizonExceeded for hunger in DiscrepancyMemory once the \
             projection collapses below the assumption's until_tick"
            )
        });
    assert!(
        disc_tick >= assumption_tick,
        "S126: discrepancy must be recorded on or after the assumption is populated"
    );
    assert_eq!(
        clearing,
        DiscrepancyClearing::TtlExpiry,
        "S126: NeedHorizonExceeded must clear via TTL expiry only"
    );
    assert_eq!(
        expires_tick,
        Tick(disc_tick.0 + u64::from(STRUCTURAL_BLOCK_TICKS)),
        "S126: discrepancy expires_tick must equal observed_tick + structural_block_ticks \
         (observed at {disc_tick:?}, structural_block_ticks={STRUCTURAL_BLOCK_TICKS}, \
         saw expires_tick={expires_tick:?})"
    );
    assert_eq!(
        suppressed_key.goal_key, original_goal_key,
        "S126: the suppressed BlockerKey must carry the original AcquireCommodity(Apple, \
         SelfConsume) goal that was committed before the breach"
    );

    // Milestone 3 (suppression status at the recording tick): captured inline
    // before TTL could prune the entry.
    assert_eq!(
        suppressed_at_disc_tick,
        Some(true),
        "S126: DiscrepancyMemory::is_suppressed must return true for the recorded BlockerKey at \
         the same tick the discrepancy is observed"
    );

    // Milestone 4: A different plan goal eventually executes during the
    // suppression window, proving horizon-aware planning hands control to a
    // non-blocked alternative rather than busy-looping the suppressed goal.
    let (alt_tick, alt_goal) = saw_alternative_plan.unwrap_or_else(|| {
        panic!(
            "S126: while the original goal is suppressed via NeedHorizonExceeded, the agent's \
             executing plan must eventually adopt a goal whose key differs from the suppressed \
             goal_key (the spec's `harvest_before_sleep` motivating-evidence pattern: a \
             different need-satisfying goal wins ranking when the long-completing path is \
             blocked)"
        )
    });
    assert!(alt_tick > disc_tick);
    assert_ne!(
        alt_goal, suppressed_key.goal_key,
        "S126: post-suppression executing plan goal must NOT equal the suppressed goal_key (saw \
         {alt_goal:?}, suppressed={:?})",
        suppressed_key.goal_key
    );

    // Milestone 5: TTL expiry releases the suppression.
    let ttl_tick = saw_ttl_expiry_at.unwrap_or_else(|| {
        panic!(
            "S126: after structural_block_ticks={STRUCTURAL_BLOCK_TICKS} ticks the TTL must \
             expire and DiscrepancyMemory::is_suppressed must return false for the previously \
             suppressed BlockerKey"
        )
    });
    assert!(
        ttl_tick >= expires_tick,
        "S126: TTL expiry must occur on or after expires_tick={expires_tick:?} (saw \
         {ttl_tick:?})"
    );
}
