use super::candidates::abandon_expired_facility_queues_with_limit;
use super::execution::resolve_step_targets;
use super::observation::{
    ReadPhaseContext, facility_queue_patience_exhausted, refresh_runtime_for_read_phase,
    update_runtime_observation_snapshot,
};
use super::planning::{
    determine_selected_plan_source, plan_and_validate_next_step, summarize_plan_replacement,
};
use super::{
    AgentTickDriver, advance_completed_step, apply_step_materialization_bindings,
    committed_action_for_step, effective_goal_switch_margin,
    handle_recoverable_travel_step_blockage, persist_blocked_memory,
    plan_and_validate_next_step_traced, update_frame_for_adopted_plan,
};
use crate::ProfileFixture;
use crate::exhaustion::{StealTargetAccessState, StealTargetSnapshot};
use crate::plan_selection::SelectionCandidatePlan;
use crate::{
    AgentDecisionRuntime, CommodityPurpose, DirtySet, ExhaustionBaseline,
    ExhaustionInvalidationCondition, ExpectedMaterialization, FrameSwitchMarginSource, GoalKey,
    GoalKind, GoalPriorityClass, HypotheticalEntityId, OpportunityAnchor, OpportunityKey,
    PlanTerminalKind, PlannedPlan, PlannedStep, PlannerOpKind, PlanningEntityRef, RankedGoal,
    RankedGoalProvenance, SelectedPlanReplacementKind, build_semantics_table,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::num::NonZeroU32;
use std::path::PathBuf;
use worldwake_core::{
    ActionDefId, BanditFactionPolicy, BeliefConfidencePolicy, BelievedInstitutionalClaim,
    BlockedIntent, BlockedIntentMemory, BlockerKey, BlockingFact, BodyCostPerTick, BodyPart,
    CarryCapacity, CauseRef, CognitiveProfile, CommodityKind, ContentionGrant, ContentionIntents,
    ContentionPolicy, ContentionQueue, ControlSource, DeadAt, DemandMemory, DemandObservation,
    DemandObservationReason, DeprivationExposure, DriveThresholds, EntityId, EntityKind, EventLog,
    EventPayload, ExecutionBudget, FrameState, HomeostaticNeeds, InstitutionalBeliefKey,
    InstitutionalClaim, InstitutionalKnowledgeSource, IntentionDispositionProfile, IntentionDomain,
    IntentionFrame, KnownRecipes, LoadUnits, MerchandiseProfile, MetabolismProfile, OfficeData,
    PatrolProfile, PatrolRoute, PendingEvent, PerceptionProfile, PerceptionSource, Permille, Place,
    Quantity, QueuedContentionIntent, RecipeId, RecordData, RecordKind, ResourceSource, Seed,
    SuccessionLaw, TellMemoryKey, TellProfile, TellTopic, Tick, ToldBeliefMemory, Topology,
    TravelEdge, TravelEdgeId, UniqueItemKind, UtilityProfile, ViolationMemory, VisibilitySpec,
    WitnessData, WorkstationMarker, WorkstationTag, World, WorldTxn, Wound, WoundCause, WoundId,
    WoundList, build_believed_entity_state, build_prototype_world,
};
use worldwake_sim::{
    ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionPayload,
    AutonomousControllerRuntime, CommitOutcome, CommittedAction, ControllerState, DeterministicRng,
    DurationExpr, Materialization, MaterializationTag, PerAgentBeliefView, RecipeDefinition,
    RecipeRegistry, RuntimeBeliefView, SaveError, SaveableRuntime, Scheduler, SystemDispatchTable,
    SystemExecutionContext, SystemId, SystemManifest, TickStepServices, TransportActionPayload,
    step_tick,
};
use worldwake_systems::{build_full_action_registries, perception_system, register_needs_actions};

struct Harness {
    world: World,
    event_log: EventLog,
    scheduler: Scheduler,
    controller: ControllerState,
    rng: DeterministicRng,
    recipes: RecipeRegistry,
    defs: ActionDefRegistry,
    handlers: ActionHandlerRegistry,
    driver: AgentTickDriver,
    actor: worldwake_core::EntityId,
}

fn exhaustion_key(goal_key: GoalKey, anchor: OpportunityAnchor) -> OpportunityKey {
    OpportunityKey { goal_key, anchor }
}

fn default_opportunity(goal_key: GoalKey) -> OpportunityKey {
    OpportunityKey {
        goal_key,
        anchor: OpportunityAnchor::None,
    }
}

fn patrol_profile(base_dwell_ticks: u32, vigilance: u16, motive: u16) -> PatrolProfile {
    PatrolProfile {
        base_dwell_ticks,
        dwell_vigilance_scale_ticks: base_dwell_ticks,
        vigilance: Permille::new(vigilance).unwrap(),
        route_adaptation_sensitivity: Permille::new(1000).unwrap(),
        patrol_motive_weight: Permille::new(motive).unwrap(),
    }
}

fn cognitive(reasoning: &ProfileFixture) -> CognitiveProfile {
    CognitiveProfile {
        max_candidates_to_plan: reasoning.max_candidates_to_plan,
        max_plan_depth: reasoning.max_plan_depth,
        snapshot_travel_horizon: reasoning.snapshot_travel_horizon,
        max_node_expansions: reasoning.max_node_expansions,
        switch_margin: reasoning.switch_margin,
        transient_block_ticks: reasoning.transient_block_ticks,
        unknown_block_ticks: reasoning.unknown_block_ticks,
        structural_block_ticks: reasoning.structural_block_ticks,
        initial_cooldown_ticks: reasoning.initial_cooldown_ticks,
        max_cooldown_ticks: reasoning.max_cooldown_ticks,
    }
}

fn execution_budget(reasoning: &ProfileFixture) -> ExecutionBudget {
    ExecutionBudget {
        beam_width: reasoning.beam_width,
        max_prerequisite_locations: reasoning.max_prerequisite_locations,
    }
}

impl Harness {
    fn new(control_source: ControlSource) -> Self {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let actor = {
            let mut txn = new_txn(&mut world, 1);
            let actor = txn.create_agent("Aster", control_source).unwrap();
            let bread = txn
                .create_item_lot(CommodityKind::Bread, Quantity(1))
                .unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(bread, place).unwrap();
            txn.set_possessor(bread, actor).unwrap();
            txn.set_component_homeostatic_needs(
                actor,
                HomeostaticNeeds::new(
                    worldwake_core::Permille::new(800).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                    worldwake_core::Permille::new(0).unwrap(),
                ),
            )
            .unwrap();
            txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
                .unwrap();
            txn.set_component_drive_thresholds(actor, DriveThresholds::default())
                .unwrap();
            txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
                .unwrap();
            commit_txn(txn);
            actor
        };

        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        register_needs_actions(&mut defs, &mut handlers);

        sync_all_beliefs(&mut world, actor, Tick(1));

        Self {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::with_entity(actor),
            rng: DeterministicRng::new(Seed([3; 32])),
            recipes: RecipeRegistry::new(),
            defs,
            handlers,
            driver: AgentTickDriver::new(),
            actor,
        }
    }

    fn with_full_action_registries(mut self) -> Self {
        let registries = build_full_action_registries(&self.recipes).unwrap();
        self.defs = registries.defs;
        self.handlers = registries.handlers;
        self
    }

    fn step_once(&mut self) -> worldwake_sim::TickStepResult {
        let mut controllers = AutonomousControllerRuntime::new(vec![&mut self.driver]);
        step_tick(
            &mut self.world,
            &mut self.event_log,
            &mut self.scheduler,
            &mut self.controller,
            &mut self.rng,
            TickStepServices {
                action_defs: &self.defs,
                action_handlers: &self.handlers,
                recipe_registry: &self.recipes,
                systems: &SystemDispatchTable::canonical_noop(),
                input_producer: Some(&mut controllers),
                action_trace: None,
                request_resolution_trace: None,
                politics_trace: None,
                perception_trace: None,
                institutional_knowledge_trace: None,
            },
        )
        .unwrap()
    }

    fn active_action_name(&self) -> Option<&str> {
        self.scheduler
            .active_actions()
            .values()
            .next()
            .and_then(|action| self.defs.get(action.def_id))
            .map(|def| def.name.as_str())
    }

    fn runtime(&self) -> Option<&crate::AgentDecisionRuntime> {
        self.driver.runtime_by_agent.get(&self.actor)
    }

    fn set_profile_fixture(&mut self, agent: EntityId, profile: ProfileFixture) {
        let mut txn = new_txn(&mut self.world, 0);
        txn.set_component_cognitive_profile(agent, cognitive(&profile))
            .expect("test harness should keep cognitive profiles writable");
        txn.set_component_execution_budget(agent, execution_budget(&profile))
            .expect("test harness should keep execution budgets writable");
        commit_txn(txn);
    }
}

#[test]
fn save_runtime_state_serializes_persisted_driver_state() {
    let agent = entity(700);
    let place = entity(701);
    let facility = entity(702);
    let authoritative = entity(703);
    let mut driver = AgentTickDriver::new();
    let mut runtime = AgentDecisionRuntime {
        current_step_index: 2,
        step_in_flight: true,
        last_effective_place: Some(place),
        last_facility_access_signature: vec![(facility, true, Some(ActionDefId(4)))],
        last_in_transit: true,
        dirty: DirtySet::NEEDS,
        last_priority_class: Some(GoalPriorityClass::Critical),
        last_frame_clear_reason: Some(worldwake_core::FrameClearReason::LostPlan),
        ..AgentDecisionRuntime::default()
    };
    let current_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    });
    let current_opportunity = OpportunityKey {
        goal_key: current_goal,
        anchor: OpportunityAnchor::Place(place),
    };
    runtime.current_plan = Some(PlannedPlan::new(
        current_opportunity,
        current_goal,
        vec![PlannedStep {
            def_id: ActionDefId(9),
            targets: vec![PlanningEntityRef::Authoritative(place)],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }],
        PlanTerminalKind::GoalSatisfied,
    ));
    runtime
        .materialization_bindings
        .bind(HypotheticalEntityId(11), authoritative);
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::TreatWounds { patient: agent }),
            OpportunityAnchor::Entity(agent),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: vec![
                ExhaustionInvalidationCondition::WoundsChanged,
                ExhaustionInvalidationCondition::TargetDead(agent),
            ],
            baseline: ExhaustionBaseline {
                position: Some(place),
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(100).unwrap(),
                    Permille::new(200).unwrap(),
                    Permille::new(300).unwrap(),
                    Permille::new(400).unwrap(),
                    Permille::new(500).unwrap(),
                )),
                commodity_quantities: vec![(CommodityKind::Medicine, Quantity(2))],
                unique_item_counts: vec![(UniqueItemKind::SimpleTool, 1)],
                steal_target_states: Vec::new(),
                wound_count: 2,
                hostile_count: 1,
            },
            next_retry_tick: Some(Tick(22)),
            consecutive_failures: 2,
        },
    );
    driver.runtime_by_agent.insert(agent, runtime);
    driver.semantics_cache = Some((1, BTreeMap::new()));
    driver.trace_sink = Some(crate::DecisionTraceSink::new());

    let bytes = driver.save_runtime_state().unwrap();
    let restored: super::AgentTickDriverState = bincode::deserialize(&bytes).unwrap();

    let restored_runtime = restored.runtime_by_agent.get(&agent).unwrap();
    assert_eq!(restored_runtime.current_step_index, 2);
    assert!(restored_runtime.step_in_flight);
    assert_eq!(restored_runtime.last_effective_place, Some(place));
    assert_eq!(
        restored_runtime
            .current_plan
            .as_ref()
            .expect("current plan should roundtrip")
            .opportunity,
        current_opportunity
    );
    assert_eq!(
        restored_runtime.last_facility_access_signature,
        vec![(facility, true, Some(ActionDefId(4)))]
    );
    assert!(restored_runtime.last_in_transit);
    assert_eq!(
        restored_runtime
            .materialization_bindings
            .resolve(HypotheticalEntityId(11)),
        Some(authoritative)
    );
    assert_eq!(
        restored_runtime.exhaustion_cache.get(&exhaustion_key(
            GoalKey::from(GoalKind::TreatWounds { patient: agent }),
            OpportunityAnchor::Entity(agent),
        )),
        Some(&crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: vec![
                ExhaustionInvalidationCondition::WoundsChanged,
                ExhaustionInvalidationCondition::TargetDead(agent),
            ],
            baseline: ExhaustionBaseline {
                position: Some(place),
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(100).unwrap(),
                    Permille::new(200).unwrap(),
                    Permille::new(300).unwrap(),
                    Permille::new(400).unwrap(),
                    Permille::new(500).unwrap(),
                )),
                commodity_quantities: vec![(CommodityKind::Medicine, Quantity(2))],
                unique_item_counts: vec![(UniqueItemKind::SimpleTool, 1)],
                steal_target_states: Vec::new(),
                wound_count: 2,
                hostile_count: 1,
            },
            next_retry_tick: Some(Tick(22)),
            consecutive_failures: 2,
        })
    );
    assert_eq!(restored_runtime.dirty, DirtySet::default());
    assert_eq!(restored_runtime.last_priority_class, None);
    assert_eq!(restored_runtime.last_frame_clear_reason, None);
}

#[test]
fn from_saved_runtime_restores_and_validates_driver_state() {
    let h = Harness::new(ControlSource::Ai);
    let live_place = h.world.topology().place_ids().next().unwrap();
    let dead_agent = entity(810);
    let dead_entity = entity(811);
    let dead_place = entity(812);
    let mut driver = AgentTickDriver::new();
    let mut runtime = AgentDecisionRuntime {
        last_effective_place: Some(dead_entity),
        last_facility_access_signature: vec![
            (dead_entity, true, None),
            (live_place, false, Some(ActionDefId(2))),
        ],
        last_priority_class: Some(GoalPriorityClass::High),
        last_frame_clear_reason: Some(worldwake_core::FrameClearReason::PlanFailed),
        ..AgentDecisionRuntime::default()
    };
    runtime
        .materialization_bindings
        .bind(HypotheticalEntityId(13), dead_entity);
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::LootCorpse {
                corpse: dead_entity,
            }),
            OpportunityAnchor::Entity(dead_entity),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: vec![ExhaustionInvalidationCondition::TargetDead(dead_entity)],
            baseline: ExhaustionBaseline {
                position: Some(dead_entity),
                needs: Some(HomeostaticNeeds::new_sated()),
                commodity_quantities: vec![(CommodityKind::Bread, Quantity(1))],
                unique_item_counts: Vec::new(),
                steal_target_states: Vec::new(),
                wound_count: 0,
                hostile_count: 0,
            },
            next_retry_tick: Some(Tick(15)),
            consecutive_failures: 1,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::TreatWounds { patient: h.actor }),
            OpportunityAnchor::Entity(h.actor),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: vec![
                ExhaustionInvalidationCondition::WoundsChanged,
                ExhaustionInvalidationCondition::TargetDead(h.actor),
            ],
            baseline: ExhaustionBaseline {
                position: Some(live_place),
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(10).unwrap(),
                    Permille::new(20).unwrap(),
                    Permille::new(30).unwrap(),
                    Permille::new(40).unwrap(),
                    Permille::new(50).unwrap(),
                )),
                commodity_quantities: vec![(CommodityKind::Medicine, Quantity(1))],
                unique_item_counts: vec![(UniqueItemKind::SimpleTool, 1)],
                steal_target_states: Vec::new(),
                wound_count: 1,
                hostile_count: 0,
            },
            next_retry_tick: Some(Tick(15)),
            consecutive_failures: 1,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            OpportunityAnchor::Place(dead_place),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
            baseline: ExhaustionBaseline {
                position: Some(live_place),
                needs: Some(HomeostaticNeeds::new_sated()),
                commodity_quantities: vec![(CommodityKind::Bread, Quantity(0))],
                unique_item_counts: Vec::new(),
                steal_target_states: Vec::new(),
                wound_count: 0,
                hostile_count: 0,
            },
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(GoalKey::from(GoalKind::Sleep), OpportunityAnchor::None),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
            baseline: ExhaustionBaseline {
                position: Some(live_place),
                needs: Some(HomeostaticNeeds::new_sated()),
                commodity_quantities: Vec::new(),
                unique_item_counts: Vec::new(),
                steal_target_states: Vec::new(),
                wound_count: 0,
                hostile_count: 0,
            },
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    driver.runtime_by_agent.insert(h.actor, runtime);
    driver
        .runtime_by_agent
        .insert(dead_agent, AgentDecisionRuntime::default());
    driver.semantics_cache = Some((3, BTreeMap::new()));
    driver.trace_sink = Some(crate::DecisionTraceSink::new());

    let restored =
        AgentTickDriver::from_saved_runtime(&driver.save_runtime_state().unwrap(), &h.world)
            .unwrap();

    assert!(!restored.runtime_by_agent.contains_key(&dead_agent));
    assert!(restored.semantics_cache.is_none());
    assert!(restored.trace_sink.is_none());

    let runtime = restored.runtime_by_agent.get(&h.actor).unwrap();
    assert_eq!(runtime.last_effective_place, None);
    assert_eq!(
        runtime.last_facility_access_signature,
        vec![(live_place, false, Some(ActionDefId(2)))]
    );
    assert_eq!(
        runtime
            .materialization_bindings
            .resolve(HypotheticalEntityId(13)),
        None
    );
    assert!(!runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::LootCorpse {
            corpse: dead_entity
        }),
        OpportunityAnchor::Entity(dead_entity),
    )));
    assert!(!runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }),
        OpportunityAnchor::Place(dead_place),
    )));
    assert_eq!(
        runtime.exhaustion_cache.get(&exhaustion_key(
            GoalKey::from(GoalKind::TreatWounds { patient: h.actor }),
            OpportunityAnchor::Entity(h.actor),
        )),
        Some(&crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: vec![
                ExhaustionInvalidationCondition::WoundsChanged,
                ExhaustionInvalidationCondition::TargetDead(h.actor),
            ],
            baseline: ExhaustionBaseline {
                position: Some(live_place),
                needs: Some(HomeostaticNeeds::new(
                    Permille::new(10).unwrap(),
                    Permille::new(20).unwrap(),
                    Permille::new(30).unwrap(),
                    Permille::new(40).unwrap(),
                    Permille::new(50).unwrap(),
                )),
                commodity_quantities: vec![(CommodityKind::Medicine, Quantity(1))],
                unique_item_counts: vec![(UniqueItemKind::SimpleTool, 1)],
                steal_target_states: Vec::new(),
                wound_count: 1,
                hostile_count: 0,
            },
            next_retry_tick: Some(Tick(15)),
            consecutive_failures: 1,
        })
    );
    assert_eq!(
        runtime.exhaustion_cache.get(&exhaustion_key(
            GoalKey::from(GoalKind::Sleep),
            OpportunityAnchor::None,
        )),
        Some(&crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
            baseline: ExhaustionBaseline {
                position: Some(live_place),
                needs: Some(HomeostaticNeeds::new_sated()),
                commodity_quantities: Vec::new(),
                unique_item_counts: Vec::new(),
                steal_target_states: Vec::new(),
                wound_count: 0,
                hostile_count: 0,
            },
            next_retry_tick: None,
            consecutive_failures: 0,
        })
    );
    assert_eq!(
        runtime.dirty,
        DirtySet::STRUCTURAL_MASK | DirtySet::SNAPSHOT_MASK | DirtySet::FRAME_MASK
    );
    assert_eq!(runtime.last_priority_class, None);
    assert_eq!(runtime.last_frame_clear_reason, None);
}

#[test]
fn from_saved_runtime_rejects_invalid_bytes() {
    let h = Harness::new(ControlSource::Ai);

    let error = AgentTickDriver::from_saved_runtime(&[], &h.world)
        .err()
        .expect("invalid bytes should fail restore");

    assert!(matches!(error, SaveError::RuntimeDeserialization(_)));
}

#[test]
fn post_load_validate_prunes_dead_runtime_references_and_marks_runtime_dirty() {
    let mut h = Harness::new(ControlSource::Ai);
    let live_place = h.world.topology().place_ids().next().unwrap();
    let dead_agent = entity(800);
    let dead_entity = entity(801);
    let dead_place = entity(802);
    let mut runtime = AgentDecisionRuntime {
        last_effective_place: Some(dead_entity),
        last_facility_access_signature: vec![
            (dead_entity, true, None),
            (live_place, false, Some(ActionDefId(2))),
        ],
        last_priority_class: Some(GoalPriorityClass::High),
        last_frame_clear_reason: Some(worldwake_core::FrameClearReason::PlanFailed),
        ..AgentDecisionRuntime::default()
    };
    runtime
        .materialization_bindings
        .bind(HypotheticalEntityId(12), dead_entity);
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::LootCorpse {
                corpse: dead_entity,
            }),
            OpportunityAnchor::Entity(dead_entity),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::TreatWounds { patient: h.actor }),
            OpportunityAnchor::Entity(h.actor),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(
            GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            OpportunityAnchor::Place(dead_place),
        ),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::FrontierExhausted,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    runtime.exhaustion_cache.insert(
        exhaustion_key(GoalKey::from(GoalKind::Sleep), OpportunityAnchor::None),
        crate::ExhaustionEntry {
            retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
            invalidation_conditions: Vec::new(),
            baseline: crate::ExhaustionBaseline::default(),
            next_retry_tick: None,
            consecutive_failures: 0,
        },
    );
    h.driver.runtime_by_agent.insert(h.actor, runtime);
    h.driver
        .runtime_by_agent
        .insert(dead_agent, AgentDecisionRuntime::default());
    h.driver.semantics_cache = Some((3, BTreeMap::new()));

    h.driver.post_load_validate(&h.world);

    assert!(!h.driver.runtime_by_agent.contains_key(&dead_agent));
    assert!(h.driver.semantics_cache.is_none());

    let runtime = h.driver.runtime_by_agent.get(&h.actor).unwrap();
    assert_eq!(runtime.last_effective_place, None);
    assert_eq!(
        runtime.last_facility_access_signature,
        vec![(live_place, false, Some(ActionDefId(2)))]
    );
    assert_eq!(
        runtime
            .materialization_bindings
            .resolve(HypotheticalEntityId(12)),
        None
    );
    assert!(!runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::LootCorpse {
            corpse: dead_entity
        }),
        OpportunityAnchor::Entity(dead_entity),
    )));
    assert!(!runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }),
        OpportunityAnchor::Place(dead_place),
    )));
    assert!(runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::TreatWounds { patient: h.actor }),
        OpportunityAnchor::Entity(h.actor),
    )));
    assert!(runtime.exhaustion_cache.contains_key(&exhaustion_key(
        GoalKey::from(GoalKind::Sleep),
        OpportunityAnchor::None,
    )));
    assert_eq!(
        runtime.dirty,
        DirtySet::STRUCTURAL_MASK | DirtySet::SNAPSHOT_MASK | DirtySet::FRAME_MASK
    );
    assert_eq!(runtime.last_priority_class, None);
    assert_eq!(runtime.last_frame_clear_reason, None);
}

fn cargo_topology(origin: EntityId, destination: EntityId) -> Topology {
    let mut topology = Topology::new();
    topology
        .add_place(
            origin,
            Place {
                name: "Origin".to_string(),
                capacity: None,
                tags: BTreeSet::default(),
            },
        )
        .unwrap();
    topology
        .add_place(
            destination,
            Place {
                name: "Destination".to_string(),
                capacity: None,
                tags: BTreeSet::default(),
            },
        )
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(1), origin, destination, 2, None).unwrap())
        .unwrap();
    topology
        .add_edge(TravelEdge::new(TravelEdgeId(2), destination, origin, 2, None).unwrap())
        .unwrap();
    topology
}

fn seed_cargo_harness_actor(
    world: &mut World,
    origin: EntityId,
    destination: EntityId,
    possessed: bool,
) -> (EntityId, EntityId) {
    let mut txn = new_txn(world, 1);
    let actor = txn.create_agent("Mira", ControlSource::Ai).unwrap();
    let water = txn
        .create_item_lot(CommodityKind::Bread, Quantity(3))
        .unwrap();
    txn.set_ground_location(actor, origin).unwrap();
    txn.set_ground_location(water, origin).unwrap();
    if possessed {
        txn.set_possessor(water, actor).unwrap();
    } else {
        txn.set_owner(water, actor).unwrap();
    }
    txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
        .unwrap();
    txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
        .unwrap();
    txn.set_component_drive_thresholds(actor, DriveThresholds::default())
        .unwrap();
    txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
        .unwrap();
    txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(3)))
        .unwrap();
    let (facility, _stock_container, _display_container) = txn
        .create_merchant_facility(destination, actor, LoadUnits(200), None)
        .unwrap();
    txn.set_component_merchandise_profile(
        actor,
        MerchandiseProfile {
            sale_kinds: [CommodityKind::Bread].into_iter().collect(),
            home_facility: Some(facility),
        },
    )
    .unwrap();
    txn.set_component_demand_memory(
        actor,
        DemandMemory {
            observations: vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(2),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        },
    )
    .unwrap();
    commit_txn(txn);
    (actor, water)
}

fn cargo_harness(possessed: bool) -> (Harness, EntityId, EntityId, EntityId) {
    let origin = entity(1);
    let destination = entity(2);
    let mut world = World::new(cargo_topology(origin, destination)).unwrap();
    let actor = seed_cargo_harness_actor(&mut world, origin, destination, possessed);
    let recipes = RecipeRegistry::new();
    let registries = build_full_action_registries(&recipes).unwrap();

    sync_all_beliefs(&mut world, actor.0, Tick(1));

    let mut harness = Harness {
        world,
        event_log: EventLog::new(),
        scheduler: Scheduler::new(SystemManifest::canonical()),
        controller: ControllerState::with_entity(actor.0),
        rng: DeterministicRng::new(Seed([9; 32])),
        recipes,
        defs: registries.defs,
        handlers: registries.handlers,
        driver: AgentTickDriver::new(),
        actor: actor.0,
    };
    harness.set_profile_fixture(
        actor.0,
        ProfileFixture {
            max_plan_depth: 3,
            ..ProfileFixture::default()
        },
    );

    (harness, actor.1, origin, destination)
}

fn step_until(harness: &mut Harness, max_ticks: usize, predicate: impl Fn(&Harness) -> bool) {
    for _ in 0..max_ticks {
        if predicate(harness) {
            return;
        }
        let _ = harness.step_once();
    }
    assert!(
        predicate(harness),
        "condition not met within {max_ticks} ticks"
    );
}

fn new_txn(world: &mut World, tick: u64) -> WorldTxn<'_> {
    WorldTxn::new(
        world,
        Tick(tick),
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    )
}

fn commit_txn(txn: WorldTxn<'_>) {
    let mut event_log = EventLog::new();
    let _ = txn.commit(&mut event_log);
}

fn sync_all_beliefs(world: &mut World, observer: EntityId, observed_tick: Tick) {
    let snapshots = world
        .entities()
        .filter(|entity| *entity != observer)
        .filter_map(|entity| {
            build_believed_entity_state(
                world,
                entity,
                observed_tick,
                PerceptionSource::DirectObservation,
            )
            .map(|state| (entity, state))
        })
        .collect::<Vec<_>>();
    let mut store = world
        .get_component_agent_belief_store(observer)
        .cloned()
        .expect("observer must have AgentBeliefStore");
    store.known_entities.clear();
    for (entity, state) in snapshots {
        store.update_entity(entity, state);
    }
    let mut txn = WorldTxn::new(
        world,
        observed_tick,
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.set_component_agent_belief_store(observer, store)
        .expect("observer belief store should remain writable");
    commit_txn(txn);
}

fn sync_selected_beliefs(
    world: &mut World,
    observer: EntityId,
    entities: &[EntityId],
    observed_tick: Tick,
    source: PerceptionSource,
) {
    let mut store = world
        .get_component_agent_belief_store(observer)
        .cloned()
        .expect("observer must have AgentBeliefStore");
    store.known_entities.clear();
    for entity in entities {
        if let Some(state) = build_believed_entity_state(world, *entity, observed_tick, source) {
            store.update_entity(*entity, state);
        }
    }
    let mut txn = WorldTxn::new(
        world,
        observed_tick,
        CauseRef::Bootstrap,
        None,
        None,
        VisibilitySpec::SamePlace,
        WitnessData::default(),
    );
    txn.set_component_agent_belief_store(observer, store)
        .expect("observer belief store should remain writable");
    commit_txn(txn);
}

fn hungry_acquisition_harness() -> (Harness, EntityId, EntityId, EntityId, EntityId) {
    let origin = entity(11);
    let destination = entity(12);
    let mut world = World::new(cargo_topology(origin, destination)).unwrap();
    let (actor, seller, bread) = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
        let seller = txn.create_agent("Seller", ControlSource::Ai).unwrap();
        let bread = txn
            .create_item_lot(CommodityKind::Bread, Quantity(3))
            .unwrap();
        txn.set_ground_location(actor, origin).unwrap();
        txn.set_ground_location(seller, origin).unwrap();
        // Create facility for seller and stage the bread in display container.
        {
            use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};
            let (facility, _stock, display) = txn
                .create_merchant_facility(origin, seller, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display = display.unwrap();
            txn.put_into_container(bread, display).unwrap();
            txn.set_component_stock_assignment(
                bread,
                StockAssignment {
                    facility,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(
                bread,
                worldwake_core::SaleListing {
                    listed_at: worldwake_core::Tick(0),
                },
            )
            .unwrap();
        }
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(
                Permille::new(800).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
            .unwrap();
        txn.set_component_perception_profile(
            actor,
            PerceptionProfile {
                entity_memory_capacity: 12,
                entity_claim_capacity: 12,
                memory_retention_ticks: 64,
                observation_fidelity: Permille::new(1000).unwrap(),
                confidence_policy: BeliefConfidencePolicy::default(),
                institutional_memory_capacity: 20,
                consultation_speed_factor: Permille::new(500).unwrap(),
                contradiction_tolerance: Permille::new(300).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            seller,
            MerchandiseProfile {
                sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                home_facility: Some(origin),
            },
        )
        .unwrap();
        commit_txn(txn);
        (actor, seller, bread)
    };

    let mut defs = ActionDefRegistry::new();
    let mut handlers = ActionHandlerRegistry::new();
    register_needs_actions(&mut defs, &mut handlers);

    (
        Harness {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::with_entity(actor),
            rng: DeterministicRng::new(Seed([5; 32])),
            recipes: RecipeRegistry::new(),
            defs,
            handlers,
            driver: AgentTickDriver::new(),
            actor,
        },
        seller,
        origin,
        destination,
        bread,
    )
}

fn stale_remote_acquisition_harness() -> (Harness, EntityId, EntityId, EntityId, EntityId, EntityId)
{
    let origin = entity(21);
    let destination = entity(22);
    let mut world = World::new(cargo_topology(origin, destination)).unwrap();
    let (actor, seller, local_witness, bread) = {
        let mut txn = new_txn(&mut world, 0);
        let actor = txn.create_agent("Hungry", ControlSource::Ai).unwrap();
        let seller = txn.create_agent("RemoteSeller", ControlSource::Ai).unwrap();
        let local_witness = txn.create_agent("Witness", ControlSource::Ai).unwrap();
        let bread = txn
            .create_item_lot(CommodityKind::Bread, Quantity(3))
            .unwrap();
        txn.set_ground_location(actor, origin).unwrap();
        txn.set_ground_location(local_witness, origin).unwrap();
        txn.set_ground_location(seller, destination).unwrap();
        // Create facility for seller and stage the bread in display container.
        {
            use worldwake_core::{LoadUnits, StockAssignment, StockAssignmentKind};
            let (facility, _stock, display) = txn
                .create_merchant_facility(destination, seller, LoadUnits(200), Some(LoadUnits(100)))
                .unwrap();
            let display = display.unwrap();
            txn.put_into_container(bread, display).unwrap();
            txn.set_component_stock_assignment(
                bread,
                StockAssignment {
                    facility,
                    kind: StockAssignmentKind::Displayed,
                },
            )
            .unwrap();
            txn.set_component_sale_listing(
                bread,
                worldwake_core::SaleListing {
                    listed_at: worldwake_core::Tick(0),
                },
            )
            .unwrap();
        }
        txn.set_component_homeostatic_needs(
            actor,
            HomeostaticNeeds::new(
                Permille::new(800).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        )
        .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
            .unwrap();
        txn.set_component_perception_profile(
            actor,
            PerceptionProfile {
                entity_memory_capacity: 12,
                entity_claim_capacity: 12,
                memory_retention_ticks: 4,
                observation_fidelity: Permille::new(1000).unwrap(),
                confidence_policy: BeliefConfidencePolicy::default(),
                institutional_memory_capacity: 20,
                consultation_speed_factor: Permille::new(500).unwrap(),
                contradiction_tolerance: Permille::new(300).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_merchandise_profile(
            seller,
            MerchandiseProfile {
                sale_kinds: [CommodityKind::Bread].into_iter().collect(),
                home_facility: Some(destination),
            },
        )
        .unwrap();
        commit_txn(txn);
        (actor, seller, local_witness, bread)
    };

    let mut defs = ActionDefRegistry::new();
    let mut handlers = ActionHandlerRegistry::new();
    register_needs_actions(&mut defs, &mut handlers);

    sync_selected_beliefs(
        &mut world,
        actor,
        &[seller, bread],
        Tick(0),
        PerceptionSource::Inference,
    );

    (
        Harness {
            world,
            event_log: EventLog::new(),
            scheduler: Scheduler::new(SystemManifest::canonical()),
            controller: ControllerState::with_entity(actor),
            rng: DeterministicRng::new(Seed([7; 32])),
            recipes: RecipeRegistry::new(),
            defs,
            handlers,
            driver: AgentTickDriver::new(),
            actor,
        },
        seller,
        local_witness,
        origin,
        destination,
        bread,
    )
}

fn ranked_goals_at(harness: &mut Harness, tick: Tick) -> Vec<RankedGoal> {
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let runtime = harness
        .driver
        .runtime_by_agent
        .entry(harness.actor)
        .or_default();
    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick,
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    )
    .ranked
}

fn has_goal(ranked: &[RankedGoal], goal: GoalKind) -> bool {
    let key = GoalKey::from(goal);
    ranked.iter().any(|candidate| candidate.grounded.key == key)
}

fn run_same_place_observation(
    harness: &mut Harness,
    tick: Tick,
    place: EntityId,
    observed_actor: EntityId,
) {
    let _ = harness
        .event_log
        .emit(PendingEvent::from_payload(EventPayload {
            tick,
            cause: CauseRef::Bootstrap,
            actor_id: Some(observed_actor),
            action_name: None,
            target_ids: vec![observed_actor],
            evidence: Vec::new(),
            place_id: Some(place),
            state_deltas: Vec::new(),
            observed_entities: BTreeMap::new(),
            visibility: VisibilitySpec::SamePlace,
            witness_data: WitnessData::default(),
            tags: BTreeSet::new(),
        }));
    let active_actions = std::collections::BTreeMap::new();
    perception_system(SystemExecutionContext {
        world: &mut harness.world,
        event_log: &mut harness.event_log,
        rng: &mut harness.rng,
        active_actions: &active_actions,
        action_defs: &harness.defs,
        politics_trace: None,
        perception_trace: None,
        tick,
        system_id: SystemId::Perception,
    })
    .unwrap();
}

fn run_perception_tick(harness: &mut Harness, tick: Tick) {
    let active_actions = std::collections::BTreeMap::new();
    perception_system(SystemExecutionContext {
        world: &mut harness.world,
        event_log: &mut harness.event_log,
        rng: &mut harness.rng,
        active_actions: &active_actions,
        action_defs: &harness.defs,
        politics_trace: None,
        perception_trace: None,
        tick,
        system_id: SystemId::Perception,
    })
    .unwrap();
}

fn relocate_entity(world: &mut World, entity: EntityId, destination: EntityId, tick: Tick) {
    let mut txn = new_txn(world, tick.0);
    txn.set_ground_location(entity, destination).unwrap();
    commit_txn(txn);
}

fn kill_entity(world: &mut World, entity: EntityId, tick: Tick) {
    let mut txn = new_txn(world, tick.0);
    txn.set_component_dead_at(entity, DeadAt(tick)).unwrap();
    commit_txn(txn);
}

fn entity(slot: u32) -> EntityId {
    EntityId {
        slot,
        generation: 1,
    }
}

fn pm(value: u16) -> Permille {
    Permille::new(value).unwrap()
}

fn harvest_apple_recipe() -> RecipeDefinition {
    RecipeDefinition {
        name: "Harvest Apples".to_string(),
        inputs: vec![],
        outputs: vec![(CommodityKind::Apple, Quantity(2))],
        work_ticks: NonZeroU32::new(3).unwrap(),
        required_workstation_tag: Some(WorkstationTag::OrchardRow),
        required_tool_kinds: vec![],
        body_cost_per_tick: BodyCostPerTick::new(pm(3), pm(2), pm(5), pm(0), pm(1)),
    }
}

struct ExclusiveQueueHarness {
    world: World,
    recipes: RecipeRegistry,
    defs: ActionDefRegistry,
    handlers: ActionHandlerRegistry,
    scheduler: Scheduler,
    actor: EntityId,
    orchard_farm: EntityId,
    orchard_row: EntityId,
}

fn build_exclusive_queue_harness() -> ExclusiveQueueHarness {
    let orchard_farm =
        worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::OrchardFarm);
    let mut recipes = RecipeRegistry::new();
    recipes.register(harvest_apple_recipe());
    let registries = build_full_action_registries(&recipes).unwrap();
    let mut world = World::new(build_prototype_world()).unwrap();
    let (actor, orchard_row) = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Merchant", ControlSource::Ai).unwrap();
        let orchard_row = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(actor, orchard_farm).unwrap();
        txn.set_ground_location(orchard_row, orchard_farm).unwrap();
        txn.set_component_homeostatic_needs(actor, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_deprivation_exposure(actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(actor, MetabolismProfile::default())
            .unwrap();
        txn.set_component_carry_capacity(actor, CarryCapacity(LoadUnits(50)))
            .unwrap();
        txn.set_component_known_recipes(actor, KnownRecipes::with([RecipeId(0)]))
            .unwrap();
        txn.set_component_workstation_marker(
            orchard_row,
            WorkstationMarker(WorkstationTag::OrchardRow),
        )
        .unwrap();
        txn.set_component_resource_source(
            orchard_row,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(10),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        )
        .unwrap();
        txn.set_component_contention_policy(
            orchard_row,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(orchard_row, ContentionQueue::default())
            .unwrap();
        commit_txn(txn);
        (actor, orchard_row)
    };

    sync_all_beliefs(&mut world, actor, Tick(1));

    ExclusiveQueueHarness {
        world,
        recipes,
        defs: registries.defs,
        handlers: registries.handlers,
        scheduler: Scheduler::new(SystemManifest::canonical()),
        actor,
        orchard_farm,
        orchard_row,
    }
}

fn set_local_queue_state(
    world: &mut World,
    actor: EntityId,
    facility: EntityId,
    queued_at: u64,
    grant_action: Option<ActionDefId>,
) {
    let mut txn = new_txn(world, queued_at.max(1));
    let mut queue = txn
        .get_component_contention_queue(facility)
        .cloned()
        .unwrap_or_default();
    queue.waiting.clear();
    queue.granted = None;
    if let Some(action_def) = grant_action {
        queue.granted = Some(ContentionGrant {
            actor,
            intended_action: action_def,
            granted_at: Tick(queued_at),
            expires_at: Tick(queued_at + 3),
        });
    } else {
        queue
            .enqueue(actor, ActionDefId(77), Tick(queued_at), None)
            .unwrap();
    }
    txn.set_component_contention_queue(facility, queue).unwrap();
    commit_txn(txn);
    sync_all_beliefs(world, actor, Tick(queued_at.max(1)));
}

fn clear_local_queue_state(world: &mut World, actor: EntityId, facility: EntityId, tick: u64) {
    let mut txn = new_txn(world, tick.max(1));
    let mut queue = txn
        .get_component_contention_queue(facility)
        .cloned()
        .unwrap_or_default();
    queue.waiting.clear();
    queue.granted = None;
    txn.set_component_contention_queue(facility, queue).unwrap();
    commit_txn(txn);
    sync_all_beliefs(world, actor, Tick(tick.max(1)));
}

fn add_local_queued_facility(world: &mut World, actor: EntityId, queued_at: u64) -> EntityId {
    let place = world.effective_place(actor).unwrap();
    let facility = {
        let mut txn = new_txn(world, queued_at.max(1));
        let facility = txn.create_entity(EntityKind::Facility);
        txn.set_ground_location(facility, place).unwrap();
        txn.set_component_contention_policy(
            facility,
            ContentionPolicy {
                grant_hold_ticks: NonZeroU32::new(3).unwrap(),
                auto_promote: true,
                max_waiters: None,
            },
        )
        .unwrap();
        txn.set_component_contention_queue(facility, ContentionQueue::default())
            .unwrap();
        commit_txn(txn);
        facility
    };
    set_local_queue_state(world, actor, facility, queued_at, None);
    facility
}

fn barrier_step() -> PlannedStep {
    PlannedStep {
        def_id: ActionDefId(8),
        targets: vec![PlanningEntityRef::Authoritative(entity(11))],
        payload_override: None,
        op_kind: PlannerOpKind::Trade,
        estimated_ticks: 3,
        is_materialization_barrier: true,
        expected_materializations: Vec::new(),
    }
}

fn travel_step(def_id: u32, target: EntityId) -> PlannedStep {
    PlannedStep {
        def_id: ActionDefId(def_id),
        targets: vec![PlanningEntityRef::Authoritative(target)],
        payload_override: None,
        op_kind: PlannerOpKind::Travel,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
    }
}

fn hypothetical_step(def_id: u32, hypothetical: u32) -> PlannedStep {
    PlannedStep {
        def_id: ActionDefId(def_id),
        targets: vec![PlanningEntityRef::Hypothetical(
            crate::HypotheticalEntityId(hypothetical),
        )],
        payload_override: None,
        op_kind: PlannerOpKind::MoveCargo,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: vec![ExpectedMaterialization {
            tag: MaterializationTag::SplitOffLot,
            hypothetical_id: crate::HypotheticalEntityId(hypothetical),
        }],
    }
}

fn active_runtime(goal: GoalKind) -> crate::AgentDecisionRuntime {
    let goal = GoalKey::from(goal);
    crate::AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            default_opportunity(goal),
            goal,
            vec![barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        )),
        current_step_index: 0,
        step_in_flight: false,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    }
}

fn ranked_goal(
    goal: GoalKind,
    evidence_entities: impl IntoIterator<Item = EntityId>,
    evidence_places: impl IntoIterator<Item = EntityId>,
) -> RankedGoal {
    RankedGoal {
        grounded: crate::GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(goal),
            evidence_entities: evidence_entities.into_iter().collect(),
            evidence_places: evidence_places.into_iter().collect(),
        },
        priority_class: crate::GoalPriorityClass::Medium,
        motive_score: 500,
        provenance: None,
        source_reliability_discount: None,
        competition_discount: None,
        feasibility: crate::feasibility::FeasibilityHint::Uncertain,
    }
}

#[derive(Default)]
struct QueuePatienceBeliefView {
    place: Option<EntityId>,
    facilities_at_place: Vec<EntityId>,
    queue_join_ticks: std::collections::BTreeMap<EntityId, Tick>,
    grants: std::collections::BTreeMap<EntityId, ContentionGrant>,
    patience_ticks: Option<NonZeroU32>,
}

impl RuntimeBeliefView for QueuePatienceBeliefView {
    fn is_alive(&self, _entity: EntityId) -> bool {
        true
    }
    fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
        None
    }
    fn effective_place(&self, _entity: EntityId) -> Option<EntityId> {
        self.place
    }
    fn is_in_transit(&self, _entity: EntityId) -> bool {
        false
    }
    fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
        self.facilities_at_place.clone()
    }
    fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
        Vec::new()
    }
    fn adjacent_places(&self, _place: EntityId) -> Vec<EntityId> {
        Vec::new()
    }
    fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
        false
    }
    fn unique_item_count(&self, _holder: EntityId, _kind: worldwake_core::UniqueItemKind) -> u32 {
        0
    }
    fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
        Quantity(0)
    }
    fn controlled_commodity_quantity_at_place(
        &self,
        _agent: EntityId,
        _place: EntityId,
        _commodity: CommodityKind,
    ) -> Quantity {
        Quantity(0)
    }
    fn local_controlled_lots_for(
        &self,
        _agent: EntityId,
        _place: EntityId,
        _commodity: CommodityKind,
    ) -> Vec<EntityId> {
        Vec::new()
    }
    fn item_lot_commodity(&self, _entity: EntityId) -> Option<CommodityKind> {
        None
    }
    fn item_lot_consumable_profile(
        &self,
        _entity: EntityId,
    ) -> Option<worldwake_core::CommodityConsumableProfile> {
        None
    }
    fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
        None
    }
    fn direct_possessor(&self, _entity: EntityId) -> Option<EntityId> {
        None
    }
    fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
        None
    }
    fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
        None
    }
    fn has_contention_policy(&self, entity: EntityId) -> bool {
        self.facilities_at_place.contains(&entity)
    }
    fn facility_queue_position(&self, facility: EntityId, _actor: EntityId) -> Option<u32> {
        self.queue_join_ticks.contains_key(&facility).then_some(0)
    }
    fn facility_grant(&self, facility: EntityId) -> Option<&ContentionGrant> {
        self.grants.get(&facility)
    }
    fn facility_queue_join_tick(&self, facility: EntityId, _actor: EntityId) -> Option<Tick> {
        self.queue_join_ticks.get(&facility).copied()
    }
    fn facility_queue_patience_ticks(&self, _agent: EntityId) -> Option<NonZeroU32> {
        self.patience_ticks
    }
    fn place_has_tag(&self, _place: EntityId, _tag: worldwake_core::PlaceTag) -> bool {
        false
    }
    fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
        None
    }
    fn has_production_job(&self, _entity: EntityId) -> bool {
        false
    }
    fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
        false
    }
    fn has_control(&self, _entity: EntityId) -> bool {
        false
    }
    fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
        None
    }
    fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
        None
    }
    fn reservation_conflicts(&self, _entity: EntityId, _range: worldwake_core::TickRange) -> bool {
        false
    }
    fn reservation_ranges(&self, _entity: EntityId) -> Vec<worldwake_core::TickRange> {
        Vec::new()
    }
    fn is_dead(&self, _entity: EntityId) -> bool {
        false
    }
    fn is_incapacitated(&self, _entity: EntityId) -> bool {
        false
    }
    fn has_wounds(&self, _entity: EntityId) -> bool {
        false
    }
    fn homeostatic_needs(&self, _agent: EntityId) -> Option<HomeostaticNeeds> {
        None
    }
    fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
        None
    }
    fn belief_confidence_policy(&self, _agent: EntityId) -> worldwake_core::BeliefConfidencePolicy {
        worldwake_core::BeliefConfidencePolicy::default()
    }
    fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
        None
    }
    fn trade_disposition_profile(
        &self,
        _agent: EntityId,
    ) -> Option<worldwake_core::TradeDispositionProfile> {
        None
    }
    fn intention_disposition_profile(
        &self,
        _agent: EntityId,
    ) -> Option<IntentionDispositionProfile> {
        None
    }
    fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
        false
    }
    fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
        None
    }
    fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
        Vec::new()
    }
    fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
        Vec::new()
    }
    fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
        Vec::new()
    }
    fn listed_sale_lots_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
        Vec::new()
    }
    fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
        None
    }
    fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
        Vec::new()
    }
    fn matching_workstations_at(&self, _place: EntityId, _tag: WorkstationTag) -> Vec<EntityId> {
        Vec::new()
    }
    fn resource_sources_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
        Vec::new()
    }
    fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
        Vec::new()
    }
    fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
        None
    }
    fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
        Vec::new()
    }
    fn in_transit_state(&self, _entity: EntityId) -> Option<worldwake_core::InTransitOnEdge> {
        None
    }
    fn adjacent_places_with_travel_ticks(&self, _place: EntityId) -> Vec<(EntityId, NonZeroU32)> {
        Vec::new()
    }
    fn estimate_duration(
        &self,
        _actor: EntityId,
        _duration: &DurationExpr,
        _targets: &[EntityId],
        _payload: &worldwake_sim::ActionPayload,
    ) -> Option<ActionDuration> {
        None
    }
}

#[test]
fn effective_goal_switch_margin_uses_route_margin_for_any_intention_frame() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_intention_disposition_profile(
            actor,
            IntentionDispositionProfile {
                commitment_switch_margin: Permille::new(300).unwrap(),
                domain_patience: BTreeMap::new(),
                default_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        actor
    };
    let budget = ProfileFixture::default();
    let view = PerAgentBeliefView::from_world(actor, &world);
    let jc_active = Some(IntentionFrame {
        goal: GoalKey::from(GoalKind::Sleep),
        domain: IntentionDomain::Travel { destination: place },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(7),
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 10,
    });

    assert_eq!(
        effective_goal_switch_margin(&view, actor, jc_active.as_ref(), &cognitive(&budget)),
        Permille::new(300).unwrap()
    );
    // Planless commitment (same jc, no plan on runtime) still has route margin.
    assert_eq!(
        effective_goal_switch_margin(&view, actor, jc_active.as_ref(), &cognitive(&budget)),
        Permille::new(300).unwrap()
    );
    // No commitment => budget default.
    assert_eq!(
        effective_goal_switch_margin(&view, actor, None, &cognitive(&budget)),
        budget.switch_margin
    );
}

#[test]
#[should_panic(expected = "lacks IntentionDispositionProfile")]
fn effective_goal_switch_margin_panics_when_committed_agent_lacks_intention_profile() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        commit_txn(txn);
        actor
    };
    {
        let mut txn = new_txn(&mut world, 2);
        txn.clear_component_intention_disposition_profile(actor)
            .unwrap();
        commit_txn(txn);
    }
    let budget = ProfileFixture::default();
    let view = PerAgentBeliefView::from_world(actor, &world);
    let jc_active = Some(IntentionFrame {
        goal: GoalKey::from(GoalKind::Sleep),
        domain: IntentionDomain::Travel { destination: place },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(7),
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 10,
    });

    let _ = effective_goal_switch_margin(&view, actor, jc_active.as_ref(), &cognitive(&budget));
}

#[test]
fn grant_arrival_marks_runtime_dirty_from_facility_access_snapshot() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
    let mut runtime = active_runtime(GoalKind::Sleep);
    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);

    set_local_queue_state(
        &mut harness.world,
        harness.actor,
        facility,
        2,
        Some(ActionDefId(77)),
    );

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &UtilityProfile::default(),
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(!runtime.dirty.is_empty());
}

#[test]
fn queue_patience_exhaustion_marks_runtime_dirty() {
    let agent = entity(1);
    let place = entity(2);
    let facility = entity(3);
    let view = QueuePatienceBeliefView {
        place: Some(place),
        facilities_at_place: vec![facility],
        queue_join_ticks: [(facility, Tick(1))].into_iter().collect(),
        patience_ticks: NonZeroU32::new(3),
        ..QueuePatienceBeliefView::default()
    };

    assert!(facility_queue_patience_exhausted(&view, agent, Tick(4)));
}

#[test]
fn abandon_expired_facility_queues_removes_actor_from_authoritative_queue() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);

    assert!(
        abandon_expired_facility_queues_with_limit(
            &mut harness.world,
            &mut harness.event_log,
            harness.actor,
            Tick(4),
            NonZeroU32::new(3).unwrap(),
            ProfileFixture::default().structural_block_ticks,
        )
        .unwrap()
    );

    let queue = harness
        .world
        .get_component_contention_queue(facility)
        .expect("facility queue should remain attached");
    assert_eq!(
        queue.position_of(harness.actor),
        None,
        "Patience expiry should remove the actor from authoritative queue state"
    );
}

#[test]
fn abandoned_queue_then_records_standard_exclusive_facility_blocker() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
    let goal = GoalKey::from(GoalKind::RestockCommodity {
        commodity: CommodityKind::Apple,
    });
    let mut runtime = crate::AgentDecisionRuntime::default();
    // Set facility queue intents as a component on the World.
    {
        let mut txn = new_txn(&mut harness.world, 1);
        txn.set_component_contention_intents(
            harness.actor,
            ContentionIntents {
                intents: [(
                    facility,
                    QueuedContentionIntent {
                        goal_key: goal,
                        intended_action: ActionDefId(77),
                    },
                )]
                .into_iter()
                .collect(),
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

    assert!(
        abandon_expired_facility_queues_with_limit(
            &mut harness.world,
            &mut harness.event_log,
            harness.actor,
            Tick(4),
            NonZeroU32::new(3).unwrap(),
            ProfileFixture::default().structural_block_ticks,
        )
        .unwrap()
    );

    let blocked = harness
        .world
        .get_component_blocked_intent_memory(harness.actor)
        .expect("queue abandonment should persist a blocked intent immediately");
    assert_eq!(blocked.intents.len(), 1);
    let intent = blocked.intents.values().next().unwrap();
    assert_eq!(
        intent.blocking_fact,
        BlockingFact::ExclusiveFacilityUnavailable
    );
    assert_eq!(intent.blocker_key.target, Some(facility));
    assert_eq!(intent.blocker_key.action_def, Some(ActionDefId(77)));
    assert!(
        harness
            .world
            .get_component_contention_intents(harness.actor)
            .is_none_or(|intents| intents.intents.is_empty())
    );
}

#[test]
fn missing_queue_patience_profile_does_not_mark_runtime_dirty() {
    let agent = entity(1);
    let place = entity(2);
    let facility = entity(3);
    let view = QueuePatienceBeliefView {
        place: Some(place),
        facilities_at_place: vec![facility],
        queue_join_ticks: [(facility, Tick(1))].into_iter().collect(),
        patience_ticks: None,
        ..QueuePatienceBeliefView::default()
    };

    assert!(!facility_queue_patience_exhausted(&view, agent, Tick(10)));
}

#[test]
fn grant_arrival_replan_can_select_direct_harvest_step() {
    let mut harness = build_exclusive_queue_harness();
    let harvest_action = harness
        .defs
        .iter()
        .find(|def| def.name == "harvest:Harvest Apples")
        .map(|def| def.id)
        .expect("harvest action should be registered");
    let mut txn = new_txn(&mut harness.world, 1);
    let mut queue = txn
        .get_component_contention_queue(harness.orchard_row)
        .cloned()
        .expect("exclusive orchard should have queue state");
    queue
        .enqueue(harness.actor, harvest_action, Tick(1), None)
        .unwrap();
    txn.set_component_contention_queue(harness.orchard_row, queue)
        .unwrap();
    commit_txn(txn);

    let mut runtime = active_runtime(GoalKind::Sleep);
    let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

    set_local_queue_state(
        &mut harness.world,
        harness.actor,
        harness.orchard_row,
        2,
        Some(harvest_action),
    );

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &UtilityProfile::default(),
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );
    assert!(!runtime.dirty.is_empty());

    let goal = ranked_goal(
        GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        },
        [harness.orchard_row],
        [harness.orchard_farm],
    );
    let semantics = build_semantics_table(&harness.defs);
    let mut jc = None;
    let mut active_goal = None;
    let (next_step, next_step_valid) = plan_and_validate_next_step(
        &harness.world,
        &harness.scheduler,
        &mut runtime,
        &mut active_goal,
        &mut jc,
        harness.actor,
        std::slice::from_ref(&goal),
        &blocked,
        ProfileFixture::default().switch_margin,
        ProfileFixture::default().switch_margin,
        UtilityProfile::default().side_benefit_weight,
        Tick(2),
        &cognitive(&ProfileFixture::default()),
        &execution_budget(&ProfileFixture::default()),
        &semantics,
        &harness.defs,
        &harness.handlers,
        &harness.recipes,
    );

    assert_eq!(active_goal.map(|ag| ag.goal_key), Some(goal.grounded.key));
    assert_eq!(next_step_valid, Some(true));
    assert_eq!(
        next_step
            .expect("grant arrival should yield an executable exclusive step")
            .op_kind,
        PlannerOpKind::Harvest
    );
}

#[test]
fn same_place_queue_invalidation_records_exclusive_facility_blocker() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
    let goal = GoalKey::from(GoalKind::RestockCommodity {
        commodity: CommodityKind::Apple,
    });
    let mut runtime = crate::AgentDecisionRuntime::default();
    {
        let mut txn = new_txn(&mut harness.world, 1);
        txn.set_component_contention_intents(
            harness.actor,
            ContentionIntents {
                intents: [(
                    facility,
                    QueuedContentionIntent {
                        goal_key: goal,
                        intended_action: ActionDefId(77),
                    },
                )]
                .into_iter()
                .collect(),
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

    clear_local_queue_state(&mut harness.world, harness.actor, facility, 2);

    let mut facility_intents = harness
        .world
        .get_component_contention_intents(harness.actor)
        .cloned()
        .unwrap_or_default();
    let mut blocked = BlockedIntentMemory::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut facility_intents,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &UtilityProfile::default(),
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert_eq!(blocked.intents.len(), 1);
    let intent = blocked.intents.values().next().unwrap();
    assert_eq!(
        intent.blocking_fact,
        BlockingFact::ExclusiveFacilityUnavailable
    );
    assert_eq!(intent.blocker_key.target, Some(facility));
    assert_eq!(intent.blocker_key.action_def, Some(ActionDefId(77)));
    assert!(facility_intents.intents.is_empty());
}

#[test]
fn grant_loss_does_not_record_hard_blocker() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);
    let goal = GoalKey::from(GoalKind::RestockCommodity {
        commodity: CommodityKind::Apple,
    });
    set_local_queue_state(
        &mut harness.world,
        harness.actor,
        facility,
        1,
        Some(ActionDefId(77)),
    );

    let mut runtime = crate::AgentDecisionRuntime::default();
    {
        let mut txn = new_txn(&mut harness.world, 1);
        txn.set_component_contention_intents(
            harness.actor,
            ContentionIntents {
                intents: [(
                    facility,
                    QueuedContentionIntent {
                        goal_key: goal,
                        intended_action: ActionDefId(77),
                    },
                )]
                .into_iter()
                .collect(),
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&initial_view, harness.actor, &mut runtime);

    clear_local_queue_state(&mut harness.world, harness.actor, facility, 2);

    let mut facility_intents = harness
        .world
        .get_component_contention_intents(harness.actor)
        .cloned()
        .unwrap_or_default();
    let mut blocked = BlockedIntentMemory::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut facility_intents,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &UtilityProfile::default(),
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(blocked.intents.is_empty());
    assert!(facility_intents.intents.is_empty());
}

#[test]
fn queued_actor_can_eat_without_losing_queue_membership() {
    let mut harness = Harness::new(ControlSource::Ai);
    let facility = add_local_queued_facility(&mut harness.world, harness.actor, 1);

    let result = harness.step_once();

    assert_eq!(result.actions_started, 1);
    assert_eq!(harness.active_action_name(), Some("eat"));
    let queue = harness
        .world
        .get_component_contention_queue(facility)
        .expect("queued facility should still exist");
    assert!(
        queue
            .waiting
            .values()
            .any(|queued| queued.actor == harness.actor)
    );
}

#[test]
fn frame_snapshot_reports_profile_margin_source_for_active_journey() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_intention_disposition_profile(
            actor,
            IntentionDispositionProfile {
                commitment_switch_margin: Permille::new(300).unwrap(),
                domain_patience: BTreeMap::new(),
                default_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        actor
    };
    {
        let mut txn = new_txn(&mut world, 2);
        txn.set_component_intention_frame(
            actor,
            IntentionFrame {
                goal: GoalKey::from(GoalKind::Sleep),
                domain: IntentionDomain::Travel { destination: place },
                assumptions: Vec::new(),
                state: FrameState::Active,
                established_at: Tick(7),
                last_progress_tick: None,
                stalled_ticks: 0,
                patience_limit: 10,
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    let mut driver = AgentTickDriver::new();
    driver.runtime_by_agent.insert(
        actor,
        crate::AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                default_opportunity(GoalKey::from(GoalKind::Sleep)),
                GoalKey::from(GoalKind::Sleep),
                vec![travel_step(1, place)],
                PlanTerminalKind::GoalSatisfied,
            )),
            ..crate::AgentDecisionRuntime::default()
        },
    );

    let snapshot = driver.frame_snapshot(&world, actor).unwrap();

    assert_eq!(
        snapshot.switch_margin_source,
        FrameSwitchMarginSource::FrameProfile
    );
    assert_eq!(
        snapshot.effective_switch_margin,
        Permille::new(300).unwrap()
    );
    assert_eq!(snapshot.runtime.committed_destination, Some(place));
    assert_eq!(snapshot.runtime.active_plan_destination, Some(place));
    assert!(snapshot.runtime.has_active_frame_travel);
}

#[test]
fn frame_snapshot_reports_budget_margin_when_no_profile_override_applies() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        commit_txn(txn);
        actor
    };
    let budget = ProfileFixture::default();
    let mut driver = AgentTickDriver::new();
    driver.runtime_by_agent.insert(
        actor,
        crate::AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                default_opportunity(GoalKey::from(GoalKind::Sleep)),
                GoalKey::from(GoalKind::Sleep),
                vec![barrier_step()],
                PlanTerminalKind::GoalSatisfied,
            )),
            ..crate::AgentDecisionRuntime::default()
        },
    );

    let snapshot = driver.frame_snapshot(&world, actor).unwrap();

    assert_eq!(
        snapshot.switch_margin_source,
        FrameSwitchMarginSource::CognitiveProfile
    );
    assert_eq!(snapshot.effective_switch_margin, budget.switch_margin);
    assert_eq!(snapshot.runtime.committed_destination, None);
    assert_eq!(snapshot.runtime.active_plan_destination, None);
    assert!(!snapshot.runtime.has_active_frame_travel);
}

#[test]
fn frame_snapshot_reports_patrol_route_provenance() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let home = world.topology().place_ids().next().unwrap();
    let remote = world
        .topology()
        .place_ids()
        .find(|candidate| *candidate != home)
        .expect("prototype world should expose a second place");
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, home).unwrap();
        txn.set_component_patrol_route(
            actor,
            PatrolRoute {
                assigned_places: vec![home, remote],
                current_index: 1,
            },
        )
        .unwrap();
        commit_txn(txn);
        actor
    };
    let mut driver = AgentTickDriver::new();
    driver
        .runtime_by_agent
        .insert(actor, AgentDecisionRuntime::default());

    let snapshot = driver.frame_snapshot(&world, actor).unwrap();

    assert_eq!(
        snapshot.patrol_route.route,
        Some(PatrolRoute {
            assigned_places: vec![home, remote],
            current_index: 1,
        })
    );
    assert_eq!(snapshot.patrol_route.current_waypoint, Some(remote));
}

#[test]
fn travel_led_plan_adoption_sets_intention_frame_anchor() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let destination = entity(11);
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![travel_step(1, destination), barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let mut runtime = crate::AgentDecisionRuntime::default();

    let jc = update_frame_for_adopted_plan(None, &plan, Tick(9), &mut runtime);

    let jc = jc.expect("should create a new intention frame");
    assert_eq!(jc.goal, goal);
    assert!(matches!(jc.domain, IntentionDomain::Travel { destination: d } if d == destination));
    assert_eq!(jc.established_at, Tick(9));
    assert_eq!(jc.last_progress_tick, None);
    assert_eq!(jc.stalled_ticks, 0);
}

#[test]
fn non_travel_plan_adoption_suspends_intention_frame() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let existing_jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel {
            destination: entity(12),
        },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(3),
        last_progress_tick: Some(Tick(7)),
        stalled_ticks: 2,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime::default();

    let jc = update_frame_for_adopted_plan(existing_jc.as_ref(), &plan, Tick(9), &mut runtime);

    let jc = jc.expect("should preserve commitment in suspended state");
    assert_eq!(jc.goal, goal);
    assert!(matches!(jc.domain, IntentionDomain::Travel { destination: d } if d == entity(12)));
    assert!(matches!(jc.state, FrameState::Suspended { .. }));
    assert_eq!(jc.established_at, Tick(3));
    assert_eq!(jc.last_progress_tick, Some(Tick(7)));
    assert_eq!(jc.stalled_ticks, 2);
    assert_eq!(runtime.last_frame_clear_reason, None);
}

#[test]
fn same_goal_same_destination_replan_preserves_intention_frame() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let destination = entity(11);
    let opportunity = OpportunityKey {
        goal_key: goal,
        anchor: OpportunityAnchor::Place(destination),
    };
    let plan = PlannedPlan::new(
        opportunity,
        goal,
        vec![travel_step(1, destination), barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let existing_jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(4),
        last_progress_tick: Some(Tick(6)),
        stalled_ticks: 3,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        ..crate::AgentDecisionRuntime::default()
    };

    let jc = update_frame_for_adopted_plan(existing_jc.as_ref(), &plan, Tick(9), &mut runtime);

    let jc = jc.expect("should preserve frame");
    assert_eq!(jc.goal, goal);
    assert_eq!(plan.opportunity, opportunity);
    assert!(matches!(jc.domain, IntentionDomain::Travel { destination: d } if d == destination));
    assert_eq!(jc.state, FrameState::Active);
    assert_eq!(jc.established_at, Tick(4));
    assert_eq!(jc.last_progress_tick, Some(Tick(6)));
    assert_eq!(jc.stalled_ticks, 3);
}

#[test]
fn same_goal_different_destination_replan_restarts_intention_frame() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let original_destination = entity(11);
    let new_destination = entity(22);
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![travel_step(1, new_destination), barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let existing_jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel {
            destination: original_destination,
        },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(4),
        last_progress_tick: Some(Tick(6)),
        stalled_ticks: 3,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        ..crate::AgentDecisionRuntime::default()
    };

    let jc = update_frame_for_adopted_plan(existing_jc.as_ref(), &plan, Tick(9), &mut runtime);

    let jc = jc.expect("should restart commitment with new destination");
    assert_eq!(jc.goal, goal);
    assert!(
        matches!(jc.domain, IntentionDomain::Travel { destination: d } if d == new_destination)
    );
    assert_eq!(jc.state, FrameState::Active);
    assert_eq!(jc.established_at, Tick(9));
    assert_eq!(jc.last_progress_tick, None);
    assert_eq!(jc.stalled_ticks, 0);
}

#[test]
fn travel_leg_completion_updates_progress_tick_and_resets_blocked_counter() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel {
            destination: entity(11),
        },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 5,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            default_opportunity(goal),
            goal,
            vec![travel_step(1, entity(11)), barrier_step()],
            PlanTerminalKind::GoalSatisfied,
        )),
        current_step_index: 0,
        ..crate::AgentDecisionRuntime::default()
    };

    let updated_jc = advance_completed_step(
        &mut runtime,
        &mut None,
        jc.as_ref(),
        PlannerOpKind::Travel,
        Tick(9),
    );

    assert_eq!(runtime.current_step_index, 1);
    let updated_jc = updated_jc.expect("intention frame should persist");
    assert_eq!(updated_jc.last_progress_tick, Some(Tick(9)));
    assert_eq!(updated_jc.stalled_ticks, 0);
}

#[test]
fn recoverable_blocked_travel_step_increments_consecutive_blocked_ticks_and_forces_replan() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![travel_step(1, entity(11)), barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let step = plan.steps[0].clone();
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_intention_disposition_profile(
            actor,
            IntentionDispositionProfile {
                commitment_switch_margin: Permille::new(300).unwrap(),
                domain_patience: BTreeMap::new(),
                default_patience_ticks: std::num::NonZeroU32::new(4).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        actor
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel {
            destination: entity(11),
        },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(2),
        last_progress_tick: None,
        stalled_ticks: 1,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(plan.clone()),
        current_step_index: 0,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };
    let mut blocked_memory = BlockedIntentMemory::default();

    let (handled, updated_jc) = handle_recoverable_travel_step_blockage(
        &view,
        jc.as_ref(),
        &mut runtime,
        Some(goal),
        &mut blocked_memory,
        actor,
        &step,
        Tick(9),
        &cognitive(&ProfileFixture::default()),
    );
    assert!(handled);
    let updated_jc = updated_jc.expect("commitment should persist with incremented blocked ticks");
    assert_eq!(updated_jc.stalled_ticks, 2);
    assert!(!runtime.dirty.is_empty());
    assert_eq!(updated_jc.goal, goal);
    assert!(
        matches!(updated_jc.domain, IntentionDomain::Travel { destination: d } if d == entity(11))
    );
    assert_eq!(runtime.current_plan, None);
    assert_eq!(runtime.current_step_index, 0);
    assert!(blocked_memory.intents.is_empty());
    assert!(
        runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty()
    );
}

#[test]
fn blocked_leg_patience_exhaustion_clears_commitment_and_records_blocker() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let destination = entity(11);
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![travel_step(1, destination), barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let step = plan.steps[0].clone();
    let mut world = World::new(build_prototype_world()).unwrap();
    let place = world.topology().place_ids().next().unwrap();
    let actor = {
        let mut txn = new_txn(&mut world, 1);
        let actor = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(actor, place).unwrap();
        txn.set_component_intention_disposition_profile(
            actor,
            IntentionDispositionProfile {
                commitment_switch_margin: Permille::new(300).unwrap(),
                domain_patience: BTreeMap::new(),
                default_patience_ticks: std::num::NonZeroU32::new(2).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
        actor
    };
    let view = PerAgentBeliefView::from_world(actor, &world);
    let jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(2),
        last_progress_tick: Some(Tick(4)),
        stalled_ticks: 1,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(plan),
        current_step_index: 0,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let budget = ProfileFixture::default();

    let (handled, updated_jc) = handle_recoverable_travel_step_blockage(
        &view,
        jc.as_ref(),
        &mut runtime,
        Some(goal),
        &mut blocked_memory,
        actor,
        &step,
        Tick(9),
        &cognitive(&budget),
    );
    assert!(handled);
    assert_eq!(runtime.current_plan, None);
    assert_eq!(runtime.current_step_index, 0);
    assert!(!runtime.dirty.is_empty());
    assert!(
        updated_jc.is_none(),
        "patience exhaustion should clear commitment"
    );
    assert_eq!(
        runtime.last_frame_clear_reason,
        Some(worldwake_core::FrameClearReason::PatienceExhausted)
    );
    assert_eq!(blocked_memory.intents.len(), 1);
    let intent = blocked_memory.intents.values().next().unwrap();
    assert_eq!(intent.blocker_key.goal_key, goal);
    assert_eq!(intent.blocking_fact, BlockingFact::NoKnownPath);
    assert_eq!(intent.blocker_key.target, None);
    assert_eq!(intent.blocker_key.place, Some(destination));
    assert_eq!(intent.observed_tick, Tick(9));
    assert_eq!(
        intent.expires_tick,
        Tick(9 + u64::from(budget.structural_block_ticks))
    );
}

#[test]
fn hungry_ai_agent_emits_request_and_starts_consume_action() {
    let mut harness = Harness::new(ControlSource::Ai);

    let result = harness.step_once();

    assert_eq!(result.inputs_processed, 1);
    assert_eq!(result.actions_started, 1);
    assert_eq!(harness.scheduler.active_actions().len(), 1);
    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
        Quantity(1)
    );
}

#[test]
fn hungry_ai_agent_completes_consume_action_over_subsequent_ticks() {
    let mut harness = Harness::new(ControlSource::Ai);

    for _ in 0..8 {
        let _ = harness.step_once();
        if harness
            .world
            .controlled_commodity_quantity(harness.actor, CommodityKind::Bread)
            == Quantity(0)
        {
            break;
        }
    }

    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
        Quantity(0)
    );
}

#[test]
fn human_controlled_agent_is_skipped_by_ai_driver() {
    let mut harness = Harness::new(ControlSource::Human);

    let result = harness.step_once();

    assert_eq!(result.inputs_processed, 0);
    assert_eq!(result.actions_started, 0);
    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
        Quantity(1)
    );
}

#[test]
fn dead_ai_agent_is_skipped_by_ai_driver() {
    let mut harness = Harness::new(ControlSource::Ai);
    {
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_intention_frame(
            harness.actor,
            IntentionFrame {
                goal: GoalKey::from(GoalKind::Sleep),
                domain: IntentionDomain::Travel {
                    destination: entity(11),
                },
                assumptions: Vec::new(),
                state: FrameState::Active,
                established_at: Tick(1),
                last_progress_tick: None,
                stalled_ticks: 0,
                patience_limit: 10,
            },
        )
        .unwrap();
        txn.set_component_dead_at(harness.actor, worldwake_core::DeadAt(Tick(2)))
            .unwrap();
        let _ = txn.commit(&mut harness.event_log);
    }

    let result = harness.step_once();

    assert_eq!(result.inputs_processed, 0);
    assert_eq!(result.actions_started, 0);
    assert_eq!(
        harness
            .world
            .controlled_commodity_quantity(harness.actor, CommodityKind::Bread),
        Quantity(1)
    );
    assert_eq!(
        harness.runtime().unwrap().last_frame_clear_reason,
        Some(worldwake_core::FrameClearReason::Death)
    );
}

#[test]
fn progress_barrier_completion_preserves_goal_and_forces_replan() {
    let goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    });
    let destination = entity(11);
    let jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            default_opportunity(goal),
            goal,
            vec![travel_step(1, destination)],
            PlanTerminalKind::ProgressBarrier,
        )),
        current_step_index: 0,
        step_in_flight: false,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };

    let mut active_goal = Some(worldwake_core::ActiveGoal {
        goal_key: goal,
        adopted_at: Tick(0),
    });
    let updated_jc = advance_completed_step(
        &mut runtime,
        &mut active_goal,
        jc.as_ref(),
        PlannerOpKind::Travel,
        Tick(4),
    );

    assert_eq!(active_goal.map(|ag| ag.goal_key), Some(goal));
    assert_eq!(runtime.current_plan, None);
    assert_eq!(runtime.current_step_index, 0);
    let updated_jc = updated_jc.expect("intention frame should persist through progress barrier");
    assert_eq!(updated_jc.goal, goal);
    assert!(
        matches!(updated_jc.domain, IntentionDomain::Travel { destination: d } if d == destination)
    );
    assert_eq!(updated_jc.last_progress_tick, Some(Tick(4)));
    assert!(!runtime.dirty.is_empty());
    assert!(
        runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty()
    );
}

#[test]
fn suspended_detour_completion_preserves_commitment_and_reactivates_it() {
    let committed_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    });
    let detour_goal = GoalKey::from(GoalKind::ConsumeOwnedCommodity {
        commodity: CommodityKind::Water,
    });
    let destination = entity(11);
    let jc = Some(IntentionFrame {
        goal: committed_goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Suspended {
            reason: worldwake_core::SuspensionReason::PriorityInterrupt,
            suspended_at: Tick(2),
        },
        established_at: Tick(1),
        last_progress_tick: Some(Tick(3)),
        stalled_ticks: 0,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            default_opportunity(detour_goal),
            detour_goal,
            vec![PlannedStep {
                def_id: ActionDefId(9),
                targets: vec![PlanningEntityRef::Authoritative(entity(12))],
                payload_override: None,
                op_kind: PlannerOpKind::Consume,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        )),
        current_step_index: 0,
        step_in_flight: false,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };

    let mut active_goal = Some(worldwake_core::ActiveGoal {
        goal_key: detour_goal,
        adopted_at: Tick(0),
    });
    let updated_jc = advance_completed_step(
        &mut runtime,
        &mut active_goal,
        jc.as_ref(),
        PlannerOpKind::Consume,
        Tick(4),
    );

    assert_eq!(active_goal, None);
    assert_eq!(runtime.current_plan, None);
    assert_eq!(runtime.current_step_index, 0);
    let updated_jc = updated_jc.expect("commitment should be reactivated after detour");
    assert_eq!(updated_jc.goal, committed_goal);
    assert!(
        matches!(updated_jc.domain, IntentionDomain::Travel { destination: d } if d == destination)
    );
    assert_eq!(updated_jc.state, FrameState::Active);
    assert_eq!(updated_jc.established_at, Tick(1));
    assert_eq!(updated_jc.last_progress_tick, Some(Tick(3)));
    assert_eq!(runtime.last_frame_clear_reason, None);
    assert!(!runtime.dirty.is_empty());
}

#[test]
fn goal_completion_records_goal_satisfied_clear_reason() {
    let goal = GoalKey::from(GoalKind::Sleep);
    let destination = entity(11);
    let jc = Some(IntentionFrame {
        goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 10,
    });
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(PlannedPlan::new(
            default_opportunity(goal),
            goal,
            vec![travel_step(1, destination)],
            PlanTerminalKind::GoalSatisfied,
        )),
        current_step_index: 0,
        ..crate::AgentDecisionRuntime::default()
    };

    let mut active_goal = Some(worldwake_core::ActiveGoal {
        goal_key: goal,
        adopted_at: Tick(0),
    });
    let updated_jc = advance_completed_step(
        &mut runtime,
        &mut active_goal,
        jc.as_ref(),
        PlannerOpKind::Travel,
        Tick(4),
    );

    assert_eq!(
        runtime.last_frame_clear_reason,
        Some(worldwake_core::FrameClearReason::GoalSatisfied)
    );
    assert!(
        updated_jc.is_none(),
        "goal satisfied should clear intention frame"
    );
}

#[test]
fn apply_step_materialization_bindings_binds_expected_outputs() {
    let mut runtime = crate::AgentDecisionRuntime::default();
    let step = hypothetical_step(4, 7);
    let created = entity(21);
    let outcome = CommitOutcome {
        materializations: vec![Materialization {
            tag: MaterializationTag::SplitOffLot,
            entity: created,
        }],
        trace: None,
    };

    apply_step_materialization_bindings(&mut runtime, &step, &outcome).unwrap();

    assert_eq!(
        runtime
            .materialization_bindings
            .resolve(crate::HypotheticalEntityId(7)),
        Some(created)
    );
}

#[test]
fn apply_step_materialization_bindings_rejects_mismatched_counts() {
    let mut runtime = crate::AgentDecisionRuntime::default();
    let step = hypothetical_step(4, 7);

    assert!(
        apply_step_materialization_bindings(&mut runtime, &step, &CommitOutcome::empty()).is_err()
    );
}

#[test]
fn resolve_step_targets_uses_materialization_bindings_for_hypothetical_refs() {
    let mut runtime = crate::AgentDecisionRuntime::default();
    let step = hypothetical_step(4, 7);
    let created = entity(21);
    runtime
        .materialization_bindings
        .bind(crate::HypotheticalEntityId(7), created);

    assert_eq!(resolve_step_targets(&runtime, &step), Some(vec![created]));
}

#[test]
fn committed_action_for_step_requires_single_matching_def() {
    let step = barrier_step();
    let matching = CommittedAction {
        actor: entity(1),
        def_id: step.def_id,
        instance_id: worldwake_sim::ActionInstanceId(4),
        tick: Tick(9),
        outcome: CommitOutcome::empty(),
    };
    let mismatched = CommittedAction {
        def_id: ActionDefId(99),
        ..matching.clone()
    };

    assert_eq!(
        committed_action_for_step(&step, std::slice::from_ref(&matching)),
        Some(&matching)
    );
    assert_eq!(committed_action_for_step(&step, &[]), None);
    assert_eq!(
        committed_action_for_step(&step, &[matching.clone(), mismatched.clone()]),
        None
    );
    assert_eq!(
        committed_action_for_step(&step, std::slice::from_ref(&mismatched)),
        None
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn materialized_pickup_binding_survives_intervening_travel_until_put_down_resolution() {
    let hypothetical_id = crate::HypotheticalEntityId(0);
    let created = entity(42);
    let goal = GoalKey::from(GoalKind::MoveCargo {
        commodity: CommodityKind::Bread,
        destination: entity(22),
    });
    let plan = PlannedPlan::new(
        default_opportunity(goal),
        goal,
        vec![
            PlannedStep {
                def_id: ActionDefId(4),
                targets: vec![PlanningEntityRef::Authoritative(entity(11))],
                payload_override: None,
                op_kind: PlannerOpKind::MoveCargo,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: vec![ExpectedMaterialization {
                    tag: MaterializationTag::SplitOffLot,
                    hypothetical_id,
                }],
            },
            PlannedStep {
                def_id: ActionDefId(5),
                targets: vec![PlanningEntityRef::Authoritative(entity(22))],
                payload_override: None,
                op_kind: PlannerOpKind::Travel,
                estimated_ticks: 2,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            },
            PlannedStep {
                def_id: ActionDefId(6),
                targets: vec![PlanningEntityRef::Hypothetical(hypothetical_id)],
                payload_override: None,
                op_kind: PlannerOpKind::MoveCargo,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            },
        ],
        PlanTerminalKind::GoalSatisfied,
    );
    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(plan.clone()),
        current_step_index: 0,
        step_in_flight: true,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };
    let mut active_goal = Some(worldwake_core::ActiveGoal {
        goal_key: goal,
        adopted_at: Tick(0),
    });

    apply_step_materialization_bindings(
        &mut runtime,
        &plan.steps[0],
        &CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: created,
            }],
            trace: None,
        },
    )
    .unwrap();
    runtime.step_in_flight = false;
    advance_completed_step(
        &mut runtime,
        &mut active_goal,
        None,
        PlannerOpKind::MoveCargo,
        Tick(3),
    );

    assert_eq!(runtime.current_step_index, 1);
    assert_eq!(
        runtime.materialization_bindings.resolve(hypothetical_id),
        Some(created)
    );

    runtime.step_in_flight = true;
    apply_step_materialization_bindings(&mut runtime, &plan.steps[1], &CommitOutcome::empty())
        .unwrap();
    runtime.step_in_flight = false;
    advance_completed_step(
        &mut runtime,
        &mut active_goal,
        None,
        PlannerOpKind::Travel,
        Tick(4),
    );

    assert_eq!(runtime.current_step_index, 2);
    assert_eq!(
        resolve_step_targets(&runtime, &plan.steps[2]),
        Some(vec![created])
    );

    runtime.step_in_flight = true;
    apply_step_materialization_bindings(&mut runtime, &plan.steps[2], &CommitOutcome::empty())
        .unwrap();
    runtime.step_in_flight = false;
    advance_completed_step(
        &mut runtime,
        &mut active_goal,
        None,
        PlannerOpKind::MoveCargo,
        Tick(5),
    );

    assert!(runtime.current_plan.is_none());
    assert!(!runtime.step_in_flight);
    assert!(
        runtime
            .materialization_bindings
            .hypothetical_to_authoritative
            .is_empty()
    );
}

#[allow(clippy::too_many_lines)]
#[test]
fn goal_stability_across_cargo_materialization_continuity() {
    let (mut harness, original_lot, origin, destination) = cargo_harness(false);
    let destination_facility = harness
        .world
        .get_component_merchandise_profile(harness.actor)
        .and_then(|profile| profile.home_facility)
        .expect("cargo harness actor should have home facility");
    let expected_goal = GoalKey::from(GoalKind::MoveCargo {
        commodity: CommodityKind::Bread,
        destination: destination_facility,
    });
    let budget = ProfileFixture {
        max_plan_depth: 4,
        ..ProfileFixture::default()
    };
    let semantics = crate::build_semantics_table(&harness.defs);
    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    let grounded = crate::generate_candidates(
        &view,
        harness.actor,
        &BlockedIntentMemory::default(),
        &harness.recipes,
        Tick(0),
    )
    .into_iter()
    .find(|candidate| candidate.key == expected_goal)
    .expect("owned ground lot with home-market demand should emit MoveCargo");
    assert_eq!(
        grounded.evidence_entities,
        [original_lot, destination_facility].into_iter().collect()
    );
    assert_eq!(
        grounded.evidence_places,
        [origin, destination].into_iter().collect()
    );
    let snapshot = crate::build_planning_snapshot(
        &view,
        harness.actor,
        &grounded.evidence_entities,
        &grounded.evidence_places,
        1,
    );
    let planning_state = crate::PlanningState::new(&snapshot);
    let planning_affordances = worldwake_sim::get_affordances(
        &planning_state,
        harness.actor,
        &harness.defs,
        &harness.handlers,
    );
    assert!(
        planning_affordances.iter().any(|affordance| {
            harness
                .defs
                .get(affordance.def_id)
                .is_some_and(|def| def.name == "pick_up")
        }),
        "planning state should expose pick_up affordance for owned ground cargo"
    );
    let pick_up_def = harness
        .defs
        .iter()
        .find(|def| def.name == "pick_up")
        .map(|def| def.id)
        .expect("pick_up action should be registered");
    let travel_def = harness
        .defs
        .iter()
        .find(|def| def.name == "travel")
        .map(|def| def.id)
        .expect("travel action should be registered");
    let store_stock_def = harness
        .defs
        .iter()
        .find(|def| def.name == "store_stock")
        .map(|def| def.id)
        .expect("store_stock action should be registered");
    let carried_hypothetical = crate::HypotheticalEntityId(0);
    let pick_up = PlannedStep {
        def_id: pick_up_def,
        targets: vec![PlanningEntityRef::Authoritative(original_lot)],
        payload_override: Some(ActionPayload::Transport(TransportActionPayload {
            quantity: Quantity(2),
        })),
        op_kind: PlannerOpKind::MoveCargo,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: vec![ExpectedMaterialization {
            tag: MaterializationTag::SplitOffLot,
            hypothetical_id: carried_hypothetical,
        }],
    };
    let travel = PlannedStep {
        def_id: travel_def,
        targets: vec![PlanningEntityRef::Authoritative(destination)],
        payload_override: None,
        op_kind: PlannerOpKind::Travel,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
    };
    let store_stock = PlannedStep {
        def_id: store_stock_def,
        targets: vec![PlanningEntityRef::Hypothetical(carried_hypothetical)],
        payload_override: None,
        op_kind: PlannerOpKind::StockManagement,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
    };
    let initial_plan = PlannedPlan::new(
        default_opportunity(expected_goal),
        expected_goal,
        vec![pick_up.clone(), travel, store_stock],
        PlanTerminalKind::GoalSatisfied,
    );

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    assert_eq!(pick_up.op_kind, PlannerOpKind::MoveCargo);
    assert_eq!(
        pick_up.targets,
        vec![PlanningEntityRef::Authoritative(original_lot)]
    );

    let mut runtime = crate::AgentDecisionRuntime {
        current_plan: Some(initial_plan),
        current_step_index: 0,
        step_in_flight: false,
        dirty: crate::DirtySet::default(),
        ..crate::AgentDecisionRuntime::default()
    };
    let mut active_goal_state = Some(worldwake_core::ActiveGoal {
        goal_key: expected_goal,
        adopted_at: Tick(1),
    });
    update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);

    let ranked = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(1),
            travel_horizon: budget.snapshot_travel_horizon,
            structural_block_ticks: budget.structural_block_ticks,
        },
        false,
    )
    .ranked;
    let mut jc = None;
    let (next_step, next_step_valid) = plan_and_validate_next_step(
        &harness.world,
        &harness.scheduler,
        &mut runtime,
        &mut active_goal_state,
        &mut jc,
        harness.actor,
        &ranked,
        &blocked,
        budget.switch_margin,
        budget.switch_margin,
        utility.side_benefit_weight,
        Tick(1),
        &cognitive(&budget),
        &execution_budget(&budget),
        &semantics,
        &harness.defs,
        &harness.handlers,
        &harness.recipes,
    );
    let next_step = next_step.expect("cargo continuity runtime should retain the initial step");
    assert_eq!(active_goal_state.map(|ag| ag.goal_key), Some(expected_goal));
    assert_eq!(next_step, pick_up);
    assert_eq!(next_step_valid, Some(true));

    update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);

    let carried_water = {
        let mut txn = new_txn(&mut harness.world, 2);
        let (_, split_off) = txn.split_lot(original_lot, Quantity(2)).unwrap();
        txn.set_ground_location(split_off, origin).unwrap();
        txn.set_possessor(split_off, harness.actor).unwrap();
        commit_txn(txn);
        split_off
    };
    assert_eq!(
        harness
            .world
            .get_component_item_lot(original_lot)
            .unwrap()
            .quantity,
        Quantity(1)
    );
    assert_eq!(
        harness.world.possessor_of(carried_water),
        Some(harness.actor)
    );
    assert_eq!(harness.world.effective_place(carried_water), Some(origin));
    assert_eq!(
        harness
            .world
            .get_component_item_lot(carried_water)
            .unwrap()
            .quantity,
        Quantity(2)
    );
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(2));

    runtime.step_in_flight = true;
    apply_step_materialization_bindings(
        &mut runtime,
        &pick_up,
        &CommitOutcome {
            materializations: vec![Materialization {
                tag: MaterializationTag::SplitOffLot,
                entity: carried_water,
            }],
            trace: None,
        },
    )
    .unwrap();
    runtime.step_in_flight = false;
    advance_completed_step(
        &mut runtime,
        &mut active_goal_state,
        None,
        PlannerOpKind::MoveCargo,
        Tick(2),
    );
    assert_eq!(active_goal_state.map(|ag| ag.goal_key), Some(expected_goal));

    let ranked_after_pickup = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        active_goal_state.map(|ag| ag.goal_key),
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(2),
            travel_horizon: budget.snapshot_travel_horizon,
            structural_block_ticks: budget.structural_block_ticks,
        },
        false,
    )
    .ranked;
    let mut jc2 = None;
    let (next_step, next_step_valid) = plan_and_validate_next_step(
        &harness.world,
        &harness.scheduler,
        &mut runtime,
        &mut active_goal_state,
        &mut jc2,
        harness.actor,
        &ranked_after_pickup,
        &blocked,
        budget.switch_margin,
        budget.switch_margin,
        utility.side_benefit_weight,
        Tick(2),
        &cognitive(&budget),
        &execution_budget(&budget),
        &semantics,
        &harness.defs,
        &harness.handlers,
        &harness.recipes,
    );
    let travel = next_step.expect("dirty cargo runtime should continue planning the same goal");
    assert_eq!(active_goal_state.map(|ag| ag.goal_key), Some(expected_goal));
    assert!(matches!(
        travel.op_kind,
        PlannerOpKind::Travel | PlannerOpKind::MoveCargo
    ));
    assert_eq!(next_step_valid, Some(true));
}

#[test]
fn irrelevant_commodity_change_does_not_trigger_replan_for_sleep_goal() {
    let mut harness = Harness::new(ControlSource::Ai);
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let runtime = harness
        .driver
        .runtime_by_agent
        .entry(harness.actor)
        .or_insert_with(|| active_runtime(GoalKind::Sleep));
    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&view, harness.actor, runtime);

    {
        let place = harness.world.effective_place(harness.actor).unwrap();
        let mut txn = new_txn(&mut harness.world, 2);
        let coin = txn
            .create_item_lot(CommodityKind::Coin, Quantity(1))
            .unwrap();
        txn.set_ground_location(coin, place).unwrap();
        txn.set_possessor(coin, harness.actor).unwrap();
        commit_txn(txn);
    }

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(runtime.dirty.is_empty());
}

#[test]
fn relevant_commodity_change_triggers_replan_for_consume_goal() {
    let mut harness = Harness::new(ControlSource::Ai);
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let runtime = harness
        .driver
        .runtime_by_agent
        .entry(harness.actor)
        .or_insert_with(|| {
            active_runtime(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            })
        });
    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&view, harness.actor, runtime);

    {
        let place = harness.world.effective_place(harness.actor).unwrap();
        let mut txn = new_txn(&mut harness.world, 2);
        let bread = txn
            .create_item_lot(CommodityKind::Bread, Quantity(1))
            .unwrap();
        txn.set_ground_location(bread, place).unwrap();
        txn.set_possessor(bread, harness.actor).unwrap();
        commit_txn(txn);
    }

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(!runtime.dirty.is_empty());
}

#[test]
fn no_plan_always_marks_runtime_dirty() {
    let harness = Harness::new(ControlSource::Ai);
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let mut runtime = crate::AgentDecisionRuntime::default();
    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);
    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();

    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(1),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(!runtime.dirty.is_empty());
}

#[test]
fn patrol_route_change_marks_runtime_dirty() {
    let mut harness = Harness::new(ControlSource::Ai);
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let place =
        worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::VillageSquare);
    let remote =
        worldwake_core::prototype_place_entity(worldwake_core::PrototypePlace::OrchardFarm);
    let route = PatrolRoute {
        assigned_places: vec![place, remote],
        current_index: 0,
    };
    let mut txn = new_txn(&mut harness.world, 1);
    txn.set_component_patrol_route(harness.actor, route.clone())
        .unwrap();
    commit_txn(txn);

    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    let mut runtime = AgentDecisionRuntime::default();
    update_runtime_observation_snapshot(&view, harness.actor, &mut runtime);

    let mut txn = new_txn(&mut harness.world, 2);
    txn.set_component_patrol_route(
        harness.actor,
        PatrolRoute {
            current_index: 1,
            ..route
        },
    )
    .unwrap();
    commit_txn(txn);

    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();
    let _ = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        &mut runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(2),
            travel_horizon: ProfileFixture::default().snapshot_travel_horizon,
            structural_block_ticks: ProfileFixture::default().structural_block_ticks,
        },
        false,
    );

    assert!(runtime.dirty.contains(DirtySet::PATROL_ROUTE));
}

#[test]
fn trace_planning_outcome_includes_patrol_route_provenance() {
    let mut harness = Harness::new(ControlSource::Ai).with_full_action_registries();
    let home = harness
        .world
        .effective_place(harness.actor)
        .expect("harness actor should start at a place");
    let remote = harness
        .world
        .topology()
        .place_ids()
        .find(|candidate| *candidate != home)
        .expect("prototype world should expose a second place");

    let mut txn = new_txn(&mut harness.world, 2);
    txn.set_component_homeostatic_needs(harness.actor, HomeostaticNeeds::default())
        .unwrap();
    txn.set_component_patrol_route(
        harness.actor,
        PatrolRoute {
            assigned_places: vec![home, remote],
            current_index: 1,
        },
    )
    .unwrap();
    txn.set_component_patrol_profile(harness.actor, patrol_profile(2, 0, 900))
        .unwrap();
    commit_txn(txn);

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .unwrap()
        .traces_for(harness.actor)
        .into_iter()
        .next()
        .expect("expected one decision trace");

    match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => {
            assert_eq!(
                planning.patrol_route.route,
                Some(PatrolRoute {
                    assigned_places: vec![home, remote],
                    current_index: 1,
                })
            );
            assert_eq!(planning.patrol_route.current_waypoint, Some(remote));
            assert_eq!(
                planning.selected_patrol_anchor,
                Some(OpportunityAnchor::Place(remote))
            );
            assert!(
                planning
                    .selection
                    .selected_goal_is(GoalKey::from(GoalKind::Patrol { place: remote })),
                "selected goal should stay aligned with the patrol provenance surface"
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn same_place_perception_seeds_seller_belief_for_runtime_candidates() {
    let (mut harness, seller, origin, _destination, bread) = hungry_acquisition_harness();

    let before = ranked_goals_at(&mut harness, Tick(1));
    assert!(!has_goal(
        &before,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
    assert!(
        harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .is_none()
    );

    run_same_place_observation(&mut harness, Tick(2), origin, seller);
    run_same_place_observation(&mut harness, Tick(2), origin, bread);

    let belief = harness
        .world
        .get_component_agent_belief_store(harness.actor)
        .unwrap()
        .get_entity(&seller)
        .cloned()
        .expect("perception should seed a direct observation for the seller");
    assert_eq!(belief.last_known_place, Some(origin));
    assert!(belief.alive);
    assert_eq!(belief.source, PerceptionSource::DirectObservation);

    let after = ranked_goals_at(&mut harness, Tick(2));
    assert!(has_goal(
        &after,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
}

#[test]
fn unseen_seller_relocation_preserves_stale_acquisition_belief() {
    let (mut harness, seller, origin, destination, bread) = hungry_acquisition_harness();
    run_same_place_observation(&mut harness, Tick(2), origin, seller);
    run_same_place_observation(&mut harness, Tick(2), origin, bread);

    relocate_entity(&mut harness.world, seller, destination, Tick(3));

    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    assert_eq!(harness.world.effective_place(seller), Some(destination));
    assert_eq!(view.effective_place(seller), Some(origin));

    let ranked = ranked_goals_at(&mut harness, Tick(3));
    assert!(has_goal(
        &ranked,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
}

#[test]
fn unseen_death_does_not_create_corpse_reaction_without_reobservation() {
    let (mut harness, seller, origin, destination, bread) = hungry_acquisition_harness();
    run_same_place_observation(&mut harness, Tick(2), origin, seller);
    run_same_place_observation(&mut harness, Tick(2), origin, bread);

    relocate_entity(&mut harness.world, seller, destination, Tick(3));
    kill_entity(&mut harness.world, seller, Tick(3));

    let view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    assert!(harness.world.get_component_dead_at(seller).is_some());
    assert!(!view.is_dead(seller));
    assert!(view.is_alive(seller));
    assert!(view.corpse_entities_at(origin).is_empty());

    let ranked = ranked_goals_at(&mut harness, Tick(3));
    assert!(!ranked.iter().any(|candidate| {
        matches!(
            candidate.grounded.key.kind,
            GoalKind::LootCorpse { corpse } if corpse == seller
        )
    }));
    assert!(!ranked.iter().any(|candidate| {
        matches!(
            candidate.grounded.key.kind,
            GoalKind::BuryCorpse { corpse, .. } if corpse == seller
        )
    }));
}

#[test]
fn expired_remote_acquisition_belief_remains_until_perception_refresh() {
    let (mut harness, seller, _local_witness, _origin, destination, _bread) =
        stale_remote_acquisition_harness();

    let before = ranked_goals_at(&mut harness, Tick(1));
    assert!(has_goal(
        &before,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
    assert_eq!(
        harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .and_then(|belief| belief.last_known_place),
        Some(destination)
    );

    let after_retention_without_refresh = ranked_goals_at(&mut harness, Tick(10));
    assert!(has_goal(
        &after_retention_without_refresh,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
    assert!(
        harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .is_some(),
        "belief retention is enforced during perception refresh, not by ranked_goals_at alone"
    );
}

#[test]
fn perception_refresh_evicts_expired_remote_acquisition_belief_and_removes_goal() {
    let (mut harness, seller, local_witness, origin, destination, _bread) =
        stale_remote_acquisition_harness();

    let before = ranked_goals_at(&mut harness, Tick(1));
    assert!(has_goal(
        &before,
        GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        }
    ));
    assert_eq!(
        harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .unwrap()
            .get_entity(&seller)
            .and_then(|belief| belief.last_known_place),
        Some(destination)
    );

    run_perception_tick(&mut harness, Tick(10));

    let store = harness
        .world
        .get_component_agent_belief_store(harness.actor)
        .unwrap();
    assert!(
        store.get_entity(&seller).is_none(),
        "expired remote seller belief should be evicted on a later perception refresh"
    );
    let local_belief = store
        .get_entity(&local_witness)
        .expect("same-place witness should be observed during refresh");
    assert_eq!(local_belief.last_known_place, Some(origin));

    let after = ranked_goals_at(&mut harness, Tick(10));
    assert!(
        !has_goal(
            &after,
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }
        ),
        "once retention enforcement prunes the stale remote seller, the acquire goal must disappear"
    );
}

#[test]
fn cargo_satisfaction_at_destination_while_carrying() {
    let (mut harness, remote_lot, _origin, destination) = cargo_harness(true);
    let destination_facility = harness
        .world
        .get_component_merchandise_profile(harness.actor)
        .and_then(|profile| profile.home_facility)
        .expect("cargo harness actor should have home facility");

    let _ = harness.step_once();
    assert_eq!(
        harness
            .world
            .get_component_active_goal(harness.actor)
            .map(|ag| ag.goal_key),
        Some(GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: destination_facility,
        }))
    );

    step_until(&mut harness, 8, |state| {
        state.world.effective_place(state.actor) == Some(destination)
            && state.scheduler.active_actions().is_empty()
    });

    let result = harness.step_once();

    assert_eq!(result.actions_started, 0);
    assert_eq!(harness.world.possessor_of(remote_lot), Some(harness.actor));
    assert_eq!(harness.world.effective_place(remote_lot), Some(destination));
    assert_eq!(
        harness
            .world
            .get_component_active_goal(harness.actor)
            .map(|ag| ag.goal_key),
        Some(GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: destination_facility,
        }))
    );
    assert!(harness.runtime().unwrap().current_plan.is_none());
    assert_eq!(harness.active_action_name(), None);
}

#[test]
fn merchant_restock_requires_delivery_to_home_facility() {
    let (mut harness, remote_lot, origin, destination) = cargo_harness(true);
    let destination_facility = harness
        .world
        .get_component_merchandise_profile(harness.actor)
        .and_then(|profile| profile.home_facility)
        .expect("cargo harness actor should have home facility");

    assert_eq!(harness.world.possessor_of(remote_lot), Some(harness.actor));
    assert_eq!(harness.world.effective_place(remote_lot), Some(origin));
    assert_ne!(origin, destination);

    let result = harness.step_once();
    assert_eq!(result.actions_started, 1);

    assert_eq!(
        harness
            .world
            .get_component_active_goal(harness.actor)
            .map(|ag| ag.goal_key),
        Some(GoalKey::from(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: destination_facility,
        }))
    );
    assert!(
        harness.world.is_in_transit(harness.actor)
            || harness.world.effective_place(remote_lot) == Some(destination)
    );
}

#[test]
fn persist_blocked_memory_skips_empty_unchanged_state() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let mut event_log = EventLog::new();
    let place = world.topology().place_ids().next().unwrap();
    let agent = {
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, place).unwrap();
        let _ = txn.commit(&mut event_log);
        agent
    };

    persist_blocked_memory(
        &mut world,
        &mut event_log,
        agent,
        Tick(2),
        &BlockedIntentMemory::default(),
        &BlockedIntentMemory::default(),
    )
    .unwrap();

    assert_eq!(world.get_component_blocked_intent_memory(agent), None);
    assert_eq!(event_log.len(), 1);
}

#[test]
fn persist_blocked_memory_commits_changed_component() {
    let mut world = World::new(build_prototype_world()).unwrap();
    let mut event_log = EventLog::new();
    let place = world.topology().place_ids().next().unwrap();
    let agent = {
        let mut txn = new_txn(&mut world, 1);
        let agent = txn.create_agent("Aster", ControlSource::Ai).unwrap();
        txn.set_ground_location(agent, place).unwrap();
        let _ = txn.commit(&mut event_log);
        agent
    };
    let mut blocked = BlockedIntentMemory::default();
    blocked.record(BlockedIntent {
        blocker_key: BlockerKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            place: None,
            target: None,
            action_def: None,
        },
        blocking_fact: BlockingFact::Unknown,
        diagnostic_context: None,
        observed_tick: Tick(2),
        expires_tick: Tick(7),
    });

    persist_blocked_memory(
        &mut world,
        &mut event_log,
        agent,
        Tick(2),
        &BlockedIntentMemory::default(),
        &blocked,
    )
    .unwrap();

    assert_eq!(
        world.get_component_blocked_intent_memory(agent),
        Some(&blocked)
    );
    assert_eq!(event_log.len(), 2);
}

#[test]
fn belief_read_modules_do_not_depend_on_world_directly() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace layout should place crate under crates/")
        .to_path_buf();
    let modules = [
        "crates/worldwake-ai/src/candidate_generation.rs",
        "crates/worldwake-ai/src/enterprise.rs",
        "crates/worldwake-ai/src/failure_handling.rs",
        "crates/worldwake-ai/src/plan_revalidation.rs",
        "crates/worldwake-ai/src/planning_snapshot.rs",
        "crates/worldwake-ai/src/planning_state.rs",
        "crates/worldwake-ai/src/pressure.rs",
        "crates/worldwake-ai/src/ranking.rs",
        "crates/worldwake-ai/src/search/mod.rs",
    ];

    for relative in modules {
        let source = fs::read_to_string(repo_root.join(relative))
            .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
        let production_source = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("split always returns at least one segment");
        assert!(
            !production_source.contains("worldwake_core::World"),
            "{relative} should read through RuntimeBeliefView instead of depending on World"
        );
        assert!(
            !production_source.contains("&World"),
            "{relative} should not take &World directly"
        );
        assert!(
            !production_source.contains("WorldTxn"),
            "{relative} should not mutate authoritative state directly"
        );
    }
}

#[test]
fn goal_read_modules_use_goal_belief_view_boundary() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .expect("workspace layout should place crate under crates/")
        .to_path_buf();
    let modules = [
        "crates/worldwake-ai/src/candidate_generation.rs",
        "crates/worldwake-ai/src/enterprise.rs",
        "crates/worldwake-ai/src/goal_explanation.rs",
        "crates/worldwake-ai/src/pressure.rs",
        "crates/worldwake-ai/src/ranking.rs",
    ];

    for relative in modules {
        let source = fs::read_to_string(repo_root.join(relative))
            .unwrap_or_else(|error| panic!("failed to read {relative}: {error}"));
        let production_source = source
            .split("\n#[cfg(test)]")
            .next()
            .expect("split always returns at least one segment");
        assert!(
            production_source.contains("GoalBeliefView"),
            "{relative} should compile against GoalBeliefView"
        );
        assert!(
            !production_source.contains("&dyn RuntimeBeliefView"),
            "{relative} should not depend on the broad RuntimeBeliefView boundary"
        );
    }
}

// ── S08AIDECTRA-002: Trace collection acceptance tests ──

#[test]
fn determine_selected_plan_source_distinguishes_search_selection_from_retention() {
    let current_goal = GoalKey::from(GoalKind::Sleep);
    let challenger_goal = GoalKey::from(GoalKind::ConsumeOwnedCommodity {
        commodity: CommodityKind::Bread,
    });
    let current_plan = PlannedPlan::new(
        default_opportunity(current_goal),
        current_goal,
        vec![barrier_step()],
        PlanTerminalKind::GoalSatisfied,
    );
    let challenger_plan = PlannedPlan::new(
        default_opportunity(challenger_goal),
        challenger_goal,
        vec![barrier_step()],
        PlanTerminalKind::ProgressBarrier,
    );

    assert_eq!(
        determine_selected_plan_source(
            default_opportunity(challenger_goal),
            Some(current_goal),
            &[
                SelectionCandidatePlan {
                    searched_opportunity: default_opportunity(current_goal),
                    found_plan: Some(current_plan.clone()),
                },
                SelectionCandidatePlan {
                    searched_opportunity: default_opportunity(challenger_goal),
                    found_plan: Some(challenger_plan),
                }
            ],
        ),
        crate::SelectedPlanSource::SearchSelection
    );
    assert_eq!(
        determine_selected_plan_source(
            default_opportunity(current_goal),
            Some(current_goal),
            &[SelectionCandidatePlan {
                searched_opportunity: default_opportunity(challenger_goal),
                found_plan: None,
            }],
        ),
        crate::SelectedPlanSource::RetainedCurrentPlan
    );
}

#[test]
fn trace_planning_outcome_for_hungry_agent() {
    let mut harness = Harness::new(ControlSource::Ai);
    harness.driver.enable_tracing();
    harness.step_once();

    let sink = harness.driver.trace_sink().unwrap();
    let traces = sink.traces_for(harness.actor);
    assert_eq!(
        traces.len(),
        1,
        "one agent processed per tick should produce one trace"
    );

    let trace = &traces[0];
    assert_eq!(trace.agent, harness.actor);

    match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => {
            assert!(
                !planning.candidates.generated.is_empty(),
                "hungry agent should generate at least one goal candidate"
            );
            assert!(
                planning.exhaustion_snapshot.is_empty(),
                "trace should not synthesize exhaustion state when the runtime cache is empty"
            );
            assert!(
                !planning.candidates.ranked.is_empty(),
                "hungry agent should have at least one ranked goal"
            );
            let selected_plan = planning
                .selection
                .selected_plan
                .as_ref()
                .expect("final trace should expose the selected plan directly");
            assert_eq!(
                planning.selection.selected_plan_source,
                Some(crate::SelectedPlanSource::SearchSelection)
            );
            assert!(
                !selected_plan.steps.is_empty(),
                "selected plan trace should preserve planned steps"
            );
            assert_eq!(selected_plan.next_step_index, Some(0));
            assert!(
                selected_plan.next_step.is_some(),
                "selected plan trace should preserve the immediate next step"
            );
            assert_eq!(
                selected_plan
                    .next_step
                    .as_ref()
                    .expect("selected plan should expose next step")
                    .op_kind,
                planning
                    .execution
                    .enqueued_step
                    .as_ref()
                    .expect("selected step should be enqueued for execution")
                    .op_kind
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn trace_planning_outcome_includes_exhaustion_snapshot() {
    let mut harness = Harness::new(ControlSource::Ai);
    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("actor should start at a concrete place");
    let retry_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Bread,
        purpose: CommodityPurpose::SelfConsume,
    });
    let frontier_goal = GoalKey::from(GoalKind::AcquireCommodity {
        commodity: CommodityKind::Water,
        purpose: CommodityPurpose::SelfConsume,
    });
    let retry_opportunity = exhaustion_key(retry_goal, OpportunityAnchor::Place(place));
    let frontier_opportunity = default_opportunity(frontier_goal);

    harness.driver.runtime_by_agent.insert(
        harness.actor,
        AgentDecisionRuntime {
            exhaustion_cache: BTreeMap::from([
                (
                    retry_opportunity,
                    crate::ExhaustionEntry {
                        retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
                        invalidation_conditions: vec![
                            ExhaustionInvalidationCondition::PositionChanged,
                        ],
                        baseline: ExhaustionBaseline {
                            position: Some(place),
                            ..ExhaustionBaseline::default()
                        },
                        next_retry_tick: Some(Tick(0)),
                        consecutive_failures: 3,
                    },
                ),
                (
                    frontier_opportunity,
                    crate::ExhaustionEntry {
                        retry_state: crate::ExhaustionRetryState::FrontierExhausted,
                        invalidation_conditions: vec![
                            ExhaustionInvalidationCondition::PositionChanged,
                        ],
                        baseline: ExhaustionBaseline {
                            position: Some(place),
                            ..ExhaustionBaseline::default()
                        },
                        next_retry_tick: None,
                        consecutive_failures: 0,
                    },
                ),
            ]),
            ..AgentDecisionRuntime::default()
        },
    );

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .expect("tracing should be enabled")
        .trace_at(harness.actor, Tick(0))
        .expect("tick 0 trace should exist");
    let planning = match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected Planning outcome, got {other:?}"),
    };

    assert_eq!(
        planning.exhaustion_snapshot,
        vec![
            crate::ExhaustionTraceEntry {
                opportunity: retry_opportunity,
                retry_state: crate::ExhaustionRetryState::BudgetRetryPending,
                consecutive_failures: 3,
                next_retry_tick: Some(Tick(0)),
                retry_eligible: true,
            },
            crate::ExhaustionTraceEntry {
                opportunity: frontier_opportunity,
                retry_state: crate::ExhaustionRetryState::FrontierExhausted,
                consecutive_failures: 0,
                next_retry_tick: None,
                retry_eligible: false,
            },
        ]
    );
}

#[test]
fn trace_planning_outcome_includes_danger_provenance_for_threatened_agent() {
    let mut harness = Harness::new(ControlSource::Ai);
    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("actor should start at a concrete place");
    let attacker = {
        let mut txn = new_txn(&mut harness.world, 2);
        let attacker = txn.create_agent("Bram", ControlSource::Ai).unwrap();
        txn.set_ground_location(attacker, place).unwrap();
        txn.add_hostility(harness.actor, attacker).unwrap();
        txn.set_component_wound_list(
            harness.actor,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: BodyPart::Torso,
                    cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                    severity: Permille::new(120).unwrap(),
                    inflicted_at: Tick(0),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
        attacker
    };
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(1));

    harness.driver.enable_tracing();
    harness.step_once();

    let planning = harness
        .driver
        .trace_sink()
        .expect("tracing should be enabled")
        .trace_at(harness.actor, Tick(0))
        .and_then(|trace| match &trace.outcome {
            crate::DecisionOutcome::Planning(planning) => Some(planning),
            _ => None,
        })
        .expect("threatened actor should produce a planning trace");
    let danger = planning
        .candidates
        .ranked
        .iter()
        .find(|summary| matches!(summary.opportunity.goal_key.kind, GoalKind::ReduceDanger))
        .and_then(|summary| summary.provenance.as_ref())
        .map(|provenance| match provenance {
            RankedGoalProvenance::Danger(assessment) => assessment,
            RankedGoalProvenance::Drive(_) => {
                panic!("reduce-danger candidate should not carry drive provenance")
            }
        })
        .expect("reduce-danger candidate should carry structured danger provenance");

    assert!(danger.current_attackers.is_empty());
    assert_eq!(danger.visible_hostiles, vec![attacker]);
    assert_eq!(danger.hostile_targets, vec![attacker]);
    assert!(danger.has_wounds);
    assert!(!danger.is_incapacitated);
    assert_eq!(danger.pressure, DriveThresholds::default().danger.high());
}

#[test]
fn trace_planning_outcome_includes_drive_provenance_for_recovery_boost() {
    let mut harness = Harness::new(ControlSource::Ai);
    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("actor should start at a concrete place");
    {
        let mut txn = new_txn(&mut harness.world, 3);
        let water = txn
            .create_item_lot(CommodityKind::Water, Quantity(1))
            .expect("water lot should be created");
        txn.set_ground_location(water, place).unwrap();
        txn.set_possessor(water, harness.actor).unwrap();
        txn.set_component_homeostatic_needs(
            harness.actor,
            HomeostaticNeeds::new(
                Permille::new(760).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(860).unwrap(),
            ),
        )
        .unwrap();
        txn.set_component_wound_list(
            harness.actor,
            WoundList {
                wounds: vec![Wound {
                    id: WoundId(1),
                    body_part: BodyPart::Torso,
                    cause: WoundCause::Deprivation(worldwake_core::DeprivationKind::Starvation),
                    severity: Permille::new(200).unwrap(),
                    inflicted_at: Tick(0),
                    bleed_rate_per_tick: Permille::new(0).unwrap(),
                }],
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(1));

    harness.driver.enable_tracing();
    harness.step_once();

    let planning = harness
        .driver
        .trace_sink()
        .expect("tracing should be enabled")
        .trace_at(harness.actor, Tick(0))
        .and_then(|trace| match &trace.outcome {
            crate::DecisionOutcome::Planning(planning) => Some(planning),
            _ => None,
        })
        .expect("recovery-boost scenario should produce a planning trace");
    let bread = planning
        .candidates
        .ranked
        .iter()
        .find(|summary| {
            summary.opportunity.goal_key.kind
                == GoalKind::ConsumeOwnedCommodity {
                    commodity: CommodityKind::Bread,
                }
        })
        .expect("bread candidate should be ranked");

    match bread
        .provenance
        .as_ref()
        .expect("bread candidate should carry drive provenance")
    {
        RankedGoalProvenance::Drive(provenance) => {
            assert_eq!(
                provenance.base_priority_class,
                crate::GoalPriorityClass::High
            );
            assert_eq!(
                provenance.final_priority_class,
                crate::GoalPriorityClass::Critical
            );
            assert_eq!(
                provenance.adjustment,
                Some(crate::RankedPriorityAdjustment::ClottedWoundRecoveryPromotion)
            );
            assert_eq!(provenance.motive_inputs.len(), 1);
            assert_eq!(
                provenance.motive_inputs[0].drive,
                crate::RankedDriveKind::Hunger
            );
            assert_eq!(
                provenance.motive_inputs[0].pressure,
                Permille::new(760).unwrap()
            );
            assert_eq!(
                provenance.motive_inputs[0].weight,
                UtilityProfile::default().hunger_weight
            );
            assert!(provenance.motive_inputs[0].recovery_relevant);
        }
        RankedGoalProvenance::Danger(_) => {
            panic!("bread candidate should not carry danger provenance")
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn planning_trace_includes_scheduler_start_failures_for_wound_abort_reasons() {
    let mut harness = Harness::new(ControlSource::Ai).with_full_action_registries();
    {
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_homeostatic_needs(
            harness.actor,
            HomeostaticNeeds::new(
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(0).unwrap(),
            ),
        )
        .unwrap();
        commit_txn(txn);
    }
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(0));
    let heal_id = harness
        .defs
        .iter()
        .find(|def| def.name == "heal")
        .map(|def| def.id)
        .expect("full registries should include heal");
    let goal = GoalKey::from(GoalKind::TreatWounds {
        patient: harness.actor,
    });
    let heal_step = PlannedStep {
        def_id: heal_id,
        targets: vec![PlanningEntityRef::Authoritative(harness.actor)],
        payload_override: None,
        op_kind: PlannerOpKind::Heal,
        estimated_ticks: 1,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
    };
    harness.driver.runtime_by_agent.insert(
        harness.actor,
        crate::AgentDecisionRuntime {
            current_plan: Some(PlannedPlan::new(
                default_opportunity(goal),
                goal,
                vec![heal_step],
                PlanTerminalKind::GoalSatisfied,
            )),
            step_in_flight: true,
            ..crate::AgentDecisionRuntime::default()
        },
    );
    harness
        .scheduler
        .record_action_start_failure(worldwake_sim::ActionStartFailure {
            tick: Tick(0),
            actor: harness.actor,
            def_id: heal_id,
            request: worldwake_sim::ResolvedRequestTrace {
                attempt: worldwake_sim::RequestAttemptTrace {
                    input_sequence_no: 17,
                    provenance: worldwake_sim::RequestProvenance::AiPlan,
                },
                binding: worldwake_sim::RequestBindingKind::ReproducedAffordance,
            },
            reason: worldwake_sim::ActionStartFailureReason::AbortRequested(
                worldwake_sim::ActionAbortRequestReason::TargetHasNoWounds {
                    target: harness.actor,
                },
            ),
        });

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .expect("tracing should be enabled")
        .trace_at(harness.actor, Tick(0))
        .expect("tick 0 trace should exist");
    let planning = match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => planning,
        other => panic!("expected Planning outcome, got {other:?}"),
    };

    assert_eq!(planning.action_start_failures.len(), 1);
    assert_eq!(planning.action_start_failures[0].tick, Tick(0));
    assert_eq!(planning.action_start_failures[0].def_id, heal_id);
    assert_eq!(
        planning.action_start_failures[0].request,
        worldwake_sim::ResolvedRequestTrace {
            attempt: worldwake_sim::RequestAttemptTrace {
                input_sequence_no: 17,
                provenance: worldwake_sim::RequestProvenance::AiPlan,
            },
            binding: worldwake_sim::RequestBindingKind::ReproducedAffordance,
        }
    );
    assert_eq!(
        planning.action_start_failures[0].reason,
        worldwake_sim::ActionStartFailureReason::AbortRequested(
            worldwake_sim::ActionAbortRequestReason::TargetHasNoWounds {
                target: harness.actor,
            }
        )
    );

    let runtime = harness
        .runtime()
        .expect("actor runtime should still exist after reconciliation");
    assert!(
        !runtime.step_in_flight,
        "missing active action should clear in-flight state after start failure reconciliation"
    );
    let blocked = harness
        .world
        .get_component_blocked_intent_memory(harness.actor)
        .expect("reconciled failure should persist blocked intent memory");
    assert_eq!(blocked.intents.len(), 1);
    assert_eq!(
        blocked
            .intents
            .values()
            .next()
            .unwrap()
            .blocker_key
            .goal_key,
        goal
    );
    assert!(
        harness.scheduler.action_start_failures().is_empty(),
        "agent tick should consume this agent's structured start failures once they are reconciled"
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn trace_snapshot_continuation_records_selected_plan_provenance() {
    let mut harness = Harness::new(ControlSource::Ai);
    let utility = harness
        .world
        .get_component_utility_profile(harness.actor)
        .cloned()
        .unwrap_or_default();
    let budget = ProfileFixture::default();
    let semantics = build_semantics_table(&harness.defs);
    let runtime = harness
        .driver
        .runtime_by_agent
        .entry(harness.actor)
        .or_default();
    let mut blocked = BlockedIntentMemory::default();
    let mut fi = ContentionIntents::default();

    let initial_read = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        runtime,
        None,
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(1),
            travel_horizon: budget.snapshot_travel_horizon,
            structural_block_ticks: budget.structural_block_ticks,
        },
        false,
    );
    let mut active_goal_state: Option<worldwake_core::ActiveGoal> = None;
    let previous_goal = active_goal_state.as_ref().map(|ag| ag.goal_key);
    let mut jc = None;
    let (_, initial_valid, initial_continued, _, initial_selection) =
        plan_and_validate_next_step_traced(
            &harness.world,
            &harness.scheduler,
            runtime,
            &mut active_goal_state,
            &mut jc,
            harness.actor,
            &initial_read.ranked,
            &blocked,
            budget.switch_margin,
            budget.switch_margin,
            utility.side_benefit_weight,
            Tick(1),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &harness.defs,
            &harness.handlers,
            true,
            previous_goal,
            &harness.recipes,
        );
    assert_eq!(initial_valid, Some(true));
    assert!(!initial_continued);
    let initial_selection = initial_selection.expect("initial traced selection should exist");
    assert_eq!(
        initial_selection.selected_plan_source,
        Some(crate::SelectedPlanSource::SearchSelection)
    );
    let initial_selected_plan = initial_selection
        .selected_plan
        .as_ref()
        .expect("initial search selection should expose a selected plan");
    let initial_search_provenance = initial_selected_plan
        .search_provenance
        .as_ref()
        .expect("fresh search selection should expose compact search provenance");
    assert!(
        initial_search_provenance.expansions_used > 0,
        "fresh search provenance should report at least one expansion for this harness setup"
    );
    assert_eq!(initial_search_provenance.root_travel_pruning, None);

    let initial_view = PerAgentBeliefView::from_world(harness.actor, &harness.world);
    update_runtime_observation_snapshot(&initial_view, harness.actor, runtime);

    {
        let place = harness.world.effective_place(harness.actor).unwrap();
        let mut txn = new_txn(&mut harness.world, 2);
        let tool = txn
            .create_unique_item(UniqueItemKind::SimpleTool, Some("Awl"), BTreeMap::new())
            .unwrap();
        txn.set_ground_location(tool, place).unwrap();
        txn.set_possessor(tool, harness.actor).unwrap();
        commit_txn(txn);
    }
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(2));

    let continuation_read = refresh_runtime_for_read_phase(
        &harness.world,
        &harness.scheduler,
        &harness.defs,
        runtime,
        active_goal_state.as_ref().map(|ag| ag.goal_key),
        &mut fi,
        &mut blocked,
        &mut ViolationMemory::default(),
        harness.actor,
        &[],
        ReadPhaseContext {
            recipe_registry: &harness.recipes,
            utility: &utility,
            tick: Tick(2),
            travel_horizon: budget.snapshot_travel_horizon,
            structural_block_ticks: budget.structural_block_ticks,
        },
        false,
    );
    // After the read phase, runtime.dirty should contain snapshot-changed bits.
    assert!(
        runtime.dirty.is_snapshot_only(),
        "expected snapshot-only dirty, got: {}",
        runtime.dirty.display_names()
    );

    let previous_goal = active_goal_state.as_ref().map(|ag| ag.goal_key);
    let mut jc2 = None;
    let (continued_step, continued_valid, plan_continued, _, continuation_selection) =
        plan_and_validate_next_step_traced(
            &harness.world,
            &harness.scheduler,
            runtime,
            &mut active_goal_state,
            &mut jc2,
            harness.actor,
            &continuation_read.ranked,
            &blocked,
            budget.switch_margin,
            budget.switch_margin,
            utility.side_benefit_weight,
            Tick(2),
            &cognitive(&budget),
            &execution_budget(&budget),
            &semantics,
            &harness.defs,
            &harness.handlers,
            true,
            previous_goal,
            &harness.recipes,
        );
    let selection = continuation_selection.expect("snapshot continuation trace should exist");
    let selected_plan = selection
        .selected_plan
        .expect("snapshot continuation should still expose the selected plan");

    assert!(plan_continued);
    assert_eq!(continued_valid, Some(true));
    assert_eq!(
        selection.selected_plan_source,
        Some(crate::SelectedPlanSource::SnapshotContinuation)
    );
    assert_eq!(selected_plan.next_step_index, Some(0));
    assert_eq!(
        selected_plan.search_provenance, None,
        "snapshot continuation should not fabricate fresh search provenance"
    );
    assert_eq!(
        selected_plan
            .next_step
            .as_ref()
            .expect("selected plan should preserve next step")
            .op_kind,
        continued_step
            .expect("snapshot continuation should keep current step")
            .op_kind
    );
}

#[test]
fn summarize_plan_replacement_records_same_goal_sibling_replacement() {
    let goal = GoalKey::from(GoalKind::RestockCommodity {
        commodity: CommodityKind::Bread,
    });
    let orchard_source = entity(12);
    let bandit_camp = entity(22);
    let orchard_opportunity = OpportunityKey {
        goal_key: goal,
        anchor: OpportunityAnchor::Place(entity(101)),
    };
    let camp_opportunity = OpportunityKey {
        goal_key: goal,
        anchor: OpportunityAnchor::Place(entity(102)),
    };
    let current_plan = PlannedPlan::new(
        orchard_opportunity,
        goal,
        vec![
            travel_step(1, entity(11)),
            PlannedStep {
                def_id: ActionDefId(2),
                targets: vec![PlanningEntityRef::Authoritative(orchard_source)],
                payload_override: None,
                op_kind: PlannerOpKind::Harvest,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            },
        ],
        PlanTerminalKind::GoalSatisfied,
    );
    let selected_plan = PlannedPlan::new(
        camp_opportunity,
        goal,
        vec![travel_step(3, bandit_camp)],
        PlanTerminalKind::GoalSatisfied,
    );
    let runtime = AgentDecisionRuntime {
        current_plan: Some(current_plan),
        current_step_index: 1,
        ..AgentDecisionRuntime::default()
    };

    let replacement = summarize_plan_replacement(
        &runtime,
        Some(goal),
        goal,
        &selected_plan,
        &ActionDefRegistry::new(),
    )
    .expect("changed same-goal branch should produce replacement provenance");

    assert_eq!(
        replacement.kind,
        SelectedPlanReplacementKind::SameGoalSiblingReplaced
    );
    assert_eq!(replacement.previous_goal, goal);
    assert_eq!(replacement.new_goal, goal);
    assert_eq!(
        replacement
            .previous_next_step
            .as_ref()
            .expect("current branch should expose its next step")
            .targets,
        vec![orchard_source]
    );
    assert_eq!(
        replacement
            .new_next_step
            .as_ref()
            .expect("fresh branch should expose its next step")
            .targets,
        vec![bandit_camp]
    );
}

#[test]
fn summarize_plan_replacement_records_same_goal_branch_refresh() {
    let goal = GoalKey::from(GoalKind::RestockCommodity {
        commodity: CommodityKind::Bread,
    });
    let orchard = entity(12);
    let opportunity = OpportunityKey {
        goal_key: goal,
        anchor: OpportunityAnchor::Place(entity(99)),
    };
    let current_plan = PlannedPlan::new(
        opportunity,
        goal,
        vec![travel_step(1, orchard)],
        PlanTerminalKind::GoalSatisfied,
    );
    let selected_plan = PlannedPlan::new(
        opportunity,
        goal,
        vec![travel_step(2, orchard)],
        PlanTerminalKind::GoalSatisfied,
    );
    let runtime = AgentDecisionRuntime {
        current_plan: Some(current_plan),
        current_step_index: 0,
        ..AgentDecisionRuntime::default()
    };

    let replacement = summarize_plan_replacement(
        &runtime,
        Some(goal),
        goal,
        &selected_plan,
        &ActionDefRegistry::new(),
    )
    .expect("fresh same-goal search should expose refresh provenance");

    assert_eq!(
        replacement.kind,
        SelectedPlanReplacementKind::SameGoalBranchRefreshed
    );
    assert_eq!(replacement.previous_goal, goal);
    assert_eq!(replacement.new_goal, goal);
}

#[test]
#[allow(clippy::too_many_lines)]
fn trace_force_law_office_skips_political_candidates_and_planning() {
    let mut harness = Harness::new(ControlSource::Ai).with_full_action_registries();

    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("harness actor should start at a place");
    let enterprise = Permille::new(800).unwrap();
    let social = Permille::new(700).unwrap();
    let (office, rival) = {
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_homeostatic_needs(harness.actor, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_utility_profile(
            harness.actor,
            UtilityProfile {
                enterprise_weight: enterprise,
                social_weight: social,
                ..UtilityProfile::default()
            },
        )
        .unwrap();

        let rival = txn.create_agent("Rival", ControlSource::Ai).unwrap();
        txn.set_ground_location(rival, place).unwrap();
        txn.set_component_homeostatic_needs(rival, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_deprivation_exposure(rival, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(rival, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(rival, MetabolismProfile::default())
            .unwrap();
        txn.set_component_utility_profile(rival, UtilityProfile::default())
            .unwrap();

        let office = txn.create_office("War Chief").unwrap();
        txn.set_component_office_data(
            office,
            OfficeData {
                title: "War Chief".to_string(),
                seat: place,
                jurisdiction: BTreeSet::from([place]),
                succession_law: SuccessionLaw::Force,
                succession_period_ticks: 5,
                eligibility_rules: Vec::new(),
                vacancy_since: Some(Tick(1)),
            },
        )
        .unwrap();
        txn.set_loyalty(harness.actor, rival, Permille::new(650).unwrap())
            .unwrap();
        commit_txn(txn);
        (office, rival)
    };

    sync_selected_beliefs(
        &mut harness.world,
        harness.actor,
        &[office, rival],
        Tick(2),
        PerceptionSource::DirectObservation,
    );

    harness.driver.enable_tracing();
    harness.step_once();

    let sink = harness.driver.trace_sink().unwrap();
    let traces = sink.traces_for(harness.actor);
    assert_eq!(traces.len(), 1, "expected one decision trace for the tick");

    match &traces[0].outcome {
        crate::DecisionOutcome::Planning(planning) => {
            let claim_goal = GoalKey::from(GoalKind::ClaimOffice { office });
            let support_goal = GoalKey::from(GoalKind::SupportCandidateForOffice {
                office,
                candidate: rival,
            });
            assert!(
                planning.candidates.generated_contains_goal(claim_goal),
                "Force-law offices should emit ClaimOffice candidates in agent_tick"
            );
            assert!(
                !planning.candidates.generated_contains_goal(support_goal),
                "Force-law offices must not emit SupportCandidateForOffice candidates in agent_tick"
            );
            assert!(
                planning.planning.attempts.iter().any(|attempt| {
                    matches!(
                        attempt.goal.kind,
                        GoalKind::ClaimOffice { office: goal_office } if goal_office == office
                    )
                }),
                "Force-law ClaimOffice should enter political plan search in agent_tick"
            );
            assert!(
                !planning.planning.attempts.iter().any(|attempt| {
                    matches!(
                        attempt.goal.kind,
                        GoalKind::SupportCandidateForOffice {
                            office: goal_office,
                            candidate
                        } if goal_office == office && candidate == rival
                    )
                }),
                "Force-law support-candidate goals must not enter political plan search in agent_tick"
            );
            let claim_attempt = planning
                .planning
                .attempts
                .iter()
                .find(|attempt| {
                    matches!(
                        attempt.goal.kind,
                        GoalKind::ClaimOffice { office: goal_office } if goal_office == office
                    )
                })
                .expect("force-law ClaimOffice attempt should be present");
            let root = claim_attempt
                .expansion_summaries
                .iter()
                .find(|summary| summary.depth == 0)
                .expect("root expansion summary should be present for ClaimOffice");
            assert!(
                root.root_candidates.iter().any(|candidate| {
                    candidate.op_kind == Some(PlannerOpKind::PressForceClaim)
                        && candidate.outcome
                            == crate::decision_trace::RootCandidateOutcome::Expanded
                }),
                "force-law ClaimOffice root trace should expose the retained PressForceClaim candidate"
            );
            let selected_plan = planning
                .selection
                .selected_plan
                .as_ref()
                .expect("force-law ClaimOffice should select a concrete executable plan");
            assert_eq!(
                selected_plan
                    .steps
                    .iter()
                    .map(|step| step.op_kind)
                    .collect::<Vec<_>>(),
                vec![PlannerOpKind::PressForceClaim],
                "force-law ClaimOffice should bind directly to PressForceClaim when already local"
            );
            assert!(
                planning
                    .candidates
                    .omitted_political
                    .iter()
                    .any(|omission| {
                        omission.family == crate::PoliticalGoalFamily::SupportCandidateForOffice
                            && omission.office == office
                            && omission.candidate.is_none()
                            && omission.reason
                                == crate::PoliticalCandidateOmissionReason::ForceSuccessionLaw
                    }),
                "Force-law omission should be preserved in the decision trace for SupportCandidateForOffice"
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn trace_social_resend_omission_reason() {
    let mut harness = Harness::new(ControlSource::Ai);
    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("harness actor should start at a place");
    let remote_place = harness
        .world
        .topology()
        .place_ids()
        .find(|candidate| *candidate != place)
        .expect("prototype world should expose a second place");
    let (listener, subject) = {
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_homeostatic_needs(harness.actor, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_tell_profile(harness.actor, TellProfile::default())
            .unwrap();
        let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
        let subject = txn.create_agent("Subject", ControlSource::Ai).unwrap();
        txn.set_ground_location(listener, place).unwrap();
        txn.set_ground_location(subject, remote_place).unwrap();
        commit_txn(txn);
        (listener, subject)
    };

    sync_selected_beliefs(
        &mut harness.world,
        harness.actor,
        &[listener, subject],
        Tick(2),
        PerceptionSource::DirectObservation,
    );
    {
        let mut store = harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .cloned()
            .expect("actor should have a belief store");
        let current = store
            .get_entity(&subject)
            .cloned()
            .expect("seeded subject belief should exist");
        store.record_told_belief(
            TellMemoryKey {
                counterparty: listener,
                topic: TellTopic::EntityBelief { subject },
            },
            ToldBeliefMemory {
                shared_state: worldwake_core::SharedTellState::EntityBelief(
                    worldwake_core::to_shared_belief_snapshot(&current),
                ),
                told_tick: Tick(2),
            },
        );
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_agent_belief_store(harness.actor, store)
            .unwrap();
        commit_txn(txn);
    }

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .unwrap()
        .traces_for(harness.actor)
        .into_iter()
        .next()
        .expect("expected one decision trace");
    let share_goal = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::EntityBelief { subject },
        communication_class: worldwake_core::CommunicationClass::Gossip,
    };

    match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => {
            assert!(
                !planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.goal_key.kind == share_goal),
                "unchanged told beliefs must not emit ShareBelief candidates"
            );
            assert!(
                    planning.candidates.omitted_social.iter().any(|omission| {
                        omission.listener == listener
                            && omission.topic == TellTopic::EntityBelief { subject }
                            && omission.reason
                                == worldwake_sim::TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief
                    }),
                    "social resend omission should be preserved in the decision trace"
                );
            assert_eq!(
                trace.goal_status(&share_goal),
                crate::GoalTraceStatus::OmittedSocial(
                    worldwake_sim::TellTopicOmissionReason::SpeakerHasAlreadyToldCurrentBelief
                )
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn trace_bandit_regroup_missing_rally_omission_reason() {
    let mut harness = Harness::new(ControlSource::Ai);
    let rally_place = harness
        .world
        .topology()
        .place_ids()
        .find(|candidate| Some(*candidate) != harness.world.effective_place(harness.actor))
        .expect("prototype world should expose a second place");

    {
        let mut txn = new_txn(&mut harness.world, 2);
        let faction = txn.create_faction("Forest Bandits").unwrap();
        txn.add_member(harness.actor, faction).unwrap();
        txn.set_component_bandit_faction_policy(
            faction,
            BanditFactionPolicy {
                min_regroup_count: 2,
                establishment_duration_ticks: NonZeroU32::new(2).unwrap(),
                abandonment_grace_ticks: NonZeroU32::new(2).unwrap(),
                flee_wound_threshold: Permille::new(300).unwrap(),
                rally_place: Some(rally_place),
            },
        )
        .unwrap();
        commit_txn(txn);
    }

    sync_all_beliefs(&mut harness.world, harness.actor, Tick(2));

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .unwrap()
        .traces_for(harness.actor)
        .into_iter()
        .next()
        .expect("expected one decision trace");
    let regroup_goal = GoalKind::RegroupWithFaction {
        faction: harness
            .world
            .factions_of(harness.actor)
            .into_iter()
            .next()
            .expect("actor should now belong to one faction"),
    };

    match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => {
            assert!(
                !planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.goal_key.kind == regroup_goal),
                "missing rally doctrine must omit RegroupWithFaction before generation"
            );
            assert!(
                planning.candidates.omitted_bandit.iter().any(|omission| {
                    omission.family == crate::BanditGoalFamily::RegroupWithFaction
                        && omission.faction
                            == harness
                                .world
                                .factions_of(harness.actor)
                                .into_iter()
                                .next()
                                .unwrap()
                        && omission.reason
                            == crate::BanditCandidateOmissionReason::MissingRallyBelief
                }),
                "bandit omission should be preserved in the live decision trace"
            );
            assert_eq!(
                trace.goal_status(&regroup_goal),
                crate::GoalTraceStatus::OmittedBandit(
                    crate::BanditCandidateOmissionReason::MissingRallyBelief
                )
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn trace_planning_records_political_over_share_belief_priority_class_reason() {
    let mut harness = Harness::new(ControlSource::Ai).with_full_action_registries();
    let place = harness
        .world
        .effective_place(harness.actor)
        .expect("harness actor should start at a place");
    let remote_place = harness
        .world
        .topology()
        .place_ids()
        .find(|candidate| *candidate != place)
        .expect("prototype world should expose a second place");
    let vacancy_tick = Tick(1);
    let (listener, office) = {
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_homeostatic_needs(harness.actor, HomeostaticNeeds::default())
            .unwrap();
        txn.set_component_deprivation_exposure(harness.actor, DeprivationExposure::default())
            .unwrap();
        txn.set_component_drive_thresholds(harness.actor, DriveThresholds::default())
            .unwrap();
        txn.set_component_metabolism_profile(harness.actor, MetabolismProfile::default())
            .unwrap();
        txn.set_component_perception_profile(
            harness.actor,
            PerceptionProfile {
                entity_memory_capacity: 32,
                entity_claim_capacity: 32,
                memory_retention_ticks: 240,
                observation_fidelity: Permille::new(1000).unwrap(),
                confidence_policy: BeliefConfidencePolicy::default(),
                institutional_memory_capacity: 20,
                consultation_speed_factor: Permille::new(500).unwrap(),
                contradiction_tolerance: Permille::new(300).unwrap(),
            },
        )
        .unwrap();
        txn.set_component_tell_profile(harness.actor, TellProfile::default())
            .unwrap();
        txn.set_component_utility_profile(
            harness.actor,
            UtilityProfile {
                enterprise_weight: Permille::new(200).unwrap(),
                social_weight: Permille::new(1000).unwrap(),
                ..UtilityProfile::default()
            },
        )
        .unwrap();

        let listener = txn.create_agent("Listener", ControlSource::Ai).unwrap();
        txn.set_ground_location(listener, place).unwrap();

        let office = txn.create_office("Speaker").unwrap();
        txn.set_component_office_data(
            office,
            OfficeData {
                title: "Speaker".to_string(),
                seat: remote_place,
                jurisdiction: BTreeSet::from([remote_place]),
                succession_law: SuccessionLaw::Support,
                succession_period_ticks: 5,
                eligibility_rules: Vec::new(),
                vacancy_since: Some(vacancy_tick),
            },
        )
        .unwrap();
        txn.create_record(RecordData {
            record_kind: RecordKind::SupportLedger,
            home_place: remote_place,
            issuer: harness.actor,
            consultation_ticks: 3,
            max_entries_per_consult: 16,
            entries: Vec::new(),
            next_entry_id: 0,
        })
        .unwrap();
        commit_txn(txn);
        (listener, office)
    };

    sync_selected_beliefs(
        &mut harness.world,
        harness.actor,
        &[listener, office],
        Tick(2),
        PerceptionSource::DirectObservation,
    );
    {
        let profile = harness
            .world
            .get_component_perception_profile(harness.actor)
            .cloned()
            .expect("actor should have a perception profile");
        let mut store = harness
            .world
            .get_component_agent_belief_store(harness.actor)
            .cloned()
            .expect("actor should have a belief store");
        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office },
            BelievedInstitutionalClaim {
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: None,
                    effective_tick: vacancy_tick,
                },
                source: InstitutionalKnowledgeSource::WitnessedEvent,
                learned_tick: Tick(2),
                learned_at: Some(remote_place),
            },
            &profile,
        );
        let mut txn = new_txn(&mut harness.world, 2);
        txn.set_component_agent_belief_store(harness.actor, store)
            .unwrap();
        commit_txn(txn);
    }

    harness.driver.enable_tracing();
    harness.step_once();

    let trace = harness
        .driver
        .trace_sink()
        .unwrap()
        .traces_for(harness.actor)
        .into_iter()
        .next()
        .expect("expected one planning trace");
    let share_goal = GoalKind::ShareBelief {
        listener,
        topic: TellTopic::InstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office,
                holder: None,
                effective_tick: vacancy_tick,
            },
        },
        communication_class: worldwake_core::CommunicationClass::Testimony,
    };
    let claim_goal = GoalKind::ClaimOffice { office };

    match &trace.outcome {
        crate::DecisionOutcome::Planning(planning) => {
            assert!(
                planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.goal_key.kind == share_goal),
                "planning trace should record the share-belief candidate"
            );
            assert!(
                planning
                    .candidates
                    .generated
                    .iter()
                    .any(|goal| goal.goal_key.kind == claim_goal),
                "planning trace should record the political claim candidate"
            );
            let comparison = planning
                .candidates
                .top_ranked_comparison
                .expect("ranked comparison should be recorded when two candidates are present");
            assert_eq!(comparison.winner.goal_key, GoalKey::new(claim_goal));
            assert_eq!(comparison.loser.goal_key, GoalKey::new(share_goal));
            assert_eq!(
                comparison.decisive_dimension,
                crate::RankedGoalComparisonDimension::PriorityClass
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn exhausted_steal_goal_is_cleared_when_target_becomes_lawfully_controllable() {
    let mut harness = Harness::new(ControlSource::Ai).with_full_action_registries();
    let place = harness.world.effective_place(harness.actor).unwrap();
    let owner;
    let target;
    {
        let mut txn = new_txn(&mut harness.world, 2);
        owner = txn.create_agent("Owner", ControlSource::Ai).unwrap();
        target = txn
            .create_item_lot(CommodityKind::Bread, Quantity(1))
            .unwrap();
        txn.set_ground_location(owner, place).unwrap();
        txn.set_ground_location(target, place).unwrap();
        txn.set_owner(target, owner).unwrap();
        txn.set_component_homeostatic_needs(harness.actor, HomeostaticNeeds::new_sated())
            .unwrap();
        txn.set_component_theft_disposition_profile(
            harness.actor,
            worldwake_core::TheftDispositionProfile {
                steal_duration_ticks: NonZeroU32::new(1).unwrap(),
                theft_motive_weight: Permille::new(500).unwrap(),
                witness_risk_penalty: Permille::new(0).unwrap(),
            },
        )
        .unwrap();
        commit_txn(txn);
    }
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(2));

    let goal = GoalKey::from(GoalKind::StealItem {
        target_item: target,
    });
    harness.driver.runtime_by_agent.insert(
        harness.actor,
        AgentDecisionRuntime {
            dirty: DirtySet::COMMODITY,
            exhaustion_cache: BTreeMap::from([(
                exhaustion_key(goal, OpportunityAnchor::Entity(target)),
                crate::ExhaustionEntry {
                    retry_state: crate::ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: vec![
                        ExhaustionInvalidationCondition::PositionChanged,
                        ExhaustionInvalidationCondition::StealTargetStateChanged(target),
                    ],
                    baseline: ExhaustionBaseline {
                        position: Some(place),
                        steal_target_states: vec![(
                            target,
                            StealTargetSnapshot {
                                effective_place: Some(place),
                                direct_possessor: None,
                                direct_container: None,
                                access_state: StealTargetAccessState::Stealable,
                                fits_carry_capacity: true,
                                is_item_lot: true,
                            },
                        )],
                        ..ExhaustionBaseline::default()
                    },
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            )]),
            ..AgentDecisionRuntime::default()
        },
    );

    {
        let mut txn = new_txn(&mut harness.world, 3);
        txn.set_owner(target, harness.actor).unwrap();
        commit_txn(txn);
    }
    sync_all_beliefs(&mut harness.world, harness.actor, Tick(3));

    harness.step_once();

    let runtime = harness.runtime().expect("runtime should exist after step");
    assert!(
        !runtime
            .exhaustion_cache
            .contains_key(&exhaustion_key(goal, OpportunityAnchor::Entity(target))),
        "steal exhaustion entry should clear once the target becomes lawfully controllable"
    );
}

#[test]
fn harness_with_full_action_registries_exposes_non_needs_actions() {
    let harness = Harness::new(ControlSource::Ai).with_full_action_registries();
    let action_names = harness
        .defs
        .iter()
        .map(|def| def.name.as_str())
        .collect::<Vec<_>>();

    for required in ["travel", "queue_for_facility_use", "declare_support"] {
        assert!(
            action_names.contains(&required),
            "full-registry harness should include {required}"
        );
    }
}

#[test]
fn trace_dead_agent() {
    let mut harness = Harness::new(ControlSource::Ai);
    // Kill the agent by setting DeadAt.
    {
        let mut txn = new_txn(&mut harness.world, 1);
        txn.set_component_dead_at(harness.actor, DeadAt(Tick(0)))
            .unwrap();
        commit_txn(txn);
    }
    harness.driver.enable_tracing();
    harness.step_once();

    let sink = harness.driver.trace_sink().unwrap();
    let traces = sink.traces_for(harness.actor);
    assert_eq!(
        traces.len(),
        1,
        "dead agent should produce exactly one trace"
    );
    assert!(
        matches!(traces[0].outcome, crate::DecisionOutcome::Dead),
        "dead agent should produce Dead outcome"
    );
}

#[test]
fn trace_active_action_interrupt() {
    let mut harness = Harness::new(ControlSource::Ai);
    // Step once without tracing to get agent into an active action.
    harness.step_once();
    assert!(
        harness.active_action_name().is_some(),
        "agent should have started an action after first tick"
    );

    // Enable tracing and step again — agent now has an active action.
    harness.driver.enable_tracing();
    harness.step_once();

    let sink = harness.driver.trace_sink().unwrap();
    let traces = sink.traces_for(harness.actor);
    assert_eq!(traces.len(), 1, "should produce one trace per tick");
    match &traces[0].outcome {
        crate::DecisionOutcome::ActiveAction {
            action_def_id: _,
            action_name,
            interrupt,
            ..
        } => {
            assert!(
                !action_name.is_empty(),
                "active action trace should include action name"
            );
            // InterruptTrace should be populated regardless of decision.
            let _ = &interrupt.decision;
        }
        other => panic!("expected ActiveAction outcome, got {other:?}"),
    }
}

#[test]
fn tracing_disabled_produces_identical_behavior() {
    // Run two identical harnesses — one with tracing, one without.
    let mut harness_no_trace = Harness::new(ControlSource::Ai);
    let mut harness_traced = Harness::new(ControlSource::Ai);
    harness_traced.driver.enable_tracing();

    let result_no_trace = harness_no_trace.step_once();
    let result_traced = harness_traced.step_once();

    // Both should produce the same tick advancement.
    assert_eq!(result_no_trace.tick, result_traced.tick);

    // Both should have identical active actions.
    assert_eq!(
        harness_no_trace.active_action_name(),
        harness_traced.active_action_name(),
        "tracing should not change which action is selected"
    );

    // Traced harness should have trace data.
    assert!(
        !harness_traced
            .driver
            .trace_sink()
            .unwrap()
            .traces()
            .is_empty()
    );

    // Non-traced harness should have no trace data.
    assert!(harness_no_trace.driver.trace_sink().is_none());
}

// ── S22-005: Frame exhaustion → BlockedIntent integration tests ──

#[test]
fn check_patience_exhaustion_creates_blocked_intent() {
    use super::frame::check_patience_exhaustion;

    let goal = GoalKey::from(GoalKind::Sleep);
    let destination = entity(20);
    let place = entity(10);
    let frame = IntentionFrame {
        goal,
        domain: IntentionDomain::Travel { destination },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 5, // >= patience_limit of 5
        patience_limit: 5,
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let mut runtime = crate::AgentDecisionRuntime::default();
    let budget = ProfileFixture::default();

    let exhausted = check_patience_exhaustion(
        &frame,
        Some(place),
        &mut blocked_memory,
        &mut runtime,
        Tick(10),
        budget.structural_block_ticks,
    );

    assert!(exhausted, "should detect patience exhaustion");
    assert_eq!(blocked_memory.intents.len(), 1);
    let intent = blocked_memory.intents.values().next().unwrap();
    assert_eq!(intent.blocking_fact, BlockingFact::PatienceExhausted);
    assert_eq!(intent.blocker_key.goal_key, goal);
    assert_eq!(intent.blocker_key.place, Some(place));
    assert_eq!(intent.blocker_key.target, Some(destination));
    assert!(intent.blocker_key.action_def.is_none());
    assert_eq!(intent.observed_tick, Tick(10));
    assert_eq!(
        intent.expires_tick,
        Tick(10 + u64::from(budget.structural_block_ticks))
    );
    assert_eq!(
        runtime.last_frame_clear_reason,
        Some(worldwake_core::FrameClearReason::PatienceExhausted)
    );
    assert!(runtime.current_plan.is_none());
    assert!(!runtime.dirty.is_empty());
}

#[test]
fn check_patience_exhaustion_below_limit_returns_false() {
    use super::frame::check_patience_exhaustion;

    let frame = IntentionFrame {
        goal: GoalKey::from(GoalKind::Sleep),
        domain: IntentionDomain::Generic,
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 4, // < patience_limit of 5
        patience_limit: 5,
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let mut runtime = crate::AgentDecisionRuntime::default();

    let exhausted = check_patience_exhaustion(
        &frame,
        None,
        &mut blocked_memory,
        &mut runtime,
        Tick(10),
        200,
    );

    assert!(!exhausted);
    assert!(blocked_memory.intents.is_empty());
    assert_eq!(runtime.last_frame_clear_reason, None);
}

#[test]
fn patience_exhaustion_care_domain_uses_patient_as_target() {
    use super::frame::check_patience_exhaustion;

    let patient = entity(5);
    let goal = GoalKey::from(GoalKind::Sleep);
    let frame = IntentionFrame {
        goal,
        domain: IntentionDomain::Care { patient },
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 10,
        patience_limit: 10,
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let mut runtime = crate::AgentDecisionRuntime::default();

    check_patience_exhaustion(
        &frame,
        Some(entity(99)),
        &mut blocked_memory,
        &mut runtime,
        Tick(20),
        100,
    );

    let intent = blocked_memory.intents.values().next().unwrap();
    assert_eq!(
        intent.blocker_key.target,
        Some(patient),
        "Care domain should use patient as target"
    );
}

#[test]
fn patience_exhaustion_generic_domain_uses_none_target() {
    use super::frame::check_patience_exhaustion;

    let frame = IntentionFrame {
        goal: GoalKey::from(GoalKind::Sleep),
        domain: IntentionDomain::Generic,
        assumptions: Vec::new(),
        state: FrameState::Active,
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 3,
        patience_limit: 3,
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let mut runtime = crate::AgentDecisionRuntime::default();

    check_patience_exhaustion(
        &frame,
        Some(entity(99)),
        &mut blocked_memory,
        &mut runtime,
        Tick(5),
        100,
    );

    let intent = blocked_memory.intents.values().next().unwrap();
    assert_eq!(
        intent.blocker_key.target, None,
        "Generic domain should use None as target"
    );
}

#[test]
fn assumption_failure_creates_blocked_intent() {
    use super::frame::record_assumption_failure_blocked_intent;

    let patient = entity(50);
    let goal = GoalKey::from(GoalKind::Sleep);
    let place = entity(10);
    let frame = IntentionFrame {
        goal,
        domain: IntentionDomain::Care { patient },
        assumptions: vec![worldwake_core::FrameAssumption::TargetAlive(patient)],
        state: FrameState::Exhausted, // already transitioned by apply_assumption_result
        established_at: Tick(1),
        last_progress_tick: None,
        stalled_ticks: 0,
        patience_limit: 30,
    };
    let mut blocked_memory = BlockedIntentMemory::default();
    let budget = ProfileFixture::default();

    record_assumption_failure_blocked_intent(
        &frame,
        Some(place),
        &mut blocked_memory,
        Tick(5),
        budget.structural_block_ticks,
    );

    assert_eq!(blocked_memory.intents.len(), 1);
    let intent = blocked_memory.intents.values().next().unwrap();
    assert_eq!(intent.blocking_fact, BlockingFact::AssumptionFailed);
    assert_eq!(intent.blocker_key.goal_key, goal);
    assert_eq!(intent.blocker_key.place, Some(place));
    assert_eq!(intent.blocker_key.target, Some(patient));
    assert!(intent.blocker_key.action_def.is_none());
    assert_eq!(intent.observed_tick, Tick(5));
    assert_eq!(
        intent.expires_tick,
        Tick(5 + u64::from(budget.structural_block_ticks))
    );
}

#[test]
fn goal_completion_does_not_create_blocked_intent() {
    // A standard hungry agent that eats bread → goal completes normally.
    // No blocked intent should be created for the completed goal.
    let mut harness = Harness::new(ControlSource::Ai);

    // Step enough to complete the eat action.
    for _ in 0..5 {
        harness.step_once();
    }

    // Check that no blocked intent was created for the completed goal.
    let blocked_memory = harness
        .world
        .get_component_blocked_intent_memory(harness.actor);
    if let Some(memory) = blocked_memory {
        let has_patience_or_assumption = memory.intents.values().any(|intent| {
            intent.blocking_fact == BlockingFact::PatienceExhausted
                || intent.blocking_fact == BlockingFact::AssumptionFailed
        });
        assert!(
            !has_patience_or_assumption,
            "goal completion must NOT create PatienceExhausted or AssumptionFailed blocked intents, \
                 got: {:?}",
            memory.intents
        );
    }
}

// ── S50AFFTRACE-001: Affordance trace in decision trace pipeline ──

#[test]
fn affordance_trace_populated_when_tracing_enabled() {
    let mut harness = Harness::new(ControlSource::Ai);
    harness.driver.enable_tracing();
    harness.step_once();

    let sink = harness.driver.trace_sink().unwrap();
    let traces = sink.traces_for(harness.actor);
    assert_eq!(traces.len(), 1);

    match &traces[0].outcome {
        crate::DecisionOutcome::Planning(planning) => {
            let aff = planning
                .affordances
                .as_ref()
                .expect("affordance trace must be populated when tracing is enabled");
            assert!(
                !aff.available.is_empty(),
                "agent at a place with resources should have at least one affordance"
            );
            // Verify each summary has a non-empty action name.
            for summary in &aff.available {
                assert!(
                    !summary.action_name.is_empty(),
                    "affordance summary action_name must not be empty"
                );
            }
            // Place should be set for a placed agent.
            assert!(
                aff.place.is_some(),
                "affordance trace place must be set for a placed agent"
            );
        }
        other => panic!("expected Planning outcome, got {other:?}"),
    }
}

#[test]
fn affordance_trace_absent_when_tracing_disabled() {
    let mut harness = Harness::new(ControlSource::Ai);
    // Do NOT enable tracing.
    harness.step_once();

    // No trace sink should exist.
    assert!(
        harness.driver.trace_sink().is_none(),
        "trace sink should be None when tracing is disabled"
    );
}
