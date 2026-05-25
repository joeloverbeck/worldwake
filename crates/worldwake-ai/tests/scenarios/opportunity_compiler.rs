//! Golden-facing regression guards for the S138 opportunity compiler.

use std::path::PathBuf;

use crate::golden_harness::*;
use worldwake_ai::opportunity_compiler::{BelievedLegalStatus, RiskFact, compile_opportunities};
use worldwake_ai::{EffectSchemaIndex, OpportunityAnchor};
use worldwake_cli::scenario::{load_scenario_file, spawn_scenario};
use worldwake_core::{
    AcquisitionQuantity, BeliefStatusTag, ClaimId, ClaimValue, CommodityKind, CommodityPurpose,
    EntityBeliefAspect, EntityBeliefClaim, GoalKey, GoalKind, HomeostaticNeeds, LawAbidingProfile,
    LearnedOpportunityMemory, LearnedOpportunitySource, MetabolismProfile, OpportunityEntry,
    OpportunityKey, PerceptionProfile, PerceptionSource, Permille, Quantity, RiskWeightProfile,
    Seed, Tick, UtilityProfile, hash_event_log,
};
use worldwake_sim::PerAgentBeliefView;

const REGRESSION_TICKS: u32 = 120;

fn survival_baseline_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../scenarios/survival-baseline.ron")
}

fn load_survival_baseline_harness() -> GoldenHarness {
    let def = load_scenario_file(&survival_baseline_path())
        .expect("survival-baseline scenario should parse");
    let spawned = spawn_scenario(&def).expect("survival-baseline scenario should spawn");
    let mut harness = GoldenHarness::from_simulation_state(&spawned.state);
    harness.driver.enable_tracing();
    harness
}

fn compile_index(h: &GoldenHarness) -> EffectSchemaIndex {
    EffectSchemaIndex::build(&h.defs)
}

fn starving_agent(h: &mut GoldenHarness, name: &str) -> worldwake_core::EntityId {
    let agent = seed_agent(
        &mut h.world,
        &mut h.event_log,
        name,
        VILLAGE_SQUARE,
        HomeostaticNeeds::new(pm(950), pm(0), pm(0), pm(0), pm(0)),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_perception_profile(
        agent,
        PerceptionProfile {
            opportunity_floor_permille: Permille::ZERO,
            ..PerceptionProfile::default()
        },
    )
    .unwrap();
    commit_txn(txn, &mut h.event_log);
    agent
}

fn owned_bread_fixture(
    risk: RiskWeightProfile,
    law: LawAbidingProfile,
) -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(Seed([138; 32]));
    let actor = starving_agent(&mut h, "Compiler Actor");
    let owner = seed_agent(
        &mut h.world,
        &mut h.event_log,
        "Bread Owner",
        VILLAGE_SQUARE,
        HomeostaticNeeds::new_sated(),
        MetabolismProfile::default(),
        UtilityProfile::default(),
    );
    let mut txn = new_txn(&mut h.world, 0);
    let lot = txn
        .create_item_lot_with_owner(
            CommodityKind::Bread,
            Quantity(5),
            VILLAGE_SQUARE,
            Some(owner),
        )
        .unwrap();
    txn.set_component_risk_weight_profile(actor, risk).unwrap();
    txn.set_component_law_abiding_profile(actor, law).unwrap();
    commit_txn(txn, &mut h.event_log);
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        actor,
        &[lot],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    let mut store = h
        .world
        .get_component_agent_belief_store(actor)
        .cloned()
        .unwrap_or_default();
    store.record_entity_claim(EntityBeliefClaim {
        claim_id: ClaimId(0),
        subject: lot,
        aspect: EntityBeliefAspect::Owner,
        value: ClaimValue::Entity(Some(owner)),
        source: PerceptionSource::DirectObservation,
        acquired_tick: Tick(0),
        claimed_event_tick: Some(Tick(0)),
        confidence: Permille::new(1000).unwrap(),
        refuted_at_tick: None,
    });
    let mut txn = new_txn(&mut h.world, 0);
    txn.set_component_agent_belief_store(actor, store).unwrap();
    commit_txn(txn, &mut h.event_log);
    (h, actor, lot)
}

fn unowned_bread_fixture() -> (
    GoldenHarness,
    worldwake_core::EntityId,
    worldwake_core::EntityId,
) {
    let mut h = GoldenHarness::new(Seed([139; 32]));
    let agent = starving_agent(&mut h, "Hungry Compiler");
    let lot = {
        let mut txn = new_txn(&mut h.world, 0);
        let lot = txn
            .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), VILLAGE_SQUARE, None)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        lot
    };
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[lot],
        Tick(0),
        PerceptionSource::DirectObservation,
    );
    (h, agent, lot)
}

fn opportunity_key_for_bread(lot: worldwake_core::EntityId) -> OpportunityKey {
    OpportunityKey {
        goal_key: GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
            quantity: AcquisitionQuantity::single(),
        }),
        anchor: OpportunityAnchor::Entity(lot),
    }
}

fn compiled_bread_opportunity(
    h: &GoldenHarness,
    agent: worldwake_core::EntityId,
    lot: worldwake_core::EntityId,
) -> worldwake_ai::opportunity_compiler::Opportunity {
    let view = PerAgentBeliefView::from_world(agent, &h.world);
    let (opportunities, load) = compile_opportunities(agent, &view, &compile_index(h));
    assert_eq!(
        load.compiled_count, 1,
        "fixture should compile one bread opportunity"
    );
    opportunities
        .into_iter()
        .find(|opportunity| opportunity.key == opportunity_key_for_bread(lot))
        .expect("compiled opportunity should be anchored on the observed bread lot")
}

// Scenario 398: Opportunity Compiler Profiles Legal Risk
// Systems: AI, Perception, Trade
// GoalKinds: AcquireCommodity
// ActionDomains: AI
// Principles: P7, P14, P20, P22
// Setup: Two starving agents observe the same owned bread lot through seeded local belief; one has default risk/legal profiles and one has high aversion.
// Proves: The S138 compiler preserves the opportunity but lowers salience through concrete per-agent risk/legal state rather than filtering it.
// Chain: local observation -> compile_opportunities -> legal/risk diagnostic -> salience-ranked opportunity trace input.
#[test]
fn profile_weighting_preserves_owned_bread_opportunity_with_lower_salience() {
    let (permissive_h, permissive, permissive_lot) =
        owned_bread_fixture(RiskWeightProfile::default(), LawAbidingProfile::default());
    let (cautious_h, cautious, cautious_lot) = owned_bread_fixture(
        RiskWeightProfile {
            theft_aversion: pm(500),
            exposure_aversion: pm(300),
            threat_aversion: Permille::ZERO,
        },
        LawAbidingProfile {
            criminal_threshold: pm(400),
            social_norm_weight: pm(300),
        },
    );

    let permissive_op = compiled_bread_opportunity(&permissive_h, permissive, permissive_lot);
    let cautious_op = compiled_bread_opportunity(&cautious_h, cautious, cautious_lot);

    assert!(matches!(
        permissive_op.legal_status,
        BelievedLegalStatus::BelievedOwned { .. }
    ));
    assert!(
        permissive_op
            .risks
            .iter()
            .any(|risk| matches!(risk, RiskFact::CriminalLiability { .. })),
        "owned bread should retain the criminal-liability diagnostic"
    );
    assert!(
        cautious_op.salience < permissive_op.salience,
        "risk/legal profiles should lower salience without erasing the opportunity: permissive={:?}, cautious={:?}",
        permissive_op.salience,
        cautious_op.salience
    );
}

// Scenario 399: Opportunity Compiler Trace Carriage
// Systems: AI, Perception
// GoalKinds: AcquireCommodity
// ActionDomains: AI
// Principles: P7, P14, P20
// Setup: A starving agent observes an unowned local bread lot and runs one real agent tick with decision tracing enabled.
// Proves: The per-agent trace carries both compiled opportunities and OpportunityCompilerLoad from the live read phase.
// Chain: local belief -> read phase -> decision trace.
#[test]
fn agent_tick_trace_carries_compiled_opportunities_and_load() {
    let (mut h, agent, lot) = unowned_bread_fixture();
    h.driver.enable_tracing();

    h.step_once();

    let trace = h
        .driver
        .trace_sink()
        .expect("decision tracing should be enabled")
        .trace_at(agent, Tick(0))
        .expect("agent should have a decision trace at tick 0");
    assert!(
        trace
            .compiled_opportunities
            .iter()
            .any(|opportunity| opportunity.key == opportunity_key_for_bread(lot)),
        "compiled bread opportunity should be retained on the public trace"
    );
    assert_eq!(
        trace
            .opportunity_compiler_load
            .as_ref()
            .expect("load should be recorded")
            .compiled_count,
        1
    );
    assert!(
        trace.snapshot_cache_counters.is_some(),
        "planning snapshot cache counters should be surfaced on the public trace"
    );
}

// Scenario 400: Opportunity Compiler Effect Index Miss
// Systems: AI
// GoalKinds: AcquireCommodity
// ActionDomains: AI
// Principles: P3, P20
// Setup: A starving agent has a bread belief, but the supplied effect-schema index contains no CommodityTransfer producer.
// Proves: Unknown or unavailable effect categories do not inflate opportunities.
// Chain: effect-schema index miss -> compile_opportunities -> zero load.
#[test]
fn effect_schema_index_miss_emits_no_opportunity() {
    let (h, agent, _lot) =
        owned_bread_fixture(RiskWeightProfile::default(), LawAbidingProfile::default());
    let view = PerAgentBeliefView::from_world(agent, &h.world);
    let (opportunities, load) = compile_opportunities(agent, &view, &EffectSchemaIndex::empty());

    assert!(opportunities.is_empty());
    assert_eq!(load.compiled_count, 0);
}

// Scenario 401: Learned Opportunity Memory Damps Repeated Opportunity
// Systems: AI, Memory
// GoalKinds: AcquireCommodity
// ActionDomains: AI
// Principles: P20, P22A
// Setup: A starving agent observes bread once with and once without a LearnedOpportunityMemory entry for that exact opportunity key.
// Proves: Concrete learned state lowers salience and increments the damping load counter.
// Chain: learned memory -> compile_opportunities -> damped salience/load.
#[test]
fn learned_opportunity_memory_damps_repeated_bread_opportunity() {
    let (baseline_h, baseline_agent, baseline_lot) =
        owned_bread_fixture(RiskWeightProfile::default(), LawAbidingProfile::default());
    let baseline = compiled_bread_opportunity(&baseline_h, baseline_agent, baseline_lot);

    let (mut damped_h, damped_agent, damped_lot) =
        owned_bread_fixture(RiskWeightProfile::default(), LawAbidingProfile::default());
    let key = opportunity_key_for_bread(damped_lot);
    let mut memory = LearnedOpportunityMemory::default();
    memory.record(OpportunityEntry {
        opportunity: key,
        observed_tick: Tick(0),
        expires_tick: Tick(40),
        observed_at: VILLAGE_SQUARE,
        source: LearnedOpportunitySource::ReadPhaseInference,
    });
    let mut txn = new_txn(&mut damped_h.world, 0);
    txn.set_component_learned_opportunity_memory(damped_agent, memory)
        .unwrap();
    commit_txn(txn, &mut damped_h.event_log);

    let view = PerAgentBeliefView::from_world(damped_agent, &damped_h.world);
    let (opportunities, load) =
        compile_opportunities(damped_agent, &view, &compile_index(&damped_h));
    let damped = opportunities
        .into_iter()
        .find(|opportunity| opportunity.key == key)
        .expect("damped opportunity should still be emitted");

    assert_eq!(load.learned_memory_damped, 1);
    assert!(damped.salience < baseline.salience);
}

// Scenario 462: Opportunity Compiler Status Diagnostics
// Systems: AI, Perception
// GoalKinds: AcquireCommodity
// ActionDomains: AI
// Principles: P14, P15, P16, P29
// Setup: A starving agent has two inventory beliefs for local bread lots, one fresh and one decayed below the confidence threshold.
// Proves: The live compiler load carries at least two opportunity source-status buckets instead of collapsing all compiled opportunities to Probable.
// Chain: mixed belief freshness -> compile_opportunities -> OpportunityCompilerLoad.
#[test]
fn mixed_freshness_status_distribution_reaches_scenario_diagnostics() {
    let mut h = GoldenHarness::new(Seed([166; 32]));
    let agent = starving_agent(&mut h, "Status Compiler");
    let (fresh_lot, stale_lot) = {
        let mut txn = new_txn(&mut h.world, 0);
        let fresh_lot = txn
            .create_item_lot_with_owner(CommodityKind::Bread, Quantity(3), VILLAGE_SQUARE, None)
            .unwrap();
        let stale_lot = txn
            .create_item_lot_with_owner(CommodityKind::Bread, Quantity(4), VILLAGE_SQUARE, None)
            .unwrap();
        commit_txn(txn, &mut h.event_log);
        (fresh_lot, stale_lot)
    };
    seed_actor_beliefs(
        &mut h.world,
        &mut h.event_log,
        agent,
        &[fresh_lot, stale_lot],
        Tick(10),
        PerceptionSource::DirectObservation,
    );
    let mut store = h
        .world
        .get_component_agent_belief_store(agent)
        .cloned()
        .expect("seeded actor should have belief store");
    store.record_entity_claim(EntityBeliefClaim {
        claim_id: ClaimId(1660),
        subject: fresh_lot,
        aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
        value: ClaimValue::Quantity(Quantity(3)),
        source: PerceptionSource::DirectObservation,
        acquired_tick: Tick(10),
        claimed_event_tick: Some(Tick(10)),
        confidence: Permille::new(1000).unwrap(),
        refuted_at_tick: None,
    });
    store.record_entity_claim(EntityBeliefClaim {
        claim_id: ClaimId(1661),
        subject: stale_lot,
        aspect: EntityBeliefAspect::Inventory(CommodityKind::Bread),
        value: ClaimValue::Quantity(Quantity(4)),
        source: PerceptionSource::DirectObservation,
        acquired_tick: Tick(10),
        claimed_event_tick: Some(Tick(10)),
        confidence: Permille::new(60).unwrap(),
        refuted_at_tick: None,
    });
    let mut txn = new_txn(&mut h.world, 10);
    txn.set_component_agent_belief_store(agent, store).unwrap();
    commit_txn(txn, &mut h.event_log);

    let view = PerAgentBeliefView::from_world_at_tick(agent, Tick(12), &h.world);
    let (_opportunities, load) = compile_opportunities(agent, &view, &compile_index(&h));
    assert_eq!(load.compiled_by_status.len(), 2);
    assert_eq!(
        load.compiled_by_status.values().sum::<u32>(),
        load.compiled_count
    );
    assert_eq!(
        load.compiled_by_status.get(&BeliefStatusTag::Stale),
        Some(&1)
    );
}

// Scenario 402: Opportunity Compiler Default Replay Bound
// Systems: AI, Replay
// GoalKinds: AcquireCommodity, ConsumeOwnedCommodity, Sleep, Relieve, ExploreLocation
// ActionDomains: AI, Needs, Travel
// Principles: P9, P12, P20
// Setup: Load authored survival-baseline.ron twice and run the same default-profile tick window with decision tracing enabled.
// Proves: S138 opportunity compilation is deterministic on the default replay and the per-tick compiled work remains bounded by the cognitive profile cap.
// Chain: authored scenario -> agent_tick read phase -> OpportunityCompilerLoad -> deterministic event-log hash.
#[test]
fn survival_baseline_replay_is_deterministic_and_compiler_load_is_bounded() {
    fn run() -> (worldwake_core::StateHash, u32, usize) {
        let mut h = load_survival_baseline_harness();
        for _ in 0..REGRESSION_TICKS {
            h.step_once();
        }
        let traces = h
            .driver
            .trace_sink()
            .expect("decision tracing should be enabled")
            .traces();
        let max_compiled = traces
            .iter()
            .filter_map(|trace| trace.opportunity_compiler_load.as_ref())
            .map(|load| load.compiled_count)
            .max()
            .unwrap_or(0);
        let samples = traces
            .iter()
            .filter(|trace| trace.opportunity_compiler_load.is_some())
            .count();
        (
            hash_event_log(&h.event_log).expect("event log should hash canonically"),
            max_compiled,
            samples,
        )
    }

    let first = run();
    let second = run();

    assert_eq!(first.0, second.0, "event-log replay hash should be stable");
    assert_eq!(first.1, second.1, "load maximum should be deterministic");
    assert!(
        first.2 > 0,
        "traced replay should record compiler-load samples"
    );
    assert!(
        first.1 <= 16,
        "compiled opportunities per tick should stay within default compile_opportunity_cap; max={}",
        first.1
    );
}
