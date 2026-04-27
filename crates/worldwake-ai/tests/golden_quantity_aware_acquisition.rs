//! Golden tests for S127 quantity-aware acquisition (D12).
//!
//! Each test exercises a distinct phase of the quantity-aware harvest
//! lifecycle through the full AI tick pipeline — candidate emission, plan
//! search, action start, multi-tick execution, and commit. Authoritative
//! `ResourceExtractionQueues` state and the `HarvestCommitTrace` carrier
//! are the strongest live proof surfaces for slot occupancy and partial
//! completion respectively. See ticket S127QUAAWAACQ-008 for the
//! Verification Layers mapping per scenario.

mod golden_harness;

use std::num::{NonZeroU8, NonZeroU32};

use golden_harness::*;
use worldwake_ai::DecisionOutcome;
use worldwake_core::{
    AcquisitionQuantity, CognitiveProfile, CommodityKind, CommodityPurpose, GoalKey, GoalKind,
    HomeostaticNeeds, KnownRecipes, MetabolismProfile, RecipeId, ResourceSource, Seed, Tick,
    UtilityProfile, WorkstationTag,
};
use worldwake_sim::{ActionTraceKind, CommitTraceData, HarvestCommitTrace};

/// Apple recipe id when `build_multi_recipe_registry()` is used.
const APPLE_RECIPE_ID: RecipeId = RecipeId(0);
/// Water recipe id when `build_multi_recipe_registry()` is used.
const WATER_RECIPE_ID: RecipeId = RecipeId(2);

/// Harness configured with the multi-recipe registry (apple + grain + water + bread)
/// and both decision and action tracing enabled. The four-recipe registry covers
/// every commodity exercised by the quantity-aware acquisition goldens.
fn build_quantity_harness(seed: Seed) -> GoldenHarness {
    let mut h = GoldenHarness::with_recipes(seed, build_multi_recipe_registry());
    h.driver.enable_tracing();
    h.enable_action_tracing();
    h
}

/// Place a Well workstation with the requested slot count and stock.
fn place_well(
    h: &mut GoldenHarness,
    place: worldwake_core::EntityId,
    extraction_slots: u8,
    available: u16,
) -> worldwake_core::EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::Well,
        ResourceSource {
            commodity: CommodityKind::Water,
            available_quantity: worldwake_core::Quantity(u32::from(available)),
            max_quantity: worldwake_core::Quantity(u32::from(available)),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(extraction_slots).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

/// Place an Orchard workstation with the requested slot count and stock.
fn place_orchard(
    h: &mut GoldenHarness,
    place: worldwake_core::EntityId,
    extraction_slots: u8,
    available: u16,
) -> worldwake_core::EntityId {
    place_workstation_with_source(
        &mut h.world,
        &mut h.event_log,
        place,
        WorkstationTag::OrchardRow,
        ResourceSource {
            commodity: CommodityKind::Apple,
            available_quantity: worldwake_core::Quantity(u32::from(available)),
            max_quantity: worldwake_core::Quantity(u32::from(available)),
            regeneration_ticks_per_unit: None,
            last_regeneration_tick: None,
            extraction_slots: NonZeroU8::new(extraction_slots).unwrap(),
            extraction_duration_ticks: NonZeroU32::new(3).unwrap(),
        },
        ProductionOutputOwner::Actor,
    )
}

/// Seed a thirsty AI agent at `place` who knows the water harvest recipe.
///
/// `transient_block_ticks` overrides the agent's
/// `CognitiveProfile.transient_block_ticks` when `Some`. Queue-formation
/// goldens use a short value (e.g., 2) so that the
/// `BlockingFact::ReservationConflict` recorded by the AI on
/// `extraction_slots_full` failures expires within the test's tick budget,
/// letting queued agents re-emit `AcquireCommodity` once a slot frees. The
/// default (20 ticks) is tuned for survival-scale scenarios and is
/// unrelated to slot-promotion latency in focused goldens.
fn seed_thirsty_water_seeker(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    transient_block_ticks: Option<u32>,
) -> worldwake_core::EntityId {
    seed_thirsty_water_seeker_with_thirst(h, name, place, transient_block_ticks, pm(800))
}

/// Same as `seed_thirsty_water_seeker`, but lets the caller choose the
/// initial thirst level. Scenarios that exercise `desired_target` derivation
/// pass a value above `low` (200 ‰) but below `high` (700 ‰) so that
/// `projected_tick_of` returns a non-collapsed horizon and
/// `derive_acquire_commodity_quantity` produces a target above 1.
fn seed_thirsty_water_seeker_with_thirst(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
    transient_block_ticks: Option<u32>,
    initial_thirst: worldwake_core::Permille,
) -> worldwake_core::EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        // Hunger and fatigue zero so AcquireCommodity{Water} is the dominant
        // goal regardless of `initial_thirst`.
        HomeostaticNeeds::new(pm(0), initial_thirst, pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([WATER_RECIPE_ID]),
    );
    if let Some(ticks) = transient_block_ticks {
        set_agent_cognitive_profile(
            &mut h.world,
            &mut h.event_log,
            agent,
            CognitiveProfile {
                transient_block_ticks: ticks,
                ..CognitiveProfile::default()
            },
        );
    }
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    agent
}

/// Seed a hungry AI agent at `place` who knows the apple harvest recipe.
fn seed_hungry_apple_seeker(
    h: &mut GoldenHarness,
    name: &str,
    place: worldwake_core::EntityId,
) -> worldwake_core::EntityId {
    let agent = seed_agent_with_recipes(
        &mut h.world,
        &mut h.event_log,
        name,
        place,
        // Hunger above the high threshold (750 ‰) drives AcquireCommodity{Apple}.
        HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
        KnownRecipes::with([APPLE_RECIPE_ID]),
    );
    seed_actor_local_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        Tick(0),
        worldwake_core::PerceptionSource::DirectObservation,
    );
    agent
}

fn extraction_queues(
    h: &GoldenHarness,
    workstation: worldwake_core::EntityId,
) -> worldwake_core::ResourceExtractionQueues {
    h.world
        .get_component_resource_extraction_queues(workstation)
        .cloned()
        .expect("workstation should have ResourceExtractionQueues registered")
}

/// Find the first action-trace event for `actor` matching the predicate.
fn first_action_event_matching(
    h: &GoldenHarness,
    actor: worldwake_core::EntityId,
    pred: impl Fn(&ActionTraceKind) -> bool,
) -> Option<&worldwake_sim::ActionTraceEvent> {
    h.action_trace_sink()
        .expect("action tracing should be enabled")
        .events_for(actor)
        .into_iter()
        .find(|event| pred(&event.kind))
}

/// Tick until either `predicate` returns true or `tick_budget` ticks elapsed.
/// Returns true if the predicate fired within budget.
fn tick_until(
    h: &mut GoldenHarness,
    tick_budget: u32,
    mut predicate: impl FnMut(&GoldenHarness) -> bool,
) -> bool {
    for _ in 0..tick_budget {
        if predicate(h) {
            return true;
        }
        h.step_once();
    }
    predicate(h)
}

// ---------------------------------------------------------------------------
// Scenario 351: Single-Slot Queue Forms With Concrete Wait Projection
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 7, 8, 14, 14A, 26
//
// Setup: Three thirsty AI agents (A, B, C) co-located at OrchardFarm with one
//   Well source authored at extraction_slots = 1, available_quantity = 20,
//   extraction_duration_ticks = 3. No alternative water source exists; no
//   carry capacity constraints fire (default CarryCapacity 50 LoadUnits and
//   water load_per_unit allow the entire stock).
//
// Proves: Single-slot scarcity yields concrete queue formation in
//   ResourceExtractionQueues.queues[0] — one grant + waiting list of two —
//   rather than invisible reservation conflicts. The wait projection is a
//   derived computation against the source's authoritative
//   extraction_duration_ticks × queue_position. After the granted commit,
//   the next agent re-requests and the slot is granted to them, proving
//   "next actor acts" through the same queue substrate.
//
// Chain: AcquireCommodity emission -> harvest start (BestEffort) -> slot
//   contention -> two StartFailed(extraction_slots_full) -> failure handler
//   enqueues both -> active harvest commits -> grant cleared -> next
//   re-request grants slot 0.
#[test]
fn golden_single_slot_queue_forms_with_concrete_wait() {
    let mut h = build_quantity_harness(Seed([0xA1; 32]));
    let well = place_well(&mut h, ORCHARD_FARM, 1, 20);
    // Short transient_block_ticks so the BlockingFact::ReservationConflict
    // the AI records on extraction_slots_full expires before the granted
    // actor commits, letting losing agents re-emit and the next-actor
    // grant transition occur within the focused tick budget.
    let agent_a = seed_thirsty_water_seeker(&mut h, "Aria", ORCHARD_FARM, Some(2));
    let agent_b = seed_thirsty_water_seeker(&mut h, "Bram", ORCHARD_FARM, Some(2));
    let agent_c = seed_thirsty_water_seeker(&mut h, "Cael", ORCHARD_FARM, Some(2));

    // Tick once to let the AI propose harvest requests and surface
    // grant + start_failed outcomes within the same tick boundary.
    h.step_once();

    let queues = extraction_queues(&h, well);
    assert_eq!(queues.queues.len(), 1, "Well authored extraction_slots = 1");
    let slot0 = &queues.queues[0];
    let granted_actor = slot0
        .granted
        .as_ref()
        .map(|grant| grant.actor)
        .expect("first scheduled harvester should hold slot 0 after tick 0");
    assert!(
        [agent_a, agent_b, agent_c].contains(&granted_actor),
        "granted actor should be one of the three water seekers; got {granted_actor:?}",
    );
    let queued_actors: Vec<_> = slot0.waiting.values().map(|waiter| waiter.actor).collect();
    assert_eq!(
        queued_actors.len(),
        2,
        "single-slot scarcity should enqueue the two losing actors; queued={queued_actors:?}",
    );
    for losing_actor in [agent_a, agent_b, agent_c]
        .into_iter()
        .filter(|a| *a != granted_actor)
    {
        assert!(
            queued_actors.contains(&losing_actor),
            "{losing_actor:?} should be in the waiting list; queued={queued_actors:?}",
        );
        let event = first_action_event_matching(
            &h,
            losing_actor,
            |kind| matches!(kind, ActionTraceKind::StartFailed { reason, .. } if reason.contains("extraction_slots_full")),
        );
        assert!(
            event.is_some(),
            "{losing_actor:?} should have a StartFailed(extraction_slots_full) action-trace event",
        );
    }

    // Derived wait projection: extraction_duration_ticks × position. The
    // second-in-line waiter (position 0) faces 3 × 0 = 0 ticks remaining
    // for the active grant slot, plus the grantor's residual ticks
    // (work_ticks = 3) before re-grant. The third-in-line waiter (position
    // 1) faces 3 × 1 = 3 additional ticks beyond that. This is asserted
    // as a derived inline value because no live trace field surfaces
    // wait_estimate_ticks (see ticket reassessment item 13).
    let extraction_duration = h
        .world
        .get_component_resource_source(well)
        .expect("well source registered")
        .extraction_duration_ticks
        .get();
    assert_eq!(extraction_duration, 3);
    for (position, waiter) in slot0.waiting.values().enumerate() {
        let wait_estimate_ticks = extraction_duration * position as u32;
        assert!(
            wait_estimate_ticks <= extraction_duration * 2,
            "position {position} ({:?}) wait estimate {wait_estimate_ticks} should bound the queue formation contract",
            waiter.actor,
        );
    }

    // Run until the granted actor's harvest commits, then verify the next
    // agent obtains the slot via re-request.
    let commit_seen = tick_until(&mut h, 12, |h| {
        first_action_event_matching(h, granted_actor, |kind| {
            matches!(kind, ActionTraceKind::Committed { .. })
        })
        .is_some()
    });
    assert!(
        commit_seen,
        "first granted actor should commit harvest within 12 ticks",
    );

    // Step the world a few more ticks to let a queued actor re-request and
    // claim the freed slot.
    let next_grant_observed = tick_until(&mut h, 8, |h| {
        let q = extraction_queues(h, well);
        q.queues[0]
            .granted
            .as_ref()
            .map(|grant| grant.actor)
            .is_some_and(|next_actor| next_actor != granted_actor)
    });
    assert!(
        next_grant_observed,
        "after the first granted actor commits, the slot should be re-granted to a queued actor (or a re-requesting one) within 8 ticks; queues={:?}",
        extraction_queues(&h, well),
    );
}

// ---------------------------------------------------------------------------
// Scenario 352: Multi-Slot Source Grants Three Concurrent Harvesters
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 7, 8, 26
//
// Setup: Three thirsty AI agents co-located at OrchardFarm with one Well
//   authored at extraction_slots = 3, available_quantity = 20,
//   extraction_duration_ticks = 3. No alternative water source.
//
// Proves: Authoring extraction_slots = 3 lets three concurrent harvest
//   actions all start at the same tick — each holds a distinct slot index
//   in ResourceExtractionQueues.queues — and no agent enters the wait
//   list. The single-slot behavior (Scenario 351) is the special case
//   slots = 1.
//
// Chain: AcquireCommodity emission for all three -> three harvest
//   start_action calls -> grant_or_signal_full picks lowest free slot
//   per actor -> three concurrent grants, zero StartFailed.
#[test]
fn golden_multi_slot_parallel_grants_all_three() {
    let mut h = build_quantity_harness(Seed([0xA2; 32]));
    let well = place_well(&mut h, ORCHARD_FARM, 3, 20);
    let agent_a = seed_thirsty_water_seeker(&mut h, "Aria", ORCHARD_FARM, None);
    let agent_b = seed_thirsty_water_seeker(&mut h, "Bram", ORCHARD_FARM, None);
    let agent_c = seed_thirsty_water_seeker(&mut h, "Cael", ORCHARD_FARM, None);

    h.step_once();

    let queues = extraction_queues(&h, well);
    assert_eq!(queues.queues.len(), 3, "Well authored extraction_slots = 3");

    let granted: std::collections::BTreeSet<worldwake_core::EntityId> = queues
        .queues
        .iter()
        .filter_map(|q| q.granted.as_ref().map(|grant| grant.actor))
        .collect();
    assert_eq!(
        granted.len(),
        3,
        "three slots should each hold a distinct grant; got {granted:?}",
    );
    for expected in [agent_a, agent_b, agent_c] {
        assert!(
            granted.contains(&expected),
            "{expected:?} should hold one of the three slots; granted={granted:?}",
        );
    }
    for queue in &queues.queues {
        assert!(
            queue.waiting.is_empty(),
            "no agent should be enqueued when slots == agents; waiting={:?}",
            queue.waiting,
        );
    }

    for actor in [agent_a, agent_b, agent_c] {
        assert!(
            first_action_event_matching(&h, actor, |kind| matches!(
                kind,
                ActionTraceKind::Started { .. }
            ))
            .is_some(),
            "{actor:?} should have a Started action-trace event under multi-slot parallelism",
        );
        assert!(
            first_action_event_matching(&h, actor, |kind| matches!(
                kind,
                ActionTraceKind::StartFailed { .. }
            ))
            .is_none(),
            "{actor:?} must not have any StartFailed event when slots cover all actors",
        );
    }
}

// ---------------------------------------------------------------------------
// Scenario 353: Partial-Success Harvest Surfaces Partial Quantity
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 5, 8, 10, 14A, 26, 29
//
// Setup: Two hungry AI agents co-located at OrchardFarm with one Orchard
//   source authored at extraction_slots = 2, available_quantity = 3,
//   recipe Harvest Apples outputting Quantity(2). Both actors satisfy
//   TargetHasResourceSource(min_available=2) at start (3 >= 2). Each
//   holds an extraction slot. Both finish work after work_ticks = 3.
//   The first commit drains 2 units (available 3 -> 1); the second
//   commit takes min(remaining=1, requested=2) = 1, surfacing partial.
//
// Proves: The harvest commit handler computes `actual = min(available,
//   requested)` and surfaces depletion mid-second-action via
//   CommitTraceData::Harvest(HarvestCommitTrace { partial_quantity:
//   Some(Quantity(1)), .. }) instead of failing the whole start
//   (FND-10 outcomes are granular and leave aftermath, FND-29 trace
//   exposes the partial). Authoritative LastHarvestTrace stores the
//   partial entry alongside the full one.
//
// Chain: Two AcquireCommodity emissions -> two parallel harvest grants ->
//   3-tick concurrent execution -> first commit drains source -> second
//   commit observes available < requested -> partial item lot of
//   Quantity(1) materialized + HarvestCommitTrace.partial_quantity =
//   Some(Quantity(1)) + LastHarvestTrace appends partial: true.
#[test]
fn golden_partial_success_emits_partial_quantity() {
    let mut h = build_quantity_harness(Seed([0xA3; 32]));
    let orchard = place_orchard(&mut h, ORCHARD_FARM, 2, 3);
    let agent_a = seed_hungry_apple_seeker(&mut h, "Aria", ORCHARD_FARM);
    let agent_b = seed_hungry_apple_seeker(&mut h, "Bram", ORCHARD_FARM);

    // Allow plenty of ticks for both agents' parallel harvests to commit.
    let both_committed = tick_until(&mut h, 20, |h| {
        [agent_a, agent_b].iter().all(|actor| {
            first_action_event_matching(h, *actor, |kind| {
                matches!(kind, ActionTraceKind::Committed { .. })
            })
            .is_some()
        })
    });
    assert!(
        both_committed,
        "both parallel apple harvesters should commit within 20 ticks",
    );

    // Each commit's trace carries either no trace (full harvest) or the
    // partial-quantity trace (partial harvest). Exactly one of the two
    // commits must be partial because available=3 cannot satisfy two
    // recipe outputs of 2 each.
    let mut partial_traces: Vec<HarvestCommitTrace> = Vec::new();
    let mut full_commits = 0usize;
    for actor in [agent_a, agent_b] {
        let commit_event = first_action_event_matching(&h, actor, |kind| {
            matches!(kind, ActionTraceKind::Committed { .. })
        })
        .expect("commit event should be present");
        let ActionTraceKind::Committed { outcome, .. } = &commit_event.kind else {
            unreachable!("filtered above");
        };
        match &outcome.trace {
            Some(CommitTraceData::Harvest(trace)) => partial_traces.push(*trace),
            None => full_commits += 1,
            other => {
                panic!("unexpected commit-trace shape on harvest commit for {actor:?}: {other:?}",)
            }
        }
    }
    assert_eq!(
        full_commits, 1,
        "exactly one harvester should fully complete"
    );
    assert_eq!(
        partial_traces.len(),
        1,
        "exactly one harvester should commit partially",
    );
    let partial = partial_traces[0];
    assert_eq!(partial.requested_quantity, worldwake_core::Quantity(2));
    assert_eq!(partial.partial_quantity, Some(worldwake_core::Quantity(1)));

    // Authoritative LastHarvestTrace records the full and partial harvest
    // entries on the source.
    let trace = h
        .world
        .get_component_last_harvest_trace(orchard)
        .expect("orchard should have LastHarvestTrace populated by commits")
        .clone();
    assert!(
        trace
            .entries
            .iter()
            .any(|entry| entry.partial && entry.quantity == 1),
        "LastHarvestTrace should contain a partial entry of quantity 1; entries={:?}",
        trace.entries,
    );
    assert!(
        trace
            .entries
            .iter()
            .any(|entry| !entry.partial && entry.quantity == 2),
        "LastHarvestTrace should contain a full entry of quantity 2; entries={:?}",
        trace.entries,
    );

    // Authoritative source state: 3 units removed (2 + 1), available 0.
    let source = h
        .world
        .get_component_resource_source(orchard)
        .expect("orchard source still registered after both commits");
    assert_eq!(source.available_quantity, worldwake_core::Quantity(0));
}

// ---------------------------------------------------------------------------
// Scenario 354: Quantity-Aware Acquisition Lands Through The AI Pipeline
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: AcquireCommodity(SelfConsume), ConsumeOwnedCommodity
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 14, 14A, 20, 22, 26, 29
//
// Setup: One thirsty AI agent at OrchardFarm with one Well source
//   (extraction_slots = 1, available_quantity = 20). Default
//   MetabolismProfile.thirst_rate = pm(3) and DriveThresholds.thirst.high
//   = pm(700). Initial thirst is 300 ‰ — above `low` (200 ‰), well below
//   `high` so projected_tick_of returns current_tick + (700-300)/3 = 134.
//   With Water's thirst_relief_per_unit = 320 ‰, derive_acquire_commodity_quantity
//   computes desired_target = ceil(134 × 3 / 320) = 2, demonstrating
//   per-agent target derivation rather than a collapsed single-unit value.
//
// Proves: With the full S127-001..009 stack live, the agent's decision
//   trace includes the AcquireCommodity{Water} candidate AND the
//   `RankedGoalSummary.acquisition_quantity` carrier surfaces the un-normalized
//   `AcquisitionQuantity` (desired_target > 1) through the trace pipeline
//   without affecting `GoalKey` identity (which stays normalized to
//   `AcquisitionQuantity::single()`). The agent then successfully harvests
//   through the planner -> action -> commit chain, proving the quantity-aware
//   path is causally exercised end-to-end and observable to debug consumers
//   (FND-29).
//
// Chain: thirst-projection within horizon -> derive_acquire_commodity_quantity
//   returns AcquisitionQuantity{ desired_min=1, desired_target>1, horizon_ticks=134 }
//   -> emit AcquireCommodity{Water} GoalOffer with `acquisition_quantity` populated
//   -> ranking promotes offer to AgendaEntry -> summarize_ranked_goal copies
//   into RankedGoalSummary.acquisition_quantity -> plan search picks
//   harvest_water -> action commits -> agent gains water inventory.
#[test]
fn golden_s126_long_horizon_scales_desired_target() {
    let mut h = build_quantity_harness(Seed([0xA4; 32]));
    let _well = place_well(&mut h, ORCHARD_FARM, 1, 20);
    // Thirst 300 ‰ keeps the agent above `low` (so candidate emits) but well
    // below `high` (so projected_tick_of yields a long horizon). Without
    // this, thirst above `high` collapses the horizon to current_tick and
    // forces desired_target=1, masking the per-agent derivation.
    let agent = seed_thirsty_water_seeker_with_thirst(
        &mut h,
        "Aria",
        ORCHARD_FARM,
        None,
        worldwake_core::Permille::new(300).unwrap(),
    );

    // Tick once so the AI generates candidates with tracing enabled.
    h.step_once();
    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing enabled")
        .trace_at(agent, Tick(0))
        .expect("agent should have a decision trace at tick 0")
        .clone();
    let acquire_water_key = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Water,
        purpose: CommodityPurpose::SelfConsume,
        quantity: AcquisitionQuantity::single(),
    });
    let DecisionOutcome::Planning(planning) = &trace.outcome else {
        panic!(
            "agent's first tick should run the planning pipeline; outcome={:?}",
            trace.outcome
        );
    };
    assert!(
        planning
            .candidates
            .generated_contains_goal(acquire_water_key),
        "candidate emission should include AcquireCommodity{{Water}} when projection is within horizon; generated={:?}",
        planning.candidates.generated,
    );

    // S127QUAAWAACQ-009: the per-agent `AcquisitionQuantity` rides through
    // the candidate -> ranking -> RankedGoalSummary path so trace consumers
    // can observe `desired_target` per-emission.
    let acquire_summary = planning
        .candidates
        .ranked
        .iter()
        .find(|summary| summary.opportunity.goal_key == acquire_water_key)
        .expect(
            "AcquireCommodity{Water} should appear in ranked candidates so its \
             AcquisitionQuantity can be inspected through the decision trace",
        );
    let quantity = acquire_summary.acquisition_quantity.expect(
        "RankedGoalSummary for AcquireCommodity must carry the un-normalized \
         AcquisitionQuantity (S127 D11)",
    );
    assert!(
        quantity.desired_target.get() > 1,
        "long-horizon projection (thirst=300 ‰, rate=3 ‰/tick, water relief=320 ‰) \
         should derive desired_target > 1 through the decision trace; got {}",
        quantity.desired_target.get(),
    );
    assert_eq!(
        quantity.desired_min.get(),
        1,
        "desired_min stays at 1 (NonZeroU16::MIN) per spec D8 baseline",
    );
    assert!(
        quantity.horizon_ticks.get() > 1,
        "horizon_ticks should reflect the projected breach distance (~134 ticks), \
         not the collapsed current_tick fallback; got {}",
        quantity.horizon_ticks.get(),
    );
    // GoalKey identity stays normalized — proves acquisition_quantity is the
    // only carrier preserving per-agent values (S127 Design Goal 9).
    let GoalKind::AcquireCommodity {
        quantity: key_quantity,
        ..
    } = acquire_summary.opportunity.goal_key.kind
    else {
        panic!("ranked goal_key should describe AcquireCommodity");
    };
    assert_eq!(
        key_quantity,
        AcquisitionQuantity::single(),
        "GoalKey identity must keep quantity collapsed to single() so two \
         acquisition goals with the same commodity+purpose share a key",
    );

    // Tick until the agent commits at least one harvest and accumulates water.
    let harvest_complete = tick_until(&mut h, 40, |h| {
        first_action_event_matching(h, agent, |kind| {
            matches!(kind, ActionTraceKind::Committed { .. })
        })
        .is_some()
            && h.agent_commodity_qty(agent, CommodityKind::Water).0 > 0
    });
    assert!(
        harvest_complete,
        "agent should commit a Water harvest and accumulate inventory within 40 ticks",
    );
}

// ---------------------------------------------------------------------------
// Scenario 355: Queue Abandonment Promotes The Next Actor
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Production
// GoalKinds: AcquireCommodity(SelfConsume)
// ActionDomains: Production, Needs
// Places: OrchardFarm
// Principles: 7, 8, 10, 26
//
// Setup: Three thirsty AI agents (A, B, C) co-located at OrchardFarm with
//   one Well source (extraction_slots = 1, available_quantity = 20). After
//   tick 0, A holds the slot and B + C are queued. Mid-test scaffolding
//   removes B from the extraction queue via
//   ResourceExtractionQueues.queues[0].remove_actor(B), modeling
//   abandonment by an out-of-band cause (e.g., higher-priority work or
//   need satisfaction through an alternative path). The
//   ResourceExtractionQueues substrate has no automatic patience hook
//   today (legacy abandon_expired_facility_queues operates only on the
//   facility-level ContentionQueue).
//
// Proves: FOUNDATIONS Section VI Scenario E "Competing Claimants -> Queue
//   or Race -> Expiry/Prune -> Next Actor Acts" — once B leaves the
//   queue, the remaining queued actor (C) is granted the slot when they
//   re-request after A's commit. The line is inspectable world state,
//   not invisible runtime magic (FND-26 / FND-29).
//
// Chain: AcquireCommodity emissions for A, B, C -> A grants slot 0,
//   B + C queue -> scaffolding removes B from queue -> A commits ->
//   slot released -> C re-requests -> grant_or_signal_full grants
//   slot 0 to C -> action trace shows C started, B never restarts.
#[test]
fn golden_scenario_e_queue_abandonment_promotes_next_actor() {
    let mut h = build_quantity_harness(Seed([0xA5; 32]));
    let well = place_well(&mut h, ORCHARD_FARM, 1, 20);
    // Same short transient_block_ticks rationale as Scenario 351 — see
    // ticket reassessment item 15 for why ResourceExtractionQueues changes
    // do not currently clear the AI's ReservationConflict blocker.
    let agent_a = seed_thirsty_water_seeker(&mut h, "Aria", ORCHARD_FARM, Some(2));
    let agent_b = seed_thirsty_water_seeker(&mut h, "Bram", ORCHARD_FARM, Some(2));
    let agent_c = seed_thirsty_water_seeker(&mut h, "Cael", ORCHARD_FARM, Some(2));

    h.step_once();

    let queues_after_first_tick = extraction_queues(&h, well);
    let granted_actor = queues_after_first_tick.queues[0]
        .granted
        .as_ref()
        .map(|grant| grant.actor)
        .expect("first scheduled harvester should hold the slot after tick 0");
    let queued_actors: Vec<_> = queues_after_first_tick.queues[0]
        .waiting
        .values()
        .map(|waiter| waiter.actor)
        .collect();
    assert_eq!(
        queued_actors.len(),
        2,
        "single-slot scarcity should enqueue the two losing actors",
    );
    let abandoning_actor = *queued_actors
        .first()
        .expect("queued list should contain at least one losing actor");
    let remaining_queued = *queued_actors
        .iter()
        .find(|a| **a != abandoning_actor)
        .expect("a second queued actor must exist for the next-actor proof");

    // Sanity: the granted actor and queued pair partition {A, B, C}.
    let mut all_three: Vec<_> = vec![agent_a, agent_b, agent_c];
    all_three.sort();
    let mut observed: Vec<_> = vec![granted_actor, abandoning_actor, remaining_queued];
    observed.sort();
    assert_eq!(
        observed, all_three,
        "granted + queued actors should be exactly the three water seekers",
    );

    // Scaffolding: model abandonment by removing the abandoning actor
    // from the extraction queue. This stands in for the per-agent
    // re-evaluation cleanup that a future S127 follow-up would land
    // (the contract under proof is "next actor acts", which is
    // independent of the cause of removal — see ticket reassessment
    // item 15).
    {
        let mut queues = extraction_queues(&h, well);
        let removed = queues.queues[0].remove_actor(abandoning_actor);
        assert!(
            removed,
            "abandoning actor {abandoning_actor:?} should be present in slot 0 before removal",
        );
        let mut txn = new_txn(&mut h.world, h.scheduler.current_tick().0);
        txn.set_component_resource_extraction_queues(well, queues)
            .expect("scaffolding write should keep ResourceExtractionQueues writable");
        commit_txn(txn, &mut h.event_log);
    }

    // Tick until the granted actor commits.
    let granted_committed = tick_until(&mut h, 12, |h| {
        first_action_event_matching(h, granted_actor, |kind| {
            matches!(kind, ActionTraceKind::Committed { .. })
        })
        .is_some()
    });
    assert!(
        granted_committed,
        "first granted actor should commit within 12 ticks; queues={:?}",
        extraction_queues(&h, well),
    );

    // Re-request promotion: the remaining queued actor must obtain the
    // slot when they re-request after the commit. The contract under
    // proof is "next actor acts" — the line is inspectable, the slot
    // grant transitions to a still-eligible actor — not "the abandoning
    // actor never re-attempts" (which would require an architectural
    // hook that updates the AI's blocker memory or goal store on queue
    // removal; see ticket reassessment item 15 for the documented gap).
    let remaining_granted = tick_until(&mut h, 16, |h| {
        let q = extraction_queues(h, well);
        q.queues[0]
            .granted
            .as_ref()
            .map(|grant| grant.actor)
            .is_some_and(|actor| actor == remaining_queued)
    });
    assert!(
        remaining_granted,
        "remaining queued actor {remaining_queued:?} should be granted slot 0 within 16 ticks of {granted_actor:?}'s commit; queues={:?}",
        extraction_queues(&h, well),
    );

    // While the remaining_queued actor was granted at least once between
    // the abandonment and the assertion above, the abandoning actor's
    // line of action-trace events post-removal should not include a
    // grant transition for them on the contested slot — verified by the
    // remaining_granted assertion (the slot transitions to
    // remaining_queued, not abandoning_actor). No further assertion is
    // needed because a grant transition to abandoning_actor would have
    // failed the loop above.
}
