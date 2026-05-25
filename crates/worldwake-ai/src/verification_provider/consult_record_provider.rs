use super::{
    VerificationCandidate, VerificationContext, VerificationNeed, VerificationRejection,
    VerificationTarget,
};
use crate::plan_repair::RepairPlanCandidate;
use crate::{PlannedStep, PlannerOpKind, PlanningEntityRef};
use worldwake_core::{
    EntityId, EntityKind, InstitutionalClaim, RecordData, RecordKind, RecordTopic, RepairKind,
    VerificationProviderKind,
};
use worldwake_sim::{ActionPayload, ConsultRecordActionPayload};

pub fn try_build(
    need: &VerificationNeed,
    ctx: &VerificationContext<'_>,
) -> Result<VerificationCandidate, VerificationRejection> {
    let record_topic = match need {
        VerificationNeed::StaleInstitutionalClaim { record_topic } => *record_topic,
        VerificationNeed::StaleEntityBelief { .. }
        | VerificationNeed::OverdueExpectationAtPlace { .. } => {
            return Err(VerificationRejection::BreachClassMismatch);
        }
    };
    let consult_record_def_id = consult_record_action_def_id(ctx.action_defs)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    let mut local_records = ctx.belief_view.entities_at(ctx.effective_place);
    local_records.sort_unstable();
    local_records.dedup();
    let local_record = local_records
        .into_iter()
        .filter(|record| ctx.belief_view.entity_kind(*record) == Some(EntityKind::Record))
        .filter(|record| ctx.belief_view.effective_place(*record) == Some(ctx.effective_place))
        .find(|record| {
            ctx.belief_view
                .record_data(*record)
                .is_some_and(|record_data| record_topic_matches(&record_data, record_topic))
        })
        .ok_or(VerificationRejection::NoLawfulLocalTarget)?;

    let step = build_consult_record_step(ctx, consult_record_def_id, local_record)
        .ok_or(VerificationRejection::PayloadValidationFailed)?;

    Ok(VerificationCandidate {
        provider_kind: VerificationProviderKind::ConsultRecord,
        target: VerificationTarget::Record(local_record),
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

fn build_consult_record_step(
    ctx: &VerificationContext<'_>,
    consult_record_def_id: worldwake_core::ActionDefId,
    record: EntityId,
) -> Option<PlannedStep> {
    let record_data = ctx.belief_view.record_data(record)?;
    Some(PlannedStep {
        def_id: consult_record_def_id,
        targets: vec![PlanningEntityRef::Authoritative(record)],
        target_place: ctx.belief_view.effective_place(record),
        payload_override: Some(ActionPayload::ConsultRecord(ConsultRecordActionPayload {
            record,
        })),
        op_kind: PlannerOpKind::ConsultRecord,
        estimated_ticks: record_data.consultation_ticks,
        is_materialization_barrier: false,
        expected_materializations: Vec::new(),
        guard: None,
        expectations: Vec::new(),
    })
}

fn consult_record_action_def_id(
    action_defs: &worldwake_sim::ActionDefRegistry,
) -> Option<worldwake_core::ActionDefId> {
    action_defs
        .iter()
        .find(|def| def.name == "consult_record")
        .map(|def| def.id)
}

fn record_topic_matches(record_data: &RecordData, record_topic: RecordTopic) -> bool {
    match record_topic {
        RecordTopic::OfficeRule { office } => {
            record_data.record_kind == RecordKind::OfficeRegister
                && record_data.entries.iter().any(|entry| {
                    matches!(
                        entry.claim,
                        InstitutionalClaim::OfficeHolder {
                            office: entry_office,
                            ..
                        }
                        | InstitutionalClaim::ForceControl {
                            office: entry_office,
                            ..
                        }
                        | InstitutionalClaim::SupportDeclaration {
                            office: entry_office,
                            ..
                        } if entry_office == office
                    )
                })
        }
        RecordTopic::BountyExists => record_data.record_kind == RecordKind::CrimeRegister,
        RecordTopic::TestifiedAbout { subject } => record_data.entries.iter().any(|entry| {
            matches!(
                entry.claim,
                InstitutionalClaim::Accusation {
                    accused,
                    ..
                }
                | InstitutionalClaim::Verdict {
                    accused,
                    ..
                }
                | InstitutionalClaim::MissingPersonStatus {
                    subject: accused,
                    ..
                } if accused == subject
            )
        }),
        RecordTopic::PriceObserved { .. } | RecordTopic::RouteSafety => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use worldwake_core::{
        BelievedRecordDataSnapshot, CausalLink, CausalProvider, CauseRef, CommodityKind,
        ControlSource, InstitutionalRecordEntry, InstitutionalSnapshotSource, Permille, Place,
        PlanningFact, Quantity, RecordEntryId, Tick, Topology, VisibilitySpec, WitnessData,
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

    fn place(slot: u32, name: &str) -> (EntityId, Place) {
        (
            entity(slot),
            Place {
                name: name.to_string(),
                capacity: None,
                tags: BTreeSet::default(),
            },
        )
    }

    fn topology_with(place: EntityId, data: Place) -> Topology {
        let mut topology = Topology::new();
        topology.add_place(place, data).unwrap();
        topology
    }

    fn record_data(place: EntityId, issuer: EntityId, office: EntityId) -> RecordData {
        RecordData {
            record_kind: RecordKind::OfficeRegister,
            home_place: place,
            issuer,
            consultation_ticks: 7,
            max_entries_per_consult: 4,
            entries: vec![InstitutionalRecordEntry {
                entry_id: RecordEntryId(0),
                claim: InstitutionalClaim::OfficeHolder {
                    office,
                    holder: Some(entity(40)),
                    effective_tick: Tick(3),
                },
                recorded_tick: Tick(3),
                supersedes: None,
            }],
            next_entry_id: 1,
        }
    }

    fn action_registries() -> (ActionDefRegistry, ActionHandlerRegistry) {
        let mut defs = ActionDefRegistry::new();
        let mut handlers = ActionHandlerRegistry::new();
        worldwake_systems::register_consult_record_action(&mut defs, &mut handlers);
        (defs, handlers)
    }

    struct Fixture {
        actor: EntityId,
        place: EntityId,
        remote_place: EntityId,
        record: EntityId,
        office: EntityId,
        store: worldwake_core::AgentBeliefStore,
        world: worldwake_core::World,
        action_defs: ActionDefRegistry,
        action_handlers: ActionHandlerRegistry,
    }

    fn fixture() -> Fixture {
        let (local_place, local_data) = place(100, "Square");
        let (remote_place, remote_data) = place(101, "Archive");
        let mut topology = topology_with(local_place, local_data);
        topology.add_place(remote_place, remote_data).unwrap();
        let mut world = worldwake_core::World::new(topology).unwrap();
        let office;
        let actor;
        let record;
        let data;
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
            office = txn.create_office("office").unwrap();
            txn.set_ground_location(actor, local_place).unwrap();
            txn.set_ground_location(office, local_place).unwrap();
            data = record_data(local_place, office, office);
            record = txn.create_record(data.clone()).unwrap();
            let mut log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut log);
        }

        let mut store = worldwake_core::AgentBeliefStore::new();
        store.record_believed_record_data(
            record,
            BelievedRecordDataSnapshot {
                data,
                source: InstitutionalSnapshotSource::RecordConsultation { record },
                learned_tick: Tick(2),
                learned_at: Some(local_place),
            },
        );
        let (action_defs, action_handlers) = action_registries();

        Fixture {
            actor,
            place: local_place,
            remote_place,
            record,
            office,
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
        record: EntityId,
        office: EntityId,
    ) -> VerificationContext<'a> {
        VerificationContext {
            actor,
            belief_view: view,
            effective_place: place,
            broken_link: CausalLink {
                provider: CausalProvider::Record {
                    record_entity: record,
                    topic: RecordTopic::OfficeRule { office },
                },
                fact: PlanningFact::CommodityAvailable {
                    place,
                    kind: CommodityKind::Bread,
                    min_quantity: Quantity(1),
                },
                consumer_step_index: 0,
                source_tick: Tick(2),
                confidence: Permille::new_unchecked(1000),
            },
            action_defs,
        }
    }

    #[test]
    fn consult_record_provider_produces_candidate_for_stale_institutional_claim() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.record,
            fixture.office,
        );
        let candidate = try_build(
            &VerificationNeed::StaleInstitutionalClaim {
                record_topic: RecordTopic::OfficeRule {
                    office: fixture.office,
                },
            },
            &ctx,
        )
        .expect("co-located believed office record should produce ConsultRecord candidate");

        assert_eq!(
            candidate.provider_kind,
            VerificationProviderKind::ConsultRecord
        );
        assert_eq!(candidate.target, VerificationTarget::Record(fixture.record));
        assert_eq!(
            candidate.repair_candidate.kind,
            RepairKind::InsertVerification
        );
        assert_eq!(
            candidate.repair_candidate.provider,
            CausalProvider::Record {
                record_entity: fixture.record,
                topic: RecordTopic::OfficeRule {
                    office: fixture.office,
                },
            }
        );
        assert_eq!(
            candidate.repair_candidate.step.def_id,
            fixture.action_defs.iter().next().unwrap().id
        );
        assert_eq!(
            candidate.repair_candidate.step.targets,
            vec![PlanningEntityRef::Authoritative(fixture.record)]
        );
        assert_eq!(
            candidate.repair_candidate.step.payload_override,
            Some(ActionPayload::ConsultRecord(ConsultRecordActionPayload {
                record: fixture.record,
            }))
        );
        assert_eq!(
            candidate.repair_candidate.step.op_kind,
            PlannerOpKind::ConsultRecord
        );
        assert_eq!(candidate.repair_candidate.step.estimated_ticks, 7);
    }

    #[test]
    fn consult_record_provider_rejects_no_local_record() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.remote_place,
            fixture.record,
            fixture.office,
        );

        assert_eq!(
            try_build(
                &VerificationNeed::StaleInstitutionalClaim {
                    record_topic: RecordTopic::OfficeRule {
                        office: fixture.office,
                    },
                },
                &ctx,
            ),
            Err(VerificationRejection::NoLawfulLocalTarget)
        );
    }

    #[test]
    fn consult_record_provider_rejects_entity_belief_breach() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.record,
            fixture.office,
        );

        assert_eq!(
            try_build(
                &VerificationNeed::StaleEntityBelief {
                    subject: entity(50),
                    aspect: worldwake_core::EntityBeliefAspect::Location,
                },
                &ctx,
            ),
            Err(VerificationRejection::BreachClassMismatch)
        );
    }

    #[test]
    fn consult_record_provider_payload_matches_registered_validator() {
        let fixture = fixture();
        let view = PerAgentBeliefView::new(fixture.actor, &fixture.world, &fixture.store);
        let ctx = context(
            &view,
            &fixture.action_defs,
            fixture.actor,
            fixture.place,
            fixture.record,
            fixture.office,
        );
        let candidate = try_build(
            &VerificationNeed::StaleInstitutionalClaim {
                record_topic: RecordTopic::OfficeRule {
                    office: fixture.office,
                },
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
            bound_targets: vec![fixture.record],
            payload_override: step.payload_override.clone(),
            explanation: None,
            contention_status: worldwake_core::ContentionStatus::Unmanaged,
        };

        assert!(requested_affordance_matches(
            &affordance,
            def,
            handler,
            fixture.actor,
            &[fixture.record],
            step.payload_override.as_ref(),
            &view as &dyn RuntimeBeliefView,
        ));
    }
}
