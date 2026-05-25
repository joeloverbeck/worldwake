use super::{
    VerificationCandidate, VerificationContext, VerificationNeed, VerificationRejection,
    VerificationTarget,
};
use crate::plan_repair::RepairPlanCandidate;
use crate::{PlannedStep, PlannerOpKind, PlanningEntityRef};
use worldwake_core::{EntityId, ExpectationRecord, ExpectationState, RepairKind};
use worldwake_sim::{ActionPayload, SearchPlaceActionPayload};

pub fn try_build(
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    let (expectation, place) = match need {
        VerificationNeed::OverdueExpectationAtPlace { expectation, place } => {
            (*expectation, *place)
        }
        VerificationNeed::StaleEntityBelief { .. }
        | VerificationNeed::StaleInstitutionalClaim { .. } => {
            return Err(VerificationRejection::BreachClassMismatch);
        }
    };

    if place != ctx.effective_place {
        return Err(VerificationRejection::NoLawfulLocalTarget);
    }

    let search_place_def_id = search_place_action_def_id(ctx.action_defs)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;
    let expectation_record = ctx
        .belief_view
        .expectation_store(ctx.actor)
        .and_then(|store| store.records.get(&expectation).copied())
        .filter(|record| overdue_expectation_matches(*record, ctx.actor, place))
        .ok_or(VerificationRejection::PayloadValidationFailed)?;
    let step = build_search_place_step(ctx, search_place_def_id, place, expectation_record)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    Ok(VerificationCandidate {
        provider_kind: worldwake_core::VerificationProviderKind::SearchPlace,
        target: VerificationTarget::Place(place),
        repair_candidate: RepairPlanCandidate {
            kind: RepairKind::InsertVerification,
            provider: ctx.broken_link.provider,
            fact: ctx.broken_link.fact,
            step,
            reusable_suffix_index: None,
        },
        source_belief: None,
    })
}

fn build_search_place_step(
    ctx: &VerificationContext<'_>,
    search_place_def_id: worldwake_core::ActionDefId,
    place: EntityId,
    expectation_record: ExpectationRecord,
) -> Option<PlannedStep> {
    let duration = ctx
        .belief_view
        .violation_disposition_profile(ctx.actor)?
        .investigation_duration_ticks
        .get();
    Some(PlannedStep {
        def_id: search_place_def_id,
        targets: vec![PlanningEntityRef::Authoritative(place)],
        target_place: Some(place),
        payload_override: Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
            subject: expectation_record.subject,
        })),
        op_kind: PlannerOpKind::SearchPlace,
        estimated_ticks: duration,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
        guard: None,
        expectations: Vec::new(),
    })
}

fn search_place_action_def_id(
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> Option<worldwake_core::ActionDefId> {
    action_defs
        .iter()
        .find(|def| def.name == "search_place")
        .map(|def| def.id)
}

fn overdue_expectation_matches(
    record: ExpectationRecord,
    actor: EntityId,
    place: EntityId,
) -> bool {
    record.owner == actor
        && record.expected_place == place
        && record.state == ExpectationState::Overdue
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        AgentBeliefStore, CausalLink, CausalProvider, CauseRef, ControlSource, ExpectationBasis,
        ExpectationId, ExpectationRecord, ExpectationStore, Permille, Place, PlanningFact, Tick,
        Topology, ViolationDispositionProfile, VisibilitySpec, WitnessData,
    };
    use worldwake_sim::{
        ActionDefRegistry, ActionHandlerRegistry, Affordance, GoalBeliefView, PerAgentBeliefView,
        RuntimeBeliefView, requested_affordance_matches,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn violation_profile() -> ViolationDispositionProfile {
        ViolationDispositionProfile {
            investigation_duration_ticks: NonZeroU32::new(6).unwrap(),
            violation_memory_retention_ticks: 30,
            investigation_motive_weight: Permille::new(500).unwrap(),
            ownership_motive_bonus: Permille::new(100).unwrap(),
        }
    }

    fn expectation_record(
        id: ExpectationId,
        actor: EntityId,
        subject: EntityId,
        place: EntityId,
    ) -> ExpectationRecord {
        ExpectationRecord {
            id,
            owner: actor,
            subject,
            expected_place: place,
            deadline_tick: Tick(4),
            grace_ticks: 0,
            basis: ExpectationBasis::RoutineReturn,
            state: ExpectationState::Overdue,
            created_tick: Tick(1),
        }
    }

    fn action_registries() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        worldwake_systems::register_search_place_action(&mut defs, &mut handlers);
        (defs, handlers)
    }

    struct Fixture {
        actor: EntityId,
        place: EntityId,
        remote_place: EntityId,
        subject: EntityId,
        expectation: ExpectationId,
        store: AgentBeliefStore,
        world: worldwake_core::World,
        action_defs: ActionDefRegistry,
        action_handlers: ActionHandlerRegistry,
    }

    fn fixture() -> Fixture {
        let place = entity(100);
        let remote_place = entity(101);
        let mut topology = Topology::new();
        topology
            .add_place(
                place,
                Place {
                    name: "Square".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        topology
            .add_place(
                remote_place,
                Place {
                    name: "Field".to_string(),
                    capacity: None,
                    tags: BTreeSet::default(),
                },
            )
            .unwrap();
        let mut world = worldwake_core::World::new(topology).unwrap();
        let actor;
        let subject;
        let expectation = ExpectationId(7);
        {
            let mut txn = worldwake_core::WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            actor = txn.create_agent("agent", ControlSource::Ai).unwrap();
            subject = txn.create_agent("missing", ControlSource::Ai).unwrap();
            txn.set_ground_location(actor, place).unwrap();
            txn.set_ground_location(subject, remote_place).unwrap();
            txn.set_component_violation_disposition_profile(actor, violation_profile())
                .unwrap();
            let mut expectations = ExpectationStore::default();
            expectations.records.insert(
                expectation,
                expectation_record(expectation, actor, subject, place),
            );
            txn.set_component_expectation_store(actor, expectations)
                .unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let store = AgentBeliefStore::new();
        let (action_defs, action_handlers) = action_registries();

        Fixture {
            actor,
            place,
            remote_place,
            subject,
            expectation,
            store,
            world,
            action_defs,
            action_handlers,
        }
    }

    fn context<'a>(
        view: &'a dyn GoalBeliefView,
        action_defs: &'a ActionDefRegistry,
        actor: EntityId,
        place: EntityId,
        expectation: ExpectationId,
        subject: EntityId,
    ) -> VerificationContext<'a> {
        VerificationContext {
            actor,
            belief_view: view,
            effective_place: place,
            broken_link: CausalLink {
                provider: CausalProvider::Expectation {
                    expectation_id: expectation,
                },
                fact: PlanningFact::TargetPresent {
                    target: subject,
                    at_place: place,
                },
                consumer_step_index: 0,
                source_tick: Tick(2),
                confidence: Permille::new_unchecked(1000),
            },
            action_defs,
        }
    }

    #[test]
    fn search_place_provider_produces_candidate_for_overdue_expectation_at_actor_place() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.expectation,
            fixture.subject,
        );
        let candidate = try_build(
            &VerificationNeed::OverdueExpectationAtPlace {
                expectation: fixture.expectation,
                place: fixture.place,
            },
            &ctx,
        )
        .expect("local overdue expectation should produce SearchPlace candidate");

        assert_eq!(
            candidate.provider_kind,
            worldwake_core::VerificationProviderKind::SearchPlace
        );
        assert_eq!(candidate.target, VerificationTarget::Place(fixture.place));
        assert_eq!(
            candidate.repair_candidate.kind,
            RepairKind::InsertVerification
        );
        assert_eq!(
            candidate.repair_candidate.step.targets,
            vec![PlanningEntityRef::Authoritative(fixture.place)]
        );
        assert_eq!(
            candidate.repair_candidate.step.payload_override,
            Some(ActionPayload::SearchPlace(SearchPlaceActionPayload {
                subject: fixture.subject,
            }))
        );
        assert_eq!(
            candidate.repair_candidate.step.op_kind,
            PlannerOpKind::SearchPlace
        );
        assert_eq!(candidate.repair_candidate.step.estimated_ticks, 6);
    }

    #[test]
    fn search_place_provider_rejects_remote_place() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.expectation,
            fixture.subject,
        );

        assert_eq!(
            try_build(
                &VerificationNeed::OverdueExpectationAtPlace {
                    expectation: fixture.expectation,
                    place: fixture.remote_place,
                },
                &ctx,
            ),
            Err(VerificationRejection::NoLawfulLocalTarget)
        );
    }

    #[test]
    fn search_place_provider_rejects_entity_belief_breach() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.expectation,
            fixture.subject,
        );

        assert_eq!(
            try_build(
                &VerificationNeed::StaleEntityBelief {
                    subject: fixture.subject,
                    aspect: worldwake_core::EntityBeliefAspect::Location,
                },
                &ctx,
            ),
            Err(VerificationRejection::BreachClassMismatch)
        );
    }

    #[test]
    fn search_place_provider_payload_matches_registered_validator() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.expectation,
            fixture.subject,
        );
        let candidate = try_build(
            &VerificationNeed::OverdueExpectationAtPlace {
                expectation: fixture.expectation,
                place: fixture.place,
            },
            &ctx,
        )
        .unwrap();
        let step = &candidate.repair_candidate.step;
        let def = fixture.action_defs.get(step.def_id).unwrap();
        let handler = fixture.action_handlers.get(def.handler).unwrap();
        let affordance = Affordance {
            def_id: step.def_id,
            actor: fixture.actor,
            bound_targets: vec![fixture.place],
            payload_override: step.payload_override.clone(),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

        assert!(requested_affordance_matches(
            &affordance,
            def,
            handler,
            fixture.actor,
            &[fixture.place],
            step.payload_override.as_ref(),
            &view as &dyn RuntimeBeliefView,
        ));
    }
}
