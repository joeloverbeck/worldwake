use crate::{
    GoalKey, GoalKind, GoalKindPlannerExt, GroundedGoal, HypotheticalEntityId, PlanningEntityRef,
    PlanningState,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use worldwake_core::{load_per_unit, ActionDefId, ActionDomain, EntityId, EntityKind, Quantity};
use worldwake_sim::{ActionDef, ActionDefRegistry, ActionPayload, MaterializationTag};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlannerOpKind {
    Travel,
    Patrol,
    Consume,
    Sleep,
    Relieve,
    Wash,
    EstablishCamp,
    Trade,
    QueueForFacilityUse,
    Harvest,
    Craft,
    MoveCargo,
    Heal,
    Loot,
    Bury,
    Tell,
    ConsultRecord,
    Attack,
    Defend,
    Bribe,
    Threaten,
    Accuse,
    Fine,
    Exile,
    DeclareSupport,
    PressForceClaim,
    YieldForceClaim,
    Investigate,
    AskWitness,
    StaffMarket,
    StockManagement,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PlannerOpSemantics {
    pub op_kind: PlannerOpKind,
    pub may_appear_mid_plan: bool,
    pub is_materialization_barrier: bool,
    pub transition_kind: PlannerTransitionKind,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PlannerTransitionKind {
    GoalModelFallback,
    ConsumeMatchingTargetCommodity,
    PickUpGroundLot,
    StealGroundLot,
    PutDownGroundLot,
    StoreStockIntoLocalFacility,
}

#[must_use]
pub fn build_semantics_table(
    registry: &ActionDefRegistry,
) -> BTreeMap<ActionDefId, PlannerOpSemantics> {
    registry
        .iter()
        .filter_map(|def| {
            classify_action_def(def).map(|op_kind| (def.id, semantics_for(def, op_kind)))
        })
        .collect()
}

fn classify_action_def(def: &ActionDef) -> Option<PlannerOpKind> {
    match (def.domain, def.name.as_str()) {
        (ActionDomain::Travel, "travel") => Some(PlannerOpKind::Travel),
        (ActionDomain::Generic, "patrol") => Some(PlannerOpKind::Patrol),
        (ActionDomain::Needs, "eat" | "drink") => Some(PlannerOpKind::Consume),
        (ActionDomain::Needs, "sleep") => Some(PlannerOpKind::Sleep),
        (ActionDomain::Needs, "toilet" | "relieve_wilderness") => Some(PlannerOpKind::Relieve),
        (ActionDomain::Needs, "wash") => Some(PlannerOpKind::Wash),
        (ActionDomain::Generic, "establish_camp") => Some(PlannerOpKind::EstablishCamp),
        (ActionDomain::Trade, "trade") => Some(PlannerOpKind::Trade),
        (ActionDomain::Trade, "staff_market") => Some(PlannerOpKind::StaffMarket),
        (ActionDomain::Production, "queue_for_facility_use") => {
            Some(PlannerOpKind::QueueForFacilityUse)
        }
        (ActionDomain::Production, name)
            if name.starts_with("harvest:") && matches!(def.payload, ActionPayload::Harvest(_)) =>
        {
            Some(PlannerOpKind::Harvest)
        }
        (ActionDomain::Production, name)
            if name.starts_with("craft:") && matches!(def.payload, ActionPayload::Craft(_)) =>
        {
            Some(PlannerOpKind::Craft)
        }
        (ActionDomain::Transport, "pick_up" | "put_down" | "steal") => {
            Some(PlannerOpKind::MoveCargo)
        }
        (
            ActionDomain::Transport,
            "store_stock" | "collect_display_stock",
        )
        | (
            ActionDomain::Trade,
            "stage_stock_for_sale" | "unstage_stock",
        ) => Some(PlannerOpKind::StockManagement),
        (ActionDomain::Care, "heal") => Some(PlannerOpKind::Heal),
        (ActionDomain::Corpse, "loot") => Some(PlannerOpKind::Loot),
        (ActionDomain::Corpse, "bury") => Some(PlannerOpKind::Bury),
        (ActionDomain::Social, "tell") => Some(PlannerOpKind::Tell),
        (ActionDomain::Social, "consult_record") => Some(PlannerOpKind::ConsultRecord),
        (ActionDomain::Social, "bribe") => Some(PlannerOpKind::Bribe),
        (ActionDomain::Social, "threaten") => Some(PlannerOpKind::Threaten),
        (ActionDomain::Social, "accuse") => Some(PlannerOpKind::Accuse),
        (ActionDomain::Social, "fine") => Some(PlannerOpKind::Fine),
        (ActionDomain::Social, "exile") => Some(PlannerOpKind::Exile),
        (ActionDomain::Social, "declare_support") => Some(PlannerOpKind::DeclareSupport),
        (ActionDomain::Social, "press_force_claim") => Some(PlannerOpKind::PressForceClaim),
        (ActionDomain::Social, "yield_force_claim") => Some(PlannerOpKind::YieldForceClaim),
        (ActionDomain::Combat, "attack") => Some(PlannerOpKind::Attack),
        (ActionDomain::Combat, "defend") => Some(PlannerOpKind::Defend),
        (ActionDomain::Generic, "investigate") => Some(PlannerOpKind::Investigate),
        (ActionDomain::Epistemic, "ask_witness") => Some(PlannerOpKind::AskWitness),
        _ => None,
    }
}

const fn base_semantics(
    op_kind: PlannerOpKind,
    may_appear_mid_plan: bool,
    is_materialization_barrier: bool,
    transition_kind: PlannerTransitionKind,
) -> PlannerOpSemantics {
    PlannerOpSemantics {
        op_kind,
        may_appear_mid_plan,
        is_materialization_barrier,
        transition_kind,
    }
}

fn semantics_for(def: &ActionDef, op_kind: PlannerOpKind) -> PlannerOpSemantics {
    if let Some(semantics) = social_or_combat_semantics(op_kind) {
        return semantics;
    }

    match op_kind {
        PlannerOpKind::Travel
        | PlannerOpKind::Sleep
        | PlannerOpKind::Relieve
        | PlannerOpKind::Wash
        | PlannerOpKind::EstablishCamp
        | PlannerOpKind::QueueForFacilityUse
        | PlannerOpKind::Heal
        | PlannerOpKind::StaffMarket
        | PlannerOpKind::StockManagement => base_semantics(
            op_kind,
            true,
            false,
            match def.name.as_str() {
                "store_stock" => PlannerTransitionKind::StoreStockIntoLocalFacility,
                _ => PlannerTransitionKind::GoalModelFallback,
            },
        ),
        PlannerOpKind::Patrol => base_semantics(
            op_kind,
            false,
            false,
            PlannerTransitionKind::GoalModelFallback,
        ),
        PlannerOpKind::Consume => base_semantics(
            op_kind,
            true,
            false,
            PlannerTransitionKind::ConsumeMatchingTargetCommodity,
        ),
        PlannerOpKind::Trade
        | PlannerOpKind::Harvest
        | PlannerOpKind::Craft
        | PlannerOpKind::Loot => base_semantics(
            op_kind,
            true,
            true,
            PlannerTransitionKind::GoalModelFallback,
        ),
        PlannerOpKind::MoveCargo => base_semantics(
            op_kind,
            true,
            false,
            match def.name.as_str() {
                "pick_up" => PlannerTransitionKind::PickUpGroundLot,
                "steal" => PlannerTransitionKind::StealGroundLot,
                "put_down" => PlannerTransitionKind::PutDownGroundLot,
                _ => PlannerTransitionKind::GoalModelFallback,
            },
        ),
        PlannerOpKind::Bury => base_semantics(
            op_kind,
            false,
            true,
            PlannerTransitionKind::GoalModelFallback,
        ),
        PlannerOpKind::Tell
        | PlannerOpKind::ConsultRecord
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend
        | PlannerOpKind::Bribe
        | PlannerOpKind::Threaten
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::Investigate
        | PlannerOpKind::AskWitness => unreachable!("handled by social_or_combat_semantics"),
    }
}

fn social_or_combat_semantics(op_kind: PlannerOpKind) -> Option<PlannerOpSemantics> {
    Some(match op_kind {
        PlannerOpKind::ConsultRecord | PlannerOpKind::Bribe | PlannerOpKind::Threaten => {
            base_semantics(
                op_kind,
                true,
                false,
                PlannerTransitionKind::GoalModelFallback,
            )
        }
        PlannerOpKind::Tell
        | PlannerOpKind::Attack
        | PlannerOpKind::Defend
        | PlannerOpKind::Accuse
        | PlannerOpKind::Fine
        | PlannerOpKind::Exile
        | PlannerOpKind::DeclareSupport
        | PlannerOpKind::PressForceClaim
        | PlannerOpKind::YieldForceClaim
        | PlannerOpKind::AskWitness
        | PlannerOpKind::Investigate => base_semantics(
            op_kind,
            false,
            false,
            PlannerTransitionKind::GoalModelFallback,
        ),
        _ => return None,
    })
}

#[must_use]
pub struct HypotheticalTransition<'snapshot> {
    pub targets: Vec<PlanningEntityRef>,
    pub state: PlanningState<'snapshot>,
    pub expected_materializations: Vec<ExpectedMaterialization>,
}

pub fn apply_hypothetical_transition<'snapshot>(
    goal: &GroundedGoal,
    semantics: PlannerOpSemantics,
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
    payload_override: Option<&ActionPayload>,
) -> Option<HypotheticalTransition<'snapshot>> {
    match semantics.transition_kind {
        PlannerTransitionKind::GoalModelFallback => Some(apply_goal_model_fallback_transition(
            goal,
            semantics,
            state,
            targets,
            payload_override,
        )),
        PlannerTransitionKind::ConsumeMatchingTargetCommodity => {
            apply_consume_matching_target_transition(goal, semantics, state, targets)
        }
        PlannerTransitionKind::PickUpGroundLot => {
            let state =
                apply_goal_model_fallback_state(goal, semantics, state, targets, payload_override);
            apply_pick_up_transition(state, targets, payload_override)
        }
        PlannerTransitionKind::StealGroundLot => {
            let state =
                apply_goal_model_fallback_state(goal, semantics, state, targets, payload_override);
            apply_steal_transition(state, targets)
        }
        PlannerTransitionKind::PutDownGroundLot => {
            let state =
                apply_goal_model_fallback_state(goal, semantics, state, targets, payload_override);
            apply_put_down_transition(state, targets)
        }
        PlannerTransitionKind::StoreStockIntoLocalFacility => {
            let state =
                apply_goal_model_fallback_state(goal, semantics, state, targets, payload_override);
            apply_store_stock_transition(state, targets)
        }
    }
}

fn apply_goal_model_fallback_state<'snapshot>(
    goal: &GroundedGoal,
    semantics: PlannerOpSemantics,
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
    payload_override: Option<&ActionPayload>,
) -> PlanningState<'snapshot> {
    let authoritative_targets = authoritative_targets(targets).unwrap_or_default();
    goal.key.kind.apply_planner_step(
        state,
        semantics.op_kind,
        &authoritative_targets,
        payload_override,
    )
}

fn apply_goal_model_fallback_transition<'snapshot>(
    goal: &GroundedGoal,
    semantics: PlannerOpSemantics,
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
    payload_override: Option<&ActionPayload>,
) -> HypotheticalTransition<'snapshot> {
    HypotheticalTransition {
        targets: targets.to_vec(),
        state: apply_goal_model_fallback_state(goal, semantics, state, targets, payload_override),
        expected_materializations: Vec::new(),
    }
}

fn apply_consume_matching_target_transition<'snapshot>(
    goal: &GroundedGoal,
    semantics: PlannerOpSemantics,
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
) -> Option<HypotheticalTransition<'snapshot>> {
    if !consume_transition_matches_goal(&goal.key.kind, &state, targets) {
        return None;
    }

    Some(apply_goal_model_fallback_transition(
        goal, semantics, state, targets, None,
    ))
}

fn consume_transition_matches_goal(
    goal_kind: &GoalKind,
    state: &PlanningState<'_>,
    targets: &[PlanningEntityRef],
) -> bool {
    match goal_kind {
        GoalKind::ConsumeOwnedCommodity { commodity } => targets
            .first()
            .copied()
            .and_then(|target| state.item_lot_commodity_ref(target))
            .is_some_and(|target_commodity| target_commodity == *commodity),
        _ => true,
    }
}

fn apply_pick_up_transition<'snapshot>(
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
    payload_override: Option<&ActionPayload>,
) -> Option<HypotheticalTransition<'snapshot>> {
    let actor_ref = PlanningEntityRef::Authoritative(state.snapshot().actor());
    let lot_ref = match targets.first().copied()? {
        PlanningEntityRef::Authoritative(lot) => PlanningEntityRef::Authoritative(lot),
        PlanningEntityRef::Hypothetical(_) => return None,
    };
    if state.entity_kind_ref(lot_ref) != Some(EntityKind::ItemLot) {
        return None;
    }
    if state.direct_possessor_ref(lot_ref).is_some()
        || state.direct_container_ref(lot_ref).is_some()
    {
        return None;
    }
    if state.effective_place_ref(lot_ref)? != state.effective_place_ref(actor_ref)? {
        return None;
    }
    let commodity = state.item_lot_commodity_ref(lot_ref)?;
    let quantity = state.commodity_quantity_ref(lot_ref, commodity);
    if quantity == Quantity(0) {
        return None;
    }
    let remaining_capacity = state.remaining_carry_capacity_ref(actor_ref)?.0;
    let per_unit = load_per_unit(commodity).0;
    if remaining_capacity < per_unit {
        return None;
    }

    if let Some(requested_quantity) = payload_override
        .and_then(ActionPayload::as_transport)
        .map(|payload| payload.quantity)
    {
        let max_fit_quantity = Quantity(remaining_capacity / per_unit);
        if requested_quantity == Quantity(0)
            || requested_quantity > max_fit_quantity
            || requested_quantity > quantity
        {
            return None;
        }

        if requested_quantity == quantity {
            return Some(HypotheticalTransition {
                targets: vec![lot_ref],
                state: state.move_lot_ref_to_holder(
                    lot_ref,
                    actor_ref,
                    commodity,
                    requested_quantity,
                ),
                expected_materializations: Vec::new(),
            });
        }

        let remaining_quantity = Quantity(quantity.0 - requested_quantity.0);
        let mut state = state.set_quantity_ref(lot_ref, commodity, remaining_quantity);
        let hypothetical_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, commodity);
        let hypothetical_ref = PlanningEntityRef::Hypothetical(hypothetical_id);
        state = state
            .set_quantity_ref(hypothetical_ref, commodity, requested_quantity)
            .move_lot_ref_to_holder(hypothetical_ref, actor_ref, commodity, requested_quantity);

        return Some(HypotheticalTransition {
            targets: vec![lot_ref],
            state,
            expected_materializations: vec![ExpectedMaterialization {
                tag: MaterializationTag::SplitOffLot,
                hypothetical_id,
            }],
        });
    }

    if state.load_of_entity_ref(lot_ref)?.0 <= remaining_capacity {
        return Some(HypotheticalTransition {
            targets: vec![lot_ref],
            state: state.move_lot_ref_to_holder(lot_ref, actor_ref, commodity, quantity),
            expected_materializations: Vec::new(),
        });
    }

    let moved_quantity = Quantity(remaining_capacity / per_unit);
    if moved_quantity == Quantity(0) || moved_quantity.0 >= quantity.0 {
        return None;
    }
    let remaining_quantity = Quantity(quantity.0 - moved_quantity.0);
    let mut state = state.set_quantity_ref(lot_ref, commodity, remaining_quantity);
    let hypothetical_id = state.spawn_hypothetical_lot(EntityKind::ItemLot, commodity);
    let hypothetical_ref = PlanningEntityRef::Hypothetical(hypothetical_id);
    state = state
        .set_quantity_ref(hypothetical_ref, commodity, moved_quantity)
        .move_lot_ref_to_holder(hypothetical_ref, actor_ref, commodity, moved_quantity);

    Some(HypotheticalTransition {
        targets: vec![lot_ref],
        state,
        expected_materializations: vec![ExpectedMaterialization {
            tag: MaterializationTag::SplitOffLot,
            hypothetical_id,
        }],
    })
}

fn apply_steal_transition<'snapshot>(
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
) -> Option<HypotheticalTransition<'snapshot>> {
    let actor_ref = PlanningEntityRef::Authoritative(state.snapshot().actor());
    let lot_ref = match targets.first().copied()? {
        PlanningEntityRef::Authoritative(lot) => PlanningEntityRef::Authoritative(lot),
        PlanningEntityRef::Hypothetical(_) => return None,
    };
    if state.entity_kind_ref(lot_ref) != Some(EntityKind::ItemLot) {
        return None;
    }
    if state.direct_possessor_ref(lot_ref).is_some()
        || state.direct_container_ref(lot_ref).is_some()
    {
        return None;
    }
    if state.effective_place_ref(lot_ref)? != state.effective_place_ref(actor_ref)? {
        return None;
    }
    let commodity = state.item_lot_commodity_ref(lot_ref)?;
    let quantity = state.commodity_quantity_ref(lot_ref, commodity);
    if quantity == Quantity(0) {
        return None;
    }
    let remaining_capacity = state.remaining_carry_capacity_ref(actor_ref)?.0;
    let per_unit = load_per_unit(commodity).0;
    if quantity.0.saturating_mul(per_unit) > remaining_capacity {
        return None;
    }

    Some(HypotheticalTransition {
        targets: targets.to_vec(),
        state: state.move_lot_ref_to_holder(lot_ref, actor_ref, commodity, quantity),
        expected_materializations: Vec::new(),
    })
}

fn apply_store_stock_transition<'snapshot>(
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
) -> Option<HypotheticalTransition<'snapshot>> {
    let actor_ref = PlanningEntityRef::Authoritative(state.snapshot().actor());
    let actor_place = state.effective_place_ref(actor_ref)?;
    let stock_container = state
        .controlled_stock_containers_at_place(actor_ref, actor_place)
        .into_iter()
        .next()?;
    let lot_ref = targets.first().copied()?;
    if state.entity_kind_ref(lot_ref) != Some(EntityKind::ItemLot) {
        return None;
    }
    if state.effective_place_ref(lot_ref)? != actor_place {
        return None;
    }
    if state.direct_possessor_ref(lot_ref) != Some(actor_ref) {
        return None;
    }

    Some(HypotheticalTransition {
        targets: vec![lot_ref],
        state: state.set_container_ref(lot_ref, stock_container),
        expected_materializations: Vec::new(),
    })
}

fn apply_put_down_transition<'snapshot>(
    state: PlanningState<'snapshot>,
    targets: &[PlanningEntityRef],
) -> Option<HypotheticalTransition<'snapshot>> {
    let actor_ref = PlanningEntityRef::Authoritative(state.snapshot().actor());
    let lot_ref = targets.first().copied()?;
    if state.entity_kind_ref(lot_ref) != Some(EntityKind::ItemLot) {
        return None;
    }
    if state.direct_possessor_ref(lot_ref) != Some(actor_ref) {
        return None;
    }
    let place = state.effective_place_ref(actor_ref)?;
    let commodity = state.item_lot_commodity_ref(lot_ref)?;
    let quantity = state.commodity_quantity_ref(lot_ref, commodity);
    if quantity == Quantity(0) {
        return None;
    }

    Some(HypotheticalTransition {
        targets: vec![lot_ref],
        state: state.move_lot_ref_to_ground(lot_ref, place, commodity, quantity),
        expected_materializations: Vec::new(),
    })
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExpectedMaterialization {
    pub tag: MaterializationTag,
    pub hypothetical_id: HypotheticalEntityId,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlannedStep {
    pub def_id: ActionDefId,
    pub targets: Vec<PlanningEntityRef>,
    pub payload_override: Option<ActionPayload>,
    pub op_kind: PlannerOpKind,
    pub estimated_ticks: u32,
    pub is_materialization_barrier: bool,
    pub expected_materializations: Vec<ExpectedMaterialization>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerSyntheticCandidate {
    pub def_id: ActionDefId,
    pub targets: Vec<PlanningEntityRef>,
    pub payload_override: Option<ActionPayload>,
}

#[must_use]
pub fn planner_only_candidates(
    state: &PlanningState<'_>,
    semantics_table: &BTreeMap<ActionDefId, PlannerOpSemantics>,
) -> Vec<PlannerSyntheticCandidate> {
    let actor_ref = PlanningEntityRef::Authoritative(state.snapshot().actor());
    semantics_table
        .iter()
        .filter(|(_, semantics)| {
            semantics.transition_kind == PlannerTransitionKind::PutDownGroundLot
        })
        .flat_map(|(def_id, _)| {
            state
                .direct_possessions_ref(actor_ref)
                .into_iter()
                .filter(|entity| matches!(entity, PlanningEntityRef::Hypothetical(_)))
                .filter(|entity| state.entity_kind_ref(*entity).is_some())
                .map(|target| PlannerSyntheticCandidate {
                    def_id: *def_id,
                    targets: vec![target],
                    payload_override: None,
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[must_use]
pub fn resolve_planning_target_with<F>(
    target: PlanningEntityRef,
    resolve_hypothetical: &mut F,
) -> Option<EntityId>
where
    F: FnMut(HypotheticalEntityId) -> Option<EntityId>,
{
    match target {
        PlanningEntityRef::Authoritative(entity) => Some(entity),
        PlanningEntityRef::Hypothetical(id) => resolve_hypothetical(id),
    }
}

#[must_use]
pub fn resolve_planning_targets_with<F>(
    targets: &[PlanningEntityRef],
    mut resolve_hypothetical: F,
) -> Option<Vec<EntityId>>
where
    F: FnMut(HypotheticalEntityId) -> Option<EntityId>,
{
    targets
        .iter()
        .copied()
        .map(|target| resolve_planning_target_with(target, &mut resolve_hypothetical))
        .collect()
}

#[must_use]
pub fn authoritative_target(target: PlanningEntityRef) -> Option<EntityId> {
    resolve_planning_target_with(target, &mut |_| None)
}

#[must_use]
pub fn authoritative_targets(targets: &[PlanningEntityRef]) -> Option<Vec<EntityId>> {
    resolve_planning_targets_with(targets, |_| None)
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum PlanTerminalKind {
    GoalSatisfied,
    ProgressBarrier,
    CombatCommitment,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct PlannedPlan {
    pub goal: GoalKey,
    pub opportunity: worldwake_core::OpportunityKey,
    pub steps: Vec<PlannedStep>,
    pub total_estimated_ticks: u32,
    pub terminal_kind: PlanTerminalKind,
}

impl PlannedPlan {
    #[must_use]
    pub fn new(
        opportunity: worldwake_core::OpportunityKey,
        goal: GoalKey,
        steps: Vec<PlannedStep>,
        terminal_kind: PlanTerminalKind,
    ) -> Self {
        Self {
            goal,
            opportunity,
            total_estimated_ticks: total_estimated_ticks(&steps),
            steps,
            terminal_kind,
        }
    }

    #[must_use]
    pub fn remaining_travel_steps_from(&self, from_index: usize) -> usize {
        self.steps
            .iter()
            .skip(from_index)
            .filter(|step| step.op_kind == PlannerOpKind::Travel)
            .count()
    }

    #[must_use]
    pub fn has_remaining_travel_steps_from(&self, from_index: usize) -> bool {
        self.remaining_travel_steps_from(from_index) > 0
    }

    #[must_use]
    pub fn terminal_travel_destination(&self) -> Option<EntityId> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op_kind == PlannerOpKind::Travel)
            .and_then(|step| step.targets.first().copied())
            .and_then(authoritative_target)
    }
}

fn total_estimated_ticks(steps: &[PlannedStep]) -> u32 {
    steps.iter().fold(0u32, |acc, step| {
        acc.checked_add(step.estimated_ticks)
            .expect("planned step ticks overflow u32")
    })
}

#[cfg(test)]
mod tests {
    use super::{
        apply_hypothetical_transition, authoritative_target, authoritative_targets,
        build_semantics_table, classify_action_def, planner_only_candidates, semantics_for,
        resolve_planning_targets_with, ExpectedMaterialization, PlanTerminalKind, PlannedPlan,
        PlannedStep, PlannerOpKind, PlannerOpSemantics, PlannerTransitionKind,
    };
    use crate::{
        build_planning_snapshot, CommodityPurpose, GoalKey, GoalKind, GroundedGoal,
        HypotheticalEntityId, PlanningEntityRef, PlanningState,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        load_per_unit, ActionDefId, ActionDomain, BodyCostPerTick, CommodityConsumableProfile,
        CommodityKind, DemandObservation, DriveThresholds, EntityId, EntityKind,
        HomeostaticNeeds, InTransitOnEdge, LoadUnits, MerchandiseProfile, MetabolismProfile,
        Permille, Quantity, RecipeId, ResourceSource, TellTopic, TickRange,
        TradeDispositionProfile, UniqueItemKind, WorkstationTag, Wound,
    };
    use worldwake_sim::{
        estimate_duration_from_beliefs, ActionDefRegistry, ActionDuration, ActionPayload,
        BribeActionPayload, ConsultRecordActionPayload, DeclareSupportActionPayload, DurationExpr,
        MaterializationTag, PressForceClaimActionPayload, QueueForFacilityUsePayload,
        RecipeDefinition, RecipeRegistry, RuntimeBeliefView, TellActionPayload,
        ThreatenActionPayload, TradeActionPayload, TransportActionPayload,
        YieldForceClaimActionPayload,
    };
    use worldwake_systems::build_full_action_registries;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    fn sample_step() -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(7),
            targets: vec![
                PlanningEntityRef::Authoritative(entity(3)),
                PlanningEntityRef::Authoritative(entity(4)),
            ],
            payload_override: Some(ActionPayload::Trade(TradeActionPayload {
                counterparty: entity(3),
                sale_lot: EntityId { slot: 50, generation: 0 },
                offered_commodity: CommodityKind::Coin,
                offered_quantity: Quantity(2),
                requested_quantity: Quantity(1),
            })),
            op_kind: PlannerOpKind::Trade,
            estimated_ticks: 5,
            is_materialization_barrier: true,
            expected_materializations: Vec::new(),
        }
    }

    fn travel_step(target: EntityId) -> PlannedStep {
        PlannedStep {
            def_id: ActionDefId(8),
            targets: vec![PlanningEntityRef::Authoritative(target)],
            payload_override: None,
            op_kind: PlannerOpKind::Travel,
            estimated_ticks: 2,
            is_materialization_barrier: false,
            expected_materializations: Vec::new(),
        }
    }

    fn build_phase_two_registry() -> ActionDefRegistry {
        let mut recipes = RecipeRegistry::new();
        recipes.register(RecipeDefinition {
            name: "Harvest Apples".to_string(),
            inputs: Vec::new(),
            outputs: vec![(CommodityKind::Apple, Quantity(2))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::OrchardRow),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        recipes.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(4).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        build_full_action_registries(&recipes).unwrap().defs
    }

    #[test]
    fn planned_plan_remaining_travel_steps_counts_from_index() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::Place(entity(11)),
            },
            goal,
            vec![
                travel_step(entity(11)),
                sample_step(),
                travel_step(entity(12)),
                travel_step(entity(13)),
            ],
            PlanTerminalKind::GoalSatisfied,
        );

        assert_eq!(plan.remaining_travel_steps_from(0), 3);
        assert_eq!(plan.remaining_travel_steps_from(2), 2);
        assert!(plan.has_remaining_travel_steps_from(2));
        assert_eq!(plan.remaining_travel_steps_from(10), 0);
        assert!(!plan.has_remaining_travel_steps_from(10));
    }

    #[test]
    fn planned_plan_terminal_travel_destination_uses_last_travel_step() {
        let last_target = entity(13);
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::Place(last_target),
            },
            goal,
            vec![
                travel_step(entity(11)),
                sample_step(),
                travel_step(last_target),
            ],
            PlanTerminalKind::GoalSatisfied,
        );

        assert_eq!(plan.terminal_travel_destination(), Some(last_target));

        let non_travel_plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::None,
            },
            goal,
            vec![sample_step()],
            PlanTerminalKind::GoalSatisfied,
        );
        assert_eq!(non_travel_plan.terminal_travel_destination(), None);
    }

    #[derive(Default)]
    struct TestBeliefView {
        alive: BTreeSet<EntityId>,
        kinds: BTreeMap<EntityId, EntityKind>,
        effective_places: BTreeMap<EntityId, EntityId>,
        entities_at: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessions: BTreeMap<EntityId, Vec<EntityId>>,
        direct_possessors: BTreeMap<EntityId, EntityId>,
        lot_commodities: BTreeMap<EntityId, CommodityKind>,
        commodity_quantities: BTreeMap<(EntityId, CommodityKind), Quantity>,
        carry_capacities: BTreeMap<EntityId, LoadUnits>,
        entity_loads: BTreeMap<EntityId, LoadUnits>,
        needs: BTreeMap<EntityId, HomeostaticNeeds>,
        thresholds: BTreeMap<EntityId, DriveThresholds>,
        adjacent: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
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
                .map(|(place, _)| place)
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
            _actor: EntityId,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Quantity {
            Quantity(0)
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
            _entity: EntityId,
        ) -> Option<CommodityConsumableProfile> {
            None
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

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
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

        fn trade_disposition_profile(&self, _agent: EntityId) -> Option<TradeDispositionProfile> {
            None
        }

        fn intention_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<worldwake_core::IntentionDispositionProfile> {
            None
        }
        fn route_exists(&self, _from: EntityId, _to: EntityId) -> bool {
            false
        }

        fn combat_profile(&self, _agent: EntityId) -> Option<worldwake_core::CombatProfile> {
            None
        }

        fn wounds(&self, _agent: EntityId) -> Vec<Wound> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn listed_sale_lots_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn seller_for_sale_lot(&self, _lot: EntityId) -> Option<EntityId> {
            None
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

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
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

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn sample_snapshot() -> (PlanningState<'static>, EntityId, EntityId, EntityId) {
        let actor = entity(1);
        let town = entity(10);
        let bread = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, town, bread]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(town, EntityKind::Place);
        view.kinds.insert(bread, EntityKind::ItemLot);
        view.effective_places.insert(actor, town);
        view.effective_places.insert(bread, town);
        view.entities_at.insert(town, vec![actor, bread]);
        view.lot_commodities.insert(bread, CommodityKind::Bread);
        view.commodity_quantities
            .insert((bread, CommodityKind::Bread), Quantity(1));
        view.carry_capacities.insert(actor, LoadUnits(4));
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(bread, LoadUnits(1));
        view.needs.insert(
            actor,
            HomeostaticNeeds::new(pm(800), pm(0), pm(0), pm(0), pm(0)),
        );
        view.thresholds.insert(actor, DriveThresholds::default());
        let snapshot = Box::leak(Box::new(build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([bread]),
            &BTreeSet::from([town]),
            1,
        )));

        (PlanningState::new(snapshot), actor, town, bread)
    }

    fn pickup_snapshot(
        commodity: CommodityKind,
        quantity: Quantity,
        carry_capacity: LoadUnits,
    ) -> (
        PlanningState<'static>,
        EntityId,
        EntityId,
        PlanningEntityRef,
    ) {
        let actor = entity(1);
        let place = entity(10);
        let lot = entity(20);
        let mut view = TestBeliefView::default();
        view.alive.extend([actor, place, lot]);
        view.kinds.insert(actor, EntityKind::Agent);
        view.kinds.insert(place, EntityKind::Place);
        view.kinds.insert(lot, EntityKind::ItemLot);
        view.effective_places.insert(actor, place);
        view.effective_places.insert(lot, place);
        view.entities_at.insert(place, vec![actor, lot]);
        view.lot_commodities.insert(lot, commodity);
        view.commodity_quantities.insert((lot, commodity), quantity);
        view.carry_capacities.insert(actor, carry_capacity);
        view.entity_loads.insert(actor, LoadUnits(0));
        view.entity_loads.insert(
            lot,
            LoadUnits(quantity.0.saturating_mul(load_per_unit(commodity).0)),
        );

        let snapshot = Box::leak(Box::new(build_planning_snapshot(
            &view,
            actor,
            &BTreeSet::from([lot]),
            &BTreeSet::from([place]),
            1,
        )));
        (
            PlanningState::new(snapshot),
            actor,
            place,
            PlanningEntityRef::Authoritative(lot),
        )
    }

    #[test]
    fn authoritative_targets_resolve_without_binding_state() {
        let step = sample_step();

        assert_eq!(
            authoritative_targets(&step.targets),
            Some(vec![entity(3), entity(4)])
        );
    }

    #[test]
    fn hypothetical_targets_require_external_resolution() {
        let targets = vec![
            PlanningEntityRef::Authoritative(entity(3)),
            PlanningEntityRef::Hypothetical(HypotheticalEntityId(9)),
        ];

        assert_eq!(authoritative_targets(&targets), None);
        assert_eq!(
            resolve_planning_targets_with(&targets, |id| {
                (id == HypotheticalEntityId(9)).then_some(entity(42))
            }),
            Some(vec![entity(3), entity(42)])
        );
    }

    #[test]
    fn authoritative_target_rejects_hypothetical_refs() {
        assert_eq!(
            authoritative_target(PlanningEntityRef::Authoritative(entity(7))),
            Some(entity(7))
        );
        assert_eq!(
            authoritative_target(PlanningEntityRef::Hypothetical(HypotheticalEntityId(1))),
            None
        );
    }

    #[test]
    fn planned_plan_new_derives_total_estimated_ticks_from_steps() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let mut second = sample_step();
        second.estimated_ticks = 9;
        second.is_materialization_barrier = true;

        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::Place(entity(21)),
            },
            goal,
            vec![sample_step(), second],
            PlanTerminalKind::ProgressBarrier,
        );

        assert_eq!(plan.total_estimated_ticks, 14);
    }

    #[test]
    fn planned_plan_new_uses_zero_ticks_for_empty_steps() {
        let goal = GoalKey::from(GoalKind::ReduceDanger);
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::Entity(entity(77)),
            },
            goal,
            Vec::new(),
            PlanTerminalKind::ProgressBarrier,
        );

        assert_eq!(plan.total_estimated_ticks, 0);
    }

    #[test]
    fn planned_plan_new_preserves_concrete_opportunity_identity() {
        let goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Bread,
            purpose: CommodityPurpose::SelfConsume,
        });
        let opportunity = worldwake_core::OpportunityKey {
            goal_key: goal,
            anchor: worldwake_core::OpportunityAnchor::Place(entity(55)),
        };

        let plan = PlannedPlan::new(
            opportunity,
            goal,
            vec![sample_step()],
            PlanTerminalKind::GoalSatisfied,
        );

        assert_eq!(plan.goal, goal);
        assert_eq!(plan.opportunity, opportunity);
        assert_eq!(plan.opportunity.goal_key, plan.goal);
    }

    #[test]
    fn planned_plan_roundtrips_through_bincode() {
        let goal = GoalKey::from(GoalKind::Sleep);
        let plan = PlannedPlan::new(
            worldwake_core::OpportunityKey {
                goal_key: goal,
                anchor: worldwake_core::OpportunityAnchor::Place(entity(6)),
            },
            goal,
            vec![PlannedStep {
                def_id: ActionDefId(2),
                targets: vec![PlanningEntityRef::Authoritative(entity(6))],
                payload_override: None,
                op_kind: PlannerOpKind::Sleep,
                estimated_ticks: 1,
                is_materialization_barrier: false,
                expected_materializations: Vec::new(),
            }],
            PlanTerminalKind::GoalSatisfied,
        );

        let bytes = bincode::serialize(&plan).unwrap();
        let roundtrip: PlannedPlan = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, plan);
    }

    #[test]
    fn planner_op_kind_covers_exactly_current_phase_two_families() {
        let all = [
            PlannerOpKind::Travel,
            PlannerOpKind::Patrol,
            PlannerOpKind::Consume,
            PlannerOpKind::Sleep,
            PlannerOpKind::Relieve,
            PlannerOpKind::Wash,
            PlannerOpKind::EstablishCamp,
            PlannerOpKind::Trade,
            PlannerOpKind::QueueForFacilityUse,
            PlannerOpKind::Harvest,
            PlannerOpKind::Craft,
            PlannerOpKind::MoveCargo,
            PlannerOpKind::Heal,
            PlannerOpKind::Loot,
            PlannerOpKind::Bury,
            PlannerOpKind::Tell,
            PlannerOpKind::ConsultRecord,
            PlannerOpKind::Attack,
            PlannerOpKind::Defend,
            PlannerOpKind::Bribe,
            PlannerOpKind::Threaten,
            PlannerOpKind::DeclareSupport,
            PlannerOpKind::AskWitness,
        ];

        assert_eq!(all.len(), 23);
    }

    #[test]
    fn build_semantics_table_classifies_registered_planner_action_defs() {
        let defs = build_phase_two_registry();
        let table = build_semantics_table(&defs);
        let semantics_by_name = defs
            .iter()
            .filter_map(|def| {
                table
                    .get(&def.id)
                    .map(|semantics| (def.name.as_str(), semantics))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let expected_ops = [
            ("tell", PlannerOpKind::Tell),
            ("consult_record", PlannerOpKind::ConsultRecord),
            ("eat", PlannerOpKind::Consume),
            ("drink", PlannerOpKind::Consume),
            ("sleep", PlannerOpKind::Sleep),
            ("toilet", PlannerOpKind::Relieve),
            ("relieve_wilderness", PlannerOpKind::Relieve),
            ("wash", PlannerOpKind::Wash),
            ("establish_camp", PlannerOpKind::EstablishCamp),
            ("travel", PlannerOpKind::Travel),
            ("pick_up", PlannerOpKind::MoveCargo),
            ("put_down", PlannerOpKind::MoveCargo),
            ("steal", PlannerOpKind::MoveCargo),
            ("trade", PlannerOpKind::Trade),
            ("queue_for_facility_use", PlannerOpKind::QueueForFacilityUse),
            ("attack", PlannerOpKind::Attack),
            ("defend", PlannerOpKind::Defend),
            ("loot", PlannerOpKind::Loot),
            ("bury", PlannerOpKind::Bury),
            ("heal", PlannerOpKind::Heal),
            ("bribe", PlannerOpKind::Bribe),
            ("threaten", PlannerOpKind::Threaten),
            ("declare_support", PlannerOpKind::DeclareSupport),
            ("ask_witness", PlannerOpKind::AskWitness),
            ("store_stock", PlannerOpKind::StockManagement),
            ("collect_display_stock", PlannerOpKind::StockManagement),
            ("stage_stock_for_sale", PlannerOpKind::StockManagement),
            ("unstage_stock", PlannerOpKind::StockManagement),
        ];
        let expected_transitions = [
            ("tell", PlannerTransitionKind::GoalModelFallback),
            ("eat", PlannerTransitionKind::ConsumeMatchingTargetCommodity),
            (
                "drink",
                PlannerTransitionKind::ConsumeMatchingTargetCommodity,
            ),
            ("pick_up", PlannerTransitionKind::PickUpGroundLot),
            ("steal", PlannerTransitionKind::StealGroundLot),
            ("put_down", PlannerTransitionKind::PutDownGroundLot),
        ];
        let unclassified = defs
            .iter()
            .filter(|def| !table.contains_key(&def.id))
            .map(|def| def.name.as_str())
            .collect::<Vec<_>>();

        assert!(
            unclassified.is_empty(),
            "unexpected unclassified actions: {unclassified:?}"
        );
        assert!(defs.iter().any(|def| def.name == "tell"));
        for (name, op_kind) in expected_ops {
            assert_eq!(semantics_by_name.get(name).unwrap().op_kind, op_kind);
        }
        assert_eq!(
            semantics_by_name.get("press_force_claim").unwrap().op_kind,
            PlannerOpKind::PressForceClaim
        );
        assert_eq!(
            semantics_by_name.get("yield_force_claim").unwrap().op_kind,
            PlannerOpKind::YieldForceClaim
        );
        for (name, transition_kind) in expected_transitions {
            assert_eq!(
                semantics_by_name.get(name).unwrap().transition_kind,
                transition_kind
            );
        }
        assert!(defs.iter().any(|def| {
            def.name.starts_with("harvest:")
                && table.get(&def.id).unwrap().op_kind == PlannerOpKind::Harvest
        }));
        assert!(defs.iter().any(|def| {
            def.name.starts_with("craft:")
                && table.get(&def.id).unwrap().op_kind == PlannerOpKind::Craft
        }));
    }

    #[test]
    fn classify_action_def_fixed_name_families_ignore_placeholder_payload_shape() {
        let defs = build_phase_two_registry();
        let variants = [
            (
                "establish_camp",
                ActionPayload::EstablishCamp(worldwake_sim::EstablishCampActionPayload {
                    faction: entity(11),
                }),
                PlannerOpKind::EstablishCamp,
            ),
            (
                "queue_for_facility_use",
                ActionPayload::QueueForFacilityUse(QueueForFacilityUsePayload {
                    intended_action: ActionDefId(999),
                }),
                PlannerOpKind::QueueForFacilityUse,
            ),
            (
                "tell",
                ActionPayload::Tell(TellActionPayload {
                    listener: entity(2),
                    topic: TellTopic::EntityBelief { subject: entity(3) },
                }),
                PlannerOpKind::Tell,
            ),
            (
                "consult_record",
                ActionPayload::ConsultRecord(ConsultRecordActionPayload { record: entity(4) }),
                PlannerOpKind::ConsultRecord,
            ),
            (
                "bribe",
                ActionPayload::Bribe(BribeActionPayload {
                    target: entity(5),
                    offered_commodity: CommodityKind::Coin,
                    offered_quantity: Quantity(1),
                }),
                PlannerOpKind::Bribe,
            ),
            (
                "threaten",
                ActionPayload::Threaten(ThreatenActionPayload { target: entity(6) }),
                PlannerOpKind::Threaten,
            ),
            (
                "declare_support",
                ActionPayload::DeclareSupport(DeclareSupportActionPayload {
                    office: entity(7),
                    candidate: entity(8),
                }),
                PlannerOpKind::DeclareSupport,
            ),
            (
                "press_force_claim",
                ActionPayload::PressForceClaim(PressForceClaimActionPayload { office: entity(9) }),
                PlannerOpKind::PressForceClaim,
            ),
            (
                "yield_force_claim",
                ActionPayload::YieldForceClaim(YieldForceClaimActionPayload { office: entity(10) }),
                PlannerOpKind::YieldForceClaim,
            ),
        ];

        for (name, payload, expected) in variants {
            let mut def = defs
                .iter()
                .find(|def| def.name == name)
                .cloned()
                .unwrap_or_else(|| panic!("missing registered action def {name}"));
            def.payload = payload;

            assert_eq!(
                classify_action_def(&def),
                Some(expected),
                "{name} should classify by stable action identity, not default payload shape"
            );
        }
    }

    #[test]
    fn build_semantics_table_marks_barriers_and_leaf_only_ops() {
        let defs = build_phase_two_registry();
        let table = build_semantics_table(&defs);

        for def in defs.iter().filter(|def| table.contains_key(&def.id)) {
            let semantics = table.get(&def.id).unwrap();
            let should_be_barrier = def.name == "trade"
                || def.name == "bury"
                || def.name == "loot"
                || def.name.starts_with("harvest:")
                || def.name.starts_with("craft:");
            assert_eq!(
                semantics.is_materialization_barrier, should_be_barrier,
                "unexpected barrier semantics for {}",
                def.name
            );
        }
        assert!(defs
            .iter()
            .filter(|def| {
                table.contains_key(&def.id)
                    && matches!(def.name.as_str(), "attack" | "defend" | "bury" | "tell")
            })
            .all(|def| !table.get(&def.id).unwrap().may_appear_mid_plan));
        assert!(defs
            .iter()
            .filter(|def| table.contains_key(&def.id) && def.name == "consult_record")
            .all(|def| table.get(&def.id).unwrap().may_appear_mid_plan));
    }

    #[test]
    fn tell_semantics_remain_standalone_non_barrier_fallback() {
        let defs = build_phase_two_registry();
        let table = build_semantics_table(&defs);
        let tell_semantics = defs
            .iter()
            .find(|def| def.name == "tell")
            .and_then(|def| table.get(&def.id))
            .copied()
            .expect("tell action should be classified into planner semantics");

        assert_eq!(tell_semantics.op_kind, PlannerOpKind::Tell);
        assert!(!tell_semantics.may_appear_mid_plan);
        assert!(!tell_semantics.is_materialization_barrier);
        assert_eq!(
            tell_semantics.transition_kind,
            PlannerTransitionKind::GoalModelFallback
        );
    }

    #[test]
    fn patrol_semantics_remain_leaf_only_non_barrier_fallback() {
        let mut def = build_phase_two_registry()
            .iter()
            .find(|def| def.name == "establish_camp")
            .cloned()
            .expect("establish_camp should exist in test registry");
        def.id = ActionDefId(77);
        def.name = "patrol".to_string();
        def.domain = ActionDomain::Generic;
        def.payload = ActionPayload::None;
        def.targets.clear();

        let semantics = semantics_for(
            &def,
            PlannerOpKind::Patrol,
        );

        assert_eq!(semantics.op_kind, PlannerOpKind::Patrol);
        assert!(!semantics.may_appear_mid_plan);
        assert!(!semantics.is_materialization_barrier);
        assert_eq!(semantics.transition_kind, PlannerTransitionKind::GoalModelFallback);
    }

    #[test]
    fn classify_action_def_maps_patrol_generic_action() {
        let mut def = build_phase_two_registry()
            .iter()
            .find(|def| def.name == "establish_camp")
            .cloned()
            .expect("establish_camp should exist in test registry");
        def.id = ActionDefId(78);
        def.name = "patrol".to_string();
        def.domain = ActionDomain::Generic;
        def.payload = ActionPayload::None;
        def.targets.clear();

        assert_eq!(classify_action_def(&def), Some(PlannerOpKind::Patrol));
    }

    #[test]
    fn hypothetical_transition_preserves_goal_model_fallback_for_non_pickup_ops() {
        let (state, actor, _town, bread) = sample_snapshot();
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };
        let semantics = build_phase_two_registry()
            .iter()
            .find(|def| def.name == "eat")
            .map(|def| build_semantics_table(&build_phase_two_registry())[&def.id])
            .unwrap();

        let advanced = apply_hypothetical_transition(
            &goal,
            semantics,
            state,
            &[PlanningEntityRef::Authoritative(bread)],
            None,
        )
        .unwrap()
        .state;
        let thresholds = advanced.drive_thresholds(actor).unwrap();

        assert!(advanced.homeostatic_needs(actor).unwrap().hunger < thresholds.hunger.low());
    }

    #[test]
    fn consume_transition_accepts_matching_target_commodity() {
        let (state, actor, _place, lot) = sample_snapshot();
        let semantics = build_phase_two_registry()
            .iter()
            .find(|def| def.name == "eat")
            .map(|def| build_semantics_table(&build_phase_two_registry())[&def.id])
            .unwrap();
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let advanced = apply_hypothetical_transition(
            &goal,
            semantics,
            state,
            &[PlanningEntityRef::Authoritative(lot)],
            None,
        )
        .unwrap()
        .state;
        let thresholds = advanced.drive_thresholds(actor).unwrap();

        assert!(advanced.homeostatic_needs(actor).unwrap().hunger < thresholds.hunger.low());
    }

    #[test]
    fn consume_transition_rejects_mismatched_target_commodity() {
        let (state, _actor, _place, lot) =
            pickup_snapshot(CommodityKind::Water, Quantity(1), LoadUnits(4));
        let semantics = build_phase_two_registry()
            .iter()
            .find(|def| def.name == "drink")
            .map(|def| build_semantics_table(&build_phase_two_registry())[&def.id])
            .unwrap();
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        assert!(apply_hypothetical_transition(&goal, semantics, state, &[lot], None).is_none());
    }

    #[test]
    fn pick_up_transition_full_fit_moves_authoritative_lot_without_materialization() {
        let (state, actor, _place, lot) =
            pickup_snapshot(CommodityKind::Bread, Quantity(1), LoadUnits(4));
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PickUpGroundLot,
        };
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Bread,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let advanced =
            apply_hypothetical_transition(&goal, semantics, state, &[lot], None).unwrap();

        assert_eq!(advanced.targets, vec![lot]);
        assert!(advanced.expected_materializations.is_empty());
        assert_eq!(
            advanced.state.direct_possessor_ref(lot),
            Some(PlanningEntityRef::Authoritative(actor))
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity(actor, CommodityKind::Bread),
            Quantity(1)
        );
    }

    #[test]
    fn pick_up_transition_partial_fit_creates_hypothetical_split_off_lot() {
        let (state, actor, _place, lot) =
            pickup_snapshot(CommodityKind::Water, Quantity(3), LoadUnits(4));
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PickUpGroundLot,
        };
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let advanced =
            apply_hypothetical_transition(&goal, semantics, state, &[lot], None).unwrap();
        assert_eq!(advanced.targets, vec![lot]);
        let split_off = match advanced.expected_materializations.as_slice() {
            [ExpectedMaterialization {
                tag: MaterializationTag::SplitOffLot,
                hypothetical_id,
            }] => PlanningEntityRef::Hypothetical(*hypothetical_id),
            _ => panic!("partial pickup should expose one split-off materialization"),
        };

        assert_eq!(
            advanced
                .state
                .commodity_quantity_ref(lot, CommodityKind::Water),
            Quantity(1)
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity_ref(split_off, CommodityKind::Water),
            Quantity(2)
        );
        assert_eq!(
            advanced.state.direct_possessor_ref(split_off),
            Some(PlanningEntityRef::Authoritative(actor))
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity(actor, CommodityKind::Water),
            Quantity(2)
        );
    }

    #[test]
    fn pick_up_transition_transport_payload_splits_exact_requested_quantity() {
        let (state, actor, _place, lot) =
            pickup_snapshot(CommodityKind::Bread, Quantity(3), LoadUnits(4));
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PickUpGroundLot,
        };
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination: entity(99),
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let advanced = apply_hypothetical_transition(
            &goal,
            semantics,
            state,
            &[lot],
            Some(&ActionPayload::Transport(TransportActionPayload {
                quantity: Quantity(1),
            })),
        )
        .unwrap();
        assert_eq!(advanced.targets, vec![lot]);
        let split_off = match advanced.expected_materializations.as_slice() {
            [ExpectedMaterialization {
                tag: MaterializationTag::SplitOffLot,
                hypothetical_id,
            }] => PlanningEntityRef::Hypothetical(*hypothetical_id),
            _ => panic!("payload split pickup should expose one split-off materialization"),
        };
        assert_eq!(
            advanced
                .state
                .commodity_quantity_ref(lot, CommodityKind::Bread),
            Quantity(2)
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity_ref(split_off, CommodityKind::Bread),
            Quantity(1)
        );
        assert_eq!(
            advanced.state.direct_possessor_ref(split_off),
            Some(PlanningEntityRef::Authoritative(actor))
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity(actor, CommodityKind::Bread),
            Quantity(1)
        );
    }

    #[test]
    fn pick_up_transition_zero_fit_is_invalid() {
        let (state, _actor, _place, lot) =
            pickup_snapshot(CommodityKind::Water, Quantity(1), LoadUnits(1));
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PickUpGroundLot,
        };
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        assert!(apply_hypothetical_transition(&goal, semantics, state, &[lot], None).is_none());
    }

    #[test]
    fn put_down_transition_moves_hypothetical_lot_to_ground_at_actor_place() {
        let (mut state, actor, place, _lot) =
            pickup_snapshot(CommodityKind::Water, Quantity(1), LoadUnits(4));
        let hypothetical_id =
            state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let hypothetical = PlanningEntityRef::Hypothetical(hypothetical_id);
        state = state
            .set_quantity_ref(hypothetical, CommodityKind::Water, Quantity(1))
            .move_lot_ref_to_holder(
                hypothetical,
                PlanningEntityRef::Authoritative(actor),
                CommodityKind::Water,
                Quantity(1),
            );
        let semantics = PlannerOpSemantics {
            op_kind: PlannerOpKind::MoveCargo,
            may_appear_mid_plan: true,
            is_materialization_barrier: false,
            transition_kind: PlannerTransitionKind::PutDownGroundLot,
        };
        let goal = GroundedGoal {
            anchor: worldwake_core::OpportunityAnchor::None,
            key: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Water,
                purpose: CommodityPurpose::SelfConsume,
            }),
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
        };

        let advanced =
            apply_hypothetical_transition(&goal, semantics, state, &[hypothetical], None).unwrap();

        assert_eq!(advanced.targets, vec![hypothetical]);
        assert_eq!(advanced.state.direct_possessor_ref(hypothetical), None);
        assert_eq!(
            advanced.state.effective_place_ref(hypothetical),
            Some(place)
        );
        assert_eq!(
            advanced
                .state
                .commodity_quantity(actor, CommodityKind::Water),
            Quantity(0)
        );
    }

    #[test]
    fn planner_only_candidates_synthesize_put_down_for_hypothetical_direct_possessions() {
        let (mut state, actor, _place, _lot) =
            pickup_snapshot(CommodityKind::Water, Quantity(1), LoadUnits(4));
        let hypothetical_id =
            state.spawn_hypothetical_lot(EntityKind::ItemLot, CommodityKind::Water);
        let hypothetical = PlanningEntityRef::Hypothetical(hypothetical_id);
        state = state
            .set_quantity_ref(hypothetical, CommodityKind::Water, Quantity(1))
            .move_lot_ref_to_holder(
                hypothetical,
                PlanningEntityRef::Authoritative(actor),
                CommodityKind::Water,
                Quantity(1),
            );
        let semantics_table = build_semantics_table(&build_phase_two_registry());

        let candidates = planner_only_candidates(&state, &semantics_table);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].targets, vec![hypothetical]);
        assert_eq!(candidates[0].payload_override, None);
    }

    #[test]
    fn classify_staff_market_action() {
        let registry = build_phase_two_registry();
        let def = registry
            .iter()
            .find(|def| def.name == "staff_market")
            .expect("staff_market action must exist");
        assert_eq!(classify_action_def(def), Some(PlannerOpKind::StaffMarket));
    }

    #[test]
    fn staff_market_semantics_allows_mid_plan_and_no_materialization_barrier() {
        let registry = build_phase_two_registry();
        let def = registry
            .iter()
            .find(|def| def.name == "staff_market")
            .expect("staff_market action must exist");
        let sem = semantics_for(def, PlannerOpKind::StaffMarket);
        assert_eq!(sem.op_kind, PlannerOpKind::StaffMarket);
        assert!(
            sem.may_appear_mid_plan,
            "StaffMarket should be allowed mid-plan"
        );
        assert!(
            !sem.is_materialization_barrier,
            "StaffMarket should NOT be a materialization barrier"
        );
        assert_eq!(sem.transition_kind, PlannerTransitionKind::GoalModelFallback);
    }
}
