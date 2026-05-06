use crate::{
    AgendaEntry, AgendaEntryKey, AgendaPhase, AgendaState, GoalKind, GoalOffer, KillCondition,
    RevivalTrigger, enterprise::restock_gap_at_destination, goal_switching::compare_goal_switch,
    ranking,
};
use std::collections::{BTreeMap, BTreeSet};
use worldwake_core::{
    AgendaProfile, BlockerKey, CommodityKind, Discrepancy, DiscrepancyMemory, EntityId,
    ExpectationId, ExpectationState, OpportunityAnchor, Permille, Quantity, Tick,
};
use worldwake_sim::GoalBeliefView;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgendaTransitions {
    pub killed: Vec<AgendaEntry>,
    pub revived: Vec<AgendaEntryKey>,
    pub commit_transition: CommitTransition,
    pub demoted_to_suspended: Vec<AgendaEntryKey>,
    pub demoted_to_pending: Vec<AgendaEntryKey>,
    pub ordered_candidates: Vec<AgendaEntry>,
}

#[derive(Clone, Copy, Debug)]
pub struct AgendaTickPolicy<'a> {
    pub profile: &'a AgendaProfile,
    pub switch_margin: Permille,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommitTransition {
    Unchanged,
    Committed {
        new_key: AgendaEntryKey,
        previous_key: Option<AgendaEntryKey>,
    },
    Cleared {
        previous_key: AgendaEntryKey,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum RejectionLifecycle {
    Satisfied,
    InfeasibleUntil { trigger: RevivalTrigger },
    Dead,
}

pub(crate) fn classify_rejection(
    actor: EntityId,
    probe_verdict: &crate::agent_tick::portfolio::FeasibilityVerdict,
    offer: &GoalOffer,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
    revive_cooldown_ticks: u32,
) -> RejectionLifecycle {
    if goal_post_conditions_already_satisfied(actor, offer, beliefs) {
        return RejectionLifecycle::Satisfied;
    }

    let reason = match probe_verdict {
        crate::agent_tick::portfolio::FeasibilityVerdict::RejectedBeforeSearch { reason } => {
            *reason
        }
        crate::agent_tick::portfolio::FeasibilityVerdict::Plausible => {
            unreachable!("classify_rejection only accepts rejected probe verdicts")
        }
    };

    match reason {
        Discrepancy::MissingObservation | Discrepancy::Omission(_) => {
            let place = offer_anchor_place(offer).or_else(|| {
                offer_anchor_entity(offer).and_then(|entity| beliefs.effective_place(entity))
            });
            if let Some(kind) = offer_commodity(offer) {
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::CommodityAvailable {
                        place: place.expect("commodity revival trigger needs a place"),
                        kind,
                        min: Quantity(1),
                    },
                }
            } else {
                let target = offer_anchor_entity(offer)
                    .or(offer.key.entity)
                    .expect("target-present revival trigger needs a target");
                RejectionLifecycle::InfeasibleUntil {
                    trigger: RevivalTrigger::TargetPresent {
                        target,
                        place: place
                            .or_else(|| beliefs.effective_place(target))
                            .expect("target-present revival trigger needs a place"),
                    },
                }
            }
        }
        Discrepancy::RouteUnknown => RejectionLifecycle::InfeasibleUntil {
            trigger: RevivalTrigger::RouteLearned {
                from: beliefs
                    .effective_place(actor)
                    .expect("route-learned revival trigger needs actor place"),
                to: offer_anchor_place(offer)
                    .or_else(|| {
                        offer_anchor_entity(offer)
                            .and_then(|entity| beliefs.effective_place(entity))
                    })
                    .expect("route-learned revival trigger needs destination place"),
            },
        },
        Discrepancy::PartialExecutionDrift
        | Discrepancy::BeliefStale
        | Discrepancy::SourceInvalidated
        | Discrepancy::SearchBudgetExhausted
        | Discrepancy::NeedHorizonExceeded { .. } => RejectionLifecycle::InfeasibleUntil {
            trigger: RevivalTrigger::TickElapsed {
                at_tick: Tick(tick.0.saturating_add(u64::from(revive_cooldown_ticks))),
            },
        },
        Discrepancy::NoWillingCounterparty => {
            let counterparty = offer_anchor_entity(offer)
                .or(offer.key.entity)
                .expect("counterparty revival trigger needs counterparty entity");
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CounterpartyAvailable {
                    counterparty,
                    place: offer_anchor_place(offer)
                        .or_else(|| beliefs.effective_place(counterparty))
                        .expect("counterparty revival trigger needs a place"),
                },
            }
        }
        Discrepancy::BeliefContradicted
        | Discrepancy::NoLegalBinding
        | Discrepancy::ImproperPlanningState => RejectionLifecycle::Dead,
    }
}

pub fn tick_agenda(
    actor: EntityId,
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    policy: AgendaTickPolicy<'_>,
    tick: Tick,
) -> AgendaTransitions {
    let previous_committed = state
        .committed
        .as_ref()
        .filter(|entry| entry.phase == AgendaPhase::Committed)
        .map(|entry| entry.key);
    let killed = drain_killed(actor, state, beliefs, tick);
    let revived_entries = promote_revived(
        actor,
        state,
        beliefs,
        discrepancy_memory,
        policy.profile,
        tick,
    );
    let revived = revived_entries.iter().map(|entry| entry.key).collect();
    let merged = merge_candidates(state, fresh_candidates, tick);
    let ranked = rank_for_commit(state, merged, revived_entries);
    let commit_transition = commit_or_keep(
        state,
        ranked.as_slice(),
        previous_committed,
        policy.switch_margin,
        tick,
    );
    let ordered_candidates = ranked.clone();
    let (demoted_to_pending, demoted_to_suspended) =
        demote_to_pending_or_suspended(state, ranked, policy.profile, tick);

    AgendaTransitions {
        killed,
        revived,
        commit_transition,
        demoted_to_suspended,
        demoted_to_pending,
        ordered_candidates,
    }
}

fn drain_killed(
    actor: EntityId,
    state: &mut AgendaState,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut killed = Vec::new();

    if state
        .committed
        .as_ref()
        .is_some_and(|entry| should_kill(actor, &entry.kill_condition, beliefs, tick))
        && let Some(entry) = state.committed.take()
    {
        killed.push(entry);
    }

    drain_killed_map(actor, &mut state.pending, beliefs, tick, &mut killed);
    drain_killed_map(actor, &mut state.suspended, beliefs, tick, &mut killed);
    killed
}

fn drain_killed_map(
    actor: EntityId,
    entries: &mut BTreeMap<AgendaEntryKey, AgendaEntry>,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
    killed: &mut Vec<AgendaEntry>,
) {
    let keys: Vec<_> = entries
        .iter()
        .filter_map(|(key, entry)| {
            should_kill(actor, &entry.kill_condition, beliefs, tick).then_some(*key)
        })
        .collect();
    for key in keys {
        if let Some(entry) = entries.remove(&key) {
            killed.push(entry);
        }
    }
}

fn promote_revived(
    actor: EntityId,
    state: &mut AgendaState,
    beliefs: &impl GoalBeliefView,
    discrepancy_memory: &DiscrepancyMemory,
    profile: &AgendaProfile,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut revived = Vec::new();
    let keys: Vec<_> = state.pending.keys().copied().collect();

    for key in keys {
        let Some(entry) = state.pending.get(&key) else {
            continue;
        };
        let cooldown_ready = tick.0
            >= entry
                .last_reconsidered_tick
                .0
                .saturating_add(u64::from(profile.revive_cooldown_ticks));
        if !cooldown_ready || discrepancy_memory.is_suppressed(&blocker_key_from(entry), tick) {
            continue;
        }
        let Some(trigger) = entry.revival_trigger.as_ref() else {
            continue;
        };
        if !evaluate_revival_trigger(actor, trigger, beliefs, tick) {
            continue;
        }

        let mut revived_entry = state
            .pending
            .remove(&key)
            .expect("pending key disappeared during revival pass");
        revived_entry.last_reconsidered_tick = tick;
        revived_entry.phase = AgendaPhase::Pending;
        revived.push(revived_entry);
    }

    revived
}

fn merge_candidates(
    state: &mut AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    tick: Tick,
) -> Vec<AgendaEntry> {
    let mut deduped = BTreeMap::new();
    for candidate in fresh_candidates {
        deduped.insert(candidate.key, candidate);
    }

    let mut merged = Vec::with_capacity(deduped.len());
    for (key, candidate) in deduped {
        if let Some(committed) = state.committed.as_mut()
            && committed.key == key
        {
            refresh_entry(committed, &candidate, tick);
            committed.phase = AgendaPhase::Committed;
            continue;
        }

        if let Some(mut existing) = state.pending.remove(&key) {
            refresh_entry(&mut existing, &candidate, tick);
            existing.phase = AgendaPhase::Pending;
            merged.push(existing);
            continue;
        }

        if let Some(mut existing) = state.suspended.remove(&key) {
            refresh_entry(&mut existing, &candidate, tick);
            existing.phase = AgendaPhase::Pending;
            merged.push(existing);
            continue;
        }

        let mut candidate = candidate;
        candidate.phase = AgendaPhase::Pending;
        candidate.last_reconsidered_tick = tick;
        merged.push(candidate);
    }

    merged
}

fn refresh_entry(existing: &mut AgendaEntry, candidate: &AgendaEntry, tick: Tick) {
    existing.offer = candidate.offer.clone();
    existing.priority_class = candidate.priority_class;
    existing.motive_score = candidate.motive_score;
    existing.provenance.clone_from(&candidate.provenance);
    existing
        .source_reliability_discount
        .clone_from(&candidate.source_reliability_discount);
    existing
        .competition_discount
        .clone_from(&candidate.competition_discount);
    existing.feasibility = candidate.feasibility;
    existing.last_reconsidered_tick = tick;
}

fn rank_for_commit(
    state: &AgendaState,
    fresh_candidates: Vec<AgendaEntry>,
    revived: Vec<AgendaEntry>,
) -> Vec<AgendaEntry> {
    let mut pool = Vec::new();
    if let Some(committed) = state.committed.clone() {
        pool.push(committed);
    }
    pool.extend(revived);
    pool.extend(fresh_candidates);
    let _ = ranking::sort_in_place(&mut pool);
    pool
}

fn commit_or_keep(
    state: &mut AgendaState,
    ranked: &[AgendaEntry],
    previous_committed: Option<AgendaEntryKey>,
    switch_margin: Permille,
    tick: Tick,
) -> CommitTransition {
    let Some(top) = ranked.first() else {
        state.committed = None;
        return previous_committed.map_or(CommitTransition::Unchanged, |previous_key| {
            CommitTransition::Cleared { previous_key }
        });
    };

    let Some(current) = state.committed.as_ref() else {
        state.committed = Some(AgendaEntry::committed_from(top, tick));
        return CommitTransition::Committed {
            new_key: top.key,
            previous_key: previous_committed,
        };
    };

    if current.phase != AgendaPhase::Committed {
        return CommitTransition::Unchanged;
    }

    if current.key == top.key {
        let mut refreshed = top.clone();
        refreshed.phase = AgendaPhase::Committed;
        refreshed.introduced_tick = current.introduced_tick;
        refreshed.last_reconsidered_tick = tick;
        state.committed = Some(refreshed);
        return CommitTransition::Unchanged;
    }

    let should_switch = compare_goal_switch(
        current.priority_class,
        Some(current.motive_score),
        top.priority_class,
        top.motive_score,
        switch_margin,
    )
    .is_some();
    if !should_switch {
        return CommitTransition::Unchanged;
    }

    state.committed = Some(AgendaEntry::committed_from(top, tick));
    CommitTransition::Committed {
        new_key: top.key,
        previous_key: previous_committed,
    }
}

fn demote_to_pending_or_suspended(
    state: &mut AgendaState,
    ranked: Vec<AgendaEntry>,
    profile: &AgendaProfile,
    tick: Tick,
) -> (Vec<AgendaEntryKey>, Vec<AgendaEntryKey>) {
    let committed_key = state.committed.as_ref().map(|entry| entry.key);
    let mut demoted_to_pending = Vec::new();
    let demoted_to_suspended = Vec::new();
    let mut seen = BTreeSet::new();

    for mut entry in ranked {
        if Some(entry.key) == committed_key || !seen.insert(entry.key) {
            continue;
        }
        if entry.revival_trigger.is_none() && entry.introduced_tick == tick {
            continue;
        }
        entry.phase = AgendaPhase::Pending;
        entry.last_reconsidered_tick = tick;
        if entry.revival_trigger.is_none() {
            entry.revival_trigger = Some(RevivalTrigger::TickElapsed {
                at_tick: Tick(
                    tick.0
                        .saturating_add(u64::from(profile.revive_cooldown_ticks.max(1))),
                ),
            });
        }
        let key = entry.key;
        insert_with_capacity(&mut state.pending, profile.pending_capacity, entry);
        demoted_to_pending.push(key);
    }

    (demoted_to_pending, demoted_to_suspended)
}

fn insert_with_capacity(
    entries: &mut BTreeMap<AgendaEntryKey, AgendaEntry>,
    capacity: u32,
    entry: AgendaEntry,
) {
    entries.insert(entry.key, entry);
    let capacity = usize::try_from(capacity).unwrap_or(usize::MAX);
    while entries.len() > capacity {
        let evict_key = entries
            .iter()
            .min_by_key(|(key, entry)| (entry.last_reconsidered_tick, **key))
            .map(|(key, _)| *key)
            .expect("entries must be non-empty when enforcing capacity");
        entries.remove(&evict_key);
    }
}

fn evaluate_revival_trigger(
    actor: EntityId,
    trigger: &RevivalTrigger,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> bool {
    match trigger {
        RevivalTrigger::CommodityAvailable { place, kind, min } => {
            beliefs.controlled_commodity_quantity_at_place(actor, *place, *kind) >= *min
        }
        RevivalTrigger::TargetPresent { target, place } => {
            beliefs.effective_place(*target) == Some(*place) && beliefs.is_alive(*target)
        }
        RevivalTrigger::RouteLearned { from, to } => {
            *from == *to
                || beliefs
                    .adjacent_places_with_travel_ticks(*from)
                    .iter()
                    .any(|(neighbor, _)| neighbor == to)
        }
        RevivalTrigger::CounterpartyAvailable {
            counterparty,
            place,
        } => {
            beliefs.effective_place(*counterparty) == Some(*place)
                && beliefs.is_alive(*counterparty)
                && !beliefs.is_incapacitated(*counterparty)
        }
        RevivalTrigger::TickElapsed { at_tick } => tick >= *at_tick,
    }
}

fn should_kill(
    actor: EntityId,
    condition: &KillCondition,
    beliefs: &impl GoalBeliefView,
    tick: Tick,
) -> bool {
    match condition {
        KillCondition::TickExpiry { at_tick } => tick >= *at_tick,
        KillCondition::ObligationResolved { expectation } => {
            expectation_is_resolved(actor, *expectation, beliefs)
        }
        KillCondition::TargetDead { target } => {
            beliefs.is_dead(*target) || !beliefs.is_alive(*target)
        }
        KillCondition::External => false,
    }
}

fn expectation_is_resolved(
    actor: EntityId,
    expectation: ExpectationId,
    beliefs: &impl GoalBeliefView,
) -> bool {
    beliefs
        .expectation_store(actor)
        .and_then(|store| store.records.get(&expectation).copied())
        .is_none_or(|record| {
            !matches!(
                record.state,
                ExpectationState::Active | ExpectationState::Overdue
            )
        })
}

pub(crate) fn goal_post_conditions_already_satisfied(
    actor: EntityId,
    offer: &GoalOffer,
    beliefs: &impl GoalBeliefView,
) -> bool {
    match offer.key.kind {
        GoalKind::MoveCargo {
            commodity,
            destination,
        } => {
            let destination_place = beliefs.effective_place(destination).unwrap_or(destination);
            (beliefs.effective_place(actor) == Some(destination_place)
                && beliefs.commodity_quantity(actor, commodity) > Quantity(0))
                || restock_gap_at_destination(beliefs, actor, destination, commodity).is_none()
        }
        _ => false,
    }
}

fn offer_commodity(offer: &GoalOffer) -> Option<CommodityKind> {
    match offer.key.kind {
        GoalKind::AcquireCommodity { commodity, .. }
        | GoalKind::ConsumeOwnedCommodity { commodity }
        | GoalKind::RestockCommodity { commodity }
        | GoalKind::MoveCargo { commodity, .. }
        | GoalKind::SellCommodity { commodity } => Some(commodity),
        _ => None,
    }
}

fn offer_anchor_place(offer: &GoalOffer) -> Option<EntityId> {
    anchor_place(offer.anchor).or(offer.key.place)
}

fn offer_anchor_entity(offer: &GoalOffer) -> Option<EntityId> {
    anchor_entity(offer.anchor).or(offer.key.entity)
}

fn blocker_key_from(entry: &AgendaEntry) -> BlockerKey {
    BlockerKey {
        goal_key: entry.offer.key,
        place: anchor_place(entry.offer.anchor).or(entry.offer.key.place),
        target: anchor_entity(entry.offer.anchor).or(entry.offer.key.entity),
        action_def: None,
    }
}

fn anchor_place(anchor: OpportunityAnchor) -> Option<EntityId> {
    match anchor {
        OpportunityAnchor::Place(place) => Some(place),
        OpportunityAnchor::Entity(_) | OpportunityAnchor::None => None,
    }
}

fn anchor_entity(anchor: OpportunityAnchor) -> Option<EntityId> {
    match anchor {
        OpportunityAnchor::Entity(entity) => Some(entity),
        OpportunityAnchor::Place(_) | OpportunityAnchor::None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AgendaTickPolicy, AgendaTransitions, CommitTransition, RejectionLifecycle,
        classify_rejection, tick_agenda,
    };
    use crate::agent_tick::portfolio::FeasibilityVerdict;
    use crate::{
        AgendaEntry, AgendaOrigin, AgendaPhase, AgendaState, FeasibilityHint, GoalKey, GoalKind,
        GoalOffer, GoalPriorityClass, KillCondition, RevivalTrigger,
    };
    use std::collections::{BTreeMap, BTreeSet};
    use std::num::NonZeroU32;
    use worldwake_core::{
        AcquisitionQuantity, AgendaProfile, BeliefConfidencePolicy, CommodityConsumableProfile,
        CommodityKind, DemandObservation, DemandObservationReason, Discrepancy, DriveThresholds,
        EntityId, EntityKind, ExpectationBasis, ExpectationId, ExpectationRecord, ExpectationState,
        ExpectationStore, LoadUnits, MerchandiseProfile, OpportunityAnchor, OpportunityKey,
        Quantity, ResourceSource, Tick, UniqueItemKind, WorkstationTag,
    };
    use worldwake_sim::GoalBeliefView;

    const AGENT: EntityId = EntityId {
        slot: 1,
        generation: 0,
    };
    const PLACE: EntityId = EntityId {
        slot: 2,
        generation: 0,
    };
    const TARGET: EntityId = EntityId {
        slot: 3,
        generation: 0,
    };

    #[derive(Clone, Default)]
    struct MockGoalBeliefView {
        controlled_quantities: BTreeMap<(EntityId, EntityId, CommodityKind), Quantity>,
        entity_places: BTreeMap<EntityId, EntityId>,
        dead: BTreeSet<EntityId>,
        incapacitated: BTreeSet<EntityId>,
        adjacency: BTreeMap<EntityId, Vec<(EntityId, NonZeroU32)>>,
        expectation_store: Option<ExpectationStore>,
        demand_memory: BTreeMap<EntityId, Vec<DemandObservation>>,
    }

    impl GoalBeliefView for MockGoalBeliefView {
        fn is_alive(&self, entity: EntityId) -> bool {
            !self.dead.contains(&entity)
        }

        fn is_dead(&self, entity: EntityId) -> bool {
            self.dead.contains(&entity)
        }

        fn entity_kind(&self, _entity: EntityId) -> Option<EntityKind> {
            None
        }

        fn effective_place(&self, entity: EntityId) -> Option<EntityId> {
            self.entity_places.get(&entity).copied()
        }

        fn entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn direct_possessions(&self, _holder: EntityId) -> Vec<EntityId> {
            Vec::new()
        }

        fn adjacent_places_with_travel_ticks(
            &self,
            place: EntityId,
        ) -> Vec<(EntityId, NonZeroU32)> {
            self.adjacency.get(&place).cloned().unwrap_or_default()
        }

        fn knows_recipe(&self, _actor: EntityId, _recipe: worldwake_core::RecipeId) -> bool {
            false
        }

        fn known_recipes(&self, _agent: EntityId) -> Vec<worldwake_core::RecipeId> {
            Vec::new()
        }

        fn unique_item_count(&self, _holder: EntityId, _kind: UniqueItemKind) -> u32 {
            0
        }

        fn commodity_quantity(&self, _holder: EntityId, _kind: CommodityKind) -> Quantity {
            Quantity(0)
        }

        fn controlled_commodity_quantity_at_place(
            &self,
            agent: EntityId,
            place: EntityId,
            commodity: CommodityKind,
        ) -> Quantity {
            self.controlled_quantities
                .get(&(agent, place, commodity))
                .copied()
                .unwrap_or(Quantity(0))
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

        fn can_control(&self, _actor: EntityId, _entity: EntityId) -> bool {
            false
        }

        fn carry_capacity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn load_of_entity(&self, _entity: EntityId) -> Option<LoadUnits> {
            None
        }

        fn is_incapacitated(&self, entity: EntityId) -> bool {
            self.incapacitated.contains(&entity)
        }

        fn has_wounds(&self, _entity: EntityId) -> bool {
            false
        }

        fn homeostatic_needs(&self, _agent: EntityId) -> Option<worldwake_core::HomeostaticNeeds> {
            None
        }

        fn drive_thresholds(&self, _agent: EntityId) -> Option<DriveThresholds> {
            None
        }

        fn belief_confidence_policy(&self, _agent: EntityId) -> BeliefConfidencePolicy {
            BeliefConfidencePolicy::default()
        }

        fn merchandise_profile(&self, _entity: EntityId) -> Option<MerchandiseProfile> {
            None
        }

        fn expectation_store(&self, _agent: EntityId) -> Option<ExpectationStore> {
            self.expectation_store.clone()
        }

        fn wounds(&self, _agent: EntityId) -> Vec<worldwake_core::Wound> {
            Vec::new()
        }

        fn hostile_targets_of(&self, _agent: EntityId) -> Vec<EntityId> {
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

        fn demand_memory(&self, agent: EntityId) -> Vec<worldwake_core::DemandObservation> {
            self.demand_memory.get(&agent).cloned().unwrap_or_default()
        }

        fn corpse_entities_at(&self, _place: EntityId) -> Vec<EntityId> {
            Vec::new()
        }
    }

    fn goal_offer(goal: GoalKind, anchor: OpportunityAnchor) -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(goal),
            anchor,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
            acquisition_quantity: None,
        }
    }

    fn agenda_entry(
        goal: GoalKind,
        anchor: OpportunityAnchor,
        motive_score: u32,
        introduced_tick: Tick,
    ) -> AgendaEntry {
        AgendaEntry {
            key: OpportunityKey {
                goal_key: GoalKey::from(goal),
                anchor,
            },
            offer: goal_offer(goal, anchor),
            phase: AgendaPhase::Pending,
            origin: AgendaOrigin::NeedDrive,
            introduced_tick,
            last_reconsidered_tick: introduced_tick,
            revival_trigger: None,
            kill_condition: KillCondition::External,
            priority_class: GoalPriorityClass::Medium,
            motive_score,
            provenance: None,
            source_reliability_discount: None,
            competition_discount: None,
            source_composite: None,
            feasibility: FeasibilityHint::Uncertain,
        }
    }

    fn default_profile() -> AgendaProfile {
        AgendaProfile {
            pending_capacity: 16,
            suspended_capacity: 8,
            revive_cooldown_ticks: 4,
        }
    }

    fn demand(place: EntityId, commodity: CommodityKind, quantity: u32) -> DemandObservation {
        DemandObservation {
            commodity,
            quantity: Quantity(quantity),
            place,
            tick: Tick(1),
            counterparty: None,
            reason: DemandObservationReason::WantedToBuyButNoSeller,
        }
    }

    fn default_switch_margin() -> worldwake_core::Permille {
        worldwake_core::Permille::new(100).unwrap()
    }

    fn classify(
        goal: GoalKind,
        anchor: OpportunityAnchor,
        reason: Discrepancy,
    ) -> RejectionLifecycle {
        classify_rejection(
            AGENT,
            &FeasibilityVerdict::RejectedBeforeSearch { reason },
            &goal_offer(goal, anchor),
            &MockGoalBeliefView::default(),
            Tick(10),
            default_profile().revive_cooldown_ticks,
        )
    }

    #[test]
    fn merge_fresh_offer_with_same_key_refreshes_without_duplicate() {
        let mut state = AgendaState::default();
        let mut committed = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Entity(TARGET),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        state.committed = Some(committed);
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(2) });
        state.pending.insert(pending.key, pending.clone());

        let fresh = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            40,
            Tick(9),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![fresh],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(state.pending.len(), 1);
        let refreshed = state
            .pending
            .get(&pending.key)
            .expect("pending entry missing");
        assert_eq!(refreshed.last_reconsidered_tick, Tick(10));
        assert_eq!(refreshed.motive_score, 40);
        assert_eq!(transitions.demoted_to_pending, vec![pending.key]);
    }

    #[test]
    fn revival_trigger_commodity_available_fires_when_belief_confirms_quantity() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::CommodityAvailable {
            place: PLACE,
            kind: CommodityKind::Bread,
            min: Quantity(3),
        });
        state.pending.insert(pending.key, pending.clone());

        let mut beliefs = MockGoalBeliefView::default();
        beliefs
            .controlled_quantities
            .insert((AGENT, PLACE, CommodityKind::Bread), Quantity(5));

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &beliefs,
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.revived, vec![pending.key]);
        assert!(state.pending.is_empty());
        assert_eq!(
            transitions.commit_transition,
            CommitTransition::Committed {
                new_key: pending.key,
                previous_key: None,
            }
        );
    }

    #[test]
    fn kill_condition_tick_expiry_drops_entry_on_or_after_expiry() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::TickExpiry { at_tick: Tick(5) };
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(5),
        );

        assert!(state.pending.is_empty());
        assert_eq!(transitions.killed, vec![pending]);
    }

    #[test]
    fn capacity_overflow_evicts_smallest_last_reconsidered_tick() {
        let mut state = AgendaState::default();
        let mut committed = agenda_entry(
            GoalKind::FreeCarryCapacity,
            OpportunityAnchor::None,
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        state.committed = Some(committed);
        let mut older = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        older.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(2) });
        let mut newer = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            20,
            Tick(3),
        );
        newer.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(4) });
        state.pending.insert(older.key, older.clone());
        state.pending.insert(newer.key, newer.clone());
        let mut remembered = agenda_entry(GoalKind::Wash, OpportunityAnchor::None, 30, Tick(2));
        remembered.phase = AgendaPhase::Suspended;
        remembered.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(5) });
        state.suspended.insert(remembered.key, remembered.clone());

        let mut profile = default_profile();
        profile.pending_capacity = 2;

        let fresh = agenda_entry(GoalKind::Wash, OpportunityAnchor::None, 40, Tick(10));
        let _ = tick_agenda(
            AGENT,
            &mut state,
            vec![fresh.clone()],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &profile,
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(state.pending.len(), 2);
        assert!(!state.pending.contains_key(&older.key));
        assert!(state.pending.contains_key(&newer.key));
        let refreshed = state
            .pending
            .get(&remembered.key)
            .expect("remembered agenda entry should be retained");
        assert_eq!(refreshed.motive_score, fresh.motive_score);
    }

    #[test]
    fn revival_cooldown_blocks_re_promotion_within_window() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(7),
        );
        pending.last_reconsidered_tick = Tick(7);
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(0) });
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.revived.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn discrepancy_memory_suppression_blocks_revival_when_is_suppressed_true() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.revival_trigger = Some(RevivalTrigger::TickElapsed { at_tick: Tick(0) });
        state.pending.insert(pending.key, pending.clone());

        let blocker_key = worldwake_core::BlockerKey {
            goal_key: pending.offer.key,
            place: Some(PLACE),
            target: None,
            action_def: None,
        };
        let mut discrepancy_memory = worldwake_core::DiscrepancyMemory::default();
        discrepancy_memory.record(worldwake_core::DiscrepancyEntry {
            blocker_key,
            discrepancy: worldwake_core::Discrepancy::BeliefStale,
            observed_tick: Tick(1),
            expires_tick: Tick(20),
            clearing_condition: worldwake_core::DiscrepancyClearing::TtlExpiry,
        });

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &discrepancy_memory,
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.revived.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn obligation_resolved_kill_condition_clears_missing_expectation() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::ObligationResolved {
            expectation: ExpectationId(9),
        };
        state.pending.insert(pending.key, pending.clone());

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.killed, vec![pending]);
    }

    #[test]
    fn obligation_resolved_kill_condition_preserves_active_expectation() {
        let mut state = AgendaState::default();
        let mut pending = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            10,
            Tick(1),
        );
        pending.kill_condition = KillCondition::ObligationResolved {
            expectation: ExpectationId(9),
        };
        state.pending.insert(pending.key, pending.clone());

        let mut store = ExpectationStore::default();
        store.records.insert(
            ExpectationId(9),
            ExpectationRecord {
                id: ExpectationId(9),
                owner: AGENT,
                subject: TARGET,
                expected_place: PLACE,
                deadline_tick: Tick(20),
                grace_ticks: 0,
                basis: ExpectationBasis::RoutineReturn,
                state: ExpectationState::Active,
                created_tick: Tick(1),
            },
        );
        let beliefs = MockGoalBeliefView {
            expectation_store: Some(store),
            ..MockGoalBeliefView::default()
        };

        let transitions = tick_agenda(
            AGENT,
            &mut state,
            Vec::new(),
            &beliefs,
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert!(transitions.killed.is_empty());
        assert!(state.pending.contains_key(&pending.key));
    }

    #[test]
    fn unchanged_commit_does_not_recommit_when_current_goal_survives() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            50,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            54,
            Tick(10),
        );
        let transitions: AgendaTransitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.commit_transition, CommitTransition::Unchanged);
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(committed_key)
        );
    }

    #[test]
    fn same_class_margin_blocks_commit_switch_when_delta_is_too_small() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            109,
            Tick(10),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(transitions.commit_transition, CommitTransition::Unchanged);
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(committed_key)
        );
    }

    #[test]
    fn same_class_margin_switches_commit_when_delta_clears_threshold() {
        let mut committed = agenda_entry(
            GoalKind::Sleep,
            OpportunityAnchor::Place(PLACE),
            100,
            Tick(1),
        );
        committed.phase = AgendaPhase::Committed;
        let committed_key = committed.key;
        let mut state = AgendaState {
            committed: Some(committed),
            pending: BTreeMap::new(),
            suspended: BTreeMap::new(),
        };

        let challenger = agenda_entry(
            GoalKind::Relieve,
            OpportunityAnchor::Place(TARGET),
            110,
            Tick(10),
        );
        let transitions = tick_agenda(
            AGENT,
            &mut state,
            vec![challenger.clone()],
            &MockGoalBeliefView::default(),
            &worldwake_core::DiscrepancyMemory::default(),
            AgendaTickPolicy {
                profile: &default_profile(),
                switch_margin: default_switch_margin(),
            },
            Tick(10),
        );

        assert_eq!(
            transitions.commit_transition,
            CommitTransition::Committed {
                new_key: challenger.key,
                previous_key: Some(committed_key),
            }
        );
        assert_eq!(
            state.committed.as_ref().map(|entry| entry.key),
            Some(challenger.key)
        );
        assert!(
            state.pending.contains_key(&committed_key),
            "previously committed goal should return to pending after a margin-clearing switch"
        );
    }

    #[test]
    fn classify_rejection_missing_observation_for_commodity_goal_uses_commodity_trigger() {
        assert_eq!(
            classify(
                GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: worldwake_core::CommodityPurpose::SelfConsume,
                    quantity: AcquisitionQuantity::single(),
                },
                OpportunityAnchor::Place(PLACE),
                Discrepancy::MissingObservation,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CommodityAvailable {
                    place: PLACE,
                    kind: CommodityKind::Bread,
                    min: Quantity(1),
                },
            }
        );
    }

    #[test]
    fn classify_rejection_missing_observation_for_target_goal_uses_target_present_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(TARGET, PLACE);
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::MissingObservation,
                },
                &goal_offer(
                    GoalKind::SearchForMissing {
                        subject: TARGET,
                        last_seen: None,
                    },
                    OpportunityAnchor::Entity(TARGET),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TargetPresent {
                    target: TARGET,
                    place: PLACE,
                },
            }
        );
    }

    #[test]
    fn classify_rejection_route_unknown_uses_route_learned_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(AGENT, PLACE);
        beliefs.entity_places.insert(
            TARGET,
            EntityId {
                slot: 4,
                generation: 0,
            },
        );
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::RouteUnknown,
                },
                &goal_offer(
                    GoalKind::SearchForMissing {
                        subject: TARGET,
                        last_seen: None
                    },
                    OpportunityAnchor::Entity(TARGET)
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::RouteLearned {
                    from: PLACE,
                    to: EntityId {
                        slot: 4,
                        generation: 0
                    },
                },
            }
        );
    }

    #[test]
    fn classify_rejection_partial_execution_drift_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::PartialExecutionDrift,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_belief_stale_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::BeliefStale,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_no_willing_counterparty_uses_counterparty_trigger() {
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(TARGET, PLACE);
        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::NoWillingCounterparty,
                },
                &goal_offer(
                    GoalKind::AcquireCommodity {
                        commodity: CommodityKind::Bread,
                        purpose: worldwake_core::CommodityPurpose::SelfConsume,
                        quantity: AcquisitionQuantity::single(),
                    },
                    OpportunityAnchor::Entity(TARGET),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::CounterpartyAvailable {
                    counterparty: TARGET,
                    place: PLACE,
                },
            }
        );
    }

    #[test]
    fn classify_rejection_search_budget_exhausted_uses_tick_elapsed_trigger() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::SearchBudgetExhausted,
            ),
            RejectionLifecycle::InfeasibleUntil {
                trigger: RevivalTrigger::TickElapsed { at_tick: Tick(14) },
            }
        );
    }

    #[test]
    fn classify_rejection_belief_contradicted_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::BeliefContradicted,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_no_legal_binding_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::NoLegalBinding,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_improper_planning_state_is_dead() {
        assert_eq!(
            classify(
                GoalKind::Sleep,
                OpportunityAnchor::Place(PLACE),
                Discrepancy::ImproperPlanningState,
            ),
            RejectionLifecycle::Dead
        );
    }

    #[test]
    fn classify_rejection_satisfied_move_cargo_short_circuits_reason_mapping() {
        let destination = EntityId {
            slot: 9,
            generation: 0,
        };
        let mut beliefs = MockGoalBeliefView::default();
        beliefs.entity_places.insert(destination, destination);
        beliefs
            .controlled_quantities
            .insert((AGENT, destination, CommodityKind::Bread), Quantity(2));
        beliefs
            .demand_memory
            .insert(AGENT, vec![demand(destination, CommodityKind::Bread, 2)]);

        assert_eq!(
            classify_rejection(
                AGENT,
                &FeasibilityVerdict::RejectedBeforeSearch {
                    reason: Discrepancy::NoLegalBinding,
                },
                &goal_offer(
                    GoalKind::MoveCargo {
                        commodity: CommodityKind::Bread,
                        destination,
                    },
                    OpportunityAnchor::Place(destination),
                ),
                &beliefs,
                Tick(10),
                default_profile().revive_cooldown_ticks,
            ),
            RejectionLifecycle::Satisfied
        );
    }
}
