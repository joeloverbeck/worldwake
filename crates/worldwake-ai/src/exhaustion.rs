use crate::{ExhaustionEntry, GoalDispatchKey, InvalidationStrategy, OpportunityKey};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    CommodityKind, CommodityPurpose, EntityId, EntityKind, GoalKind, HomeostaticNeedId,
    HomeostaticNeeds, Permille, Quantity, ThresholdBand, UniqueItemKind,
};
use worldwake_sim::{GoalBeliefView, RecipeRegistry};

/// Condition that would make a previously exhausted goal worth re-searching.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum ExhaustionInvalidationCondition {
    PositionChanged,
    CommodityChanged(CommodityKind),
    UniqueItemChanged(UniqueItemKind),
    WoundsChanged,
    FacilitiesChanged,
    BlockerExpired,
    HostilesChanged,
    NeedChangedBands {
        need: HomeostaticNeedId,
        band: ThresholdBand,
    },
    StealTargetStateChanged(EntityId),
    TargetDead(EntityId),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub enum StealTargetAccessState {
    #[default]
    UnownedOrMissing,
    Stealable,
    LawfulControl,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct StealTargetSnapshot {
    pub effective_place: Option<EntityId>,
    pub direct_possessor: Option<EntityId>,
    pub direct_container: Option<EntityId>,
    pub access_state: StealTargetAccessState,
    pub fits_carry_capacity: bool,
    pub is_item_lot: bool,
}

/// Snapshot of the goal-relevant state when a goal exhausted planning.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Serialize, Deserialize)]
pub struct ExhaustionBaseline {
    pub position: Option<EntityId>,
    pub needs: Option<HomeostaticNeeds>,
    pub commodity_quantities: Vec<(CommodityKind, Quantity)>,
    pub unique_item_counts: Vec<(UniqueItemKind, u32)>,
    pub steal_target_states: Vec<(EntityId, StealTargetSnapshot)>,
    pub wound_count: usize,
    pub hostile_count: usize,
}

#[allow(dead_code)]
pub(crate) fn derive_invalidation_conditions(
    goal: &GoalKind,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    recipe_registry: &RecipeRegistry,
) -> (Vec<ExhaustionInvalidationCondition>, ExhaustionBaseline) {
    let mut conditions = BTreeSet::new();

    match GoalDispatchKey::from_goal_kind(goal)
        .declaration()
        .invalidation_strategy
    {
        InvalidationStrategy::CommodityOnly => commodity_only_conditions(goal, &mut conditions),
        InvalidationStrategy::AcquireCommodity => {
            acquire_commodity_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::AcquireRestock => {
            acquire_restock_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::NeedWithFacilities(need) => {
            need_with_facilities_conditions(need, agent, view, &mut conditions);
        }
        InvalidationStrategy::NeedWithPosition(need) => {
            need_with_position_conditions(need, agent, view, &mut conditions);
        }
        InvalidationStrategy::CombatTarget => combat_target_conditions(goal, &mut conditions),
        InvalidationStrategy::DangerReduction => danger_reduction_conditions(&mut conditions),
        InvalidationStrategy::FactionRegroup | InvalidationStrategy::InvestigateViolation => {
            conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
        }
        InvalidationStrategy::TreatWounds => treat_wounds_conditions(goal, &mut conditions),
        InvalidationStrategy::ProduceCommodity => {
            produce_commodity_conditions(goal, recipe_registry, &mut conditions);
        }
        InvalidationStrategy::PositionAndCommodity => {
            position_and_commodity_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::PositionCommodityAndCoin => {
            position_commodity_and_coin_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::PositionAndTargetDead => {
            position_and_target_dead_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::StealTargetState => {
            steal_target_state_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::ClaimOffice => claim_office_conditions(&mut conditions),
        InvalidationStrategy::SupportCandidateForOffice => {
            support_candidate_conditions(goal, &mut conditions);
        }
        InvalidationStrategy::PunishAccused => punish_accused_conditions(goal, &mut conditions),
    }

    let conditions = conditions.into_iter().collect::<Vec<_>>();
    let baseline = build_baseline(agent, view, &conditions);
    (conditions, baseline)
}

fn commodity_only_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::ConsumeOwnedCommodity { commodity } = *goal else {
        unreachable!("CommodityOnly strategy requires ConsumeOwnedCommodity goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(commodity));
}

fn acquire_commodity_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::AcquireCommodity { commodity, .. } = *goal else {
        unreachable!("AcquireCommodity strategy requires AcquireCommodity goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(commodity));
}

fn acquire_restock_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::AcquireCommodity { purpose, .. } = *goal else {
        unreachable!("AcquireRestock strategy requires AcquireCommodity goal");
    };
    debug_assert!(matches!(purpose, CommodityPurpose::Restock));
    acquire_commodity_conditions(goal, conditions);
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(
        CommodityKind::Coin,
    ));
}

fn need_with_facilities_conditions(
    need: HomeostaticNeedId,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    conditions.insert(ExhaustionInvalidationCondition::NeedChangedBands {
        need,
        band: need_threshold_band(view, agent, need),
    });
    conditions.insert(ExhaustionInvalidationCondition::FacilitiesChanged);
}

fn need_with_position_conditions(
    need: HomeostaticNeedId,
    agent: EntityId,
    view: &dyn GoalBeliefView,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    conditions.insert(ExhaustionInvalidationCondition::NeedChangedBands {
        need,
        band: need_threshold_band(view, agent, need),
    });
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
}

fn combat_target_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let (GoalKind::EngageHostile { target } | GoalKind::RaidTarget { target }) = *goal else {
        unreachable!("CombatTarget strategy requires EngageHostile or RaidTarget goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::WoundsChanged);
    conditions.insert(ExhaustionInvalidationCondition::TargetDead(target));
}

fn danger_reduction_conditions(conditions: &mut BTreeSet<ExhaustionInvalidationCondition>) {
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::WoundsChanged);
    conditions.insert(ExhaustionInvalidationCondition::HostilesChanged);
}

fn treat_wounds_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::TreatWounds { patient } = *goal else {
        unreachable!("TreatWounds strategy requires TreatWounds goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::WoundsChanged);
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(
        CommodityKind::Medicine,
    ));
    conditions.insert(ExhaustionInvalidationCondition::TargetDead(patient));
}

fn produce_commodity_conditions(
    goal: &GoalKind,
    recipe_registry: &RecipeRegistry,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::ProduceCommodity { recipe_id } = *goal else {
        unreachable!("ProduceCommodity strategy requires ProduceCommodity goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::FacilitiesChanged);
    if let Some(recipe) = recipe_registry.get(recipe_id) {
        for (commodity, _) in &recipe.inputs {
            conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(
                *commodity,
            ));
        }
    }
}

fn position_and_commodity_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let (GoalKind::SellCommodity { commodity } | GoalKind::MoveCargo { commodity, .. }) = *goal
    else {
        unreachable!("PositionAndCommodity strategy requires SellCommodity or MoveCargo");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(commodity));
}

fn position_commodity_and_coin_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::RestockCommodity { commodity } = *goal else {
        unreachable!("PositionCommodityAndCoin strategy requires RestockCommodity goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(commodity));
    conditions.insert(ExhaustionInvalidationCondition::CommodityChanged(
        CommodityKind::Coin,
    ));
}

fn position_and_target_dead_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let target = match *goal {
        GoalKind::LootCorpse { corpse } | GoalKind::BuryCorpse { corpse, .. } => corpse,
        GoalKind::ShareBelief { listener, .. } => listener,
        GoalKind::Accuse { accused, .. } => accused,
        _ => unreachable!("PositionAndTargetDead strategy requires corpse, share, or accuse goal"),
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::TargetDead(target));
}

fn steal_target_state_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::StealItem { target_item } = *goal else {
        unreachable!("StealTargetState strategy requires StealItem goal");
    };
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::StealTargetStateChanged(
        target_item,
    ));
}

fn claim_office_conditions(conditions: &mut BTreeSet<ExhaustionInvalidationCondition>) {
    conditions.insert(ExhaustionInvalidationCondition::PositionChanged);
    conditions.insert(ExhaustionInvalidationCondition::BlockerExpired);
}

fn support_candidate_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::SupportCandidateForOffice { candidate, .. } = *goal else {
        unreachable!("SupportCandidateForOffice strategy requires SupportCandidateForOffice goal");
    };
    claim_office_conditions(conditions);
    conditions.insert(ExhaustionInvalidationCondition::TargetDead(candidate));
}

fn punish_accused_conditions(
    goal: &GoalKind,
    conditions: &mut BTreeSet<ExhaustionInvalidationCondition>,
) {
    let GoalKind::PunishAccused { accused, .. } = *goal else {
        unreachable!("PunishAccused strategy requires PunishAccused goal");
    };
    claim_office_conditions(conditions);
    conditions.insert(ExhaustionInvalidationCondition::TargetDead(accused));
}

pub(crate) fn invalidate_exhausted_goals(
    exhaustion_cache: &mut BTreeMap<OpportunityKey, ExhaustionEntry>,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
    facilities_changed: bool,
    blocker_expired: bool,
) {
    exhaustion_cache.retain(|_, entry| {
        if entry.invalidation_conditions.is_empty() {
            return false;
        }

        !entry.invalidation_conditions.iter().any(|condition| {
            condition_changed(
                condition,
                &entry.baseline,
                view,
                agent,
                currently_in_transit,
                facilities_changed,
                blocker_expired,
            )
        })
    });
}

#[allow(dead_code)]
pub(crate) fn condition_changed(
    condition: &ExhaustionInvalidationCondition,
    baseline: &ExhaustionBaseline,
    view: &dyn GoalBeliefView,
    agent: EntityId,
    currently_in_transit: bool,
    facilities_changed: bool,
    blocker_expired: bool,
) -> bool {
    match condition {
        ExhaustionInvalidationCondition::PositionChanged => {
            !currently_in_transit && view.effective_place(agent) != baseline.position
        }
        ExhaustionInvalidationCondition::CommodityChanged(kind) => {
            let current = view.commodity_quantity(agent, *kind);
            baseline
                .commodity_quantities
                .iter()
                .find(|(baseline_kind, _)| baseline_kind == kind)
                .map_or(current > Quantity(0), |(_, baseline_quantity)| {
                    *baseline_quantity != current
                })
        }
        ExhaustionInvalidationCondition::UniqueItemChanged(kind) => {
            let current = view.unique_item_count(agent, *kind);
            baseline
                .unique_item_counts
                .iter()
                .find(|(baseline_kind, _)| baseline_kind == kind)
                .map_or(current > 0, |(_, baseline_count)| {
                    *baseline_count != current
                })
        }
        ExhaustionInvalidationCondition::WoundsChanged => {
            view.wounds(agent).len() != baseline.wound_count
        }
        ExhaustionInvalidationCondition::FacilitiesChanged => facilities_changed,
        ExhaustionInvalidationCondition::BlockerExpired => blocker_expired,
        ExhaustionInvalidationCondition::HostilesChanged => {
            view.visible_hostiles_for(agent).len() != baseline.hostile_count
        }
        ExhaustionInvalidationCondition::NeedChangedBands { need, band } => {
            let Some(current_needs) = view.homeostatic_needs(agent) else {
                return false;
            };
            let Some(baseline_needs) = baseline.needs else {
                return true;
            };
            classify_need_band(need_value(&current_needs, *need), *band)
                != classify_need_band(need_value(&baseline_needs, *need), *band)
        }
        ExhaustionInvalidationCondition::StealTargetStateChanged(target) => {
            let current = capture_steal_target_snapshot(view, agent, *target);
            baseline
                .steal_target_states
                .iter()
                .find(|(baseline_target, _)| baseline_target == target)
                .map_or(
                    current != StealTargetSnapshot::default(),
                    |(_, baseline_snapshot)| *baseline_snapshot != current,
                )
        }
        ExhaustionInvalidationCondition::TargetDead(target) => !view.is_alive(*target),
    }
}

#[allow(dead_code)]
pub(crate) fn need_value(needs: &HomeostaticNeeds, need: HomeostaticNeedId) -> Permille {
    match need {
        HomeostaticNeedId::Hunger => needs.hunger,
        HomeostaticNeedId::Thirst => needs.thirst,
        HomeostaticNeedId::Fatigue => needs.fatigue,
        HomeostaticNeedId::Bladder => needs.bladder,
        HomeostaticNeedId::Dirtiness => needs.dirtiness,
    }
}

#[allow(dead_code)]
fn build_baseline(
    agent: EntityId,
    view: &dyn GoalBeliefView,
    conditions: &[ExhaustionInvalidationCondition],
) -> ExhaustionBaseline {
    let commodity_kinds = conditions
        .iter()
        .filter_map(|condition| match condition {
            ExhaustionInvalidationCondition::CommodityChanged(kind) => Some(*kind),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let unique_item_kinds = conditions
        .iter()
        .filter_map(|condition| match condition {
            ExhaustionInvalidationCondition::UniqueItemChanged(kind) => Some(*kind),
            _ => None,
        })
        .collect::<BTreeSet<_>>();
    let steal_targets = conditions
        .iter()
        .filter_map(|condition| match condition {
            ExhaustionInvalidationCondition::StealTargetStateChanged(target) => Some(*target),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    ExhaustionBaseline {
        position: view.effective_place(agent),
        needs: view.homeostatic_needs(agent),
        commodity_quantities: commodity_kinds
            .into_iter()
            .map(|kind| (kind, view.commodity_quantity(agent, kind)))
            .collect(),
        unique_item_counts: unique_item_kinds
            .into_iter()
            .map(|kind| (kind, view.unique_item_count(agent, kind)))
            .collect(),
        steal_target_states: steal_targets
            .into_iter()
            .map(|target| (target, capture_steal_target_snapshot(view, agent, target)))
            .collect(),
        wound_count: view.wounds(agent).len(),
        hostile_count: view.visible_hostiles_for(agent).len(),
    }
}

fn capture_steal_target_snapshot(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    target: EntityId,
) -> StealTargetSnapshot {
    let is_item_lot = view.entity_kind(target) == Some(EntityKind::ItemLot);
    let direct_possessor = view.direct_possessor(target);
    let direct_container = view.direct_container(target);
    let access_state = match view.believed_owner_of(target) {
        None => StealTargetAccessState::UnownedOrMissing,
        Some(_) if view.can_control(agent, target) => StealTargetAccessState::LawfulControl,
        Some(_) => StealTargetAccessState::Stealable,
    };
    let fits_carry_capacity = fits_target_load(view, agent, target);

    StealTargetSnapshot {
        effective_place: view.effective_place(target),
        direct_possessor,
        direct_container,
        access_state,
        fits_carry_capacity,
        is_item_lot,
    }
}

fn fits_target_load(view: &dyn GoalBeliefView, agent: EntityId, target: EntityId) -> bool {
    let Some(target_load) = view.load_of_entity(target) else {
        return false;
    };
    let Some(carry_capacity) = view.carry_capacity(agent) else {
        return false;
    };
    let Some(actor_load) = view.load_of_entity(agent) else {
        return false;
    };

    carry_capacity.0.saturating_sub(actor_load.0) >= target_load.0
}

#[allow(dead_code)]
fn need_threshold_band(
    view: &dyn GoalBeliefView,
    agent: EntityId,
    need: HomeostaticNeedId,
) -> ThresholdBand {
    let thresholds = view.drive_thresholds(agent).unwrap_or_default();
    match need {
        HomeostaticNeedId::Hunger => thresholds.hunger,
        HomeostaticNeedId::Thirst => thresholds.thirst,
        HomeostaticNeedId::Fatigue => thresholds.fatigue,
        HomeostaticNeedId::Bladder => thresholds.bladder,
        HomeostaticNeedId::Dirtiness => thresholds.dirtiness,
    }
}

#[allow(dead_code)]
fn classify_need_band(value: worldwake_core::Permille, band: ThresholdBand) -> u8 {
    if value < band.low() {
        0
    } else if value < band.medium() {
        1
    } else if value < band.high() {
        2
    } else if value < band.critical() {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::{
        classify_need_band, condition_changed, derive_invalidation_conditions,
        invalidate_exhausted_goals, need_threshold_band, need_value, ExhaustionBaseline,
        ExhaustionInvalidationCondition, StealTargetAccessState, StealTargetSnapshot,
    };
    use crate::{ExhaustionEntry, ExhaustionRetryState, GoalKey, OpportunityKey};
    use std::collections::BTreeMap;
    use std::num::NonZeroU32;
    use worldwake_core::{
        BeliefConfidencePolicy, BodyCostPerTick, BodyPart, CommodityConsumableProfile,
        CommodityKind, DemandObservation, DriveThresholds, EntityId, EntityKind, GoalKind,
        HomeostaticNeedId, HomeostaticNeeds, InstitutionalBeliefRead, JusticeDispositionProfile,
        LoadUnits, MerchandiseProfile, OfficeData, Permille, PunishmentKind, Quantity,
        RecipientKnowledgeStatus, RecordEntryId, ResourceSource, TellMemoryKey, TellProfile,
        TellTopic, TheftDispositionProfile, ThresholdBand, UniqueItemKind,
        ViolationDispositionProfile, ViolationId, WorkstationTag, Wound, WoundCause, WoundId,
    };
    use worldwake_sim::{GoalBeliefView, RecipeDefinition, RecipeRegistry};

    #[derive(Default)]
    struct MockView {
        entity_kinds: Vec<(EntityId, EntityKind)>,
        effective_places: Vec<(EntityId, EntityId)>,
        commodity_quantities: Vec<((EntityId, CommodityKind), Quantity)>,
        unique_item_counts: Vec<((EntityId, UniqueItemKind), u32)>,
        direct_containers: Vec<(EntityId, EntityId)>,
        direct_possessors: Vec<(EntityId, EntityId)>,
        owners: Vec<(EntityId, EntityId)>,
        controllable: Vec<(EntityId, EntityId)>,
        carry_capacities: Vec<(EntityId, LoadUnits)>,
        loads: Vec<(EntityId, LoadUnits)>,
        needs: Vec<(EntityId, HomeostaticNeeds)>,
        drive_thresholds: Vec<(EntityId, DriveThresholds)>,
        wounds: Vec<(EntityId, Vec<Wound>)>,
        visible_hostiles: Vec<(EntityId, Vec<EntityId>)>,
        dead: Vec<EntityId>,
    }

    impl GoalBeliefView for MockView {
        fn is_alive(&self, entity: EntityId) -> bool {
            !self.dead.contains(&entity)
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn entity_kind(&self, entity: EntityId) -> Option<EntityKind> {
            self.entity_kinds
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, kind)| *kind)
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.effective_places
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, place)| *place)
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            _place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            Vec::new()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }

        fn unique_item_count(&self, holder: EntityId, kind: UniqueItemKind) -> u32 {
            self.unique_item_counts
                .iter()
                .find(|((entity, item_kind), _)| *entity == holder && *item_kind == kind)
                .map_or(0, |(_, count)| *count)
        }

        fn commodity_quantity(&self, holder: EntityId, kind: CommodityKind) -> Quantity {
            self.commodity_quantities
                .iter()
                .find(|((entity, commodity), _)| *entity == holder && *commodity == kind)
                .map_or(Quantity(0), |(_, quantity)| *quantity)
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
        ) -> Option<CommodityConsumableProfile> {
            None
        }

        fn direct_container(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_containers
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, container)| *container)
        }

        fn direct_possessor(&self, entity: EntityId) -> Option<EntityId> {
            self.direct_possessors
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, possessor)| *possessor)
        }

        fn believed_owner_of(&self, entity: EntityId) -> Option<EntityId> {
            self.owners
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, owner)| *owner)
        }

        fn workstation_tag(&self, _entity: EntityId) -> Option<WorkstationTag> {
            None
        }

        fn resource_source(&self, _entity: EntityId) -> Option<ResourceSource> {
            None
        }

        fn resource_sources_at(
            &self,
            _place: EntityId,
            _commodity: CommodityKind,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn matching_workstations_at(
            &self,
            _place: EntityId,
            _tag: WorkstationTag,
        ) -> Vec<EntityId> {
            Vec::new()
        }

        fn has_production_job(&self, _entity: EntityId) -> bool {
            false
        }

        fn can_control(&self, actor: EntityId, entity: EntityId) -> bool {
            self.controllable.contains(&(actor, entity))
        }

        fn carry_capacity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.carry_capacities
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, capacity)| *capacity)
        }

        fn load_of_entity(&self, entity: EntityId) -> Option<LoadUnits> {
            self.loads
                .iter()
                .find(|(subject, _)| *subject == entity)
                .map(|(_, load)| *load)
        }

        fn is_incapacitated(&self, _entity: EntityId) -> bool {
            false
        }

        fn has_wounds(&self, entity: EntityId) -> bool {
            !self.wounds(entity).is_empty()
        }

        fn homeostatic_needs(&self, agent: EntityId) -> Option<HomeostaticNeeds> {
            self.needs
                .iter()
                .find(|(subject, _)| *subject == agent)
                .map(|(_, needs)| *needs)
        }

        fn drive_thresholds(&self, agent: EntityId) -> Option<DriveThresholds> {
            self.drive_thresholds
                .iter()
                .find(|(subject, _)| *subject == agent)
                .map(|(_, thresholds)| *thresholds)
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn theft_disposition_profile(&self, _agent: EntityId) -> Option<TheftDispositionProfile> {
            None
        }

        fn justice_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<JusticeDispositionProfile> {
            None
        }

        fn tell_profile(&self, _agent: EntityId) -> Option<TellProfile> {
            None
        }

        fn told_belief_memories(
            &self,
            _agent: EntityId,
        ) -> Vec<(TellMemoryKey, worldwake_core::ToldBeliefMemory)> {
            Vec::new()
        }

        fn told_belief_memory(
            &self,
            _actor: EntityId,
            _counterparty: EntityId,
            _topic: &TellTopic,
        ) -> Option<worldwake_core::ToldBeliefMemory> {
            None
        }

        fn recipient_knowledge_status(
            &self,
            _actor: EntityId,
            _counterparty: EntityId,
            _topic: &TellTopic,
        ) -> Option<RecipientKnowledgeStatus> {
            None
        }

        fn courage(&self, _agent: EntityId) -> Option<Permille> {
            None
        }

        fn violation_disposition_profile(
            &self,
            _agent: EntityId,
        ) -> Option<ViolationDispositionProfile> {
            None
        }

        fn merchandise_profile(&self, _agent: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn wounds(&self, agent: EntityId) -> Vec<Wound> {
            self.wounds
                .iter()
                .find(|(subject, _)| *subject == agent)
                .map_or_else(Vec::new, |(_, wounds)| wounds.clone())
        }

        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn visible_hostiles_for(&self, agent: EntityId) -> Vec<EntityId> {
            self.visible_hostiles
                .iter()
                .find(|(subject, _)| *subject == agent)
                .map_or_else(Vec::new, |(_, hostiles)| hostiles.clone())
        }

        fn current_attackers_of(&self, _agent: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn agents_selling_at(&self, _place: EntityId, _commodity: CommodityKind) -> Vec<EntityId> {
            Vec::new()
        }

        fn demand_memory(&self, _agent: EntityId) -> Vec<DemandObservation> {
            Vec::new()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn office_data(&self, _office: EntityId) -> Option<OfficeData> {
            None
        }

        fn believed_office_holder(
            &self,
            _office: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            InstitutionalBeliefRead::Unknown
        }

        fn believed_force_controller(
            &self,
            _office: EntityId,
        ) -> InstitutionalBeliefRead<(Option<EntityId>, bool)> {
            InstitutionalBeliefRead::Unknown
        }

        fn believed_membership(
            &self,
            _faction: EntityId,
            _member: EntityId,
        ) -> InstitutionalBeliefRead<bool> {
            InstitutionalBeliefRead::Unknown
        }

        fn offices_contested_by(&self, _claimant: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn loyalty_to(&self, _subject: EntityId, _target: EntityId) -> Option<Permille> {
            None
        }

        fn believed_support_declaration(
            &self,
            _office: EntityId,
            _supporter: EntityId,
        ) -> InstitutionalBeliefRead<Option<EntityId>> {
            InstitutionalBeliefRead::Unknown
        }

        fn believed_support_declarations_for_office(
            &self,
            _office: EntityId,
        ) -> Vec<(EntityId, InstitutionalBeliefRead<Option<EntityId>>)> {
            Vec::new()
        }
    }

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn pm(value: u16) -> Permille {
        Permille::new(value).unwrap()
    }

    fn wound(id: u64, attacker: EntityId) -> Wound {
        Wound {
            id: WoundId(id),
            body_part: BodyPart::Torso,
            cause: WoundCause::Combat {
                attacker,
                weapon: worldwake_core::CombatWeaponRef::Unarmed,
            },
            severity: pm(200),
            inflicted_at: worldwake_core::Tick(1),
            bleed_rate_per_tick: pm(10),
        }
    }

    fn recipe_definition(name: &str) -> RecipeDefinition {
        RecipeDefinition {
            name: name.to_string(),
            inputs: vec![
                (CommodityKind::Grain, Quantity(2)),
                (CommodityKind::Water, Quantity(1)),
            ],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        }
    }

    fn assert_has_condition(
        conditions: &[ExhaustionInvalidationCondition],
        expected: &ExhaustionInvalidationCondition,
    ) {
        assert!(
            conditions.contains(expected),
            "missing condition {expected:?} from {conditions:?}"
        );
    }

    fn baseline_with_position(position: EntityId) -> ExhaustionBaseline {
        ExhaustionBaseline {
            position: Some(position),
            ..ExhaustionBaseline::default()
        }
    }

    #[test]
    fn exhaustion_baseline_default_is_zero_value() {
        let baseline = ExhaustionBaseline::default();

        assert_eq!(baseline.position, None);
        assert_eq!(baseline.needs, None);
        assert!(baseline.commodity_quantities.is_empty());
        assert!(baseline.unique_item_counts.is_empty());
        assert!(baseline.steal_target_states.is_empty());
        assert_eq!(baseline.wound_count, 0);
        assert_eq!(baseline.hostile_count, 0);
    }

    #[test]
    fn need_value_reads_named_need_field() {
        let needs = HomeostaticNeeds::new(pm(10), pm(20), pm(30), pm(40), pm(50));

        assert_eq!(need_value(&needs, HomeostaticNeedId::Hunger), pm(10));
        assert_eq!(need_value(&needs, HomeostaticNeedId::Thirst), pm(20));
        assert_eq!(need_value(&needs, HomeostaticNeedId::Fatigue), pm(30));
        assert_eq!(need_value(&needs, HomeostaticNeedId::Bladder), pm(40));
        assert_eq!(need_value(&needs, HomeostaticNeedId::Dirtiness), pm(50));
    }

    #[test]
    fn condition_changed_position_ignores_mid_transit_waypoints() {
        let agent = entity(1);
        let baseline_place = entity(2);
        let moved_place = entity(3);
        let view = MockView {
            effective_places: vec![(agent, moved_place)],
            ..MockView::default()
        };

        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::PositionChanged,
            &baseline_with_position(baseline_place),
            &view,
            agent,
            true,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_position_detects_settled_arrival_delta() {
        let agent = entity(1);
        let baseline_place = entity(2);
        let moved_place = entity(3);
        let view = MockView {
            effective_places: vec![(agent, moved_place)],
            ..MockView::default()
        };

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::PositionChanged,
            &baseline_with_position(baseline_place),
            &view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_position_false_when_settled_place_matches_baseline() {
        let agent = entity(1);
        let place = entity(2);
        let view = MockView {
            effective_places: vec![(agent, place)],
            ..MockView::default()
        };

        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::PositionChanged,
            &baseline_with_position(place),
            &view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_commodity_detects_quantity_delta_and_absent_baseline_nonzero() {
        let agent = entity(1);
        let bread = CommodityKind::Bread;
        let changed_view = MockView {
            commodity_quantities: vec![((agent, bread), Quantity(3))],
            ..MockView::default()
        };
        let baseline = ExhaustionBaseline {
            commodity_quantities: vec![(bread, Quantity(1))],
            ..ExhaustionBaseline::default()
        };

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::CommodityChanged(bread),
            &baseline,
            &changed_view,
            agent,
            false,
            false,
            false,
        ));

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Apple),
            &ExhaustionBaseline::default(),
            &MockView {
                commodity_quantities: vec![((agent, CommodityKind::Apple), Quantity(1))],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));

        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::CommodityChanged(bread),
            &ExhaustionBaseline {
                commodity_quantities: vec![(bread, Quantity(3))],
                ..ExhaustionBaseline::default()
            },
            &changed_view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_unique_item_detects_count_delta() {
        let agent = entity(1);
        let item_kind = UniqueItemKind::SimpleTool;
        let view = MockView {
            unique_item_counts: vec![((agent, item_kind), 2)],
            ..MockView::default()
        };

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::UniqueItemChanged(item_kind),
            &ExhaustionBaseline {
                unique_item_counts: vec![(item_kind, 1)],
                ..ExhaustionBaseline::default()
            },
            &view,
            agent,
            false,
            false,
            false,
        ));

        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::UniqueItemChanged(item_kind),
            &ExhaustionBaseline {
                unique_item_counts: vec![(item_kind, 2)],
                ..ExhaustionBaseline::default()
            },
            &view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_wounds_hostiles_and_target_death_use_current_counts() {
        let agent = entity(1);
        let attacker = entity(2);
        let hostile = entity(3);
        let dead_target = entity(4);
        let view = MockView {
            wounds: vec![(agent, vec![wound(1, attacker), wound(2, attacker)])],
            visible_hostiles: vec![(agent, vec![hostile])],
            dead: vec![dead_target],
            ..MockView::default()
        };

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::WoundsChanged,
            &ExhaustionBaseline {
                wound_count: 1,
                ..ExhaustionBaseline::default()
            },
            &view,
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::HostilesChanged,
            &ExhaustionBaseline {
                hostile_count: 2,
                ..ExhaustionBaseline::default()
            },
            &view,
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::TargetDead(dead_target),
            &ExhaustionBaseline::default(),
            &view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_need_band_detects_boundary_crossing_without_fixed_delta() {
        let agent = entity(1);
        let band = DriveThresholds::default().fatigue;
        let baseline_needs = HomeostaticNeeds::new(pm(100), pm(100), pm(250), pm(100), pm(100));
        let crossed_view = MockView {
            needs: vec![(
                agent,
                HomeostaticNeeds::new(pm(100), pm(100), pm(310), pm(100), pm(100)),
            )],
            ..MockView::default()
        };
        let condition = ExhaustionInvalidationCondition::NeedChangedBands {
            need: HomeostaticNeedId::Fatigue,
            band,
        };
        let baseline = ExhaustionBaseline {
            needs: Some(baseline_needs),
            ..ExhaustionBaseline::default()
        };

        assert!(condition_changed(
            &condition,
            &baseline,
            &crossed_view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_need_band_ignores_large_within_band_drift() {
        let agent = entity(1);
        let band = DriveThresholds::default().fatigue;
        let baseline_needs = HomeostaticNeeds::new(pm(100), pm(100), pm(410), pm(100), pm(100));
        let same_band_view = MockView {
            needs: vec![(
                agent,
                HomeostaticNeeds::new(pm(100), pm(100), pm(520), pm(100), pm(100)),
            )],
            ..MockView::default()
        };
        let condition = ExhaustionInvalidationCondition::NeedChangedBands {
            need: HomeostaticNeedId::Fatigue,
            band,
        };
        let baseline = ExhaustionBaseline {
            needs: Some(baseline_needs),
            ..ExhaustionBaseline::default()
        };

        assert!(!condition_changed(
            &condition,
            &baseline,
            &same_band_view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_need_band_is_conservative_when_baseline_missing_and_stable_when_current_missing(
    ) {
        let agent = entity(1);
        let condition = ExhaustionInvalidationCondition::NeedChangedBands {
            need: HomeostaticNeedId::Fatigue,
            band: DriveThresholds::default().fatigue,
        };

        assert!(condition_changed(
            &condition,
            &ExhaustionBaseline::default(),
            &MockView {
                needs: vec![(
                    agent,
                    HomeostaticNeeds::new(pm(100), pm(100), pm(500), pm(100), pm(100))
                )],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));
        assert!(!condition_changed(
            &condition,
            &ExhaustionBaseline {
                needs: Some(HomeostaticNeeds::new(
                    pm(100),
                    pm(100),
                    pm(500),
                    pm(100),
                    pm(100),
                )),
                ..ExhaustionBaseline::default()
            },
            &MockView::default(),
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn condition_changed_facility_and_blocker_branches_forward_runtime_flags() {
        let agent = entity(1);
        let baseline = ExhaustionBaseline::default();
        let view = MockView::default();

        assert!(condition_changed(
            &ExhaustionInvalidationCondition::FacilitiesChanged,
            &baseline,
            &view,
            agent,
            false,
            true,
            false,
        ));
        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::FacilitiesChanged,
            &baseline,
            &view,
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::BlockerExpired,
            &baseline,
            &view,
            agent,
            false,
            false,
            true,
        ));
        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::BlockerExpired,
            &baseline,
            &view,
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn derive_invalidation_conditions_covers_every_live_goalkind_variant() {
        let agent = entity(1);
        let target = entity(2);
        let office = entity(3);
        let destination = entity(4);
        let crime_register = entity(5);
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(recipe_definition("Bake Bread"));
        let view = MockView::default();
        let goals = vec![
            GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: worldwake_core::CommodityPurpose::SelfConsume,
            },
            GoalKind::Sleep,
            GoalKind::Relieve,
            GoalKind::Wash,
            GoalKind::EngageHostile { target },
            GoalKind::ReduceDanger,
            GoalKind::TreatWounds { patient: target },
            GoalKind::ProduceCommodity { recipe_id },
            GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::RestockCommodity {
                commodity: CommodityKind::Bread,
            },
            GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            },
            GoalKind::LootCorpse { corpse: target },
            GoalKind::BuryCorpse {
                corpse: target,
                burial_site: destination,
            },
            GoalKind::ShareBelief {
                listener: target,
                topic: TellTopic::EntityBelief { subject: office },
            },
            GoalKind::ClaimOffice { office },
            GoalKind::SupportCandidateForOffice {
                office,
                candidate: target,
            },
            GoalKind::InvestigateViolation {
                violation_id: ViolationId(1),
                place: destination,
            },
            GoalKind::StealItem {
                target_item: target,
            },
            GoalKind::Accuse {
                crime_register,
                accused: target,
                violation_id: ViolationId(2),
            },
            GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(3),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(5),
                },
            },
        ];

        assert_eq!(goals.len(), 21);
        for goal in goals {
            let (conditions, _) = derive_invalidation_conditions(&goal, agent, &view, &recipes);
            assert!(
                !conditions.is_empty(),
                "goal {goal:?} should derive at least one invalidation condition"
            );
        }
    }

    #[test]
    fn consume_owned_commodity_derives_only_matching_commodity_condition() {
        let goal = GoalKind::ConsumeOwnedCommodity {
            commodity: CommodityKind::Bread,
        };

        let (conditions, baseline) = derive_invalidation_conditions(
            &goal,
            entity(1),
            &MockView::default(),
            &RecipeRegistry::new(),
        );

        assert_eq!(
            conditions,
            vec![ExhaustionInvalidationCondition::CommodityChanged(
                CommodityKind::Bread
            )]
        );
        assert_eq!(
            baseline.commodity_quantities,
            vec![(CommodityKind::Bread, Quantity(0))]
        );
    }

    #[test]
    fn acquire_restock_includes_coin_but_self_consume_does_not() {
        let agent = entity(1);
        let view = MockView {
            commodity_quantities: vec![
                ((agent, CommodityKind::Apple), Quantity(2)),
                ((agent, CommodityKind::Coin), Quantity(9)),
            ],
            ..MockView::default()
        };
        let restock = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: worldwake_core::CommodityPurpose::Restock,
        };
        let self_consume = GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
        };

        let (restock_conditions, restock_baseline) =
            derive_invalidation_conditions(&restock, agent, &view, &RecipeRegistry::new());
        let (self_consume_conditions, self_consume_baseline) =
            derive_invalidation_conditions(&self_consume, agent, &view, &RecipeRegistry::new());

        assert_has_condition(
            &restock_conditions,
            &ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Coin),
        );
        assert!(!self_consume_conditions.contains(
            &ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Coin)
        ));
        assert_eq!(
            restock_baseline.commodity_quantities,
            vec![
                (CommodityKind::Apple, Quantity(2)),
                (CommodityKind::Coin, Quantity(9)),
            ]
        );
        assert_eq!(
            self_consume_baseline.commodity_quantities,
            vec![(CommodityKind::Apple, Quantity(2))]
        );
    }

    #[test]
    fn produce_commodity_derives_per_input_conditions_from_recipe_registry() {
        let agent = entity(1);
        let mut recipes = RecipeRegistry::new();
        let recipe_id = recipes.register(recipe_definition("Bake Bread"));
        let view = MockView {
            commodity_quantities: vec![
                ((agent, CommodityKind::Grain), Quantity(4)),
                ((agent, CommodityKind::Water), Quantity(1)),
            ],
            ..MockView::default()
        };

        let (conditions, baseline) = derive_invalidation_conditions(
            &GoalKind::ProduceCommodity { recipe_id },
            agent,
            &view,
            &recipes,
        );

        assert_has_condition(
            &conditions,
            &ExhaustionInvalidationCondition::PositionChanged,
        );
        assert_has_condition(
            &conditions,
            &ExhaustionInvalidationCondition::FacilitiesChanged,
        );
        assert_has_condition(
            &conditions,
            &ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Grain),
        );
        assert_has_condition(
            &conditions,
            &ExhaustionInvalidationCondition::CommodityChanged(CommodityKind::Water),
        );
        assert_eq!(
            baseline.commodity_quantities,
            vec![
                (CommodityKind::Grain, Quantity(4)),
                (CommodityKind::Water, Quantity(1)),
            ]
        );
    }

    #[test]
    fn produce_commodity_without_registered_recipe_keeps_non_recipe_conditions() {
        let goal = GoalKind::ProduceCommodity {
            recipe_id: worldwake_core::RecipeId(99),
        };

        let (conditions, baseline) = derive_invalidation_conditions(
            &goal,
            entity(1),
            &MockView::default(),
            &RecipeRegistry::new(),
        );

        assert_eq!(
            conditions,
            vec![
                ExhaustionInvalidationCondition::PositionChanged,
                ExhaustionInvalidationCondition::FacilitiesChanged,
            ]
        );
        assert!(baseline.commodity_quantities.is_empty());
    }

    #[test]
    fn sleep_relieve_and_wash_use_live_threshold_bands() {
        let sleep = GoalKind::Sleep;
        let relieve = GoalKind::Relieve;
        let wash = GoalKind::Wash;
        let view = MockView {
            drive_thresholds: vec![(
                entity(1),
                DriveThresholds::new(
                    ThresholdBand::new(pm(250), pm(500), pm(700), pm(900)).unwrap(),
                    ThresholdBand::new(pm(200), pm(450), pm(700), pm(850)).unwrap(),
                    ThresholdBand::new(pm(350), pm(600), pm(780), pm(930)).unwrap(),
                    ThresholdBand::new(pm(450), pm(650), pm(820), pm(940)).unwrap(),
                    ThresholdBand::new(pm(500), pm(700), pm(860), pm(960)).unwrap(),
                    ThresholdBand::new(pm(150), pm(350), pm(600), pm(850)).unwrap(),
                    ThresholdBand::new(pm(100), pm(300), pm(550), pm(800)).unwrap(),
                ),
            )],
            ..MockView::default()
        };

        let (sleep_conditions, _) =
            derive_invalidation_conditions(&sleep, entity(1), &view, &RecipeRegistry::new());
        let (relieve_conditions, _) =
            derive_invalidation_conditions(&relieve, entity(1), &view, &RecipeRegistry::new());
        let (wash_conditions, _) =
            derive_invalidation_conditions(&wash, entity(1), &view, &RecipeRegistry::new());

        assert_has_condition(
            &sleep_conditions,
            &ExhaustionInvalidationCondition::NeedChangedBands {
                need: HomeostaticNeedId::Fatigue,
                band: need_threshold_band(&view, entity(1), HomeostaticNeedId::Fatigue),
            },
        );
        assert_has_condition(
            &relieve_conditions,
            &ExhaustionInvalidationCondition::NeedChangedBands {
                need: HomeostaticNeedId::Bladder,
                band: need_threshold_band(&view, entity(1), HomeostaticNeedId::Bladder),
            },
        );
        assert_has_condition(
            &wash_conditions,
            &ExhaustionInvalidationCondition::NeedChangedBands {
                need: HomeostaticNeedId::Dirtiness,
                band: need_threshold_band(&view, entity(1), HomeostaticNeedId::Dirtiness),
            },
        );
    }

    #[test]
    fn classify_need_band_distinguishes_below_low_from_critical() {
        let band = DriveThresholds::default().fatigue;

        assert_eq!(classify_need_band(pm(0), band), 0);
        assert_eq!(classify_need_band(band.low(), band), 1);
        assert_eq!(classify_need_band(band.medium(), band), 2);
        assert_eq!(classify_need_band(band.high(), band), 3);
        assert_eq!(classify_need_band(band.critical(), band), 4);
    }

    #[test]
    fn engage_hostile_tracks_target_death() {
        let target = entity(2);
        let (conditions, _) = derive_invalidation_conditions(
            &GoalKind::EngageHostile { target },
            entity(1),
            &MockView::default(),
            &RecipeRegistry::new(),
        );

        assert_has_condition(
            &conditions,
            &ExhaustionInvalidationCondition::TargetDead(target),
        );
    }

    #[test]
    fn steal_item_derives_target_state_condition_instead_of_target_dead() {
        let agent = entity(1);
        let target = entity(2);
        let owner = entity(3);
        let place = entity(4);
        let view = MockView {
            entity_kinds: vec![(target, EntityKind::ItemLot)],
            effective_places: vec![(agent, place), (target, place)],
            owners: vec![(target, owner)],
            carry_capacities: vec![(agent, LoadUnits(10))],
            loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(4))],
            ..MockView::default()
        };

        let (conditions, baseline) = derive_invalidation_conditions(
            &GoalKind::StealItem {
                target_item: target,
            },
            agent,
            &view,
            &RecipeRegistry::new(),
        );

        assert_eq!(
            conditions,
            vec![
                ExhaustionInvalidationCondition::PositionChanged,
                ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            ]
        );
        assert_eq!(
            baseline.steal_target_states,
            vec![(
                target,
                StealTargetSnapshot {
                    effective_place: Some(place),
                    direct_possessor: None,
                    direct_container: None,
                    access_state: StealTargetAccessState::Stealable,
                    fits_carry_capacity: true,
                    is_item_lot: true,
                },
            )]
        );
    }

    #[test]
    fn derive_invalidation_conditions_snapshots_relevant_baseline_state_deterministically() {
        let agent = entity(1);
        let place = entity(9);
        let attacker = entity(10);
        let hostile_a = entity(11);
        let hostile_b = entity(12);
        let needs = HomeostaticNeeds::new(pm(100), pm(200), pm(300), pm(400), pm(500));
        let view = MockView {
            effective_places: vec![(agent, place)],
            commodity_quantities: vec![((agent, CommodityKind::Medicine), Quantity(3))],
            needs: vec![(agent, needs)],
            wounds: vec![(agent, vec![wound(1, attacker), wound(2, attacker)])],
            visible_hostiles: vec![(agent, vec![hostile_a, hostile_b])],
            ..MockView::default()
        };
        let goal = GoalKind::TreatWounds {
            patient: entity(20),
        };

        let first = derive_invalidation_conditions(&goal, agent, &view, &RecipeRegistry::new());
        let second = derive_invalidation_conditions(&goal, agent, &view, &RecipeRegistry::new());

        assert_eq!(first, second);
        assert_eq!(
            first.1,
            ExhaustionBaseline {
                position: Some(place),
                needs: Some(needs),
                commodity_quantities: vec![(CommodityKind::Medicine, Quantity(3))],
                unique_item_counts: Vec::new(),
                steal_target_states: Vec::new(),
                wound_count: 2,
                hostile_count: 2,
            }
        );
    }

    #[test]
    fn shared_invalidation_strategies_preserve_identical_conditions_for_shared_families() {
        let agent = entity(1);
        let target = entity(2);
        let office = entity(3);
        let destination = entity(4);

        let loot = derive_invalidation_conditions(
            &GoalKind::LootCorpse { corpse: target },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );
        let bury = derive_invalidation_conditions(
            &GoalKind::BuryCorpse {
                corpse: target,
                burial_site: destination,
            },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );
        let sell = derive_invalidation_conditions(
            &GoalKind::SellCommodity {
                commodity: CommodityKind::Bread,
            },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );
        let move_cargo = derive_invalidation_conditions(
            &GoalKind::MoveCargo {
                commodity: CommodityKind::Bread,
                destination,
            },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );
        let punish_fine = derive_invalidation_conditions(
            &GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(5),
                punishment: PunishmentKind::Fine {
                    commodity: CommodityKind::Coin,
                    amount: Quantity(7),
                },
            },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );
        let punish_exile = derive_invalidation_conditions(
            &GoalKind::PunishAccused {
                office,
                accused: target,
                accusation_entry: RecordEntryId(5),
                punishment: PunishmentKind::Exile {
                    from_faction: destination,
                },
            },
            agent,
            &MockView::default(),
            &RecipeRegistry::new(),
        );

        assert_eq!(loot, bury);
        assert_eq!(sell, move_cargo);
        assert_eq!(punish_fine, punish_exile);
    }

    #[test]
    fn condition_changed_steal_target_detects_control_and_possession_delta() {
        let agent = entity(1);
        let target = entity(2);
        let owner = entity(3);
        let other = entity(4);
        let container = entity(6);
        let place = entity(5);
        let baseline = ExhaustionBaseline {
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
        };
        let unchanged = MockView {
            entity_kinds: vec![(target, EntityKind::ItemLot)],
            effective_places: vec![(agent, place), (target, place)],
            owners: vec![(target, owner)],
            carry_capacities: vec![(agent, LoadUnits(10))],
            loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(3))],
            ..MockView::default()
        };
        let lawfully_controllable = MockView {
            controllable: vec![(agent, target)],
            ..unchanged
        };

        assert!(!condition_changed(
            &ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            &baseline,
            &MockView {
                entity_kinds: vec![(target, EntityKind::ItemLot)],
                effective_places: vec![(agent, place), (target, place)],
                owners: vec![(target, owner)],
                carry_capacities: vec![(agent, LoadUnits(10))],
                loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(3))],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            &baseline,
            &lawfully_controllable,
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            &baseline,
            &MockView {
                entity_kinds: vec![(target, EntityKind::ItemLot)],
                effective_places: vec![(agent, place), (target, place)],
                owners: vec![(target, owner)],
                direct_possessors: vec![(target, other)],
                carry_capacities: vec![(agent, LoadUnits(10))],
                loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(3))],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            &baseline,
            &MockView {
                entity_kinds: vec![(target, EntityKind::ItemLot)],
                effective_places: vec![(agent, place), (target, place)],
                owners: vec![(target, owner)],
                direct_containers: vec![(target, container)],
                carry_capacities: vec![(agent, LoadUnits(10))],
                loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(3))],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));
        assert!(condition_changed(
            &ExhaustionInvalidationCondition::StealTargetStateChanged(target),
            &baseline,
            &MockView {
                entity_kinds: vec![(target, EntityKind::ItemLot)],
                effective_places: vec![(agent, place), (target, place)],
                owners: vec![(target, owner)],
                carry_capacities: vec![(agent, LoadUnits(2))],
                loads: vec![(agent, LoadUnits(0)), (target, LoadUnits(3))],
                ..MockView::default()
            },
            agent,
            false,
            false,
            false,
        ));
    }

    #[test]
    fn invalidate_exhausted_goals_removes_only_entries_with_fired_conditions() {
        let agent = entity(1);
        let bread_goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: CommodityKind::Bread,
            }),
            anchor: worldwake_core::OpportunityAnchor::Place(entity(20)),
        };
        let sleep_goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::Sleep),
            anchor: worldwake_core::OpportunityAnchor::Place(entity(21)),
        };
        let legacy_goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ReduceDanger),
            anchor: worldwake_core::OpportunityAnchor::None,
        };
        let mut cache = BTreeMap::from([
            (
                bread_goal,
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
                    baseline: ExhaustionBaseline {
                        position: Some(entity(10)),
                        ..ExhaustionBaseline::default()
                    },
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
            (
                sleep_goal,
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: vec![
                        ExhaustionInvalidationCondition::FacilitiesChanged,
                    ],
                    baseline: ExhaustionBaseline::default(),
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
            (
                legacy_goal,
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: Vec::new(),
                    baseline: ExhaustionBaseline::default(),
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
        ]);
        let view = MockView {
            effective_places: vec![(agent, entity(11))],
            ..MockView::default()
        };

        invalidate_exhausted_goals(&mut cache, &view, agent, false, false, false);

        assert!(!cache.contains_key(&bread_goal));
        assert_eq!(
            cache.get(&sleep_goal),
            Some(&ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: vec![ExhaustionInvalidationCondition::FacilitiesChanged],
                baseline: ExhaustionBaseline::default(),
                next_retry_tick: None,
                consecutive_failures: 0,
            })
        );
        assert!(!cache.contains_key(&legacy_goal));
    }

    #[test]
    fn invalidate_exhausted_goals_clears_only_matching_opportunity_for_same_goal() {
        let agent = entity(1);
        let shared_goal = GoalKey::from(GoalKind::AcquireCommodity {
            commodity: CommodityKind::Apple,
            purpose: worldwake_core::CommodityPurpose::SelfConsume,
        });
        let orchard = OpportunityKey {
            goal_key: shared_goal,
            anchor: worldwake_core::OpportunityAnchor::Place(entity(20)),
        };
        let market = OpportunityKey {
            goal_key: shared_goal,
            anchor: worldwake_core::OpportunityAnchor::Place(entity(21)),
        };
        let mut cache = BTreeMap::from([
            (
                orchard,
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
                    baseline: ExhaustionBaseline {
                        position: Some(entity(10)),
                        ..ExhaustionBaseline::default()
                    },
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
            (
                market,
                ExhaustionEntry {
                    retry_state: ExhaustionRetryState::FrontierExhausted,
                    invalidation_conditions: vec![
                        ExhaustionInvalidationCondition::FacilitiesChanged,
                    ],
                    baseline: ExhaustionBaseline::default(),
                    next_retry_tick: None,
                    consecutive_failures: 0,
                },
            ),
        ]);
        let view = MockView {
            effective_places: vec![(agent, entity(11))],
            ..MockView::default()
        };

        invalidate_exhausted_goals(&mut cache, &view, agent, false, false, false);

        assert!(
            !cache.contains_key(&orchard),
            "position invalidation should clear only the orchard opportunity"
        );
        assert!(
            cache.contains_key(&market),
            "a sibling opportunity for the same GoalKey should remain cached when its conditions did not fire"
        );
    }

    #[test]
    fn invalidate_exhausted_goals_preserves_position_entries_while_still_in_transit() {
        let agent = entity(1);
        let goal = OpportunityKey {
            goal_key: GoalKey::from(GoalKind::ReduceDanger),
            anchor: worldwake_core::OpportunityAnchor::None,
        };
        let mut cache = BTreeMap::from([(
            goal,
            ExhaustionEntry {
                retry_state: ExhaustionRetryState::FrontierExhausted,
                invalidation_conditions: vec![ExhaustionInvalidationCondition::PositionChanged],
                baseline: ExhaustionBaseline {
                    position: Some(entity(2)),
                    ..ExhaustionBaseline::default()
                },
                next_retry_tick: None,
                consecutive_failures: 0,
            },
        )]);
        let view = MockView {
            effective_places: vec![(agent, entity(3))],
            ..MockView::default()
        };

        invalidate_exhausted_goals(&mut cache, &view, agent, true, false, false);

        assert!(cache.contains_key(&goal));
    }
}
