use crate::{
    ActionHandlerId, ActionPayload, Constraint, DurationExpr, EffectSchema, Interruptibility,
    Precondition, ReservationReq, TargetSpec,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{
    ActionDefId, ActionDomain, BodyCostPerTick, CommodityKind, EventTag, EvidenceKind,
    ExpectationKindTag, Permille, Quantity, VisibilitySpec,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BindingStrictness {
    ExactIdentity,
    FungibleEquivalentCommodity,
    EquivalentWorkstationTagAtSamePlace,
    EquivalentRouteStep,
    AnyLegalTarget,
}

impl BindingStrictness {
    pub(crate) fn exact_identity_default() -> Self {
        Self::ExactIdentity
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuardTemplateSpec {
    pub required_facts: Vec<RequiredFactSpec>,
    pub min_confidence: Permille,
    pub invalidators: Vec<InvalidatorSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RequiredFactSpec {
    TargetPresent,
    CommodityAvailable { min_quantity: Quantity },
    RouteKnown,
    ResourceAccess,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum InvalidatorSpec {
    TargetMoved,
    BeliefStatusChange,
    CommodityDepleted { min_quantity: Quantity },
    NewBlockerRecorded,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ExpectationTemplateSpec {
    pub kind_tag: ExpectationKindTag,
    pub observe_by_offset: Option<u32>,
    pub event_tag: Option<EventTag>,
    pub state_predicate_spec: Option<StatePredicateSpec>,
    pub observation_predicate_spec: Option<ObservationPredicateSpec>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum StatePredicateSpec {
    CommodityAtPlaceAtLeast {
        place_source: PlaceSource,
        kind_source: KindSource,
        quantity_source: QuantitySource,
    },
    EntityAtPlace {
        entity_source: EntitySource,
        place_source: PlaceSource,
    },
    ActorHoldsCommodity {
        kind_source: KindSource,
        quantity_source: QuantitySource,
    },
    ClaimEstablished {
        claim_source: ClaimSource,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ObservationPredicateSpec {
    EntityPerceivedAtPlace {
        entity_source: EntitySource,
        place_source: PlaceSource,
    },
    EvidencePerceived {
        kind: EvidenceKind,
        place_source: PlaceSource,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PlaceSource {
    StepTargetPlace,
    ActorPlace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EntitySource {
    StepPrimaryTarget,
    Actor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimSource {
    StepTargetClaim,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum KindSource {
    PayloadCommodity,
    LiteralCommodity(CommodityKind),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum QuantitySource {
    Literal(Quantity),
    PayloadCommodityQuantity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDef {
    pub id: ActionDefId,
    pub name: String,
    pub domain: ActionDomain,
    pub actor_constraints: Vec<Constraint>,
    pub targets: Vec<TargetSpec>,
    pub preconditions: Vec<Precondition>,
    pub reservation_requirements: Vec<ReservationReq>,
    pub duration: DurationExpr,
    pub body_cost_per_tick: BodyCostPerTick,
    pub attention_cost: Permille,
    pub interruptibility: Interruptibility,
    pub commit_conditions: Vec<Precondition>,
    pub visibility: VisibilitySpec,
    pub causal_event_tags: BTreeSet<EventTag>,
    pub payload: ActionPayload,
    pub handler: ActionHandlerId,
    #[serde(default = "BindingStrictness::exact_identity_default")]
    pub binding_strictness: BindingStrictness,
    #[serde(default)]
    pub guard_template: Option<GuardTemplateSpec>,
    #[serde(default)]
    pub expectation_template: Vec<ExpectationTemplateSpec>,
    pub effect_schema: EffectSchema,
}

#[cfg(test)]
mod tests {
    use super::{
        ActionDef, BindingStrictness, ClaimSource, EntitySource, ExpectationTemplateSpec,
        GuardTemplateSpec, InvalidatorSpec, KindSource, ObservationPredicateSpec, PlaceSource,
        QuantitySource, RequiredFactSpec, StatePredicateSpec,
    };
    use crate::{
        ActionHandlerId, ActionPayload, Constraint, DurationExpr, EffectSchema, Interruptibility,
        Precondition, ReservationReq, TargetSpec,
    };
    use serde::{Serialize, de::DeserializeOwned};
    use std::collections::BTreeSet;
    use std::num::NonZeroU32;
    use worldwake_core::{
        ActionDefId, ActionDomain, BeliefClaimKey, BodyCostPerTick, CommodityKind,
        EntityBeliefAspect, EntityId, EntityKind, EventTag, EvidenceKind, ExpectationKindTag,
        Permille, Quantity, VisibilitySpec,
    };

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn sample_action_def(id: ActionDefId) -> ActionDef {
        ActionDef {
            id,
            name: format!("action-{}", id.0),
            domain: ActionDomain::Generic,
            actor_constraints: vec![
                Constraint::ActorAlive,
                Constraint::ActorHasCommodity {
                    kind: CommodityKind::Bread,
                    min_qty: Quantity(2),
                },
            ],
            targets: vec![
                TargetSpec::SpecificEntity(EntityId {
                    slot: 4,
                    generation: 1,
                }),
                TargetSpec::EntityAtActorPlace {
                    kind: EntityKind::Facility,
                },
            ],
            preconditions: vec![
                Precondition::ActorAlive,
                Precondition::TargetExists(0),
                Precondition::TargetAtActorPlace(1),
            ],
            reservation_requirements: vec![ReservationReq { target_index: 0 }],
            duration: DurationExpr::Fixed(NonZeroU32::new(3).unwrap()),
            body_cost_per_tick: BodyCostPerTick::new(
                Permille::new(2).unwrap(),
                Permille::new(3).unwrap(),
                Permille::new(5).unwrap(),
                Permille::new(0).unwrap(),
                Permille::new(1).unwrap(),
            ),
            attention_cost: Permille::ZERO,
            interruptibility: Interruptibility::InterruptibleWithPenalty,
            commit_conditions: vec![Precondition::TargetKind {
                target_index: 0,
                kind: EntityKind::Facility,
            }],
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::from([EventTag::ActionCommitted, EventTag::Travel]),
            payload: ActionPayload::None,
            handler: ActionHandlerId(7),
            binding_strictness: BindingStrictness::EquivalentRouteStep,
            guard_template: None,
            expectation_template: vec![],
            effect_schema: EffectSchema::empty(),
        }
    }

    fn sample_action_def_with_templates(id: ActionDefId) -> ActionDef {
        let mut action_def = sample_action_def(id);
        action_def.guard_template = Some(GuardTemplateSpec {
            required_facts: vec![
                RequiredFactSpec::TargetPresent,
                RequiredFactSpec::CommodityAvailable {
                    min_quantity: Quantity(2),
                },
            ],
            min_confidence: Permille::new(700).unwrap(),
            invalidators: vec![
                InvalidatorSpec::TargetMoved,
                InvalidatorSpec::CommodityDepleted {
                    min_quantity: Quantity(1),
                },
                InvalidatorSpec::BeliefStatusChange,
                InvalidatorSpec::NewBlockerRecorded,
            ],
        });
        action_def.expectation_template = vec![
            ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Immediate,
                observe_by_offset: Some(5),
                event_tag: Some(EventTag::ExpectationMismatch),
                state_predicate_spec: None,
                observation_predicate_spec: None,
            },
            ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Regression,
                observe_by_offset: None,
                event_tag: None,
                state_predicate_spec: Some(StatePredicateSpec::ClaimEstablished {
                    claim_source: ClaimSource::StepTargetClaim,
                }),
                observation_predicate_spec: None,
            },
            ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::Informed,
                observe_by_offset: Some(3),
                event_tag: None,
                state_predicate_spec: None,
                observation_predicate_spec: Some(
                    ObservationPredicateSpec::EntityPerceivedAtPlace {
                        entity_source: EntitySource::StepPrimaryTarget,
                        place_source: PlaceSource::StepTargetPlace,
                    },
                ),
            },
            ExpectationTemplateSpec {
                kind_tag: ExpectationKindTag::State,
                observe_by_offset: Some(2),
                event_tag: None,
                state_predicate_spec: Some(StatePredicateSpec::CommodityAtPlaceAtLeast {
                    place_source: PlaceSource::StepTargetPlace,
                    kind_source: KindSource::LiteralCommodity(CommodityKind::Bread),
                    quantity_source: QuantitySource::Literal(Quantity(1)),
                }),
                observation_predicate_spec: None,
            },
        ];
        action_def
    }

    #[test]
    fn action_def_satisfies_required_traits() {
        assert_traits::<ActionDef>();
    }

    #[test]
    fn action_def_requires_all_expected_fields_with_concrete_non_optional_semantics() {
        let action_def = sample_action_def(ActionDefId(2));

        let ActionDef {
            id,
            name,
            domain,
            actor_constraints,
            targets,
            preconditions,
            reservation_requirements,
            duration,
            body_cost_per_tick,
            attention_cost,
            interruptibility,
            commit_conditions,
            visibility,
            causal_event_tags,
            payload,
            handler,
            binding_strictness,
            guard_template,
            expectation_template,
            effect_schema,
        } = action_def;

        let _: ActionDefId = id;
        let _: String = name;
        let _: ActionDomain = domain;
        let _: Vec<Constraint> = actor_constraints;
        let _: Vec<TargetSpec> = targets;
        let _: Vec<Precondition> = preconditions;
        let _: Vec<ReservationReq> = reservation_requirements;
        let _: DurationExpr = duration;
        let _: BodyCostPerTick = body_cost_per_tick;
        let _: Permille = attention_cost;
        let _: Interruptibility = interruptibility;
        let _: Vec<Precondition> = commit_conditions;
        let _: VisibilitySpec = visibility;
        let _: BTreeSet<EventTag> = causal_event_tags;
        let _: ActionPayload = payload;
        let _: ActionHandlerId = handler;
        let _: BindingStrictness = binding_strictness;
        let _: Option<GuardTemplateSpec> = guard_template;
        let _: Vec<ExpectationTemplateSpec> = expectation_template;
        let _: EffectSchema = effect_schema;
    }

    #[test]
    fn action_def_without_guard_template_round_trips_through_bincode() {
        let action_def = sample_action_def(ActionDefId(5));

        let bytes = bincode::serialize(&action_def).unwrap();
        let roundtrip: ActionDef = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, action_def);
    }

    #[test]
    fn action_def_with_guard_template_round_trips_through_bincode() {
        let action_def = sample_action_def_with_templates(ActionDefId(6));

        let bytes = bincode::serialize(&action_def).unwrap();
        let roundtrip: ActionDef = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, action_def);
    }

    #[test]
    fn action_def_body_cost_is_explicit_even_for_zero_cost_actions() {
        let mut action_def = sample_action_def(ActionDefId(3));
        action_def.body_cost_per_tick = BodyCostPerTick::zero();

        assert_eq!(action_def.body_cost_per_tick, BodyCostPerTick::zero());
    }

    #[test]
    fn template_spec_types_roundtrip_through_bincode() {
        let guard = GuardTemplateSpec {
            required_facts: vec![
                RequiredFactSpec::RouteKnown,
                RequiredFactSpec::ResourceAccess,
            ],
            min_confidence: Permille::new(500).unwrap(),
            invalidators: vec![InvalidatorSpec::BeliefStatusChange],
        };
        let expectation = ExpectationTemplateSpec {
            kind_tag: ExpectationKindTag::Informed,
            observe_by_offset: Some(4),
            event_tag: None,
            state_predicate_spec: None,
            observation_predicate_spec: Some(ObservationPredicateSpec::EvidencePerceived {
                kind: EvidenceKind::MovementTrace {
                    entity: EntityId {
                        slot: 40,
                        generation: 0,
                    },
                    departed_from: EntityId {
                        slot: 41,
                        generation: 0,
                    },
                    direction: EntityId {
                        slot: 42,
                        generation: 0,
                    },
                    observed_at: worldwake_core::Tick(9),
                },
                place_source: PlaceSource::ActorPlace,
            }),
        };
        let claim_source = ClaimSource::StepTargetClaim;
        let bytes =
            bincode::serialize(&(guard.clone(), expectation.clone(), claim_source)).unwrap();
        let roundtrip: (GuardTemplateSpec, ExpectationTemplateSpec, ClaimSource) =
            bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, (guard, expectation, claim_source));
    }

    #[test]
    fn state_predicate_spec_can_represent_claim_binding() {
        let claim = BeliefClaimKey {
            subject: EntityId {
                slot: 77,
                generation: 0,
            },
            aspect: EntityBeliefAspect::Location,
        };
        let predicate = StatePredicateSpec::ClaimEstablished {
            claim_source: ClaimSource::StepTargetClaim,
        };

        assert_eq!(
            predicate,
            StatePredicateSpec::ClaimEstablished {
                claim_source: ClaimSource::StepTargetClaim,
            }
        );
        assert_eq!(claim.aspect, EntityBeliefAspect::Location);
    }
}
