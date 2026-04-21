//! Golden test for S112 portfolio planning probe behavior.

mod golden_harness;

use std::collections::BTreeMap;
use std::path::PathBuf;

use golden_harness::{GoldenHarness, commit_txn, new_txn, seed_actor_local_beliefs};
use serde::{Deserialize, Serialize};
use worldwake_ai::{
    AgentDecisionRuntime, AgentTickDriver, DecisionOutcome, DirtySet, PlanTerminalKind,
    PlannedPlan,
};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario, types::ScenarioDef};
use worldwake_core::{
    ActiveGoal, Blocker, BlockerClearingCondition, BlockerKey, BlockerMemory, BlockingFact,
    DecisionEventPayload, EntityId, EventTag, EventView, ExpectationBasis, ExpectationId,
    ExpectationRecord, ExpectationState, ExpectationStore, GoalKey, GoalKind,
    GoalRejectionReason, OpportunityAnchor, OpportunityKey, PerceptionSource, Tick,
    ViolationDispositionProfile,
};
use std::num::NonZeroU32;
use worldwake_sim::SaveableRuntime;

fn scenario_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/portfolio-planning.ron")
}

fn load_portfolio_planning_harness() -> (GoldenHarness, ScenarioDef) {
    let path = scenario_path();
    let def = load_scenario_file(&path).expect("portfolio-planning scenario should parse");
    let spawned = spawn_scenario(&def).expect("portfolio-planning scenario should spawn");
    let harness = GoldenHarness::from_simulation_state(&spawned.state);
    (harness, def)
}

fn named_entities(h: &GoldenHarness) -> BTreeMap<String, EntityId> {
    h.world
        .query_name()
        .map(|(entity, name)| (name.0.clone(), entity))
        .collect()
}

fn scenario_place_id(def: &ScenarioDef, place_name: &str) -> EntityId {
    let slot = def
        .places
        .iter()
        .position(|place| place.name == place_name)
        .and_then(|index| u32::try_from(index).ok())
        .expect("scenario place should exist within u32 slot bounds");
    EntityId {
        slot,
        generation: 0,
    }
}

#[derive(Deserialize, Serialize)]
struct DriverStateMirror {
    runtime_by_agent: BTreeMap<EntityId, AgentDecisionRuntime>,
}

fn inject_prior_commitment(
    harness: &mut GoldenHarness,
    agent: EntityId,
    committed_goal: GoalKey,
    committed_opportunity: OpportunityKey,
) {
    let runtime = AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            committed_opportunity,
            committed_goal,
            Vec::new(),
            PlanTerminalKind::GoalSatisfied,
        )),
        dirty: DirtySet::REPLAN_SIGNAL,
        ..AgentDecisionRuntime::default()
    };
    let bytes = bincode::serialize(&DriverStateMirror {
        runtime_by_agent: BTreeMap::from([(agent, runtime)]),
    })
    .expect("runtime mirror should serialize");
    harness.driver = AgentTickDriver::from_saved_runtime(&bytes, &harness.world)
        .expect("golden harness should accept the injected runtime");
    let restored_state: DriverStateMirror = bincode::deserialize(
        &harness
            .driver
            .save_runtime_state()
            .expect("golden harness runtime should serialize after injection"),
    )
    .expect("golden harness runtime mirror should deserialize after injection");
    let restored_runtime = restored_state
        .runtime_by_agent
        .get(&agent)
        .expect("golden harness should preserve injected runtime");
    assert_eq!(restored_runtime.current_plan.as_ref().map(|plan| plan.opportunity), Some(committed_opportunity));
    harness.driver.enable_tracing();

    let mut txn = new_txn(&mut harness.world, 0);
    txn.set_component_active_goal(
        agent,
        ActiveGoal {
            goal_key: committed_goal,
            adopted_at: Tick(0),
        },
    )
    .expect("golden harness should keep active goals writable");
    commit_txn(txn, &mut harness.event_log);
}

fn inject_probe_only_sleep_blocker(harness: &mut GoldenHarness, agent: EntityId, place: EntityId) {
    let mut memory = BlockerMemory::default();
    memory.record(Blocker {
        blocker_key: BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: Some(place),
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::NoKnownPath,
        diagnostic_context: None,
        observed_tick: Tick(0),
        expires_tick: Tick(8),
        clearing_condition: BlockerClearingCondition::TtlOnly,
        baseline_snapshot: None,
    });

    let mut txn = new_txn(&mut harness.world, 0);
    txn.set_component_blocker_memory(agent, memory)
        .expect("golden harness should keep blocker memory writable");
    commit_txn(txn, &mut harness.event_log);
}

fn inject_missing_expectation_commitment(
    harness: &mut GoldenHarness,
    agent: EntityId,
    subject: EntityId,
    expected_place: EntityId,
) -> GoalKey {
    let mut store = ExpectationStore::default();
    let expectation_id = ExpectationId(1);
    store.records.insert(
        expectation_id,
        ExpectationRecord {
            id: expectation_id,
            owner: agent,
            subject,
            expected_place,
            deadline_tick: Tick(0),
            grace_ticks: 0,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Overdue,
            created_tick: Tick(0),
        },
    );

    let mut txn = new_txn(&mut harness.world, 0);
    txn.set_component_violation_disposition_profile(
        agent,
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(3).expect("non-zero duration"),
            violation_memory_retention_ticks: 50,
            investigation_motive_weight: worldwake_core::Permille::new(500).unwrap(),
            ownership_motive_bonus: worldwake_core::Permille::new(200).unwrap(),
        },
    )
    .expect("golden harness should keep violation profiles writable");
    txn.set_component_expectation_store(agent, store)
        .expect("golden harness should keep expectation stores writable");
    commit_txn(txn, &mut harness.event_log);

    GoalKey::from(GoalKind::ReportMissing {
        subject,
        to_office: None,
        expectation_id: Some(expectation_id),
    })
}

// ---------------------------------------------------------------------------
// Scenario 167: Portfolio Rejects Infeasible Survival And Commitment Slots
// ---------------------------------------------------------------------------
//
// Systems: AI, Needs, Social, Production, Decision History
// GoalKinds: Sleep, ReportMissing, ProduceCommodity
// ActionDomains: Needs, Social, Production
// Places: Camp Workshop
// Principles: 7, 14, 20, 28
//
// Setup: Load the authored `portfolio-planning.ron` scenario, seed explicit
// local beliefs, seed a scoped blocker-memory entry that only the probe sees
// for `Sleep`, and inject an overdue `ReportMissing` commitment into runtime
// and matching `ActiveGoal`.
//
// Proves: on the winning planning tick, the landed portfolio contains exactly
// three populated slots: rejected survival, rejected commitment, and
// plausible economic. The agent commits the economic `ProduceCommodity`
// branch within two ticks, and the two rejected slots appear in
// `GoalCommittedPayload::rejected_alternatives` tagged
// `FeasibilityProbeFailed`.
//
// Chain: scoped blocker-memory rejection for `Sleep` + seeded overdue
// `ReportMissing` commitment with no believed subject -> portfolio admission
// rejects survival and commitment before search -> local mill-backed bread
// production remains plausible -> economic goal commits on the same planning
// pass.
#[test]
fn portfolio_rejects_infeasible_slots_and_commits_feasible_economic_goal() {
    let (mut harness, def) = load_portfolio_planning_harness();
    let named = named_entities(&harness);
    let agent = *named
        .get("Planner")
        .expect("scenario should include the Planner agent");
    let camp_workshop = scenario_place_id(&def, "Camp Workshop");
    let missing_subject = EntityId {
        slot: 90_106,
        generation: 0,
    };

    seed_actor_local_beliefs(
        &mut harness.world,
        &mut harness.event_log,
        agent,
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    inject_probe_only_sleep_blocker(&mut harness, agent, camp_workshop);

    let committed_goal =
        inject_missing_expectation_commitment(&mut harness, agent, missing_subject, camp_workshop);
    let rejected_survival_goal = GoalKey::from(GoalKind::Sleep);
    let rejected_commitment_goal = committed_goal;

    inject_prior_commitment(
        &mut harness,
        agent,
        committed_goal,
        OpportunityKey {
            goal_key: committed_goal,
            anchor: OpportunityAnchor::Entity(missing_subject),
        },
    );

    for _ in 0..2 {
        harness.step_once();
    }

    let (bake_bread_recipe_id, _) = harness
        .recipes
        .recipe_by_name("Bake Bread")
        .expect("Bake Bread should exist in the full runtime registry");
    let committed_goal = GoalKey::from(GoalKind::ProduceCommodity {
        recipe_id: bake_bread_recipe_id,
    });
    let decision_sink = harness
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled");
    let trace_debug = decision_sink
        .traces_for(agent)
        .into_iter()
        .map(|trace| format!("{trace:?}"))
        .collect::<Vec<_>>()
        .join("\n\n");
    let (commit_tick, planning) = decision_sink
        .traces_for(agent)
        .into_iter()
        .find_map(|trace| {
            let DecisionOutcome::Planning(planning) = &trace.outcome else {
                return None;
            };
            planning
                .selection
                .selected_goal_is(committed_goal)
                .then_some((trace.tick, planning))
        })
        .unwrap_or_else(|| {
            panic!("scenario should commit the feasible economic goal within two ticks\n{trace_debug}")
        });

    assert!(
        commit_tick.0 <= 1,
        "portfolio scenario should commit within two planning ticks; commit_tick={commit_tick:?}"
    );

    let portfolio = planning
        .portfolio
        .as_ref()
        .expect("winning planning tick should populate a portfolio trace");
    let portfolio_debug = format!("{portfolio:?}");
    assert!(
        portfolio_debug.contains("Survival")
            && portfolio_debug.contains("Commitment")
            && portfolio_debug.contains("Economic"),
        "portfolio trace should expose all three slot kinds: {portfolio_debug}\nplanning={planning:?}"
    );
    assert!(
        portfolio_debug.contains("slots_attempted: 1"),
        "portfolio trace should record exactly one attempted plausible slot: {portfolio_debug}"
    );
    assert!(
        portfolio_debug.contains("Sleep"),
        "portfolio trace should include the rejected survival goal: {portfolio_debug}"
    );
    assert!(
        portfolio_debug.contains("ReportMissing"),
        "portfolio trace should include the rejected commitment goal: {portfolio_debug}"
    );
    assert!(
        portfolio_debug.contains("ProduceCommodity")
            && portfolio_debug.contains("RejectedBeforeSearch")
            && portfolio_debug.contains("Plausible"),
        "portfolio trace should show two rejections and one plausible slot: {portfolio_debug}"
    );

    let committed_event = harness
        .event_log
        .events_by_tag(EventTag::GoalCommitted)
        .iter()
        .filter_map(|id| harness.event_log.get(*id))
        .find(|record| {
            matches!(
                record.decision_payload(),
                Some(DecisionEventPayload::GoalCommitted(payload))
                    if payload.goal_key == committed_goal
            )
        })
        .expect("goal commit event for the selected economic goal should exist");

    let payload = match committed_event.decision_payload() {
        Some(DecisionEventPayload::GoalCommitted(payload)) => payload,
        other => panic!("unexpected decision payload for committed event: {other:?}"),
    };
    assert_eq!(payload.goal_key, committed_goal);
    assert_eq!(payload.rejected_alternatives.len(), 2);
    assert!(
        payload.rejected_alternatives.iter().any(|alternative| {
            alternative.goal_key == rejected_survival_goal
                && alternative.rejection_reason == GoalRejectionReason::FeasibilityProbeFailed
        }),
        "goal commit payload should include the rejected survival slot: {:?}",
        payload.rejected_alternatives
    );
    assert!(
        payload.rejected_alternatives.iter().any(|alternative| {
            alternative.goal_key == rejected_commitment_goal
                && alternative.rejection_reason == GoalRejectionReason::FeasibilityProbeFailed
        }),
        "goal commit payload should include the rejected commitment slot: {:?}",
        payload.rejected_alternatives
    );
}
