use crate::{
    derive_danger_pressure, enterprise::restock_gap_at_destination, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlanningEntityRef, PlanningState,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{
    CommodityKind, CommodityPurpose, EntityId, GoalKey, GoalKind, Permille, Quantity,
};
use worldwake_sim::{
    ActionDef, ActionPayload, CombatActionPayload, DeclareSupportActionPayload,
    LootActionPayload, RecipeRegistry, RuntimeBeliefView, TellActionPayload,
    TradeActionPayload, TransportActionPayload,
};

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalKindTag {
    ConsumeOwnedCommodity,
    AcquireCommodity,
    Sleep,
    Relieve,
    Wash,
    EngageHostile,
    ReduceDanger,
    Heal,
    ProduceCommodity,
    SellCommodity,
    RestockCommodity,
    MoveCargo,
    LootCorpse,
    BuryCorpse,
    ShareBelief,
    ClaimOffice,
    SupportCandidateForOffice,
}

pub trait GoalKindPlannerExt {
    fn goal_kind_tag(&self) -> GoalKindTag;
    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind];
    fn relevant_observed_commodities(
        &self,
        recipes: &RecipeRegistry,
    ) -> Option<BTreeSet<CommodityKind>>;
    fn build_payload_override(
        &self,
        affordance_payload: Option<&ActionPayload>,
        state: &PlanningState<'_>,
        targets: &[EntityId],
        def: &ActionDef,
        semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, GoalPayloadOverrideError>;
    fn apply_planner_step<'snapshot>(
        &self,
        state: PlanningState<'snapshot>,
        op_kind: PlannerOpKind,
        targets: &[EntityId],
        payload_override: Option<&ActionPayload>,
    ) -> PlanningState<'snapshot>;
    fn is_progress_barrier(&self, step: &PlannedStep) -> bool;
    fn is_satisfied(&self, state: &PlanningState<'_>) -> bool;
}

const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const ACQUIRE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SLEEP_OPS: &[PlannerOpKind] = &[PlannerOpKind::Sleep, PlannerOpKind::Travel];
const RELIEVE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Relieve, PlannerOpKind::Travel];
const WASH_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Wash,
    PlannerOpKind::Travel,
    PlannerOpKind::MoveCargo,
];
const ENGAGE_HOSTILE_OPS: &[PlannerOpKind] = &[PlannerOpKind::Attack];
const REDUCE_DANGER_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Defend,
    PlannerOpKind::Heal,
];
const HEAL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Heal,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
];
const PRODUCE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const SELL_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::MoveCargo,
];
const RESTOCK_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Harvest,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
];
const MOVE_CARGO_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::MoveCargo];
const LOOT_OPS: &[PlannerOpKind] = &[PlannerOpKind::Travel, PlannerOpKind::Loot];
const BURY_OPS: &[PlannerOpKind] = &[PlannerOpKind::Bury];
const SHARE_BELIEF_OPS: &[PlannerOpKind] = &[PlannerOpKind::Tell];
const CLAIM_OFFICE_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Bribe,
    PlannerOpKind::Threaten,
    PlannerOpKind::DeclareSupport,
];
const SUPPORT_OFFICE_OPS: &[PlannerOpKind] =
    &[PlannerOpKind::Travel, PlannerOpKind::DeclareSupport];

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GoalPayloadOverrideError {
    MissingTarget,
    UnsupportedGoal,
    MissingActorPlace,
    SellerUnavailable,
    SellerOutOfStock,
    ActorCannotPay,
}

fn payload_override_from_affordance(
    goal: &GoalKind,
    affordance_payload: Option<&ActionPayload>,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    let Some(payload) = affordance_payload else {
        return Ok(None);
    };

    match goal {
        GoalKind::EngageHostile { target } => payload
            .as_combat()
            .filter(|combat| combat.target == *target)
            .map(|_| Some(payload.clone()))
            .ok_or(GoalPayloadOverrideError::UnsupportedGoal),
        _ => Ok(Some(payload.clone())),
    }
}

fn build_attack_payload_override(
    goal: &GoalKind,
    targets: &[EntityId],
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::EngageHostile { target } => {
            let Some(actual_target) = targets.first().copied() else {
                return Err(GoalPayloadOverrideError::MissingTarget);
            };
            if actual_target != *target {
                return Err(GoalPayloadOverrideError::UnsupportedGoal);
            }
            Ok(Some(ActionPayload::Combat(CombatActionPayload {
                target: actual_target,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            })))
        }
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_declare_support_payload_override(
    goal: &GoalKind,
    actor: EntityId,
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    match goal {
        GoalKind::ClaimOffice { office } => Ok(Some(ActionPayload::DeclareSupport(
            DeclareSupportActionPayload {
                office: *office,
                candidate: actor,
            },
        ))),
        GoalKind::SupportCandidateForOffice { office, candidate } => {
            Ok(Some(ActionPayload::DeclareSupport(
                DeclareSupportActionPayload {
                    office: *office,
                    candidate: *candidate,
                },
            )))
        }
        _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
    }
}

fn build_loot_payload_override(
    targets: &[EntityId],
) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
    let Some(target) = targets.first().copied() else {
        return Err(GoalPayloadOverrideError::MissingTarget);
    };
    Ok(Some(ActionPayload::Loot(LootActionPayload { target })))
}

impl GoalKindPlannerExt for GoalKind {
    fn goal_kind_tag(&self) -> GoalKindTag {
        match self {
            GoalKind::ConsumeOwnedCommodity { .. } => GoalKindTag::ConsumeOwnedCommodity,
            GoalKind::AcquireCommodity { .. } => GoalKindTag::AcquireCommodity,
            GoalKind::Sleep => GoalKindTag::Sleep,
            GoalKind::Relieve => GoalKindTag::Relieve,
            GoalKind::Wash => GoalKindTag::Wash,
            GoalKind::EngageHostile { .. } => GoalKindTag::EngageHostile,
            GoalKind::ReduceDanger => GoalKindTag::ReduceDanger,
            GoalKind::Heal { .. } => GoalKindTag::Heal,
            GoalKind::ProduceCommodity { .. } => GoalKindTag::ProduceCommodity,
            GoalKind::SellCommodity { .. } => GoalKindTag::SellCommodity,
            GoalKind::RestockCommodity { .. } => GoalKindTag::RestockCommodity,
            GoalKind::MoveCargo { .. } => GoalKindTag::MoveCargo,
            GoalKind::LootCorpse { .. } => GoalKindTag::LootCorpse,
            GoalKind::BuryCorpse { .. } => GoalKindTag::BuryCorpse,
            GoalKind::ShareBelief { .. } => GoalKindTag::ShareBelief,
            GoalKind::ClaimOffice { .. } => GoalKindTag::ClaimOffice,
            GoalKind::SupportCandidateForOffice { .. } => {
                GoalKindTag::SupportCandidateForOffice
            }
        }
    }

    fn relevant_op_kinds(&self) -> &'static [PlannerOpKind] {
        match self {
            GoalKind::ConsumeOwnedCommodity { .. } => CONSUME_OPS,
            GoalKind::AcquireCommodity { .. } => ACQUIRE_OPS,
            GoalKind::Sleep => SLEEP_OPS,
            GoalKind::Relieve => RELIEVE_OPS,
            GoalKind::Wash => WASH_OPS,
            GoalKind::EngageHostile { .. } => ENGAGE_HOSTILE_OPS,
            GoalKind::ReduceDanger => REDUCE_DANGER_OPS,
            GoalKind::Heal { .. } => HEAL_OPS,
            GoalKind::ProduceCommodity { .. } => PRODUCE_OPS,
            GoalKind::SellCommodity { .. } => SELL_OPS,
            GoalKind::RestockCommodity { .. } => RESTOCK_OPS,
            GoalKind::MoveCargo { .. } => MOVE_CARGO_OPS,
            GoalKind::LootCorpse { .. } => LOOT_OPS,
            GoalKind::BuryCorpse { .. } => BURY_OPS,
            GoalKind::ShareBelief { .. } => SHARE_BELIEF_OPS,
            GoalKind::ClaimOffice { .. } => CLAIM_OFFICE_OPS,
            GoalKind::SupportCandidateForOffice { .. } => SUPPORT_OFFICE_OPS,
        }
    }

    fn relevant_observed_commodities(
        &self,
        recipes: &RecipeRegistry,
    ) -> Option<BTreeSet<CommodityKind>> {
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity }
            | GoalKind::AcquireCommodity { commodity, .. }
            | GoalKind::SellCommodity { commodity }
            | GoalKind::RestockCommodity { commodity }
            | GoalKind::MoveCargo { commodity, .. } => Some([*commodity].into_iter().collect()),
            GoalKind::ProduceCommodity { recipe_id } => recipes.get(*recipe_id).map(|recipe| {
                recipe
                    .inputs
                    .iter()
                    .chain(recipe.outputs.iter())
                    .map(|(commodity, _)| *commodity)
                    .collect()
            }),
            GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::EngageHostile { .. }
            | GoalKind::ReduceDanger
            | GoalKind::Heal { .. }
            | GoalKind::LootCorpse { .. }
            | GoalKind::BuryCorpse { .. }
            | GoalKind::ShareBelief { .. }
            | GoalKind::ClaimOffice { .. }
            | GoalKind::SupportCandidateForOffice { .. } => Some(BTreeSet::new()),
        }
    }

    fn build_payload_override(
        &self,
        affordance_payload: Option<&ActionPayload>,
        state: &PlanningState<'_>,
        targets: &[EntityId],
        def: &ActionDef,
        semantics: &PlannerOpSemantics,
    ) -> Result<Option<ActionPayload>, GoalPayloadOverrideError> {
        if let Some(payload) = payload_override_from_affordance(self, affordance_payload)? {
            return Ok(Some(payload));
        }

        let actor = state.snapshot().actor();
        match semantics.op_kind {
            PlannerOpKind::Trade => {
                let Some(counterparty) = targets.first().copied() else {
                    return Err(GoalPayloadOverrideError::MissingTarget);
                };
                let requested_commodity = match self {
                    GoalKind::AcquireCommodity { commodity, .. }
                    | GoalKind::RestockCommodity { commodity }
                    | GoalKind::ConsumeOwnedCommodity { commodity } => *commodity,
                    GoalKind::Heal { .. } => CommodityKind::Medicine,
                    _ => return Err(GoalPayloadOverrideError::UnsupportedGoal),
                };
                let Some(actor_place) = state.effective_place(actor) else {
                    return Err(GoalPayloadOverrideError::MissingActorPlace);
                };
                if !state
                    .agents_selling_at(actor_place, requested_commodity)
                    .contains(&counterparty)
                {
                    return Err(GoalPayloadOverrideError::SellerUnavailable);
                }
                if state.commodity_quantity(counterparty, requested_commodity) == Quantity(0) {
                    return Err(GoalPayloadOverrideError::SellerOutOfStock);
                }
                if state.commodity_quantity(actor, CommodityKind::Coin) == Quantity(0) {
                    return Err(GoalPayloadOverrideError::ActorCannotPay);
                }
                Ok(Some(ActionPayload::Trade(TradeActionPayload {
                    counterparty,
                    offered_commodity: CommodityKind::Coin,
                    offered_quantity: Quantity(1),
                    requested_commodity,
                    requested_quantity: Quantity(1),
                })))
            }
            PlannerOpKind::Attack => build_attack_payload_override(self, targets),
            PlannerOpKind::Tell => match self {
                GoalKind::ShareBelief { listener, subject } => {
                    let Some(target_listener) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if target_listener != *listener {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::Tell(TellActionPayload {
                        listener: *listener,
                        subject_entity: *subject,
                    })))
                }
                _ => Err(GoalPayloadOverrideError::UnsupportedGoal),
            },
            PlannerOpKind::DeclareSupport => build_declare_support_payload_override(self, actor),
            PlannerOpKind::Loot => build_loot_payload_override(targets),
            PlannerOpKind::MoveCargo => match self {
                GoalKind::MoveCargo {
                    commodity,
                    destination,
                } if def.name == "pick_up" => {
                    let Some(target) = targets.first().copied() else {
                        return Err(GoalPayloadOverrideError::MissingTarget);
                    };
                    if state.item_lot_commodity(target) != Some(*commodity) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    let lot_quantity = state.commodity_quantity(target, *commodity);
                    let Some(restock_gap) =
                        restock_gap_at_destination(state, actor, *destination, *commodity)
                    else {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    };
                    let remaining_capacity = state
                        .remaining_carry_capacity_ref(crate::PlanningEntityRef::Authoritative(
                            actor,
                        ))
                        .ok_or(GoalPayloadOverrideError::UnsupportedGoal)?
                        .0;
                    let per_unit = worldwake_core::load_per_unit(*commodity).0;
                    let carry_fit = Quantity(remaining_capacity / per_unit);
                    let quantity = Quantity(lot_quantity.0.min(restock_gap.0).min(carry_fit.0));
                    if quantity == Quantity(0) {
                        return Err(GoalPayloadOverrideError::UnsupportedGoal);
                    }
                    Ok(Some(ActionPayload::Transport(TransportActionPayload {
                        quantity,
                    })))
                }
                _ => Ok((!matches!(def.payload, ActionPayload::None)).then(|| def.payload.clone())),
            },
            _ => Ok((!matches!(def.payload, ActionPayload::None)).then(|| def.payload.clone())),
        }
    }

    fn apply_planner_step<'snapshot>(
        &self,
        state: PlanningState<'snapshot>,
        op_kind: PlannerOpKind,
        targets: &[EntityId],
        payload_override: Option<&ActionPayload>,
    ) -> PlanningState<'snapshot> {
        let actor = state.snapshot().actor();
        // Cargo uses transport transition kinds in planner_ops.rs for hypothetical state changes,
        // so MoveCargo intentionally falls through the default no-op path here.
        match op_kind {
            PlannerOpKind::Travel => {
                if let Some(destination) = targets.first().copied() {
                    state.move_actor_to(destination)
                } else {
                    state
                }
            }
            PlannerOpKind::Consume => match self {
                GoalKind::ConsumeOwnedCommodity { commodity }
                | GoalKind::AcquireCommodity { commodity, .. } => {
                    state.consume_commodity(*commodity)
                }
                _ => state,
            },
            PlannerOpKind::Sleep => update_actor_needs(state, |needs, thresholds| {
                needs.fatigue = below_medium(thresholds.fatigue.medium());
            }),
            PlannerOpKind::Relieve => update_actor_needs(state, |needs, thresholds| {
                needs.bladder = below_medium(thresholds.bladder.medium());
            }),
            PlannerOpKind::Wash => update_actor_needs(state, |needs, thresholds| {
                needs.dirtiness = below_medium(thresholds.dirtiness.medium());
            }),
            PlannerOpKind::Heal => match self {
                GoalKind::Heal { target } => state.with_pain(*target, Permille::new_unchecked(0)),
                _ => state,
            },
            PlannerOpKind::Loot => match self {
                GoalKind::LootCorpse { corpse } => {
                    let actor = state.snapshot().actor();
                    CommodityKind::ALL
                        .iter()
                        .copied()
                        .fold(state, |next, commodity| {
                            let quantity = next.commodity_quantity(*corpse, commodity);
                            if quantity == Quantity(0) {
                                return next;
                            }
                            let actor_quantity = next.commodity_quantity(actor, commodity);
                            next.with_commodity_quantity(*corpse, commodity, Quantity(0))
                                .with_commodity_quantity(
                                    actor,
                                    commodity,
                                    Quantity(actor_quantity.0.saturating_add(quantity.0)),
                                )
                        })
                }
                _ => state,
            },
            PlannerOpKind::Bury => match self {
                GoalKind::BuryCorpse {
                    corpse,
                    burial_site,
                } => state.set_container_ref(
                    PlanningEntityRef::Authoritative(*corpse),
                    PlanningEntityRef::Authoritative(*burial_site),
                ),
                _ => state,
            },
            PlannerOpKind::QueueForFacilityUse => {
                let queued_use = targets.first().copied().zip(
                    payload_override
                        .and_then(ActionPayload::as_queue_for_facility_use)
                        .map(|payload| payload.intended_action),
                );
                if let Some((facility, intended_action)) = queued_use {
                    state.simulate_queue_join(facility, intended_action)
                } else {
                    state
                }
            }
            PlannerOpKind::DeclareSupport => match self {
                GoalKind::ClaimOffice { office } => {
                    state.with_support_declaration(actor, *office, actor)
                }
                GoalKind::SupportCandidateForOffice { office, candidate } => {
                    state.with_support_declaration(actor, *office, *candidate)
                }
                _ => state,
            },
            _ => state,
        }
    }

    fn is_progress_barrier(&self, step: &PlannedStep) -> bool {
        if step.op_kind == PlannerOpKind::QueueForFacilityUse {
            return matches!(
                self,
                GoalKind::ConsumeOwnedCommodity { .. }
                    | GoalKind::AcquireCommodity { .. }
                    | GoalKind::Heal { .. }
                    | GoalKind::ProduceCommodity { .. }
                    | GoalKind::RestockCommodity { .. }
            );
        }

        if matches!(self, GoalKind::ShareBelief { .. }) && step.op_kind == PlannerOpKind::Tell {
            return true;
        }

        if matches!(
            self,
            GoalKind::ClaimOffice { .. } | GoalKind::SupportCandidateForOffice { .. }
        ) && step.op_kind == PlannerOpKind::DeclareSupport
        {
            return true;
        }

        if !step.is_materialization_barrier {
            return false;
        }

        // Cargo state changes are modeled by transport transition kinds in planner_ops.rs, and
        // the commodity+destination goal identity survives lot splitting, so cargo intentionally
        // falls through the default non-barrier behavior here.
        match self {
            GoalKind::AcquireCommodity { .. }
            | GoalKind::ProduceCommodity { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::LootCorpse { .. }
            | GoalKind::BuryCorpse { .. } => true,
            GoalKind::ConsumeOwnedCommodity { .. } => matches!(
                step.op_kind,
                PlannerOpKind::Trade
                    | PlannerOpKind::Harvest
                    | PlannerOpKind::Craft
                    | PlannerOpKind::MoveCargo
            ),
            GoalKind::Heal { .. } => step.op_kind == PlannerOpKind::Trade,
            _ => false,
        }
    }

    fn is_satisfied(&self, state: &PlanningState<'_>) -> bool {
        let actor = state.snapshot().actor();
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity } => {
                let Some(needs) = state.homeostatic_needs(actor) else {
                    return false;
                };
                let Some(thresholds) = state.drive_thresholds(actor) else {
                    return false;
                };
                match commodity {
                    CommodityKind::Bread | CommodityKind::Apple | CommodityKind::Grain => {
                        needs.hunger < thresholds.hunger.medium()
                    }
                    CommodityKind::Water => needs.thirst < thresholds.thirst.medium(),
                    _ => false,
                }
            }
            GoalKind::AcquireCommodity { commodity, purpose } => match purpose {
                CommodityPurpose::SelfConsume
                | CommodityPurpose::Restock
                | CommodityPurpose::Treatment
                | CommodityPurpose::RecipeInput(_) => {
                    state.commodity_quantity(actor, *commodity) > Quantity(0)
                }
            },
            GoalKind::Sleep => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.fatigue < thresholds.fatigue.medium()),
            GoalKind::Relieve => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.bladder < thresholds.bladder.medium()),
            GoalKind::Wash => state
                .homeostatic_needs(actor)
                .zip(state.drive_thresholds(actor))
                .is_some_and(|(needs, thresholds)| needs.dirtiness < thresholds.dirtiness.medium()),
            GoalKind::EngageHostile { target } => {
                state.is_dead(*target) || !state.visible_hostiles_for(actor).contains(target)
            }
            GoalKind::ReduceDanger => state.drive_thresholds(actor).is_some_and(|thresholds| {
                derive_danger_pressure(state, actor) < thresholds.danger.high()
            }),
            GoalKind::Heal { target } => state
                .pain_summary(*target)
                .is_some_and(|pain| pain == Permille::new_unchecked(0)),
            GoalKind::MoveCargo {
                commodity,
                destination,
            } => restock_gap_at_destination(state, actor, *destination, *commodity).is_none(),
            GoalKind::LootCorpse { corpse } => CommodityKind::ALL
                .iter()
                .copied()
                .all(|commodity| state.commodity_quantity(*corpse, commodity) == Quantity(0)),
            GoalKind::BuryCorpse { corpse, .. } => state.direct_container(*corpse).is_some(),
            GoalKind::SupportCandidateForOffice { office, candidate } => {
                state.support_declaration(actor, *office) == Some(*candidate)
            }
            GoalKind::ProduceCommodity { .. }
            | GoalKind::ShareBelief { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::SellCommodity { .. }
            | GoalKind::ClaimOffice { .. } => false,
        }
    }
}

fn update_actor_needs(
    state: PlanningState<'_>,
    apply: impl FnOnce(&mut worldwake_core::HomeostaticNeeds, worldwake_core::DriveThresholds),
) -> PlanningState<'_> {
    let actor = state.snapshot().actor();
    let Some(mut needs) = state.homeostatic_needs(actor) else {
        return state;
    };
    let Some(thresholds) = state.drive_thresholds(actor) else {
        return state;
    };
    apply(&mut needs, thresholds);
    state.with_homeostatic_needs(actor, needs)
}

fn below_medium(medium: Permille) -> Permille {
    medium.saturating_sub(Permille::new(1).unwrap())
}

#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum GoalPriorityClass {
    Background,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GroundedGoal {
    pub key: GoalKey,
    pub evidence_entities: BTreeSet<EntityId>,
    pub evidence_places: BTreeSet<EntityId>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RankedGoal {
    pub grounded: GroundedGoal,
    pub priority_class: GoalPriorityClass,
    pub motive_score: u32,
}

#[cfg(test)]
mod tests {
    use super::{GoalKindPlannerExt, GoalKindTag, GoalPriorityClass, GroundedGoal, RankedGoal};
    use crate::{
        build_planning_snapshot, CommodityPurpose, GoalKey, GoalKind, PlannedStep, PlannerOpKind,
        PlannerOpSemantics, PlannerTransitionKind, PlanningState,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::{BTreeMap, BTreeSet};
    use std::fmt::Debug;
    use std::num::NonZeroU32;
    use worldwake_core::{
        test_utils::{entity_id, sample_trade_disposition_profile},
        ActionDefId, BodyCostPerTick, CombatProfile, CommodityConsumableProfile, CommodityKind,
        DemandObservation, DemandObservationReason, DriveThresholds, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile,
        Permille, Quantity, RecipeId, ResourceSource, Tick, TickRange, TradeDispositionProfile,
        UniqueItemKind, VisibilitySpec, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDef, ActionDomain, ActionDuration, ActionHandlerId,
        ActionPayload, DurationExpr, Interruptibility, QueueForFacilityUsePayload,
        RuntimeBeliefView, TellActionPayload, TradeActionPayload, TransportActionPayload,
    };

    fn assert_value_bounds<T: Clone + Eq + Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn goal_priority_class_satisfies_required_bounds() {
        assert_value_bounds::<GoalPriorityClass>();
        assert!(GoalPriorityClass::Critical > GoalPriorityClass::High);
        assert!(GoalPriorityClass::High > GoalPriorityClass::Medium);
        assert!(GoalPriorityClass::Medium > GoalPriorityClass::Low);
        assert!(GoalPriorityClass::Low > GoalPriorityClass::Background);
    }

    #[test]
    fn grounded_goal_satisfies_required_bounds() {
        assert_value_bounds::<GroundedGoal>();
        assert_value_bounds::<RankedGoal>();
    }

    #[test]
    fn crate_re_exports_the_canonical_shared_goal_identity() {
        let kind = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Water,
            purpose: CommodityPurpose::Treatment,
        };
        let key = GoalKey::from(kind);

        assert_eq!(key.kind, kind);
        assert_eq!(key.commodity, Some(CommodityKind::Water));
    }

    #[test]
    fn grounded_goal_roundtrips_through_bincode() {
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::Heal {
                target: entity_id(7, 1),
            }),
            evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
            evidence_places: BTreeSet::from([entity_id(10, 0)]),
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: GroundedGoal = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn ranked_goal_roundtrips_through_bincode() {
        let goal = RankedGoal {
            grounded: GroundedGoal {
                key: GoalKey::from(GoalKind::Heal {
                    target: entity_id(7, 1),
                }),
                evidence_entities: BTreeSet::from([entity_id(3, 0), entity_id(3, 1)]),
                evidence_places: BTreeSet::from([entity_id(10, 0)]),
            },
            priority_class: GoalPriorityClass::High,
            motive_score: 900,
        };

        let bytes = bincode::serialize(&goal).unwrap();
        let roundtrip: RankedGoal = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, goal);
    }

    #[test]
    fn goal_kind_tag_tracks_goal_families_without_payload_identity() {
        assert_eq!(
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::Treatment,
            }
            .goal_kind_tag(),
            GoalKindTag::AcquireCommodity
        );
        assert_eq!(
            GoalKind::BuryCorpse {
                corpse: entity_id(1, 0),
                burial_site: entity_id(2, 0),
            }
            .goal_kind_tag(),
            GoalKindTag::BuryCorpse
        );
        assert_eq!(
            GoalKind::ShareBelief {
                listener: entity_id(3, 0),
                subject: entity_id(4, 0),
            }
            .goal_kind_tag(),
            GoalKindTag::ShareBelief
        );
    }

    #[test]
    fn consume_goal_relevant_ops_include_consumption_and_access_paths() {
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Consume));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Attack));
    }

    #[test]
    fn reduce_danger_goal_relevant_ops_include_defense_leaf_options() {
        let goal = GoalKind::ReduceDanger;

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Defend));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Heal));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Attack));
    }

    #[test]
    fn engage_hostile_goal_relevant_ops_are_attack_only() {
        let goal = GoalKind::EngageHostile {
            target: entity_id(4, 0),
        };

        assert_eq!(goal.relevant_op_kinds(), &[PlannerOpKind::Attack]);
    }

    #[test]
    fn share_belief_goal_relevant_ops_are_tell_only() {
        let goal = GoalKind::ShareBelief {
            listener: entity_id(4, 0),
            subject: entity_id(5, 0),
        };

        assert_eq!(goal.relevant_op_kinds(), &[PlannerOpKind::Tell]);
    }

    #[test]
    fn sleep_goal_observed_commodities_are_empty() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::Sleep.relevant_observed_commodities(&recipes),
            Some(BTreeSet::new())
        );
    }

    #[test]
    fn share_belief_goal_observed_commodities_are_empty() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::ShareBelief {
                listener: entity_id(6, 0),
                subject: entity_id(7, 0),
            }
            .relevant_observed_commodities(&recipes),
            Some(BTreeSet::new())
        );
    }

    #[test]
    fn share_belief_tell_step_is_a_progress_barrier() {
        let goal = GoalKind::ShareBelief {
            listener: entity_id(6, 0),
            subject: entity_id(7, 0),
        };
        let step = PlannedStep {
            def_id: ActionDefId(77),
            op_kind: PlannerOpKind::Tell,
            targets: vec![crate::PlanningEntityRef::Authoritative(entity_id(6, 0))],
            payload_override: None,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        };

        assert!(goal.is_progress_barrier(&step));
    }

    #[test]
    fn move_cargo_goal_observed_commodities_track_goal_commodity_only() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: entity_id(5, 0),
            }
            .relevant_observed_commodities(&recipes),
            Some(BTreeSet::from([CommodityKind::Bread]))
        );
    }

    #[test]
    fn produce_goal_observed_commodities_include_recipe_inputs_and_outputs() {
        let mut recipes = worldwake_sim::RecipeRegistry::new();
        let recipe_id = recipes.register(worldwake_sim::RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
                Permille::new(1).unwrap(),
            ),
        });

        assert_eq!(
            GoalKind::ProduceCommodity { recipe_id }.relevant_observed_commodities(&recipes),
            Some(BTreeSet::from([CommodityKind::Bread, CommodityKind::Grain]))
        );
    }

    #[test]
    fn missing_produce_recipe_falls_back_to_full_observed_commodity_tracking() {
        let recipes = worldwake_sim::RecipeRegistry::new();

        assert_eq!(
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(999),
            }
            .relevant_observed_commodities(&recipes),
            None
        );
    }

    #[test]
    fn restock_goal_relevant_ops_include_trade_production_and_cargo() {
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Trade));
        assert!(goal
            .relevant_op_kinds()
            .contains(&PlannerOpKind::QueueForFacilityUse));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Harvest));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Craft));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::MoveCargo));
    }

    #[test]
    fn bury_goal_uses_bury_op_family() {
        let goal = GoalKind::BuryCorpse {
            corpse: entity_id(1, 0),
            burial_site: entity_id(2, 0),
        };

        assert_eq!(goal.relevant_op_kinds(), &[PlannerOpKind::Bury]);
    }

    #[test]
    fn move_cargo_satisfied_when_destination_stocked() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let bread = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at.insert(destination, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        }
        .is_satisfied(&state));
    }

    #[test]
    fn move_cargo_not_satisfied_when_destination_understocked() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let bread = entity_id(3, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, destination);
        view.effective_places.insert(bread, destination);
        view.entities_at.insert(destination, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(!GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        }
        .is_satisfied(&state));
    }

    #[test]
    fn move_cargo_satisfaction_is_destination_local() {
        let actor = entity_id(1, 0);
        let destination = entity_id(2, 0);
        let remote = entity_id(3, 0);
        let bread = entity_id(4, 0);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, remote);
        view.effective_places.insert(bread, remote);
        view.entities_at.insert(remote, vec![actor, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(3));

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([destination, remote]),
            1,
        );
        let state = PlanningState::new(&snapshot);

        assert!(!GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        }
        .is_satisfied(&state));
    }

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        consumable_profiles: BTreeMap<EntityId, CommodityConsumableProfile>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        controlled_quantities: BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        trade_profiles: BTreeMap<EntityId, TradeDispositionProfile>,
        merchandise_profiles: BTreeMap<EntityId, MerchandiseProfile>,
        wounds: BTreeMap<EntityId, Vec<Wound>>,
    }

    impl RuntimeBeliefView for TestBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            self.alive.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.kinds.get(&entity).copied()
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places.get(&entity).copied()
        }

        fn is_in_transit(&self, _entity: EntityId) -> bool {
            false
        }

        fn entities_at(&self, place: EntityId) -> Vec<EntityId> {
            self.entities_at.get(&place).cloned().unwrap_or_default()
        }

        fn direct_possessions(&self, holder: EntityId) -> Vec<EntityId> {
            self.direct_possessions
                .get(&holder)
                .cloned()
                .unwrap_or_default()
        }

        fn adjacent_places(&self, place: EntityId) -> Vec<EntityId> {
            self.adjacent_places_with_travel_ticks(place)
                .into_iter()
                .map(|(adjacent, _)| adjacent)
                .collect()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: RecipeId) -> bool {
            false
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .get(&(holder, kind))
                .copied()
                .unwrap_or(Quantity(0))
        }
        fn controlled_commodity_quantity_at_place(
            &self,
            actor: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.controlled_quantities
                .get(&(actor, place, commodity))
                .copied()
                .unwrap_or(Quantity(0))
        }
        fn local_controlled_lots_for(
            &self,
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn item_lot_commodity(&self, entity: EntityId) -> Option<CommodityKind> {
            self.lot_commodities.get(&entity).copied()
        }

        fn item_lot_consumable_profile(
            &self,
            entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            self.consumable_profiles.get(&entity).copied()
        }

        fn direct_container(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors.get(&entity).copied()
        }

        fn believed_owner_of(&self, _entity: EntityId) -> Option<EntityId> {
            None
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            actor == entity || self.direct_possessor(entity) == Some(actor)
        }

        fn has_control(&self, entity: EntityId) -> bool {
            self.kinds.get(&entity) == Some(&EntityKind::Agent)
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities.get(&entity).copied()
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.entity_loads.get(&entity).copied()
        }

        fn reservation_conflicts(&self, _entity: EntityId, _range: TickRange) -> bool {
            false
        }

        fn reservation_ranges(&self, _entity: EntityId) -> Vec<TickRange> {
            Vec::new()
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            !self.is_alive(entity)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            self.wounds
                .get(&entity)
                .is_some_and(|wounds| !wounds.is_empty())
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs.get(&agent).copied()
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.thresholds.get(&agent).copied()
        }
        fn belief_confidence_policy(
            &self,
            _agent: EntityId,
        ) -> worldwake_core::BeliefConfidencePolicy {
            worldwake_core::BeliefConfidencePolicy::default()
        }

        fn metabolism_profile(&self, _agent: EntityId) -> Option<MetabolismProfile> {
            Some(MetabolismProfile::default())
        }

        fn trade_disposition_profile(&self, agent: EntityId) -> Option<TradeDispositionProfile> {
            self.trade_profiles.get(&agent).cloned()
        }

        fn travel_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::TravelDispositionProfile> {
            None
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<CombatProfile> {
            Some(CombatProfile::new(
                pm(1000),
                pm(700),
                pm(620),
                pm(580),
                pm(80),
                pm(25),
                pm(18),
                pm(120),
                pm(35),
                NonZeroU32::new(6).unwrap(),
            ))
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds.get(&agent).cloned().unwrap_or_default()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn agents_selling_at(&self, place: EntityId, commodity: CommodityKind) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.merchandise_profiles
                        .get(entity)
                        .is_some_and(|profile| profile.sale_kinds.contains(&commodity))
                })
                .collect()
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<RecipeId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn demand_memory(&self, agent: EntityId) -> Vec<DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn merchandise_profile(&self, agent: EntityId) -> Option<MerchandiseProfile> {
            self.merchandise_profiles.get(&agent).cloned()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn in_transit_state(&self, _entity: EntityId) -> Option<InTransitOnEdge> {
            None
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacent.get(&place).cloned().unwrap_or_default()
        }

        fn estimate_duration(
            &self,
            actor: EntityId,
            duration: &DurationExpr,
            targets: &[EntityId],
            payload: &ActionPayload,
        ) -> Option<ActionDuration> {
            estimate_duration_from_beliefs(self, actor, duration, targets, payload)
        }
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

    fn base_view() -> (TestBeliefView, EntityId, EntityId) {
        let actor = entity(1);
        let seller = entity(2);
        let town = entity(10);
        let bread = entity(20);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, seller, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(seller, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(seller, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, seller, bread]);
        view.direct_possessions.insert(actor, vec![bread]);
        view.direct_possessors.insert(bread, actor);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.consumable_profiles.insert(
            bread,
            CommodityConsumableProfile::new(NonZeroU32::new(2).unwrap(), pm(250), pm(0), pm(0)),
        );
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        view.commodity_quantities
            .insert((actor, CommodityKind::Coin), Quantity(3));
        view.commodity_quantities
            .insert((seller, CommodityKind::Bread), Quantity(2));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(700), pm(0), pm(700), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        view.trade_profiles
            .insert(seller, sample_trade_disposition_profile());
        view.merchandise_profiles.insert(
            seller,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: None,
            },
        );
        (view, actor, seller)
    }

    #[test]
    fn acquire_goal_builds_trade_payload_override_from_goal_semantics() {
        let (view, actor, seller) = base_view();
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let def = ActionDef {
            id: ActionDefId(9),
            name: "trade".to_string(),
            domain: ActionDomain::Trade,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Trade,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::GoalModelFallback,
            relevant_goal_kinds: &[],
        };

        let payload = goal
            .build_payload_override(None, &state, &[seller], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: seller,
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(1),
                requested_commodity: CommodityKind::Bread,
                requested_quantity: Quantity(1),
            }))
        );
    }

    #[test]
    fn move_cargo_pickup_builds_exact_transport_quantity_payload() {
        let actor = entity(1);
        let origin = entity(10);
        let destination = entity(11);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, origin, destination, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(origin, EntityKind::Place);
        view.kinds.insert(destination, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, origin);
        view.effective_places.insert(bread, origin);
        view.entities_at.insert(origin, vec![actor, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(5));
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(0));
        view.direct_possessions.insert(actor, Vec::new());
        view.carry_capacities.insert(actor, LoadUnits(2));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(3),
                place: destination,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([origin, destination]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination,
        };
        let def = ActionDef {
            id: ActionDefId(9),
            name: "pick_up".to_string(),
            domain: ActionDomain::Transport,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PickUpGroundLot,
            relevant_goal_kinds: &[],
        };

        let payload = goal
            .build_payload_override(None, &state, &[bread], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(2),
            }))
        );
    }

    #[test]
    fn share_belief_goal_builds_tell_payload_override() {
        let actor = entity(1);
        let listener = entity(2);
        let subject = entity(3);
        let place = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, listener, subject, place]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(listener, EntityKind::Agent);
        view.kinds.insert(subject, EntityKind::Facility);
        view.kinds.insert(place, EntityKind::Place);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(listener, place);
        view.effective_places.insert(subject, place);
        view.entities_at.insert(place, vec![actor, listener, subject]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([listener, subject]),
            &BTreeSet::from([place]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ShareBelief { listener, subject };
        let def = ActionDef {
            id: ActionDefId(10),
            name: "tell".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: BodyCostPerTick::zero(),
            interruptibility: Interruptibility::FreelyInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::Hidden,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::Tell,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::GoalModelFallback,
            relevant_goal_kinds: &[],
        };

        let payload = goal
            .build_payload_override(None, &state, &[listener], &def, &semantics)
            .unwrap();

        assert_eq!(
            payload,
            Some(ActionPayload::Tell(TellActionPayload {
                listener,
                subject_entity: subject,
            }))
        );
    }

    #[test]
    fn consume_goal_satisfaction_is_owned_by_goal_model() {
        let (mut view, actor, _seller) = base_view();
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        let hungry_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let hungry_state = PlanningState::new(&hungry_snapshot);
        assert!(!goal.is_satisfied(&hungry_state));

        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(100), pm(0), pm(700), pm(0), pm(0)),
        );
        let satiated_snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let satiated_state = PlanningState::new(&satiated_snapshot);
        assert!(goal.is_satisfied(&satiated_state));
    }

    #[test]
    fn progress_barrier_semantics_move_with_goal_model() {
        let acquire_goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let sleep_goal = GoalKind::Sleep;
        let barrier_step = PlannedStep {
            def_id: ActionDefId(1),
            targets: Vec::new(),
            payload_override: None,
            op_kind: PlannerOpKind::Harvest,
            estimated_ticks: 3,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
        };

        assert!(acquire_goal.is_progress_barrier(&barrier_step));
        assert!(!sleep_goal.is_progress_barrier(&barrier_step));
    }

    #[test]
    fn apply_planner_step_updates_hypothetical_state_via_goal_semantics() {
        let (view, actor, _seller) = base_view();
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };
        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let base_state = PlanningState::new(&snapshot);

        let advanced = goal.apply_planner_step(base_state, PlannerOpKind::Consume, &[], None);

        assert!(
            advanced.homeostatic_needs(actor).unwrap().hunger
                < DriveThresholds::default().hunger.low()
        );
    }

    #[test]
    fn loot_goal_step_transfers_believed_corpse_inventory_and_satisfies_goal() {
        let (mut view, actor, _seller) = base_view();
        let corpse = entity(30);
        let town = entity(10);
        view.kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(corpse, town);
        view.entities_at.entry(town).or_default().push(corpse);
        view.commodity_quantities
            .insert((corpse, CommodityKind::Coin), Quantity(5));

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let base_state = PlanningState::new(&snapshot);
        let goal = GoalKind::LootCorpse { corpse };

        assert!(!goal.is_satisfied(&base_state));

        let advanced = goal.apply_planner_step(base_state, PlannerOpKind::Loot, &[corpse], None);

        assert_eq!(
            advanced.commodity_quantity(corpse, CommodityKind::Coin),
            Quantity(0)
        );
        assert_eq!(
            advanced.commodity_quantity(actor, CommodityKind::Coin),
            Quantity(8)
        );
        assert!(goal.is_satisfied(&advanced));
    }

    #[test]
    fn bury_goal_step_marks_corpse_contained_and_satisfies_goal() {
        let (mut view, actor, _seller) = base_view();
        let corpse = entity(30);
        let grave_plot = entity(31);
        let town = entity(10);
        view.kinds.insert(corpse, EntityKind::Agent);
        view.kinds.insert(grave_plot, EntityKind::Facility);
        view.effective_places.insert(corpse, town);
        view.effective_places.insert(grave_plot, town);
        view.entities_at
            .entry(town)
            .or_default()
            .extend([corpse, grave_plot]);

        let snapshot = build_planning_snapshot(&view, actor, &BTreeSet::new(), &BTreeSet::new(), 2);
        let base_state = PlanningState::new(&snapshot);
        let goal = GoalKind::BuryCorpse {
            corpse,
            burial_site: grave_plot,
        };

        assert!(!goal.is_satisfied(&base_state));

        let advanced =
            goal.apply_planner_step(base_state, PlannerOpKind::Bury, &[corpse, grave_plot], None);

        assert_eq!(advanced.direct_container(corpse), Some(grave_plot));
        assert!(goal.is_satisfied(&advanced));
    }

    #[test]
    fn queue_for_facility_use_step_simulates_queue_join_from_payload() {
        let (view, actor, _seller) = base_view();
        let field = entity(30);
        let snapshot =
            build_planning_snapshot(&view, actor, &BTreeSet::from([field]), &BTreeSet::new(), 1);
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };

        let advanced = goal.apply_planner_step(
            PlanningState::new(&snapshot),
            PlannerOpKind::QueueForFacilityUse,
            &[field],
            Some(&ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: ActionDefId(44),
                },
            )),
        );

        assert!(advanced.is_actor_queued_at_facility(field));
        assert!(!advanced.has_actor_facility_grant(field, ActionDefId(44)));
    }

    #[test]
    fn queue_for_facility_use_is_progress_barrier_for_exclusive_goal_families() {
        let queue_step = PlannedStep {
            def_id: ActionDefId(7),
            targets: Vec::new(),
            payload_override: Some(ActionPayload::QueueForFacilityUse(
                QueueForFacilityUsePayload {
                    intended_action: ActionDefId(19),
                },
            )),
            op_kind: PlannerOpKind::QueueForFacilityUse,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        };

        assert!(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: CommodityPurpose::Restock,
        }
        .is_progress_barrier(&queue_step));
        assert!(GoalKind::ProduceCommodity {
            recipe_id: RecipeId(0),
        }
        .is_progress_barrier(&queue_step));
        assert!(!GoalKind::Sleep.is_progress_barrier(&queue_step));
    }

    #[test]
    fn political_goals_expose_political_op_families() {
        assert_eq!(
            GoalKind::ClaimOffice { office: entity(40) }.relevant_op_kinds(),
            &[
                PlannerOpKind::Travel,
                PlannerOpKind::Bribe,
                PlannerOpKind::Threaten,
                PlannerOpKind::DeclareSupport,
            ]
        );
        assert_eq!(
            GoalKind::SupportCandidateForOffice {
                office: entity(40),
                candidate: entity(41),
            }
            .relevant_op_kinds(),
            &[PlannerOpKind::Travel, PlannerOpKind::DeclareSupport]
        );
    }

    #[test]
    fn support_candidate_builds_declare_support_payload_and_satisfies_after_step() {
        let actor = entity(1);
        let office = entity(40);
        let candidate = entity(41);
        let town = entity(10);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, candidate, office]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(candidate, EntityKind::Agent);
        view.kinds.insert(office, EntityKind::Office);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(candidate, town);
        view.effective_places.insert(office, town);
        view.entities_at.insert(town, vec![actor, candidate, office]);

        let snapshot = build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([office, candidate]),
            &BTreeSet::from([town]),
            1,
        );
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::SupportCandidateForOffice { office, candidate };
        let def = ActionDef {
            id: ActionDefId(77),
            name: "declare_support".to_string(),
            domain: ActionDomain::Social,
            actor_constraints: Vec::new(),
            targets: Vec::new(),
            preconditions: Vec::new(),
            reservation_requirements: Vec::new(),
            duration: DurationExpr::Fixed(NonZeroU32::new(1).unwrap()),
            body_cost_per_tick: worldwake_core::BodyCostPerTick::zero(),
            interruptibility: Interruptibility::NonInterruptible,
            commit_conditions: Vec::new(),
            visibility: VisibilitySpec::SamePlace,
            causal_event_tags: BTreeSet::new(),
            payload: ActionPayload::None,
            handler: ActionHandlerId(0),
        };
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::DeclareSupport,
            may_appear_mid_plan: false,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::GoalModelFallback,
            relevant_goal_kinds: &[GoalKindTag::SupportCandidateForOffice],
        };

        let payload = goal
            .build_payload_override(None, &state, &[], &def, &semantics)
            .unwrap()
            .unwrap();
        assert_eq!(
            payload.as_declare_support(),
            Some(&worldwake_sim::DeclareSupportActionPayload { office, candidate })
        );

        let progressed =
            goal.apply_planner_step(state, PlannerOpKind::DeclareSupport, &[], Some(&payload));
        assert!(goal.is_satisfied(&progressed));
        assert!(goal.is_progress_barrier(&PlannedStep {
            def_id: def.id,
            targets: Vec::new(),
            payload_override: Some(payload),
            op_kind: PlannerOpKind::DeclareSupport,
            estimated_ticks: 1,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }));
    }
}
