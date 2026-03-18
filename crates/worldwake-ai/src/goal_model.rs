use crate::{
    derive_danger_pressure, enterprise::restock_gap_at_destination, PlannedStep, PlannerOpKind,
    PlannerOpSemantics, PlanningEntityRef, PlanningState,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use worldwake_core::{
    CommodityKind, CommodityPurpose, EntityId, GoalKey, GoalKind, Permille, PlaceTag, Quantity,
    WorkstationTag,
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
    TreatWounds,
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
    /// Places where this goal can potentially be achieved.
    /// Used by the A* heuristic to guide travel toward goal-relevant locations.
    /// Returns empty if the goal has no spatial preference (heuristic defaults to h=0).
    fn goal_relevant_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
    ) -> Vec<EntityId>;
    /// Whether the given `op_kind` acting on `authoritative_targets` satisfies
    /// this goal's target-binding requirement.
    ///
    /// - Empty `authoritative_targets` → always `true` (planner-only synthetic candidates).
    /// - Auxiliary ops → always `true` (they serve the goal indirectly).
    /// - Terminal ops on exact-bound goals → `true` only if targets contain the
    ///   goal's canonical entity.
    /// - Flexible goals → always `true` regardless of op or targets.
    fn matches_binding(
        &self,
        authoritative_targets: &[EntityId],
        op_kind: PlannerOpKind,
    ) -> bool;
}

const CONSUME_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Consume,
    PlannerOpKind::Travel,
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
const TREAT_WOUNDS_OPS: &[PlannerOpKind] = &[
    PlannerOpKind::Travel,
    PlannerOpKind::Heal,
    PlannerOpKind::Trade,
    PlannerOpKind::QueueForFacilityUse,
    PlannerOpKind::Craft,
    PlannerOpKind::MoveCargo,
    PlannerOpKind::Harvest,
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
            GoalKind::TreatWounds { .. } => GoalKindTag::TreatWounds,
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
            GoalKind::TreatWounds { .. } => TREAT_WOUNDS_OPS,
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
            | GoalKind::TreatWounds { .. }
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
                    GoalKind::TreatWounds { .. } => CommodityKind::Medicine,
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

    #[allow(clippy::too_many_lines)]
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
                GoalKind::TreatWounds { patient } => state.with_pain(*patient, Permille::new_unchecked(0)),
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
            PlannerOpKind::Bribe => match self {
                GoalKind::ClaimOffice { office } => {
                    apply_bribe_for_office(state, actor, *office, payload_override)
                }
                _ => state,
            },
            PlannerOpKind::Threaten => match self {
                GoalKind::ClaimOffice { office } => {
                    apply_threaten_for_office(state, actor, *office, payload_override)
                }
                _ => state,
            },
            PlannerOpKind::Trade
            | PlannerOpKind::Harvest
            | PlannerOpKind::Craft
            | PlannerOpKind::Attack
            | PlannerOpKind::Defend
            | PlannerOpKind::Tell
            | PlannerOpKind::MoveCargo => state,
        }
    }

    fn is_progress_barrier(&self, step: &PlannedStep) -> bool {
        if step.op_kind == PlannerOpKind::QueueForFacilityUse {
            return matches!(
                self,
                GoalKind::ConsumeOwnedCommodity { .. }
                    | GoalKind::AcquireCommodity { .. }
                    | GoalKind::TreatWounds { .. }
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

        // ConsumeOwnedCommodity treats pick_up (MoveCargo) as a progress barrier
        // because the planner cannot model possession transfer in hypothetical state.
        // This check runs before the is_materialization_barrier guard because MoveCargo
        // is not a materialization barrier but IS a logical barrier for consumption goals.
        if matches!(self, GoalKind::ConsumeOwnedCommodity { .. })
            && step.op_kind == PlannerOpKind::MoveCargo
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
            GoalKind::TreatWounds { .. } => step.op_kind == PlannerOpKind::Trade,
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
            GoalKind::TreatWounds { patient } => state
                .pain_summary(*patient)
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

    fn goal_relevant_places(
        &self,
        state: &PlanningState<'_>,
        recipes: &RecipeRegistry,
    ) -> Vec<EntityId> {
        let actor = state.snapshot().actor();
        match self {
            GoalKind::ConsumeOwnedCommodity { commodity } => {
                if state.commodity_quantity(actor, *commodity) > Quantity(0) {
                    state.effective_place(actor).into_iter().collect()
                } else {
                    places_with_resource_source(state, *commodity)
                }
            }
            GoalKind::AcquireCommodity { commodity, .. } => {
                let mut places = places_with_resource_source(state, *commodity);
                places_with_sellers(state, *commodity, &mut places);
                places
            }
            GoalKind::Relieve => places_with_place_tag(state, PlaceTag::Latrine),
            GoalKind::EngageHostile { target }
            | GoalKind::TreatWounds { patient: target } => {
                state.effective_place(*target).into_iter().collect()
            }
            GoalKind::Sleep | GoalKind::Wash | GoalKind::ReduceDanger => Vec::new(),
            GoalKind::ProduceCommodity { recipe_id } => {
                let required_tag = recipes
                    .get(*recipe_id)
                    .and_then(|recipe| recipe.required_workstation_tag);
                match required_tag {
                    Some(tag) => places_with_workstation(state, tag),
                    None => Vec::new(),
                }
            }
            GoalKind::SellCommodity { commodity } => {
                demand_memory_places(state, actor, *commodity)
            }
            GoalKind::RestockCommodity { commodity } => {
                if state.commodity_quantity(actor, *commodity) > Quantity(0) {
                    demand_memory_places(state, actor, *commodity)
                } else {
                    places_with_resource_source(state, *commodity)
                }
            }
            GoalKind::MoveCargo { destination, .. } => vec![*destination],
            GoalKind::LootCorpse { corpse } | GoalKind::BuryCorpse { corpse, .. } => {
                state.effective_place(*corpse).into_iter().collect()
            }
            GoalKind::ShareBelief { listener, .. } => {
                state.effective_place(*listener).into_iter().collect()
            }
            GoalKind::ClaimOffice { .. } | GoalKind::SupportCandidateForOffice { .. } => {
                Vec::new()
            }
        }
    }

    fn matches_binding(
        &self,
        authoritative_targets: &[EntityId],
        op_kind: PlannerOpKind,
    ) -> bool {
        // Planner-only synthetic candidates have empty targets — always pass.
        if authoritative_targets.is_empty() {
            return true;
        }

        // Auxiliary ops serve the goal indirectly — always pass.
        match op_kind {
            PlannerOpKind::Travel
            | PlannerOpKind::Trade
            | PlannerOpKind::Harvest
            | PlannerOpKind::Craft
            | PlannerOpKind::QueueForFacilityUse
            | PlannerOpKind::MoveCargo
            | PlannerOpKind::Consume
            | PlannerOpKind::Sleep
            | PlannerOpKind::Relieve
            | PlannerOpKind::Wash
            | PlannerOpKind::Defend
            | PlannerOpKind::Bribe
            | PlannerOpKind::Threaten => return true,
            // Terminal ops — fall through to goal-specific binding check.
            PlannerOpKind::Attack
            | PlannerOpKind::Loot
            | PlannerOpKind::Heal
            | PlannerOpKind::Tell
            | PlannerOpKind::DeclareSupport
            | PlannerOpKind::Bury => {}
        }

        // Terminal ops on flexible goals — always pass.
        // Terminal ops on exact-bound goals — verify target identity.
        match self {
            // Flexible goals and DeclareSupport edge case: no binding requirement.
            // ClaimOffice/SupportCandidateForOffice have empty bound_targets in
            // practice (handled by the empty-targets bypass above). If non-empty,
            // payload override handles correctness.
            GoalKind::ConsumeOwnedCommodity { .. }
            | GoalKind::AcquireCommodity { .. }
            | GoalKind::Sleep
            | GoalKind::Relieve
            | GoalKind::Wash
            | GoalKind::ReduceDanger
            | GoalKind::ProduceCommodity { .. }
            | GoalKind::SellCommodity { .. }
            | GoalKind::RestockCommodity { .. }
            | GoalKind::ClaimOffice { .. }
            | GoalKind::SupportCandidateForOffice { .. } => true,

            // Exact-bound goals: target must match.
            GoalKind::EngageHostile { target }
            | GoalKind::TreatWounds { patient: target } => {
                authoritative_targets.contains(target)
            }
            GoalKind::LootCorpse { corpse } => authoritative_targets.contains(corpse),
            GoalKind::BuryCorpse {
                corpse,
                burial_site,
            } => {
                authoritative_targets.contains(corpse)
                    || authoritative_targets.contains(burial_site)
            }
            GoalKind::ShareBelief { listener, .. } => authoritative_targets.contains(listener),
            GoalKind::MoveCargo { destination, .. } => {
                authoritative_targets.contains(destination)
            }
        }
    }
}

/// Collect places containing entities with a `ResourceSource` for the given commodity.
fn places_with_resource_source(state: &PlanningState<'_>, commodity: CommodityKind) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state
            .resource_source(entity_id)
            .is_some_and(|s| s.commodity == commodity)
        {
            if let Some(place) = state.effective_place(entity_id) {
                places.insert(place);
            }
        }
    }
    places.into_iter().collect()
}

/// Append places where merchants are selling the given commodity (deduplicating with `existing`).
fn places_with_sellers(
    state: &PlanningState<'_>,
    commodity: CommodityKind,
    existing: &mut Vec<EntityId>,
) {
    let already: BTreeSet<EntityId> = existing.iter().copied().collect();
    for &entity_id in state.snapshot().entities.keys() {
        if let Some(profile) = state.merchandise_profile(entity_id) {
            if profile.sale_kinds.contains(&commodity) {
                if let Some(place) = state.effective_place(entity_id) {
                    if !already.contains(&place) && !existing.contains(&place) {
                        existing.push(place);
                    }
                }
            }
        }
    }
}

/// Collect places with the given `PlaceTag`.
fn places_with_place_tag(state: &PlanningState<'_>, tag: PlaceTag) -> Vec<EntityId> {
    state
        .snapshot()
        .places
        .iter()
        .filter(|(_, place)| place.tags.contains(&tag))
        .map(|(id, _)| *id)
        .collect()
}

/// Collect places containing entities with the given `WorkstationTag`.
fn places_with_workstation(state: &PlanningState<'_>, tag: WorkstationTag) -> Vec<EntityId> {
    let mut places = BTreeSet::new();
    for &entity_id in state.snapshot().entities.keys() {
        if state.workstation_tag(entity_id) == Some(tag) {
            if let Some(place) = state.effective_place(entity_id) {
                places.insert(place);
            }
        }
    }
    places.into_iter().collect()
}

/// Collect places from the actor's demand memory for the given commodity,
/// filtered to places present in the planning snapshot.
fn demand_memory_places(
    state: &PlanningState<'_>,
    actor: EntityId,
    commodity: CommodityKind,
) -> Vec<EntityId> {
    let snapshot_places = &state.snapshot().places;
    let places: BTreeSet<EntityId> = state
        .demand_memory(actor)
        .into_iter()
        .filter(|obs| obs.commodity == commodity)
        .map(|obs| obs.place)
        .filter(|place| snapshot_places.contains_key(place))
        .collect();
    places.into_iter().collect()
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

/// Hypothetical bribe outcome: actor pays commodity, target declares support.
fn apply_bribe_for_office<'s>(
    state: PlanningState<'s>,
    actor: EntityId,
    office: EntityId,
    payload_override: Option<&ActionPayload>,
) -> PlanningState<'s> {
    let Some(bribe) = payload_override.and_then(ActionPayload::as_bribe) else {
        return state;
    };
    let current_qty = state.commodity_quantity(actor, bribe.offered_commodity);
    if current_qty >= bribe.offered_quantity {
        let remaining = Quantity(current_qty.0.saturating_sub(bribe.offered_quantity.0));
        state
            .with_commodity_quantity(actor, bribe.offered_commodity, remaining)
            .with_support_declaration(bribe.target, office, actor)
    } else {
        state
    }
}

fn apply_threaten_for_office<'s>(
    state: PlanningState<'s>,
    actor: EntityId,
    office: EntityId,
    payload_override: Option<&ActionPayload>,
) -> PlanningState<'s> {
    let Some(threaten) = payload_override.and_then(ActionPayload::as_threaten) else {
        return state;
    };
    let attack_skill = state
        .combat_profile(actor)
        .map_or(Permille::new_unchecked(0), |p| p.attack_skill);
    let target_courage = state
        .courage(threaten.target)
        .unwrap_or(Permille::new_unchecked(1000));
    if attack_skill > target_courage {
        state.with_support_declaration(threaten.target, office, actor)
    } else {
        state
    }
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
            purpose: CommodityPurpose::SelfConsume,
        };
        let key = GoalKey::from(kind);

        assert_eq!(key.kind, kind);
        assert_eq!(key.commodity, Some(CommodityKind::Water));
    }

    #[test]
    fn grounded_goal_roundtrips_through_bincode() {
        let goal = GroundedGoal {
            key: GoalKey::from(GoalKind::TreatWounds {
                patient: entity_id(7, 1),
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
                key: GoalKey::from(GoalKind::TreatWounds {
                    patient: entity_id(7, 1),
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
                purpose: CommodityPurpose::SelfConsume,
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
    fn consume_goal_relevant_ops_include_consumption_and_pickup_only() {
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Consume));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::Travel));
        assert!(goal.relevant_op_kinds().contains(&PlannerOpKind::MoveCargo));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Harvest));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Craft));
        assert!(!goal.relevant_op_kinds().contains(&PlannerOpKind::Trade));
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
        resource_sources: BTreeMap<EntityId, ResourceSource>,
        workstation_tags: BTreeMap<EntityId, WorkstationTag>,
        place_tags: BTreeMap<EntityId, BTreeSet<worldwake_core::PlaceTag>>,
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

        fn workstation_tag(&self, entity: EntityId) -> Option<WorkstationTag> {
            self.workstation_tags.get(&entity).copied()
        }

        fn resource_source(&self, entity: EntityId) -> Option<ResourceSource> {
            self.resource_sources.get(&entity).cloned()
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

        fn place_has_tag(&self, place: EntityId, tag: worldwake_core::PlaceTag) -> bool {
            self.place_tags
                .get(&place)
                .is_some_and(|tags| tags.contains(&tag))
        }

        fn resource_sources_at(
            &self,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Vec<EntityId> {
            self.entities_at(place)
                .into_iter()
                .filter(|entity| {
                    self.resource_sources
                        .get(entity)
                        .is_some_and(|s| s.commodity == commodity)
                })
                .collect()
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

    // ── goal_relevant_places tests ─────────────────────────────────────

    fn spatial_view() -> (TestBeliefView, EntityId, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let place_a = entity(10);
        let place_b = entity(11);
        let place_c = entity(12);

        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place_a, place_b, place_c]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place_a, EntityKind::Place);
        view.kinds.insert(place_b, EntityKind::Place);
        view.kinds.insert(place_c, EntityKind::Place);
        view.effective_places.insert(actor, place_a);
        view.entities_at
            .insert(place_a, vec![actor]);
        view.adjacent.insert(
            place_a,
            vec![
                (place_b, NonZeroU32::new(2).unwrap()),
            ],
        );
        view.adjacent.insert(
            place_b,
            vec![
                (place_a, NonZeroU32::new(2).unwrap()),
                (place_c, NonZeroU32::new(3).unwrap()),
            ],
        );
        view.adjacent.insert(
            place_c,
            vec![(place_b, NonZeroU32::new(3).unwrap())],
        );
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(500), pm(0), pm(500), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        (view, actor, place_a, place_b, place_c)
    }

    fn snapshot_and_state(
        view: &TestBeliefView,
        actor: EntityId,
    ) -> crate::planning_snapshot::PlanningSnapshot {
        build_planning_snapshot(view, actor, &BTreeSet::new(), &BTreeSet::new(), 3)
    }

    #[test]
    fn move_cargo_returns_destination_place() {
        let (view, actor, _place_a, place_b, _place_c) = spatial_view();
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::MoveCargo {
            commodity: CommodityKind::Bread,
            destination: place_b,
        };
        assert_eq!(goal.goal_relevant_places(&state, &recipes), vec![place_b]);
    }

    #[test]
    fn restock_without_commodity_returns_resource_source_places() {
        let (mut view, actor, _place_a, place_b, _place_c) = spatial_view();
        let resource_entity = entity(20);
        view.alive.insert(resource_entity);
        view.kinds.insert(resource_entity, EntityKind::ItemLot);
        view.effective_places.insert(resource_entity, place_b);
        view.entities_at
            .entry(place_b)
            .or_default()
            .push(resource_entity);
        view.resource_sources.insert(
            resource_entity,
            ResourceSource {
                commodity: CommodityKind::Apple,
                available_quantity: Quantity(5),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Apple,
        };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_b]);
    }

    #[test]
    fn restock_with_commodity_returns_demand_memory_places() {
        let (mut view, actor, _place_a, place_b, _place_c) = spatial_view();
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(3));
        view.demand_memory.insert(
            actor,
            vec![DemandObservation {
                commodity: CommodityKind::Bread,
                quantity: Quantity(5),
                place: place_b,
                tick: Tick(1),
                counterparty: None,
                reason: DemandObservationReason::WantedToBuyButNoSeller,
            }],
        );
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::RestockCommodity {
            commodity: CommodityKind::Bread,
        };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_b]);
    }

    #[test]
    fn consume_owned_with_possession_returns_actor_place() {
        let (mut view, actor, place_a, _place_b, _place_c) = spatial_view();
        view.commodity_quantities
            .insert((actor, CommodityKind::Bread), Quantity(1));
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_a]);
    }

    #[test]
    fn acquire_returns_resource_source_and_merchant_places() {
        let (mut view, actor, _place_a, place_b, place_c) = spatial_view();
        // Resource source at place_b
        let resource_entity = entity(20);
        view.alive.insert(resource_entity);
        view.kinds.insert(resource_entity, EntityKind::ItemLot);
        view.effective_places.insert(resource_entity, place_b);
        view.entities_at
            .entry(place_b)
            .or_default()
            .push(resource_entity);
        view.resource_sources.insert(
            resource_entity,
            ResourceSource {
                commodity: CommodityKind::Bread,
                available_quantity: Quantity(3),
                max_quantity: Quantity(10),
                regeneration_ticks_per_unit: None,
                last_regeneration_tick: None,
            },
        );
        // Merchant at place_c
        let merchant = entity(30);
        view.alive.insert(merchant);
        view.kinds.insert(merchant, EntityKind::Agent);
        view.effective_places.insert(merchant, place_c);
        view.entities_at
            .entry(place_c)
            .or_default()
            .push(merchant);
        view.merchandise_profiles.insert(
            merchant,
            MerchandiseProfile {
                sale_kinds: BTreeSet::from([CommodityKind::Bread]),
                home_market: Some(place_c),
            },
        );
        view.trade_profiles
            .insert(merchant, sample_trade_disposition_profile());
        view.commodity_quantities
            .insert((merchant, CommodityKind::Bread), Quantity(2));
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert!(places.contains(&place_b), "should contain resource source place");
        assert!(places.contains(&place_c), "should contain merchant place");
    }

    #[test]
    fn engage_hostile_returns_target_place() {
        let (mut view, actor, _place_a, place_b, _place_c) = spatial_view();
        let target = entity(25);
        view.alive.insert(target);
        view.kinds.insert(target, EntityKind::Agent);
        view.effective_places.insert(target, place_b);
        view.entities_at
            .entry(place_b)
            .or_default()
            .push(target);
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::EngageHostile { target };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_b]);
    }

    #[test]
    fn loot_corpse_returns_corpse_place() {
        let (mut view, actor, _place_a, _place_b, place_c) = spatial_view();
        let corpse = entity(26);
        view.alive.remove(&corpse); // corpse is dead
        view.kinds.insert(corpse, EntityKind::Agent);
        view.effective_places.insert(corpse, place_c);
        view.entities_at
            .entry(place_c)
            .or_default()
            .push(corpse);
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::LootCorpse { corpse };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_c]);
    }

    #[test]
    fn reduce_danger_returns_empty() {
        let (view, actor, _place_a, _place_b, _place_c) = spatial_view();
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let places = GoalKind::ReduceDanger.goal_relevant_places(&state, &recipes);
        assert!(places.is_empty());
    }

    #[test]
    fn sleep_returns_empty() {
        let (view, actor, _place_a, _place_b, _place_c) = spatial_view();
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let places = GoalKind::Sleep.goal_relevant_places(&state, &recipes);
        assert!(places.is_empty());
    }

    #[test]
    fn wash_returns_empty() {
        let (view, actor, _place_a, _place_b, _place_c) = spatial_view();
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let places = GoalKind::Wash.goal_relevant_places(&state, &recipes);
        assert!(places.is_empty());
    }

    #[test]
    fn relieve_returns_latrine_places() {
        let (mut view, actor, _place_a, place_b, _place_c) = spatial_view();
        view.place_tags.insert(
            place_b,
            BTreeSet::from([worldwake_core::PlaceTag::Latrine]),
        );
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let places = GoalKind::Relieve.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_b]);
    }

    #[test]
    fn produce_returns_places_with_specific_workstation() {
        let (mut view, actor, _place_a, place_b, _place_c) = spatial_view();
        let forge = entity(40);
        view.alive.insert(forge);
        view.kinds.insert(forge, EntityKind::UniqueItem);
        view.effective_places.insert(forge, place_b);
        view.entities_at
            .entry(place_b)
            .or_default()
            .push(forge);
        view.workstation_tags
            .insert(forge, WorkstationTag::Forge);
        let mut recipes = worldwake_sim::RecipeRegistry::new();
        let recipe_id = recipes.register(worldwake_sim::RecipeDefinition {
            name: "Smelt Iron".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(5).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Forge),
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1)),
        });
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ProduceCommodity { recipe_id };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert_eq!(places, vec![place_b]);
    }

    #[test]
    fn produce_without_workstation_requirement_returns_empty() {
        let (view, actor, _place_a, _place_b, _place_c) = spatial_view();
        let mut recipes = worldwake_sim::RecipeRegistry::new();
        let recipe_id = recipes.register(worldwake_sim::RecipeDefinition {
            name: "HandCraft".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(1))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: None,
            required_tool_kinds: Vec::new(),
            body_cost_per_tick: BodyCostPerTick::new(pm(1), pm(1), pm(1), pm(1)),
        });
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);
        let goal = GoalKind::ProduceCommodity { recipe_id };
        let places = goal.goal_relevant_places(&state, &recipes);
        assert!(places.is_empty());
    }

    #[test]
    fn all_goal_kind_variants_have_goal_relevant_places_impl() {
        // This test ensures exhaustive coverage by creating all 17 variants
        // and calling goal_relevant_places. If a new variant is added without
        // an arm in the match, this will fail to compile.
        let (view, actor, _place_a, place_b, _place_c) = spatial_view();
        let recipes = worldwake_sim::RecipeRegistry::new();
        let snapshot = snapshot_and_state(&view, actor);
        let state = PlanningState::new(&snapshot);

        let goals: Vec<GoalKind> = vec![
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
            GoalKind::EngageHostile {
                target: entity(99),
            },
            GoalKind::ReduceDanger,
            GoalKind::TreatWounds {
                patient: entity(99),
            },
            GoalKind::ProduceCommodity {
                recipe_id: RecipeId(0),
            },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: place_b,
            },
            GoalKind::LootCorpse {
                corpse: entity(99),
            },
            GoalKind::BuryCorpse {
                corpse: entity(99),
                burial_site: entity(98),
            },
            GoalKind::ShareBelief {
                listener: entity(99),
                subject: entity(98),
            },
            GoalKind::ClaimOffice {
                office: entity(99),
            },
            GoalKind::SupportCandidateForOffice {
                office: entity(99),
                candidate: entity(98),
            },
        ];

        // All 17 variants must be callable without panicking.
        assert_eq!(goals.len(), 17);
        for goal in &goals {
            let _ = goal.goal_relevant_places(&state, &recipes);
        }
    }

    // ── matches_binding tests ──────────────────────────────────────────

    mod matches_binding_tests {
        use super::*;

        fn id(slot: u32) -> EntityId {
            entity_id(slot, 1)
        }

        // ── LootCorpse ────────────────────────────────────────────────

        #[test]
        fn loot_corpse_match() {
            let corpse = id(1);
            let goal = GoalKind::LootCorpse { corpse };
            assert!(goal.matches_binding(&[corpse], PlannerOpKind::Loot));
        }

        #[test]
        fn loot_corpse_mismatch() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(!goal.matches_binding(&[id(2)], PlannerOpKind::Loot));
        }

        #[test]
        fn auxiliary_bypass() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Travel));
        }

        #[test]
        fn empty_targets_bypass() {
            let goal = GoalKind::LootCorpse { corpse: id(1) };
            assert!(goal.matches_binding(&[], PlannerOpKind::Loot));
        }

        // ── Flexible goals ────────────────────────────────────────────

        #[test]
        fn flexible_goal_sleep() {
            let goal = GoalKind::Sleep;
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Attack));
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::Loot));
            assert!(goal.matches_binding(&[], PlannerOpKind::Sleep));
        }

        #[test]
        fn flexible_goal_consume_owned() {
            let goal = GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Water,
            };
            assert!(goal.matches_binding(&[id(5)], PlannerOpKind::Loot));
        }

        #[test]
        fn flexible_goal_reduce_danger() {
            let goal = GoalKind::ReduceDanger;
            assert!(goal.matches_binding(&[id(5)], PlannerOpKind::Attack));
        }

        // ── EngageHostile ─────────────────────────────────────────────

        #[test]
        fn engage_hostile_match() {
            let target = id(10);
            let goal = GoalKind::EngageHostile { target };
            assert!(goal.matches_binding(&[target], PlannerOpKind::Attack));
        }

        #[test]
        fn engage_hostile_mismatch() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            assert!(!goal.matches_binding(&[id(11)], PlannerOpKind::Attack));
        }

        // ── TreatWounds ──────────────────────────────────────────────

        #[test]
        fn treat_wounds_match() {
            let patient = id(20);
            let goal = GoalKind::TreatWounds { patient };
            assert!(goal.matches_binding(&[patient], PlannerOpKind::Heal));
        }

        #[test]
        fn treat_wounds_mismatch() {
            let goal = GoalKind::TreatWounds { patient: id(20) };
            assert!(!goal.matches_binding(&[id(21)], PlannerOpKind::Heal));
        }

        // ── ShareBelief ───────────────────────────────────────────────

        #[test]
        fn share_belief_match() {
            let listener = id(30);
            let goal = GoalKind::ShareBelief {
                listener,
                subject: id(99),
            };
            assert!(goal.matches_binding(&[listener], PlannerOpKind::Tell));
        }

        #[test]
        fn share_belief_mismatch() {
            let goal = GoalKind::ShareBelief {
                listener: id(30),
                subject: id(99),
            };
            assert!(!goal.matches_binding(&[id(31)], PlannerOpKind::Tell));
        }

        // ── MoveCargo ─────────────────────────────────────────────────

        #[test]
        fn move_cargo_destination_match() {
            let dest = id(40);
            let goal = GoalKind::MoveCargo {
                commodity: CommodityKind::Water,
                destination: dest,
            };
            assert!(goal.matches_binding(&[dest], PlannerOpKind::Loot));
        }

        #[test]
        fn move_cargo_destination_mismatch() {
            let goal = GoalKind::MoveCargo {
                commodity: CommodityKind::Water,
                destination: id(40),
            };
            assert!(!goal.matches_binding(&[id(41)], PlannerOpKind::Loot));
        }

        // ── BuryCorpse ───────────────────────────────────────────────

        #[test]
        fn bury_corpse_matches_corpse() {
            let corpse = id(50);
            let goal = GoalKind::BuryCorpse {
                corpse,
                burial_site: id(51),
            };
            assert!(goal.matches_binding(&[corpse], PlannerOpKind::Bury));
        }

        #[test]
        fn bury_corpse_matches_burial_site() {
            let burial_site = id(51);
            let goal = GoalKind::BuryCorpse {
                corpse: id(50),
                burial_site,
            };
            assert!(goal.matches_binding(&[burial_site], PlannerOpKind::Bury));
        }

        #[test]
        fn bury_corpse_mismatch() {
            let goal = GoalKind::BuryCorpse {
                corpse: id(50),
                burial_site: id(51),
            };
            assert!(!goal.matches_binding(&[id(52)], PlannerOpKind::Bury));
        }

        // ── DeclareSupport (always passes) ────────────────────────────

        #[test]
        fn claim_office_declare_support_passes() {
            let goal = GoalKind::ClaimOffice { office: id(60) };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::DeclareSupport));
        }

        #[test]
        fn support_candidate_declare_support_passes() {
            let goal = GoalKind::SupportCandidateForOffice {
                office: id(60),
                candidate: id(61),
            };
            assert!(goal.matches_binding(&[id(99)], PlannerOpKind::DeclareSupport));
        }

        // ── All auxiliary ops bypass on exact-bound goal ──────────────

        #[test]
        fn all_auxiliary_ops_bypass() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            let unrelated = &[id(99)];
            let auxiliary_ops = [
                PlannerOpKind::Travel,
                PlannerOpKind::Trade,
                PlannerOpKind::Harvest,
                PlannerOpKind::Craft,
                PlannerOpKind::QueueForFacilityUse,
                PlannerOpKind::MoveCargo,
                PlannerOpKind::Consume,
                PlannerOpKind::Sleep,
                PlannerOpKind::Relieve,
                PlannerOpKind::Wash,
                PlannerOpKind::Defend,
                PlannerOpKind::Bribe,
                PlannerOpKind::Threaten,
            ];
            for op in auxiliary_ops {
                assert!(
                    goal.matches_binding(unrelated, op),
                    "auxiliary op {op:?} should bypass binding"
                );
            }
        }

        // ── Empty targets bypass on all terminal ops ──────────────────

        #[test]
        fn empty_targets_bypass_all_terminal_ops() {
            let goal = GoalKind::EngageHostile { target: id(10) };
            let terminal_ops = [
                PlannerOpKind::Attack,
                PlannerOpKind::Loot,
                PlannerOpKind::Heal,
                PlannerOpKind::Tell,
                PlannerOpKind::DeclareSupport,
                PlannerOpKind::Bury,
            ];
            for op in terminal_ops {
                assert!(
                    goal.matches_binding(&[], op),
                    "empty targets should bypass terminal op {op:?}"
                );
            }
        }
    }
}
