use crate::{ExpectationKind, PlanExpectation, PlanGuard, PlannedStep, RequiredFact};
use worldwake_core::{
    CommodityKind, EntityId, ExpectationKindTag, ObservationPredicate, Quantity, StatePredicate,
    Tick,
};
use worldwake_sim::{
    ActionDef, ActionPayload, ClaimSource, EntitySource, ExpectationTemplateSpec,
    GuardTemplateSpec, InvalidatorSpec, KindSource, ObservationPredicateSpec, PlaceSource,
    QuantitySource, RequiredFactSpec, StatePredicateSpec,
};

fn payload_commodity(payload: &ActionPayload) -> Option<CommodityKind> {
    payload
        .as_harvest()
        .map(|harvest| harvest.output_commodity)
        .or_else(|| {
            payload
                .as_craft()
                .and_then(|craft| craft.outputs.first().map(|(kind, _)| *kind))
        })
        .or_else(|| payload.as_trade().map(|trade| trade.offered_commodity))
}

fn payload_quantity(payload: &ActionPayload) -> Option<Quantity> {
    payload
        .as_harvest()
        .map(|harvest| harvest.requested_quantity)
        .or_else(|| {
            payload
                .as_craft()
                .and_then(|craft| craft.outputs.first().map(|(_, quantity)| *quantity))
        })
        .or_else(|| payload.as_trade().map(|trade| trade.requested_quantity))
}

fn resolve_entity_source(source: EntitySource, step: &PlannedStep) -> Option<EntityId> {
    match source {
        EntitySource::StepPrimaryTarget => step.primary_target(),
        EntitySource::Actor => None,
    }
}

fn resolve_place_source(source: PlaceSource, step: &PlannedStep) -> Option<EntityId> {
    match source {
        PlaceSource::StepTargetPlace => step.target_place(),
        PlaceSource::ActorPlace => None,
    }
}

fn resolve_claim_source(
    source: ClaimSource,
    step: &PlannedStep,
) -> Option<worldwake_core::BeliefClaimKey> {
    match source {
        ClaimSource::StepTargetClaim => step.target_claim(),
    }
}

fn resolve_kind_source(source: &KindSource, step: &PlannedStep) -> Option<CommodityKind> {
    match source {
        KindSource::PayloadCommodity => step.payload_override.as_ref().and_then(payload_commodity),
        KindSource::LiteralCommodity(kind) => Some(*kind),
    }
}

fn resolve_quantity_source(source: &QuantitySource, step: &PlannedStep) -> Option<Quantity> {
    match source {
        QuantitySource::Literal(quantity) => Some(*quantity),
        QuantitySource::PayloadCommodityQuantity => {
            step.payload_override.as_ref().and_then(payload_quantity)
        }
    }
}

fn route_known_endpoints(step: &PlannedStep) -> Option<(EntityId, EntityId)> {
    let from = step
        .targets
        .first()
        .copied()
        .and_then(|target| match target {
            crate::PlanningEntityRef::Authoritative(entity) => Some(entity),
            crate::PlanningEntityRef::Hypothetical(_) => None,
        })?;
    let to = step
        .targets
        .get(1)
        .copied()
        .and_then(|target| match target {
            crate::PlanningEntityRef::Authoritative(entity) => Some(entity),
            crate::PlanningEntityRef::Hypothetical(_) => None,
        })?;
    Some((from, to))
}

fn build_required_fact(spec: &RequiredFactSpec, step: &PlannedStep) -> Option<RequiredFact> {
    match spec {
        RequiredFactSpec::TargetPresent => step
            .primary_target()
            .zip(step.target_place())
            .map(|(target, at_place)| RequiredFact::TargetPresent { target, at_place }),
        RequiredFactSpec::CommodityAvailable { min_quantity } => step
            .target_place()
            .zip(step.payload_override.as_ref().and_then(payload_commodity))
            .map(|(place, kind)| RequiredFact::CommodityAvailable {
                place,
                kind,
                min_quantity: *min_quantity,
            }),
        RequiredFactSpec::RouteKnown => {
            route_known_endpoints(step).map(|(from, to)| RequiredFact::RouteKnown { from, to })
        }
        RequiredFactSpec::ResourceAccess => {
            step.primary_target()
                .map(|resource| RequiredFact::ResourceAccess {
                    resource,
                    agent_holds_permission: true,
                })
        }
    }
}

fn build_invalidator(
    spec: &InvalidatorSpec,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Option<crate::Invalidator> {
    match spec {
        InvalidatorSpec::TargetMoved => step
            .primary_target()
            .map(|target| crate::Invalidator::TargetMoved { target }),
        InvalidatorSpec::BeliefStatusChange => step
            .target_claim()
            .map(|claim| crate::Invalidator::BeliefStatusChange { claim }),
        InvalidatorSpec::CommodityDepleted { .. } => step
            .target_place()
            .zip(step.payload_override.as_ref().and_then(payload_commodity))
            .map(|(place, kind)| crate::Invalidator::CommodityDepleted { place, kind }),
        InvalidatorSpec::NewBlockerRecorded => Some(crate::Invalidator::NewBlockerRecorded {
            baseline_tick: adoption_tick,
        }),
    }
}

fn build_state_predicate(spec: &StatePredicateSpec, step: &PlannedStep) -> Option<StatePredicate> {
    match spec {
        StatePredicateSpec::CommodityAtPlaceAtLeast {
            place_source,
            kind_source,
            quantity_source,
        } => Some(StatePredicate::CommodityAtPlaceAtLeast {
            place: resolve_place_source(*place_source, step)?,
            kind: resolve_kind_source(kind_source, step)?,
            quantity: resolve_quantity_source(quantity_source, step)?,
        }),
        StatePredicateSpec::EntityAtPlace {
            entity_source,
            place_source,
        } => Some(StatePredicate::EntityAtPlace {
            entity: resolve_entity_source(*entity_source, step)?,
            place: resolve_place_source(*place_source, step)?,
        }),
        StatePredicateSpec::ActorHoldsCommodity {
            kind_source,
            quantity_source,
        } => Some(StatePredicate::ActorHoldsCommodity {
            kind: resolve_kind_source(kind_source, step)?,
            min_quantity: resolve_quantity_source(quantity_source, step)?,
        }),
        StatePredicateSpec::ClaimEstablished { claim_source } => {
            Some(StatePredicate::ClaimEstablished {
                claim: resolve_claim_source(*claim_source, step)?,
            })
        }
    }
}

fn build_observation_predicate(
    spec: &ObservationPredicateSpec,
    step: &PlannedStep,
) -> Option<ObservationPredicate> {
    match spec {
        ObservationPredicateSpec::EntityPerceivedAtPlace {
            entity_source,
            place_source,
        } => Some(ObservationPredicate::EntityPerceivedAtPlace {
            entity: resolve_entity_source(*entity_source, step)?,
            place: resolve_place_source(*place_source, step)?,
        }),
        ObservationPredicateSpec::EvidencePerceived { kind, place_source } => {
            Some(ObservationPredicate::EvidencePerceived {
                kind: *kind,
                place: resolve_place_source(*place_source, step)?,
            })
        }
    }
}

fn build_one_expectation(
    template: &ExpectationTemplateSpec,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Option<PlanExpectation> {
    let kind = match template.kind_tag {
        ExpectationKindTag::Immediate => Some(ExpectationKind::Immediate {
            event_tag: template.event_tag?,
        }),
        ExpectationKindTag::State => Some(ExpectationKind::State {
            predicate: build_state_predicate(template.state_predicate_spec.as_ref()?, step)?,
        }),
        ExpectationKindTag::Informed => Some(ExpectationKind::Informed {
            observation: build_observation_predicate(
                template.observation_predicate_spec.as_ref()?,
                step,
            )?,
        }),
        ExpectationKindTag::Regression => Some(ExpectationKind::Regression {
            predicate: build_state_predicate(template.state_predicate_spec.as_ref()?, step)?,
        }),
    }?;

    Some(PlanExpectation {
        kind,
        observe_by: template
            .observe_by_offset
            .map(|offset| Tick(adoption_tick.0.saturating_add(u64::from(offset)))),
    })
}

#[must_use]
pub fn build_plan_guard(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Option<PlanGuard> {
    let GuardTemplateSpec {
        required_facts,
        min_confidence,
        invalidators,
    } = def.guard_template.as_ref()?;

    Some(PlanGuard {
        required_facts: required_facts
            .iter()
            .filter_map(|spec| build_required_fact(spec, step))
            .collect(),
        min_confidence: *min_confidence,
        invalidators: invalidators
            .iter()
            .filter_map(|spec| build_invalidator(spec, step, adoption_tick))
            .collect(),
    })
}

#[must_use]
pub fn build_plan_expectations(
    def: &ActionDef,
    step: &PlannedStep,
    adoption_tick: Tick,
) -> Vec<PlanExpectation> {
    def.expectation_template
        .iter()
        .filter_map(|template| build_one_expectation(template, step, adoption_tick))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{build_plan_expectations, build_plan_guard};
    use crate::{ExpectationKind, PlannedStep, PlannerOpKind, PlanningEntityRef, RequiredFact};
    use worldwake_core::{
        ActionDefId, ActionDomain, BeliefClaimKey, BodyCostPerTick, CommodityKind,
        EntityBeliefAspect, EntityId, EventTag, ExpectationKindTag, Permille, Quantity,
        VisibilitySpec,
    };
    use worldwake_sim::{
        ActionDef, ActionHandlerId, ActionPayload, Constraint, DurationExpr, GuardTemplateSpec,
        Interruptibility, InvalidatorSpec, ObservationPredicateSpec, PlaceSource, Precondition,
        QuantitySource, RequiredFactSpec, ReservationReq, StatePredicateSpec, TargetSpec,
        TradeActionPayload,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn sample_action_def() -> ActionDef {
        ActionDef {
            id: ActionDefId(0),
            name: "sample".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: vec![Constraint::ActorAlive],
            targets: vec![TargetSpec::SpecificEntity(entity(1))],
            preconditions: vec![Precondition::ActorAlive],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(std::num::NonZeroU32::new(2).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            attention_cost: Permille::ZERO,
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: vec![Precondition::TargetExists(0)],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: std::collections::BTreeSet::from([EventTag::Trade]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
            binding_strictness: worldwake_sim::BindingStrictness::ExactIdentity,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: worldwake_sim::EffectSchema::empty(),
        }
    }

    fn sample_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(0),
            targets: vec![PlanningEntityRef::Authoritative(entity(10))],
            target_place: Some(entity(12)),
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: entity(10),
                sale_lot: entity(11),
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(4),
                requested_quantity: Quantity(2),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 3,
            is_materialization_barrier: false,
            expected_materializations: vec![],
            guard: None,
            expectations: vec![],
        }
    }

    #[test]
    fn build_plan_guard_translates_target_present_binding() {
        let mut def = sample_action_def();
        let step = sample_step();
        def.guard_template = Some(GuardTemplateSpec {
            required_facts: vec![RequiredFactSpec::TargetPresent],
            min_confidence: Permille::new(500).unwrap(),
            invalidators: vec![],
        });

        let guard = build_plan_guard(&def, &step, worldwake_core::Tick(8)).unwrap();

        assert_eq!(
            guard.required_facts,
            vec![RequiredFact::TargetPresent {
                target: entity(10),
                at_place: entity(12),
            }]
        );
        assert_eq!(guard.min_confidence, Permille::new(500).unwrap());
    }

    #[test]
    fn build_plan_guard_returns_none_when_template_absent() {
        let def = sample_action_def();
        let step = sample_step();

        assert_eq!(build_plan_guard(&def, &step, worldwake_core::Tick(3)), None);
    }

    #[test]
    fn build_plan_expectations_maps_every_spec() {
        let mut def = sample_action_def();
        let step = sample_step();
        def.expectation_template = vec![
            worldwake_sim::ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Immediate,
                observe_by_offset: Some(5),
                event_tag: Some(EventTag::ExpectationMismatch),
                state_predicate_spec: None,
                observation_predicate_spec: None,
            },
            worldwake_sim::ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::State,
                observe_by_offset: Some(2),
                event_tag: None,
                state_predicate_spec: Some(StatePredicateSpec::EntityAtPlace {
                    entity_source: worldwake_sim::EntitySource::StepPrimaryTarget,
                    place_source: PlaceSource::StepTargetPlace,
                }),
                observation_predicate_spec: None,
            },
            worldwake_sim::ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Informed,
                observe_by_offset: Some(4),
                event_tag: None,
                state_predicate_spec: None,
                observation_predicate_spec: Some(
                    ObservationPredicateSpec::EntityPerceivedAtPlace {
                        entity_source: worldwake_sim::EntitySource::StepPrimaryTarget,
                        place_source: PlaceSource::StepTargetPlace,
                    },
                ),
            },
            worldwake_sim::ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Regression,
                observe_by_offset: None,
                event_tag: None,
                state_predicate_spec: Some(StatePredicateSpec::ActorHoldsCommodity {
                    kind_source: worldwake_sim::KindSource::PayloadCommodity,
                    quantity_source: QuantitySource::PayloadCommodityQuantity,
                }),
                observation_predicate_spec: None,
            },
        ];

        let expectations = build_plan_expectations(&def, &step, worldwake_core::Tick(10));

        assert_eq!(expectations.len(), 4);
        assert_eq!(
            expectations[0],
            crate::PlanExpectation {
                kind: ExpectationKind::Immediate {
                    event_tag: EventTag::ExpectationMismatch,
                },
                observe_by: Some(worldwake_core::Tick(15)),
            }
        );
        assert_eq!(
            expectations[1].kind,
            ExpectationKind::State {
                predicate: worldwake_core::StatePredicate::EntityAtPlace {
                    entity: entity(10),
                    place: entity(12),
                },
            }
        );
        assert_eq!(
            expectations[2].kind,
            ExpectationKind::Informed {
                observation: worldwake_core::ObservationPredicate::EntityPerceivedAtPlace {
                    entity: entity(10),
                    place: entity(12),
                },
            }
        );
        assert_eq!(
            expectations[3].kind,
            ExpectationKind::Regression {
                predicate: worldwake_core::StatePredicate::ActorHoldsCommodity {
                    kind: CommodityKind::Coin,
                    min_quantity: Quantity(2),
                },
            }
        );
    }

    #[test]
    fn build_plan_guard_sets_belief_status_change_claim_from_step_target() {
        let mut def = sample_action_def();
        let step = sample_step();
        def.guard_template = Some(GuardTemplateSpec {
            required_facts: vec![],
            min_confidence: Permille::new(600).unwrap(),
            invalidators: vec![InvalidatorSpec::BeliefStatusChange],
        });

        let guard = build_plan_guard(&def, &step, worldwake_core::Tick(4)).unwrap();

        assert_eq!(
            guard.invalidators,
            vec![crate::Invalidator::BeliefStatusChange {
                claim: BeliefClaimKey {
                    subject: entity(10),
                    aspect: EntityBeliefAspect::Location,
                },
            }]
        );
    }
}
