//! Authoritative belief and perception state for E14.

use crate::{
    BelievedInstitutionalClaim, CommodityKind, Component, EntityId, InstitutionalBeliefKey,
    Permille, Quantity, ResourceSource, Tick, WorkstationTag, World, Wound,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-agent subjective view of observed entities and social evidence.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentBeliefStore {
    pub known_entities: BTreeMap<EntityId, BelievedEntityState>,
    pub social_observations: Vec<SocialObservation>,
    pub told_beliefs: BTreeMap<TellMemoryKey, ToldBeliefMemory>,
    pub heard_beliefs: BTreeMap<TellMemoryKey, HeardBeliefMemory>,
    pub institutional_beliefs:
        BTreeMap<InstitutionalBeliefKey, Vec<BelievedInstitutionalClaim>>,
}

impl AgentBeliefStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_entity(&mut self, id: EntityId, state: BelievedEntityState) {
        match self.known_entities.get(&id) {
            Some(existing) if existing.observed_tick > state.observed_tick => {}
            _ => {
                self.known_entities.insert(id, state);
            }
        }
    }

    #[must_use]
    pub fn get_entity(&self, id: &EntityId) -> Option<&BelievedEntityState> {
        self.known_entities.get(id)
    }

    pub fn record_social_observation(&mut self, observation: SocialObservation) {
        self.social_observations.push(observation);
    }

    pub fn record_told_belief(&mut self, key: TellMemoryKey, memory: ToldBeliefMemory) {
        self.told_beliefs.insert(key, memory);
    }

    pub fn record_heard_belief(&mut self, key: TellMemoryKey, memory: HeardBeliefMemory) {
        self.heard_beliefs.insert(key, memory);
    }

    pub fn record_institutional_belief(
        &mut self,
        key: InstitutionalBeliefKey,
        belief: BelievedInstitutionalClaim,
        profile: &PerceptionProfile,
    ) {
        self.institutional_beliefs
            .entry(key)
            .or_default()
            .push(belief);
        self.enforce_institutional_capacity(profile);
    }

    pub fn enforce_capacity(&mut self, profile: &PerceptionProfile, current_tick: Tick) {
        self.known_entities.retain(|_, state| {
            within_retention_window(
                state.observed_tick,
                current_tick,
                profile.memory_retention_ticks,
            )
        });
        self.social_observations.retain(|observation| {
            within_retention_window(
                observation.observed_tick,
                current_tick,
                profile.memory_retention_ticks,
            )
        });

        if profile.memory_capacity == 0 {
            self.known_entities.clear();
            return;
        }

        let excess = self
            .known_entities
            .len()
            .saturating_sub(profile.memory_capacity as usize);
        if excess == 0 {
            return;
        }

        let mut eviction_order = self
            .known_entities
            .iter()
            .map(|(entity, state)| (state.observed_tick, *entity))
            .collect::<Vec<_>>();
        eviction_order.sort_unstable();

        for (_, entity) in eviction_order.into_iter().take(excess) {
            self.known_entities.remove(&entity);
        }
    }

    pub fn enforce_conversation_memory(&mut self, profile: &TellProfile, current_tick: Tick) {
        self.told_beliefs.retain(|_, memory| {
            within_retention_window(
                memory.told_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        });
        self.heard_beliefs.retain(|_, memory| {
            within_retention_window(
                memory.heard_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        });

        enforce_memory_lane_capacity(
            &mut self.told_beliefs,
            usize::from(profile.conversation_memory_capacity),
            |memory| memory.told_tick,
        );
        enforce_memory_lane_capacity(
            &mut self.heard_beliefs,
            usize::from(profile.conversation_memory_capacity),
            |memory| memory.heard_tick,
        );
    }

    #[must_use]
    pub fn told_belief_memory(
        &self,
        key: &TellMemoryKey,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> Option<&ToldBeliefMemory> {
        self.told_beliefs.get(key).filter(|memory| {
            within_retention_window(
                memory.told_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        })
    }

    #[must_use]
    pub fn heard_belief_memory(
        &self,
        key: &TellMemoryKey,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> Option<&HeardBeliefMemory> {
        self.heard_beliefs.get(key).filter(|memory| {
            within_retention_window(
                memory.heard_tick,
                current_tick,
                profile.conversation_memory_retention_ticks,
            )
        })
    }

    #[must_use]
    pub fn recipient_knowledge_status(
        &self,
        key: &TellMemoryKey,
        current_belief: &BelievedEntityState,
        current_tick: Tick,
        profile: &TellProfile,
    ) -> RecipientKnowledgeStatus {
        match self.told_belief_memory(key, current_tick, profile) {
            Some(memory) => recipient_knowledge_status(current_belief, Some(memory)),
            None if self.told_beliefs.contains_key(key) => {
                RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
            }
            None => RecipientKnowledgeStatus::UnknownToSpeaker,
        }
    }

    fn enforce_institutional_capacity(&mut self, profile: &PerceptionProfile) {
        let capacity = profile.institutional_memory_capacity as usize;
        if capacity == 0 {
            self.institutional_beliefs.clear();
            return;
        }

        while self.total_institutional_beliefs() > capacity {
            let Some((key, index)) = self.oldest_institutional_belief_position() else {
                break;
            };
            let remove_key = {
                let beliefs = self
                    .institutional_beliefs
                    .get_mut(&key)
                    .expect("selected institutional belief key should still exist");
                beliefs.remove(index);
                beliefs.is_empty()
            };
            if remove_key {
                self.institutional_beliefs.remove(&key);
            }
        }
    }

    fn total_institutional_beliefs(&self) -> usize {
        self.institutional_beliefs
            .values()
            .map(std::vec::Vec::len)
            .sum()
    }

    fn oldest_institutional_belief_position(&self) -> Option<(InstitutionalBeliefKey, usize)> {
        self.institutional_beliefs
            .iter()
            .flat_map(|(key, beliefs)| {
                beliefs
                    .iter()
                    .enumerate()
                    .map(move |(index, belief)| (belief.learned_tick, *key, index))
            })
            .min()
            .map(|(_, key, index)| (key, index))
    }
}

impl Component for AgentBeliefStore {}

/// Snapshot of what an agent believes about a specific entity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ObservedEntitySnapshot {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub courage: Option<Permille>,
}

impl ObservedEntitySnapshot {
    #[must_use]
    pub fn to_believed_entity_state(
        &self,
        observed_tick: Tick,
        source: PerceptionSource,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: self.last_known_place,
            last_known_inventory: self.last_known_inventory.clone(),
            workstation_tag: self.workstation_tag,
            resource_source: self.resource_source.clone(),
            alive: self.alive,
            wounds: self.wounds.clone(),
            last_known_courage: self.courage,
            observed_tick,
            source,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BelievedEntityState {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct TellMemoryKey {
    pub counterparty: EntityId,
    pub subject: EntityId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ToldBeliefMemory {
    pub shared_state: SharedBeliefSnapshot,
    pub told_tick: Tick,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct HeardBeliefMemory {
    pub heard_state: SharedBeliefSnapshot,
    pub heard_tick: Tick,
    pub disposition: HeardBeliefDisposition,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum HeardBeliefDisposition {
    Accepted,
    Rejected,
    AlreadyHeldEqualOrNewer,
    NotInternalized,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SharedBeliefSnapshot {
    pub last_known_place: Option<EntityId>,
    pub last_known_inventory: BTreeMap<CommodityKind, Quantity>,
    pub workstation_tag: Option<WorkstationTag>,
    pub resource_source: Option<ResourceSource>,
    pub alive: bool,
    pub wounds: Vec<Wound>,
    pub last_known_courage: Option<Permille>,
    pub source: PerceptionSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RecipientKnowledgeStatus {
    UnknownToSpeaker,
    SpeakerHasAlreadyToldCurrentBelief,
    SpeakerHasOnlyToldStaleBelief,
    SpeakerPreviouslyToldButMemoryExpired,
}

#[must_use]
pub fn to_shared_belief_snapshot(state: &BelievedEntityState) -> SharedBeliefSnapshot {
    SharedBeliefSnapshot {
        last_known_place: state.last_known_place,
        last_known_inventory: state.last_known_inventory.clone(),
        workstation_tag: state.workstation_tag,
        resource_source: state.resource_source.clone(),
        alive: state.alive,
        wounds: state.wounds.clone(),
        last_known_courage: state.last_known_courage,
        source: state.source,
    }
}

#[must_use]
pub fn share_equivalent(
    current_belief: &BelievedEntityState,
    prior_shared_state: &SharedBeliefSnapshot,
) -> bool {
    to_shared_belief_snapshot(current_belief) == *prior_shared_state
}

#[must_use]
pub fn recipient_knowledge_status(
    current_belief: &BelievedEntityState,
    prior_tell: Option<&ToldBeliefMemory>,
) -> RecipientKnowledgeStatus {
    match prior_tell {
        None => RecipientKnowledgeStatus::UnknownToSpeaker,
        Some(memory) if share_equivalent(current_belief, &memory.shared_state) => {
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        }
        Some(_) => RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief,
    }
}

#[must_use]
pub fn build_observed_entity_snapshot(
    world: &World,
    entity: EntityId,
) -> Option<ObservedEntitySnapshot> {
    world.entity_kind(entity)?;

    let mut inventory = BTreeMap::new();
    for commodity in CommodityKind::ALL {
        let quantity = world.controlled_commodity_quantity(entity, commodity);
        if quantity > Quantity(0) {
            inventory.insert(commodity, quantity);
        }
    }

    Some(ObservedEntitySnapshot {
        last_known_place: world.effective_place(entity),
        last_known_inventory: inventory,
        workstation_tag: world
            .get_component_workstation_marker(entity)
            .map(|marker| marker.0),
        resource_source: world.get_component_resource_source(entity).cloned(),
        alive: world.get_component_dead_at(entity).is_none(),
        wounds: world
            .get_component_wound_list(entity)
            .map(|wounds| wounds.wounds.clone())
            .unwrap_or_default(),
        courage: world
            .get_component_utility_profile(entity)
            .map(|p| p.courage),
    })
}

#[must_use]
pub fn build_believed_entity_state(
    world: &World,
    entity: EntityId,
    observed_tick: Tick,
    source: PerceptionSource,
) -> Option<BelievedEntityState> {
    build_observed_entity_snapshot(world, entity)
        .map(|snapshot| snapshot.to_believed_entity_state(observed_tick, source))
}

/// How the agent acquired a belief snapshot.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PerceptionSource {
    DirectObservation,
    Report { from: EntityId, chain_len: u8 },
    Rumor { chain_len: u8 },
    Inference,
}

/// Explicit per-agent policy for deriving belief confidence from provenance and age.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BeliefConfidencePolicy {
    pub direct_observation_base: Permille,
    pub report_base: Permille,
    pub rumor_base: Permille,
    pub inference_base: Permille,
    pub report_chain_penalty: Permille,
    pub rumor_chain_penalty: Permille,
    pub staleness_penalty_per_tick: Permille,
}

impl Default for BeliefConfidencePolicy {
    fn default() -> Self {
        Self {
            direct_observation_base: Permille::new(950).unwrap(),
            report_base: Permille::new(780).unwrap(),
            rumor_base: Permille::new(560).unwrap(),
            inference_base: Permille::new(420).unwrap(),
            report_chain_penalty: Permille::new(90).unwrap(),
            rumor_chain_penalty: Permille::new(110).unwrap(),
            staleness_penalty_per_tick: Permille::new(12).unwrap(),
        }
    }
}

/// Derives confidence from provenance and age without storing abstract authority state.
#[must_use]
pub fn belief_confidence(
    source: &PerceptionSource,
    staleness_ticks: u64,
    policy: &BeliefConfidencePolicy,
) -> Permille {
    let base = match *source {
        PerceptionSource::DirectObservation => policy.direct_observation_base.value(),
        PerceptionSource::Report { chain_len, .. } => policy.report_base.value().saturating_sub(
            policy
                .report_chain_penalty
                .value()
                .saturating_mul(u16::from(chain_len.saturating_sub(1))),
        ),
        PerceptionSource::Rumor { chain_len } => policy.rumor_base.value().saturating_sub(
            policy
                .rumor_chain_penalty
                .value()
                .saturating_mul(u16::from(chain_len.saturating_sub(1))),
        ),
        PerceptionSource::Inference => policy.inference_base.value(),
    };
    let staleness_penalty = u16::try_from(staleness_ticks)
        .unwrap_or(u16::MAX)
        .saturating_mul(policy.staleness_penalty_per_tick.value());

    Permille::new(base.saturating_sub(staleness_penalty))
        .expect("belief confidence derivation always yields a valid permille")
}

/// A witnessed social fact retained in belief memory.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SocialObservation {
    pub kind: SocialObservationKind,
    pub subjects: (EntityId, EntityId),
    pub place: EntityId,
    pub observed_tick: Tick,
    pub source: PerceptionSource,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum SocialObservationKind {
    WitnessedCooperation,
    WitnessedConflict,
    WitnessedObligation,
    WitnessedTelling,
    CoPresence,
}

/// Concrete differences between a prior belief and a new observation.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum MismatchKind {
    EntityMissing,
    AliveStatusChanged,
    InventoryDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    ResourceSourceDiscrepancy {
        commodity: CommodityKind,
        believed: Quantity,
        observed: Quantity,
    },
    PlaceChanged {
        believed_place: EntityId,
        observed_place: EntityId,
    },
}

/// Per-agent parameters controlling belief retention and observation quality.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PerceptionProfile {
    pub memory_capacity: u32,
    pub memory_retention_ticks: u64,
    pub observation_fidelity: Permille,
    pub confidence_policy: BeliefConfidencePolicy,
    pub institutional_memory_capacity: u32,
    pub consultation_speed_factor: Permille,
    pub contradiction_tolerance: Permille,
}

impl Component for PerceptionProfile {}

impl Default for PerceptionProfile {
    fn default() -> Self {
        Self {
            memory_capacity: 12,
            memory_retention_ticks: 48,
            observation_fidelity: Permille::new(875).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 20,
            consultation_speed_factor: Permille::new(500).unwrap(),
            contradiction_tolerance: Permille::new(300).unwrap(),
        }
    }
}

/// Per-agent parameters controlling what information an agent relays and accepts.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TellProfile {
    pub max_tell_candidates: u8,
    pub max_relay_chain_len: u8,
    pub acceptance_fidelity: Permille,
    pub conversation_memory_capacity: u16,
    pub conversation_memory_retention_ticks: u64,
}

impl Component for TellProfile {}

impl Default for TellProfile {
    fn default() -> Self {
        Self {
            max_tell_candidates: 3,
            max_relay_chain_len: 3,
            acceptance_fidelity: Permille::new(800).unwrap(),
            conversation_memory_capacity: 12,
            conversation_memory_retention_ticks: 48,
        }
    }
}

fn enforce_memory_lane_capacity<T, F>(
    lane: &mut BTreeMap<TellMemoryKey, T>,
    capacity: usize,
    tick_of: F,
) where
    F: Fn(&T) -> Tick,
{
    if capacity == 0 {
        lane.clear();
        return;
    }

    let excess = lane.len().saturating_sub(capacity);
    if excess == 0 {
        return;
    }

    let mut eviction_order = lane
        .iter()
        .map(|(key, memory)| (tick_of(memory), *key))
        .collect::<Vec<_>>();
    eviction_order.sort_unstable();

    for (_, key) in eviction_order.into_iter().take(excess) {
        lane.remove(&key);
    }
}

fn within_retention_window(observed_tick: Tick, current_tick: Tick, retention_ticks: u64) -> bool {
    current_tick.0.saturating_sub(observed_tick.0) <= retention_ticks
}

#[cfg(test)]
mod tests {
    use super::{
        belief_confidence, build_believed_entity_state, build_observed_entity_snapshot,
        recipient_knowledge_status, share_equivalent, to_shared_belief_snapshot, AgentBeliefStore,
        BeliefConfidencePolicy, BelievedEntityState, HeardBeliefDisposition, HeardBeliefMemory,
        MismatchKind, ObservedEntitySnapshot, PerceptionProfile, PerceptionSource,
        RecipientKnowledgeStatus, SocialObservation, SocialObservationKind, TellMemoryKey,
        TellProfile, ToldBeliefMemory,
    };
    use crate::{
        build_prototype_world, traits::Component, BelievedInstitutionalClaim, BodyPart,
        CommodityKind, ControlSource, DeadAt, EntityId, InstitutionalBeliefKey,
        InstitutionalClaim, InstitutionalKnowledgeSource, Permille, Quantity, Tick, World, Wound,
        WoundCause, WoundId, WoundList,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use std::collections::BTreeMap;

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn profile(memory_capacity: u32, memory_retention_ticks: u64) -> PerceptionProfile {
        PerceptionProfile {
            memory_capacity,
            memory_retention_ticks,
            observation_fidelity: Permille::new(750).unwrap(),
            confidence_policy: BeliefConfidencePolicy::default(),
            institutional_memory_capacity: 9,
            consultation_speed_factor: Permille::new(650).unwrap(),
            contradiction_tolerance: Permille::new(275).unwrap(),
        }
    }

    fn sample_institutional_belief(observed_tick: u64) -> BelievedInstitutionalClaim {
        BelievedInstitutionalClaim {
            claim: InstitutionalClaim::OfficeHolder {
                office: entity(50),
                holder: Some(entity(51)),
                effective_tick: Tick(observed_tick.saturating_sub(1)),
            },
            source: InstitutionalKnowledgeSource::RecordConsultation {
                record: entity(52),
                entry_id: crate::RecordEntryId(3),
            },
            learned_tick: Tick(observed_tick),
            learned_at: Some(entity(53)),
        }
    }

    fn policy() -> BeliefConfidencePolicy {
        BeliefConfidencePolicy::default()
    }

    fn sample_wound(id: u64, observed_tick: u64) -> Wound {
        Wound {
            id: WoundId(id),
            body_part: BodyPart::Torso,
            cause: WoundCause::Combat {
                attacker: entity(99),
                weapon: crate::CombatWeaponRef::Unarmed,
            },
            severity: Permille::new(125).unwrap(),
            inflicted_at: Tick(observed_tick),
            bleed_rate_per_tick: Permille::new(10).unwrap(),
        }
    }

    fn sample_state(observed_tick: u64, commodity_qty: u32) -> BelievedEntityState {
        let mut inventory = BTreeMap::new();
        inventory.insert(CommodityKind::Apple, Quantity(commodity_qty));
        BelievedEntityState {
            last_known_place: Some(entity(10)),
            last_known_inventory: inventory,
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![sample_wound(1, observed_tick)],
            last_known_courage: None,
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn sample_social_observation(observed_tick: u64) -> SocialObservation {
        SocialObservation {
            kind: SocialObservationKind::WitnessedConflict,
            subjects: (entity(1), entity(2)),
            place: entity(10),
            observed_tick: Tick(observed_tick),
            source: PerceptionSource::DirectObservation,
        }
    }

    fn tell_profile() -> TellProfile {
        TellProfile {
            max_tell_candidates: 3,
            max_relay_chain_len: 2,
            acceptance_fidelity: Permille::new(800).unwrap(),
            conversation_memory_capacity: 2,
            conversation_memory_retention_ticks: 5,
        }
    }

    fn tell_memory_key(counterparty: u32, subject: u32) -> TellMemoryKey {
        TellMemoryKey {
            counterparty: entity(counterparty),
            subject: entity(subject),
        }
    }

    fn told_memory(
        counterparty: u32,
        subject: u32,
        told_tick: u64,
        state: &BelievedEntityState,
    ) -> (TellMemoryKey, ToldBeliefMemory) {
        (
            tell_memory_key(counterparty, subject),
            ToldBeliefMemory {
                shared_state: to_shared_belief_snapshot(state),
                told_tick: Tick(told_tick),
            },
        )
    }

    fn heard_memory(
        counterparty: u32,
        subject: u32,
        heard_tick: u64,
        state: &BelievedEntityState,
        disposition: HeardBeliefDisposition,
    ) -> (TellMemoryKey, HeardBeliefMemory) {
        (
            tell_memory_key(counterparty, subject),
            HeardBeliefMemory {
                heard_state: to_shared_belief_snapshot(state),
                heard_tick: Tick(heard_tick),
                disposition,
            },
        )
    }

    fn assert_component_bounds<T: Component>() {}

    fn assert_serde_bounds<T: Eq + Clone + Serialize + DeserializeOwned>() {}

    fn assert_ordered_traits<T: Copy + Eq + Ord + std::hash::Hash>() {}

    #[test]
    fn new_creates_empty_store() {
        let store = AgentBeliefStore::new();

        assert!(store.known_entities.is_empty());
        assert!(store.social_observations.is_empty());
        assert!(store.told_beliefs.is_empty());
        assert!(store.heard_beliefs.is_empty());
        assert!(store.institutional_beliefs.is_empty());
    }

    #[test]
    fn update_entity_inserts_new_snapshot() {
        let mut store = AgentBeliefStore::new();
        let target = entity(3);
        let state = sample_state(7, 4);

        store.update_entity(target, state.clone());

        assert_eq!(store.get_entity(&target), Some(&state));
    }

    #[test]
    fn update_entity_replaces_with_equal_or_newer_snapshot_only() {
        let mut store = AgentBeliefStore::new();
        let target = entity(4);

        store.update_entity(target, sample_state(8, 2));
        store.update_entity(target, sample_state(7, 9));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(2)
        );

        store.update_entity(target, sample_state(8, 5));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(5)
        );

        store.update_entity(target, sample_state(9, 6));
        assert_eq!(
            store.get_entity(&target).unwrap().last_known_inventory[&CommodityKind::Apple],
            Quantity(6)
        );
    }

    #[test]
    fn get_entity_returns_none_for_unknown_entity() {
        let store = AgentBeliefStore::new();

        assert_eq!(store.get_entity(&entity(404)), None);
    }

    #[test]
    fn record_social_observation_appends_to_list() {
        let mut store = AgentBeliefStore::new();
        let first = sample_social_observation(3);
        let second = SocialObservation {
            kind: SocialObservationKind::CoPresence,
            ..sample_social_observation(4)
        };

        store.record_social_observation(first.clone());
        store.record_social_observation(second.clone());

        assert_eq!(store.social_observations, vec![first, second]);
    }

    #[test]
    fn enforce_capacity_evicts_oldest_entities_deterministically() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(3), sample_state(5, 1));
        store.update_entity(entity(2), sample_state(5, 2));
        store.update_entity(entity(4), sample_state(6, 3));

        store.enforce_capacity(&profile(2, 100), Tick(20));

        assert_eq!(store.known_entities.len(), 2);
        assert!(!store.known_entities.contains_key(&entity(2)));
        assert!(store.known_entities.contains_key(&entity(3)));
        assert!(store.known_entities.contains_key(&entity(4)));
    }

    #[test]
    fn enforce_capacity_removes_stale_entities_and_social_observations() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(2, 1));
        store.update_entity(entity(2), sample_state(9, 2));
        store.record_social_observation(sample_social_observation(3));
        store.record_social_observation(sample_social_observation(9));

        store.enforce_capacity(&profile(10, 3), Tick(12));

        assert!(!store.known_entities.contains_key(&entity(1)));
        assert!(store.known_entities.contains_key(&entity(2)));
        assert_eq!(
            store.social_observations,
            vec![sample_social_observation(9)]
        );
    }

    #[test]
    fn enforce_capacity_clears_entities_when_capacity_is_zero() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(10, 1));
        store.update_entity(entity(2), sample_state(11, 2));

        store.enforce_capacity(&profile(0, 100), Tick(12));

        assert!(store.known_entities.is_empty());
    }

    #[test]
    fn record_institutional_belief_enforces_capacity_deterministically() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 100);
        profile.institutional_memory_capacity = 2;

        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(70) },
            sample_institutional_belief(5),
            &profile,
        );
        store.record_institutional_belief(
            InstitutionalBeliefKey::FactionMembersOf { faction: entity(71) },
            sample_institutional_belief(4),
            &profile,
        );
        store.record_institutional_belief(
            InstitutionalBeliefKey::SupportFor {
                supporter: entity(72),
                office: entity(73),
            },
            sample_institutional_belief(6),
            &profile,
        );

        assert!(!store
            .institutional_beliefs
            .contains_key(&InstitutionalBeliefKey::FactionMembersOf {
                faction: entity(71)
            }));
        assert_eq!(store.total_institutional_beliefs(), 2);
        assert!(store
            .institutional_beliefs
            .contains_key(&InstitutionalBeliefKey::OfficeHolderOf { office: entity(70) }));
        assert!(store
            .institutional_beliefs
            .contains_key(&InstitutionalBeliefKey::SupportFor {
                supporter: entity(72),
                office: entity(73),
            }));
    }

    #[test]
    fn record_institutional_belief_breaks_ties_by_key_then_position() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 100);
        profile.institutional_memory_capacity = 2;
        let first_key = InstitutionalBeliefKey::FactionMembersOf { faction: entity(80) };
        let second_key = InstitutionalBeliefKey::SupportFor {
            supporter: entity(81),
            office: entity(82),
        };

        store.record_institutional_belief(first_key, sample_institutional_belief(5), &profile);
        store.record_institutional_belief(second_key, sample_institutional_belief(5), &profile);
        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(83) },
            sample_institutional_belief(6),
            &profile,
        );

        assert!(!store.institutional_beliefs.contains_key(&first_key));
        assert!(store.institutional_beliefs.contains_key(&second_key));
    }

    #[test]
    fn record_institutional_belief_clears_all_when_capacity_is_zero() {
        let mut store = AgentBeliefStore::new();
        let mut profile = profile(12, 100);
        profile.institutional_memory_capacity = 0;

        store.record_institutional_belief(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(90) },
            sample_institutional_belief(7),
            &profile,
        );

        assert!(store.institutional_beliefs.is_empty());
    }

    #[test]
    fn believed_entity_state_roundtrips_through_bincode() {
        let state = sample_state(11, 7);

        let bytes = bincode::serialize(&state).unwrap();
        let roundtrip: BelievedEntityState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, state);
    }

    #[test]
    fn observed_entity_snapshot_roundtrips_through_bincode() {
        let snapshot = ObservedEntitySnapshot {
            last_known_place: Some(entity(10)),
            last_known_inventory: BTreeMap::from([(CommodityKind::Bread, Quantity(3))]),
            workstation_tag: None,
            resource_source: None,
            alive: true,
            wounds: vec![sample_wound(1, 4)],
            courage: None,
        };

        let bytes = bincode::serialize(&snapshot).unwrap();
        let roundtrip: ObservedEntitySnapshot = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, snapshot);
    }

    #[test]
    fn perception_source_roundtrips_and_compares() {
        let source = PerceptionSource::Report {
            from: entity(7),
            chain_len: 2,
        };

        let bytes = bincode::serialize(&source).unwrap();
        let roundtrip: PerceptionSource = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, source);
        assert_ne!(source, PerceptionSource::Inference);
    }

    #[test]
    fn social_observation_kind_roundtrips_and_compares() {
        let kind = SocialObservationKind::WitnessedTelling;

        let bytes = bincode::serialize(&kind).unwrap();
        let roundtrip: SocialObservationKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, kind);
        assert_ne!(kind, SocialObservationKind::WitnessedConflict);
        assert_ne!(kind, SocialObservationKind::WitnessedCooperation);
    }

    #[test]
    fn mismatch_kind_variants_construct_and_sort_stably() {
        let mut variants = [
            MismatchKind::PlaceChanged {
                believed_place: entity(4),
                observed_place: entity(5),
            },
            MismatchKind::EntityMissing,
            MismatchKind::InventoryDiscrepancy {
                commodity: CommodityKind::Bread,
                believed: Quantity(5),
                observed: Quantity(2),
            },
            MismatchKind::ResourceSourceDiscrepancy {
                commodity: CommodityKind::Apple,
                believed: Quantity(9),
                observed: Quantity(1),
            },
            MismatchKind::AliveStatusChanged,
        ];

        variants.sort_unstable();

        assert_eq!(
            variants,
            [
                MismatchKind::EntityMissing,
                MismatchKind::AliveStatusChanged,
                MismatchKind::InventoryDiscrepancy {
                    commodity: CommodityKind::Bread,
                    believed: Quantity(5),
                    observed: Quantity(2),
                },
                MismatchKind::ResourceSourceDiscrepancy {
                    commodity: CommodityKind::Apple,
                    believed: Quantity(9),
                    observed: Quantity(1),
                },
                MismatchKind::PlaceChanged {
                    believed_place: entity(4),
                    observed_place: entity(5),
                },
            ]
        );
    }

    #[test]
    fn mismatch_kind_roundtrips_through_bincode() {
        let mismatch = MismatchKind::InventoryDiscrepancy {
            commodity: CommodityKind::Water,
            believed: Quantity(7),
            observed: Quantity(3),
        };

        let bytes = bincode::serialize(&mismatch).unwrap();
        let roundtrip: MismatchKind = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, mismatch);
    }

    #[test]
    fn perception_profile_roundtrips_through_bincode() {
        let profile = profile(12, 34);

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: PerceptionProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn agent_belief_store_roundtrips_through_bincode_with_institutional_beliefs() {
        let mut store = AgentBeliefStore::new();
        store.update_entity(entity(1), sample_state(7, 2));
        store.institutional_beliefs.insert(
            InstitutionalBeliefKey::OfficeHolderOf { office: entity(50) },
            vec![sample_institutional_belief(12)],
        );

        let bytes = bincode::serialize(&store).unwrap();
        let roundtrip: AgentBeliefStore = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, store);
    }

    #[test]
    fn belief_confidence_policy_roundtrips_through_bincode() {
        let policy = BeliefConfidencePolicy {
            direct_observation_base: Permille::new(920).unwrap(),
            report_base: Permille::new(730).unwrap(),
            rumor_base: Permille::new(510).unwrap(),
            inference_base: Permille::new(390).unwrap(),
            report_chain_penalty: Permille::new(70).unwrap(),
            rumor_chain_penalty: Permille::new(95).unwrap(),
            staleness_penalty_per_tick: Permille::new(8).unwrap(),
        };

        let bytes = bincode::serialize(&policy).unwrap();
        let roundtrip: BeliefConfidencePolicy = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, policy);
    }

    #[test]
    fn tell_profile_defaults_match_e15_spec() {
        assert_eq!(
            TellProfile::default(),
            TellProfile {
                max_tell_candidates: 3,
                max_relay_chain_len: 3,
                acceptance_fidelity: Permille::new(800).unwrap(),
                conversation_memory_capacity: 12,
                conversation_memory_retention_ticks: 48,
            }
        );
    }

    #[test]
    fn tell_profile_roundtrips_through_bincode() {
        let profile = TellProfile {
            max_tell_candidates: 5,
            max_relay_chain_len: 2,
            acceptance_fidelity: Permille::new(650).unwrap(),
            conversation_memory_capacity: 9,
            conversation_memory_retention_ticks: 21,
        };

        let bytes = bincode::serialize(&profile).unwrap();
        let roundtrip: TellProfile = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, profile);
    }

    #[test]
    fn shared_belief_snapshot_ignores_observed_tick_and_matches_shareable_content() {
        let older = sample_state(3, 4);
        let mut newer = older.clone();
        newer.observed_tick = Tick(9);
        let snapshot = to_shared_belief_snapshot(&older);

        assert_eq!(snapshot, to_shared_belief_snapshot(&newer));
        assert!(share_equivalent(&newer, &snapshot));
    }

    #[test]
    fn conversation_memory_read_helpers_ignore_expired_entries_before_cleanup() {
        let mut store = AgentBeliefStore::new();
        let profile = tell_profile();
        let fresh_state = sample_state(9, 3);
        let stale_state = sample_state(1, 2);
        let stale_key = tell_memory_key(2, 21);
        let fresh_key = tell_memory_key(3, 22);

        let stale_told = told_memory(2, 21, 1, &stale_state);
        let fresh_told = told_memory(3, 22, 9, &fresh_state);
        let stale_heard = heard_memory(2, 21, 1, &stale_state, HeardBeliefDisposition::Accepted);
        let fresh_heard = heard_memory(
            3,
            22,
            9,
            &fresh_state,
            HeardBeliefDisposition::AlreadyHeldEqualOrNewer,
        );

        store.record_told_belief(stale_told.0, stale_told.1);
        store.record_told_belief(fresh_told.0, fresh_told.1);
        store.record_heard_belief(stale_heard.0, stale_heard.1);
        store.record_heard_belief(fresh_heard.0, fresh_heard.1);

        assert_eq!(
            store.told_belief_memory(&stale_key, Tick(9), &profile),
            None
        );
        assert_eq!(
            store.heard_belief_memory(&stale_key, Tick(9), &profile),
            None
        );
        assert!(store.told_beliefs.contains_key(&stale_key));
        assert!(store.heard_beliefs.contains_key(&stale_key));
        assert_eq!(
            store.recipient_knowledge_status(&stale_key, &fresh_state, Tick(9), &profile),
            RecipientKnowledgeStatus::SpeakerPreviouslyToldButMemoryExpired
        );

        assert_eq!(
            store
                .told_belief_memory(&fresh_key, Tick(9), &profile)
                .map(|_| fresh_key),
            Some(fresh_key)
        );
        assert_eq!(
            store
                .heard_belief_memory(&fresh_key, Tick(9), &profile)
                .map(|_| fresh_key),
            Some(fresh_key)
        );
    }

    #[test]
    fn enforce_conversation_memory_evicts_oldest_told_and_heard_entries_independently() {
        let mut store = AgentBeliefStore::new();
        let profile = tell_profile();

        let told_a = told_memory(2, 20, 4, &sample_state(4, 1));
        let told_b = told_memory(1, 10, 4, &sample_state(4, 2));
        let told_c = told_memory(3, 30, 6, &sample_state(6, 3));
        let heard_a = heard_memory(
            5,
            50,
            3,
            &sample_state(3, 1),
            HeardBeliefDisposition::Accepted,
        );
        let heard_b = heard_memory(
            4,
            40,
            3,
            &sample_state(3, 2),
            HeardBeliefDisposition::Accepted,
        );
        let heard_c = heard_memory(
            6,
            60,
            7,
            &sample_state(7, 3),
            HeardBeliefDisposition::Accepted,
        );

        store.record_told_belief(told_a.0, told_a.1);
        store.record_told_belief(told_b.0, told_b.1);
        store.record_told_belief(told_c.0, told_c.1);
        store.record_heard_belief(heard_a.0, heard_a.1);
        store.record_heard_belief(heard_b.0, heard_b.1);
        store.record_heard_belief(heard_c.0, heard_c.1);

        store.enforce_conversation_memory(&profile, Tick(8));

        assert_eq!(
            store.told_beliefs.keys().copied().collect::<Vec<_>>(),
            vec![tell_memory_key(2, 20), tell_memory_key(3, 30)]
        );
        assert_eq!(
            store.heard_beliefs.keys().copied().collect::<Vec<_>>(),
            vec![tell_memory_key(5, 50), tell_memory_key(6, 60)]
        );
    }

    #[test]
    fn recipient_knowledge_status_distinguishes_current_and_stale_tells() {
        let current = sample_state(8, 4);
        let stale = sample_state(8, 9);
        let (_, remembered) = told_memory(7, 44, 6, &current);

        assert_eq!(
            recipient_knowledge_status(&current, Some(&remembered)),
            RecipientKnowledgeStatus::SpeakerHasAlreadyToldCurrentBelief
        );
        assert_eq!(
            recipient_knowledge_status(&stale, Some(&remembered)),
            RecipientKnowledgeStatus::SpeakerHasOnlyToldStaleBelief
        );
        assert_eq!(
            recipient_knowledge_status(&current, None),
            RecipientKnowledgeStatus::UnknownToSpeaker
        );
    }

    #[test]
    fn belief_confidence_orders_sources_by_provenance() {
        let policy = policy();
        let direct = belief_confidence(&PerceptionSource::DirectObservation, 0, &policy);
        let report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(7),
                chain_len: 1,
            },
            0,
            &policy,
        );
        let rumor = belief_confidence(&PerceptionSource::Rumor { chain_len: 1 }, 0, &policy);
        let inference = belief_confidence(&PerceptionSource::Inference, 0, &policy);

        assert!(direct > report);
        assert!(report > rumor);
        assert!(rumor > inference);
        assert_eq!(direct, policy.direct_observation_base);
    }

    #[test]
    fn belief_confidence_penalizes_deeper_report_and_rumor_chains() {
        let policy = policy();
        let report_shallow = belief_confidence(
            &PerceptionSource::Report {
                from: entity(1),
                chain_len: 1,
            },
            0,
            &policy,
        );
        let report_deep = belief_confidence(
            &PerceptionSource::Report {
                from: entity(1),
                chain_len: 3,
            },
            0,
            &policy,
        );
        let rumor_shallow =
            belief_confidence(&PerceptionSource::Rumor { chain_len: 1 }, 0, &policy);
        let rumor_deep = belief_confidence(&PerceptionSource::Rumor { chain_len: 3 }, 0, &policy);

        assert!(report_deep < report_shallow);
        assert!(rumor_deep < rumor_shallow);
    }

    #[test]
    fn belief_confidence_monotonically_decays_with_staleness() {
        let policy = policy();
        let fresh = belief_confidence(&PerceptionSource::DirectObservation, 0, &policy);
        let slightly_stale = belief_confidence(&PerceptionSource::DirectObservation, 5, &policy);
        let stale = belief_confidence(&PerceptionSource::DirectObservation, 10, &policy);

        assert!(slightly_stale < fresh);
        assert!(stale < slightly_stale);
    }

    #[test]
    fn belief_confidence_saturates_at_zero_for_large_staleness() {
        let policy = policy();
        let stale_report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(4),
                chain_len: 5,
            },
            u64::MAX,
            &policy,
        );

        assert_eq!(stale_report, Permille::new(0).unwrap());
    }

    #[test]
    fn belief_confidence_is_deterministic_for_identical_inputs() {
        let policy = policy();
        let source = PerceptionSource::Report {
            from: entity(9),
            chain_len: 2,
        };

        assert_eq!(
            belief_confidence(&source, 7, &policy),
            belief_confidence(&source, 7, &policy)
        );
    }

    #[test]
    fn belief_confidence_uses_custom_policy_values() {
        let custom_policy = BeliefConfidencePolicy {
            direct_observation_base: Permille::new(700).unwrap(),
            report_base: Permille::new(680).unwrap(),
            rumor_base: Permille::new(660).unwrap(),
            inference_base: Permille::new(640).unwrap(),
            report_chain_penalty: Permille::new(15).unwrap(),
            rumor_chain_penalty: Permille::new(20).unwrap(),
            staleness_penalty_per_tick: Permille::new(3).unwrap(),
        };

        let custom_report = belief_confidence(
            &PerceptionSource::Report {
                from: entity(11),
                chain_len: 3,
            },
            4,
            &custom_policy,
        );

        assert_eq!(custom_report, Permille::new(638).unwrap());
    }

    #[test]
    fn default_perception_profile_carries_default_confidence_policy() {
        let profile = PerceptionProfile::default();

        assert_eq!(profile.confidence_policy, BeliefConfidencePolicy::default());
        assert_eq!(profile.institutional_memory_capacity, 20);
        assert_eq!(profile.consultation_speed_factor, Permille::new(500).unwrap());
        assert_eq!(profile.contradiction_tolerance, Permille::new(300).unwrap());
    }

    #[test]
    fn belief_types_satisfy_component_and_serde_bounds() {
        assert_component_bounds::<AgentBeliefStore>();
        assert_component_bounds::<PerceptionProfile>();
        assert_component_bounds::<TellProfile>();
        assert_ordered_traits::<MismatchKind>();
        assert_serde_bounds::<BeliefConfidencePolicy>();
        assert_serde_bounds::<BelievedEntityState>();
        assert_serde_bounds::<MismatchKind>();
        assert_serde_bounds::<SocialObservation>();
        assert_serde_bounds::<TellProfile>();
    }

    #[test]
    fn build_believed_entity_state_projects_authoritative_snapshot() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let holder = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();
        let water = world
            .create_item_lot(CommodityKind::Water, Quantity(3), Tick(1))
            .unwrap();
        let wound = sample_wound(4, 2);

        world.set_ground_location(holder, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_ground_location(water, place).unwrap();
        world.set_possessor(bread, holder).unwrap();
        world.set_possessor(water, holder).unwrap();
        world
            .insert_component_wound_list(
                holder,
                WoundList {
                    wounds: vec![wound.clone()],
                },
            )
            .unwrap();

        let snapshot = build_believed_entity_state(
            &world,
            holder,
            Tick(9),
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            },
        )
        .unwrap();

        assert_eq!(snapshot.last_known_place, Some(place));
        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([
                (CommodityKind::Bread, Quantity(2)),
                (CommodityKind::Water, Quantity(3)),
            ])
        );
        assert!(snapshot.alive);
        assert_eq!(snapshot.wounds, vec![wound]);
        assert_eq!(snapshot.observed_tick, Tick(9));
        assert_eq!(
            snapshot.source,
            PerceptionSource::Report {
                from: entity(8),
                chain_len: 2,
            }
        );
    }

    #[test]
    fn build_observed_entity_snapshot_projects_authoritative_state_without_metadata() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let holder = world
            .create_agent("Holder", ControlSource::Ai, Tick(1))
            .unwrap();
        let bread = world
            .create_item_lot(CommodityKind::Bread, Quantity(2), Tick(1))
            .unwrap();

        world.set_ground_location(holder, place).unwrap();
        world.set_ground_location(bread, place).unwrap();
        world.set_possessor(bread, holder).unwrap();

        let snapshot = build_observed_entity_snapshot(&world, holder).unwrap();

        assert_eq!(snapshot.last_known_place, Some(place));
        assert_eq!(
            snapshot.last_known_inventory,
            BTreeMap::from([(CommodityKind::Bread, Quantity(2))])
        );
        assert!(snapshot.alive);
        assert!(snapshot.wounds.is_empty());
        assert_eq!(snapshot.courage, None); // no UtilityProfile set
    }

    #[test]
    fn build_observed_entity_snapshot_captures_courage() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = world
            .create_agent("Brave", ControlSource::Ai, Tick(1))
            .unwrap();
        world.set_ground_location(agent, place).unwrap();

        let courage = Permille::new(600).unwrap();
        world
            .insert_component_utility_profile(
                agent,
                crate::UtilityProfile {
                    courage,
                    ..crate::UtilityProfile::default()
                },
            )
            .unwrap();

        let snapshot = build_observed_entity_snapshot(&world, agent).unwrap();
        assert_eq!(snapshot.courage, Some(courage));

        // Verify it propagates through to_believed_entity_state
        let believed =
            snapshot.to_believed_entity_state(Tick(2), PerceptionSource::DirectObservation);
        assert_eq!(believed.last_known_courage, Some(courage));
    }

    #[test]
    fn build_believed_entity_state_handles_dead_or_missing_entities() {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let dead = world
            .create_agent("Dead", ControlSource::Ai, Tick(1))
            .unwrap();

        world.set_ground_location(dead, place).unwrap();
        world
            .insert_component_dead_at(dead, DeadAt(Tick(5)))
            .unwrap();

        let dead_snapshot = build_believed_entity_state(
            &world,
            dead,
            Tick(7),
            PerceptionSource::Rumor { chain_len: 1 },
        )
        .unwrap();
        assert!(!dead_snapshot.alive);
        assert_eq!(dead_snapshot.last_known_place, Some(place));
        assert_eq!(
            dead_snapshot.source,
            PerceptionSource::Rumor { chain_len: 1 }
        );

        assert_eq!(
            build_believed_entity_state(
                &world,
                entity(999),
                Tick(7),
                PerceptionSource::DirectObservation,
            ),
            None
        );
    }
}
