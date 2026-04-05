use crate::{SaveableRuntime, SimulationState};
use std::fmt;
use std::path::Path;

pub const SAVE_MAGIC: [u8; 4] = *b"WWAK";
pub const SAVE_FORMAT_VERSION: u32 = 25;
const COEXISTENCE_SAVE_FORMAT_VERSION: u32 = 24;
const LEGACY_SAVE_FORMAT_VERSION: u32 = 5;

const SAVE_HEADER_LEN: usize = SAVE_MAGIC.len() + std::mem::size_of::<u32>();
const PAYLOAD_LEN_WIDTH: usize = std::mem::size_of::<u64>();

#[derive(Debug)]
pub enum SaveError {
    Io(std::io::Error),
    Serialization(String),
    RuntimeSerialization(String),
    InvalidMagic,
    UnsupportedVersion { found: u32, expected: u32 },
    Deserialization(String),
    RuntimeDeserialization(String),
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(source) => write!(f, "save/load I/O failed: {source}"),
            Self::Serialization(message) => {
                write!(f, "failed to serialize simulation state: {message}")
            }
            Self::RuntimeSerialization(message) => {
                write!(f, "failed to serialize runtime state: {message}")
            }
            Self::InvalidMagic => f.write_str("save data does not start with Worldwake save magic"),
            Self::UnsupportedVersion { found, expected } => write!(
                f,
                "unsupported save format version {found}; expected {expected}"
            ),
            Self::Deserialization(message) => {
                write!(f, "failed to deserialize simulation state: {message}")
            }
            Self::RuntimeDeserialization(message) => {
                write!(f, "failed to deserialize runtime state: {message}")
            }
        }
    }
}

impl std::error::Error for SaveError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(source) => Some(source),
            Self::Serialization(_)
            | Self::RuntimeSerialization(_)
            | Self::InvalidMagic
            | Self::UnsupportedVersion { .. }
            | Self::Deserialization(_)
            | Self::RuntimeDeserialization(_) => None,
        }
    }
}

impl From<std::io::Error> for SaveError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn save(
    state: &SimulationState,
    runtime: Option<&dyn SaveableRuntime>,
    path: &Path,
) -> Result<(), SaveError> {
    let bytes = save_to_bytes(state, runtime)?;
    std::fs::write(path, bytes).map_err(SaveError::Io)
}

pub fn load(path: &Path) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let bytes = std::fs::read(path).map_err(SaveError::Io)?;
    load_from_bytes(&bytes)
}

pub fn save_to_bytes(
    state: &SimulationState,
    runtime: Option<&dyn SaveableRuntime>,
) -> Result<Vec<u8>, SaveError> {
    let sim_payload =
        bincode::serialize(state).map_err(|error| SaveError::Serialization(error.to_string()))?;
    let runtime_payload = runtime
        .map(SaveableRuntime::save_runtime_state)
        .transpose()?
        .unwrap_or_default();
    let sim_payload_len = u64::try_from(sim_payload.len()).map_err(|_| {
        SaveError::Serialization("simulation payload exceeds u64 length".to_string())
    })?;
    let runtime_payload_len = u64::try_from(runtime_payload.len()).map_err(|_| {
        SaveError::RuntimeSerialization("runtime payload exceeds u64 length".to_string())
    })?;
    let mut bytes = Vec::with_capacity(
        SAVE_HEADER_LEN + PAYLOAD_LEN_WIDTH * 2 + sim_payload.len() + runtime_payload.len(),
    );
    bytes.extend_from_slice(&SAVE_MAGIC);
    bytes.extend_from_slice(&SAVE_FORMAT_VERSION.to_le_bytes());
    bytes.extend_from_slice(&sim_payload_len.to_le_bytes());
    bytes.extend_from_slice(&sim_payload);
    bytes.extend_from_slice(&runtime_payload_len.to_le_bytes());
    bytes.extend_from_slice(&runtime_payload);
    Ok(bytes)
}

pub fn load_from_bytes(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    if bytes.len() < SAVE_HEADER_LEN {
        return Err(SaveError::Deserialization(
            "save data is truncated before the fixed header completes".to_string(),
        ));
    }

    let (magic, rest) = bytes.split_at(SAVE_MAGIC.len());
    if magic != SAVE_MAGIC {
        return Err(SaveError::InvalidMagic);
    }

    let (version_bytes, payload) = rest.split_at(std::mem::size_of::<u32>());
    let found = u32::from_le_bytes(
        version_bytes
            .try_into()
            .expect("validated fixed-width save header"),
    );

    match found {
        LEGACY_SAVE_FORMAT_VERSION => load_legacy_v5(payload),
        COEXISTENCE_SAVE_FORMAT_VERSION => load_coexistence_v24(payload),
        SAVE_FORMAT_VERSION => load_current_format(payload),
        _ => Err(SaveError::UnsupportedVersion {
            found,
            expected: SAVE_FORMAT_VERSION,
        }),
    }
}

fn load_legacy_v5(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let state = bincode::deserialize(bytes)
        .map_err(|error| SaveError::Deserialization(error.to_string()))?;
    Ok((state, None))
}

fn load_coexistence_v24(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let (sim_payload, rest) = split_length_prefixed_payload(bytes, "simulation")?;
    let state = legacy_v24::load_simulation_state(sim_payload)?;
    let (runtime_payload, trailing) = split_length_prefixed_payload(rest, "runtime")?;
    if !trailing.is_empty() {
        return Err(SaveError::Deserialization(
            "save data has trailing bytes after runtime payload".to_string(),
        ));
    }

    let runtime = (!runtime_payload.is_empty()).then(|| runtime_payload.to_vec());
    Ok((state, runtime))
}

fn load_current_format(bytes: &[u8]) -> Result<(SimulationState, Option<Vec<u8>>), SaveError> {
    let (sim_payload, rest) = split_length_prefixed_payload(bytes, "simulation")?;
    let state = bincode::deserialize(sim_payload)
        .map_err(|error| SaveError::Deserialization(error.to_string()))?;
    let (runtime_payload, trailing) = split_length_prefixed_payload(rest, "runtime")?;
    if !trailing.is_empty() {
        return Err(SaveError::Deserialization(
            "save data has trailing bytes after runtime payload".to_string(),
        ));
    }

    let runtime = (!runtime_payload.is_empty()).then(|| runtime_payload.to_vec());
    Ok((state, runtime))
}

fn split_length_prefixed_payload<'a>(
    bytes: &'a [u8],
    label: &str,
) -> Result<(&'a [u8], &'a [u8]), SaveError> {
    if bytes.len() < PAYLOAD_LEN_WIDTH {
        return Err(SaveError::Deserialization(format!(
            "save data is truncated before the {label} length prefix completes"
        )));
    }

    let (len_bytes, rest) = bytes.split_at(PAYLOAD_LEN_WIDTH);
    let payload_len = u64::from_le_bytes(
        len_bytes
            .try_into()
            .expect("validated fixed-width payload length"),
    );
    let payload_len = usize::try_from(payload_len).map_err(|_| {
        SaveError::Deserialization(format!(
            "{label} payload length does not fit into platform usize"
        ))
    })?;
    if rest.len() < payload_len {
        return Err(SaveError::Deserialization(format!(
            "save data is truncated before the full {label} payload completes"
        )));
    }

    Ok(rest.split_at(payload_len))
}

#[allow(clippy::wildcard_imports, clippy::zero_sized_map_values)]
mod legacy_v24 {
    use super::SaveError;
    use crate::{
        ControllerState, DeterministicRng, RecipeRegistry, ReplayState, Scheduler, SimulationState,
    };
    use serde::{Deserialize, Serialize};
    use std::collections::BTreeMap;
    use worldwake_core::*;

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct LegacyCombinedProfileV24 {
        max_candidates_to_plan: u8,
        max_plan_depth: u8,
        snapshot_travel_horizon: u8,
        max_prerequisite_locations: u8,
        max_node_expansions: u16,
        beam_width: u8,
        switch_margin: Permille,
        transient_block_ticks: u32,
        unknown_block_ticks: u32,
        structural_block_ticks: u32,
        initial_cooldown_ticks: u32,
        max_cooldown_ticks: u32,
    }

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct LegacyComponentTablesV24(
        BTreeMap<EntityId, Name>,
        BTreeMap<EntityId, AgentData>,
        BTreeMap<EntityId, WoundList>,
        BTreeMap<EntityId, CombatProfile>,
        BTreeMap<EntityId, DeadAt>,
        BTreeMap<EntityId, CombatStance>,
        BTreeMap<EntityId, ContentionDispositionProfile>,
        BTreeMap<EntityId, TheftDispositionProfile>,
        BTreeMap<EntityId, JusticeDispositionProfile>,
        BTreeMap<EntityId, UtilityProfile>,
        BTreeMap<EntityId, CommodityValuationProfile>,
        BTreeMap<EntityId, RouteExperience>,
        BTreeMap<EntityId, SourceReliability>,
        BTreeMap<EntityId, PreferenceProfile>,
        BTreeMap<EntityId, PatrolRoute>,
        BTreeMap<EntityId, PatrolProfile>,
        BTreeMap<EntityId, OfficeData>,
        BTreeMap<EntityId, OfficeForceProfile>,
        BTreeMap<EntityId, OfficeForceState>,
        BTreeMap<EntityId, FactionData>,
        BTreeMap<EntityId, RecordData>,
        BTreeMap<EntityId, ArtifactHeader>,
        BTreeMap<EntityId, BountyTerms>,
        BTreeMap<EntityId, NoticeContent>,
        BTreeMap<EntityId, BlockedIntentMemory>,
        BTreeMap<EntityId, AgentBeliefStore>,
        BTreeMap<EntityId, PerceptionProfile>,
        BTreeMap<EntityId, TellProfile>,
        BTreeMap<EntityId, CommunicationProfile>,
        BTreeMap<EntityId, LegacyCombinedProfileV24>,
        BTreeMap<EntityId, CognitiveProfile>,
        BTreeMap<EntityId, ExecutionBudget>,
        BTreeMap<EntityId, DriveThresholds>,
        BTreeMap<EntityId, HomeostaticNeeds>,
        BTreeMap<EntityId, DeprivationExposure>,
        BTreeMap<EntityId, MetabolismProfile>,
        BTreeMap<EntityId, CarryCapacity>,
        BTreeMap<EntityId, KnownRecipes>,
        BTreeMap<EntityId, DemandMemory>,
        BTreeMap<EntityId, TradeDispositionProfile>,
        BTreeMap<EntityId, MerchandiseProfile>,
        BTreeMap<EntityId, SubstitutePreferences>,
        BTreeMap<EntityId, ContentionPolicy>,
        BTreeMap<EntityId, ContentionQueue>,
        BTreeMap<EntityId, WorkstationMarker>,
        BTreeMap<EntityId, ResourceSource>,
        BTreeMap<EntityId, ProductionOutputOwnershipPolicy>,
        BTreeMap<EntityId, BanditCamp>,
        BTreeMap<EntityId, SceneEvidence>,
        BTreeMap<EntityId, BanditFactionPolicy>,
        BTreeMap<EntityId, ProductionJob>,
        BTreeMap<EntityId, InTransitOnEdge>,
        BTreeMap<EntityId, ActiveGoal>,
        BTreeMap<EntityId, ContentionIntents>,
        BTreeMap<EntityId, IntentionFrame>,
        BTreeMap<EntityId, IntentionDispositionProfile>,
        BTreeMap<EntityId, ViolationMemory>,
        BTreeMap<EntityId, ViolationDispositionProfile>,
        BTreeMap<EntityId, EpistemicDispositionProfile>,
        BTreeMap<EntityId, PursuitProfile>,
        BTreeMap<EntityId, ItemLot>,
        BTreeMap<EntityId, UniqueItem>,
        BTreeMap<EntityId, Container>,
        BTreeMap<EntityId, SaleListing>,
        BTreeMap<EntityId, StockStoragePolicy>,
        BTreeMap<EntityId, StockAssignment>,
    );

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct CurrentComponentTablesWire(
        BTreeMap<EntityId, Name>,
        BTreeMap<EntityId, AgentData>,
        BTreeMap<EntityId, WoundList>,
        BTreeMap<EntityId, CombatProfile>,
        BTreeMap<EntityId, DeadAt>,
        BTreeMap<EntityId, CombatStance>,
        BTreeMap<EntityId, ContentionDispositionProfile>,
        BTreeMap<EntityId, TheftDispositionProfile>,
        BTreeMap<EntityId, JusticeDispositionProfile>,
        BTreeMap<EntityId, UtilityProfile>,
        BTreeMap<EntityId, CommodityValuationProfile>,
        BTreeMap<EntityId, RouteExperience>,
        BTreeMap<EntityId, SourceReliability>,
        BTreeMap<EntityId, PreferenceProfile>,
        BTreeMap<EntityId, PatrolRoute>,
        BTreeMap<EntityId, PatrolProfile>,
        BTreeMap<EntityId, OfficeData>,
        BTreeMap<EntityId, OfficeForceProfile>,
        BTreeMap<EntityId, OfficeForceState>,
        BTreeMap<EntityId, FactionData>,
        BTreeMap<EntityId, RecordData>,
        BTreeMap<EntityId, ArtifactHeader>,
        BTreeMap<EntityId, BountyTerms>,
        BTreeMap<EntityId, NoticeContent>,
        BTreeMap<EntityId, BlockedIntentMemory>,
        BTreeMap<EntityId, AgentBeliefStore>,
        BTreeMap<EntityId, PerceptionProfile>,
        BTreeMap<EntityId, TellProfile>,
        BTreeMap<EntityId, CommunicationProfile>,
        BTreeMap<EntityId, CognitiveProfile>,
        BTreeMap<EntityId, ExecutionBudget>,
        BTreeMap<EntityId, DriveThresholds>,
        BTreeMap<EntityId, HomeostaticNeeds>,
        BTreeMap<EntityId, DeprivationExposure>,
        BTreeMap<EntityId, MetabolismProfile>,
        BTreeMap<EntityId, CarryCapacity>,
        BTreeMap<EntityId, KnownRecipes>,
        BTreeMap<EntityId, DemandMemory>,
        BTreeMap<EntityId, TradeDispositionProfile>,
        BTreeMap<EntityId, MerchandiseProfile>,
        BTreeMap<EntityId, SubstitutePreferences>,
        BTreeMap<EntityId, ContentionPolicy>,
        BTreeMap<EntityId, ContentionQueue>,
        BTreeMap<EntityId, WorkstationMarker>,
        BTreeMap<EntityId, ResourceSource>,
        BTreeMap<EntityId, ProductionOutputOwnershipPolicy>,
        BTreeMap<EntityId, BanditCamp>,
        BTreeMap<EntityId, SceneEvidence>,
        BTreeMap<EntityId, BanditFactionPolicy>,
        BTreeMap<EntityId, ProductionJob>,
        BTreeMap<EntityId, InTransitOnEdge>,
        BTreeMap<EntityId, ActiveGoal>,
        BTreeMap<EntityId, ContentionIntents>,
        BTreeMap<EntityId, IntentionFrame>,
        BTreeMap<EntityId, IntentionDispositionProfile>,
        BTreeMap<EntityId, ViolationMemory>,
        BTreeMap<EntityId, ViolationDispositionProfile>,
        BTreeMap<EntityId, EpistemicDispositionProfile>,
        BTreeMap<EntityId, PursuitProfile>,
        BTreeMap<EntityId, ItemLot>,
        BTreeMap<EntityId, UniqueItem>,
        BTreeMap<EntityId, Container>,
        BTreeMap<EntityId, SaleListing>,
        BTreeMap<EntityId, StockStoragePolicy>,
        BTreeMap<EntityId, StockAssignment>,
    );

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct LegacyWorldV24(
        EntityAllocator,
        LegacyComponentTablesV24,
        RelationTables,
        Topology,
    );

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct CurrentWorldWire(EntityAllocator, ComponentTables, RelationTables, Topology);

    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct LegacySimulationStateV24(
        LegacyWorldV24,
        EventLog,
        Scheduler,
        RecipeRegistry,
        ReplayState,
        ControllerState,
        DeterministicRng,
    );

    #[cfg(test)]
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct CurrentWorldCompatWire(
        EntityAllocator,
        CurrentComponentTablesWire,
        RelationTables,
        Topology,
    );

    #[cfg(test)]
    #[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
    struct CurrentSimulationStateWire(
        CurrentWorldCompatWire,
        EventLog,
        Scheduler,
        RecipeRegistry,
        ReplayState,
        ControllerState,
        DeterministicRng,
    );

    pub(super) fn load_simulation_state(bytes: &[u8]) -> Result<SimulationState, SaveError> {
        let legacy: LegacySimulationStateV24 = bincode::deserialize(bytes)
            .map_err(|error| SaveError::Deserialization(error.to_string()))?;
        legacy.into_current()
    }

    #[cfg(test)]
    pub(super) fn save_bytes(
        state: &SimulationState,
        runtime: Option<&[u8]>,
    ) -> Result<Vec<u8>, SaveError> {
        let sim_payload = bincode::serialize(state)
            .map_err(|error| SaveError::Serialization(error.to_string()))?;
        let current: CurrentSimulationStateWire = bincode::deserialize(&sim_payload)
            .map_err(|error| SaveError::Deserialization(error.to_string()))?;
        let legacy = current.into_legacy();
        let legacy_payload = bincode::serialize(&legacy)
            .map_err(|error| SaveError::Serialization(error.to_string()))?;
        let runtime_payload = runtime.unwrap_or_default();
        let sim_payload_len = u64::try_from(legacy_payload.len()).map_err(|_| {
            SaveError::Serialization("simulation payload exceeds u64 length".to_string())
        })?;
        let runtime_payload_len = u64::try_from(runtime_payload.len()).map_err(|_| {
            SaveError::RuntimeSerialization("runtime payload exceeds u64 length".to_string())
        })?;
        let mut bytes = Vec::with_capacity(
            super::SAVE_HEADER_LEN
                + super::PAYLOAD_LEN_WIDTH * 2
                + legacy_payload.len()
                + runtime_payload.len(),
        );
        bytes.extend_from_slice(&super::SAVE_MAGIC);
        bytes.extend_from_slice(&super::COEXISTENCE_SAVE_FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&sim_payload_len.to_le_bytes());
        bytes.extend_from_slice(&legacy_payload);
        bytes.extend_from_slice(&runtime_payload_len.to_le_bytes());
        bytes.extend_from_slice(runtime_payload);
        Ok(bytes)
    }

    impl LegacySimulationStateV24 {
        fn into_current(self) -> Result<SimulationState, SaveError> {
            let Self(
                world,
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                controller_state,
                rng_state,
            ) = self;
            Ok(SimulationState::new(
                world.into_current()?,
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                controller_state,
                rng_state,
            ))
        }
    }

    #[cfg(test)]
    impl CurrentSimulationStateWire {
        fn into_legacy(self) -> LegacySimulationStateV24 {
            let Self(
                world,
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                controller_state,
                rng_state,
            ) = self;
            LegacySimulationStateV24(
                world.into_legacy(),
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                controller_state,
                rng_state,
            )
        }
    }

    #[cfg(test)]
    impl CurrentWorldCompatWire {
        fn into_legacy(self) -> LegacyWorldV24 {
            let Self(allocator, components, relations, topology) = self;
            LegacyWorldV24(allocator, components.into_legacy(), relations, topology)
        }
    }

    impl LegacyWorldV24 {
        fn into_current(self) -> Result<World, SaveError> {
            let Self(allocator, components, relations, topology) = self;
            let wire = CurrentWorldWire(allocator, components.into_current()?, relations, topology);
            let bytes = bincode::serialize(&wire)
                .map_err(|error| SaveError::Serialization(error.to_string()))?;
            bincode::deserialize(&bytes).map_err(|error| SaveError::Deserialization(error.to_string()))
        }
    }

    #[cfg(test)]
    impl CurrentComponentTablesWire {
        fn into_legacy(self) -> LegacyComponentTablesV24 {
            let Self(
                names,
                agents,
                wound_lists,
                combat_profiles,
                dead_ats,
                combat_stances,
                contention_disposition_profiles,
                theft_disposition_profiles,
                justice_disposition_profiles,
                utility_profiles,
                commodity_valuation_profiles,
                route_experiences,
                source_reliabilities,
                preference_profiles,
                patrol_routes,
                patrol_profiles,
                office_data,
                office_force_profile,
                office_force_state,
                faction_data,
                record_data,
                artifact_headers,
                bounty_terms,
                notice_content,
                blocked_intent_memories,
                agent_belief_stores,
                perception_profiles,
                tell_profiles,
                communication_profiles,
                cognitive_profiles,
                execution_budgets,
                drive_thresholds,
                homeostatic_needs,
                deprivation_exposures,
                metabolism_profiles,
                carry_capacities,
                known_recipes,
                demand_memories,
                trade_disposition_profiles,
                merchandise_profiles,
                substitute_preferences,
                contention_policies,
                contention_queues,
                workstation_markers,
                resource_sources,
                production_output_ownership_policies,
                bandit_camps,
                scene_evidences,
                bandit_faction_policies,
                production_jobs,
                in_transit_on_edges,
                active_goals,
                contention_intents,
                intention_frames,
                intention_disposition_profiles,
                violation_memories,
                violation_disposition_profiles,
                epistemic_disposition_profiles,
                pursuit_profiles,
                item_lots,
                unique_items,
                containers,
                sale_listings,
                stock_storage_policies,
                stock_assignments,
            ) = self;
            let legacy_profiles = cognitive_profiles
                .iter()
                .map(|(entity, cognitive)| {
                    let execution = execution_budgets
                        .get(entity)
                        .expect("coexistence-format save requires execution budget for every cognitive profile");
                    (
                        *entity,
                        LegacyCombinedProfileV24 {
                            max_candidates_to_plan: cognitive.max_candidates_to_plan,
                            max_plan_depth: cognitive.max_plan_depth,
                            snapshot_travel_horizon: execution.snapshot_travel_horizon,
                            max_prerequisite_locations: execution.max_prerequisite_locations,
                            max_node_expansions: execution.max_node_expansions,
                            beam_width: execution.beam_width,
                            switch_margin: cognitive.switch_margin,
                            transient_block_ticks: cognitive.transient_block_ticks,
                            unknown_block_ticks: cognitive.unknown_block_ticks,
                            structural_block_ticks: cognitive.structural_block_ticks,
                            initial_cooldown_ticks: cognitive.initial_cooldown_ticks,
                            max_cooldown_ticks: cognitive.max_cooldown_ticks,
                        },
                    )
                })
                .collect();
            LegacyComponentTablesV24(
                names,
                agents,
                wound_lists,
                combat_profiles,
                dead_ats,
                combat_stances,
                contention_disposition_profiles,
                theft_disposition_profiles,
                justice_disposition_profiles,
                utility_profiles,
                commodity_valuation_profiles,
                route_experiences,
                source_reliabilities,
                preference_profiles,
                patrol_routes,
                patrol_profiles,
                office_data,
                office_force_profile,
                office_force_state,
                faction_data,
                record_data,
                artifact_headers,
                bounty_terms,
                notice_content,
                blocked_intent_memories,
                agent_belief_stores,
                perception_profiles,
                tell_profiles,
                communication_profiles,
                legacy_profiles,
                cognitive_profiles,
                execution_budgets,
                drive_thresholds,
                homeostatic_needs,
                deprivation_exposures,
                metabolism_profiles,
                carry_capacities,
                known_recipes,
                demand_memories,
                trade_disposition_profiles,
                merchandise_profiles,
                substitute_preferences,
                contention_policies,
                contention_queues,
                workstation_markers,
                resource_sources,
                production_output_ownership_policies,
                bandit_camps,
                scene_evidences,
                bandit_faction_policies,
                production_jobs,
                in_transit_on_edges,
                active_goals,
                contention_intents,
                intention_frames,
                intention_disposition_profiles,
                violation_memories,
                violation_disposition_profiles,
                epistemic_disposition_profiles,
                pursuit_profiles,
                item_lots,
                unique_items,
                containers,
                sale_listings,
                stock_storage_policies,
                stock_assignments,
            )
        }
    }

    impl LegacyComponentTablesV24 {
        fn into_current(self) -> Result<ComponentTables, SaveError> {
            let Self(
                names,
                agents,
                wound_lists,
                combat_profiles,
                dead_ats,
                combat_stances,
                contention_disposition_profiles,
                theft_disposition_profiles,
                justice_disposition_profiles,
                utility_profiles,
                commodity_valuation_profiles,
                route_experiences,
                source_reliabilities,
                preference_profiles,
                patrol_routes,
                patrol_profiles,
                office_data,
                office_force_profile,
                office_force_state,
                faction_data,
                record_data,
                artifact_headers,
                bounty_terms,
                notice_content,
                blocked_intent_memories,
                agent_belief_stores,
                perception_profiles,
                tell_profiles,
                communication_profiles,
                _legacy_profiles,
                cognitive_profiles,
                execution_budgets,
                drive_thresholds,
                homeostatic_needs,
                deprivation_exposures,
                metabolism_profiles,
                carry_capacities,
                known_recipes,
                demand_memories,
                trade_disposition_profiles,
                merchandise_profiles,
                substitute_preferences,
                contention_policies,
                contention_queues,
                workstation_markers,
                resource_sources,
                production_output_ownership_policies,
                bandit_camps,
                scene_evidences,
                bandit_faction_policies,
                production_jobs,
                in_transit_on_edges,
                active_goals,
                contention_intents,
                intention_frames,
                intention_disposition_profiles,
                violation_memories,
                violation_disposition_profiles,
                epistemic_disposition_profiles,
                pursuit_profiles,
                item_lots,
                unique_items,
                containers,
                sale_listings,
                stock_storage_policies,
                stock_assignments,
            ) = self;
            let wire = CurrentComponentTablesWire(
                names,
                agents,
                wound_lists,
                combat_profiles,
                dead_ats,
                combat_stances,
                contention_disposition_profiles,
                theft_disposition_profiles,
                justice_disposition_profiles,
                utility_profiles,
                commodity_valuation_profiles,
                route_experiences,
                source_reliabilities,
                preference_profiles,
                patrol_routes,
                patrol_profiles,
                office_data,
                office_force_profile,
                office_force_state,
                faction_data,
                record_data,
                artifact_headers,
                bounty_terms,
                notice_content,
                blocked_intent_memories,
                agent_belief_stores,
                perception_profiles,
                tell_profiles,
                communication_profiles,
                cognitive_profiles,
                execution_budgets,
                drive_thresholds,
                homeostatic_needs,
                deprivation_exposures,
                metabolism_profiles,
                carry_capacities,
                known_recipes,
                demand_memories,
                trade_disposition_profiles,
                merchandise_profiles,
                substitute_preferences,
                contention_policies,
                contention_queues,
                workstation_markers,
                resource_sources,
                production_output_ownership_policies,
                bandit_camps,
                scene_evidences,
                bandit_faction_policies,
                production_jobs,
                in_transit_on_edges,
                active_goals,
                contention_intents,
                intention_frames,
                intention_disposition_profiles,
                violation_memories,
                violation_disposition_profiles,
                epistemic_disposition_profiles,
                pursuit_profiles,
                item_lots,
                unique_items,
                containers,
                sale_listings,
                stock_storage_policies,
                stock_assignments,
            );
            let bytes = bincode::serialize(&wire)
                .map_err(|error| SaveError::Serialization(error.to_string()))?;
            bincode::deserialize(&bytes).map_err(|error| SaveError::Deserialization(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LEGACY_SAVE_FORMAT_VERSION, SAVE_FORMAT_VERSION, SAVE_MAGIC, SaveError, legacy_v24, load,
        load_from_bytes, save, save_to_bytes,
    };
    use crate::{
        ActionDefRegistry, ActionDuration, ActionHandlerRegistry, ActionInstance, ActionInstanceId,
        ActionPayload, ActionState, ActionStatus, ControllerState, DeterministicRng, InputKind,
        RecipeDefinition, RecipeRegistry, ReplayCheckpoint, ReplayRecordingConfig, ReplayState,
        SaveableRuntime, Scheduler, SimulationState, SystemDispatchTable, SystemError,
        SystemExecutionContext, SystemId, SystemManifest, TickStepServices, step_tick,
    };
    use std::num::NonZeroU64;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};
    use worldwake_core::{
        ActionDefId, ActionDomain, AgentBeliefStore, BelievedActivity, BelievedEntityState,
        BodyCostPerTick, CauseRef, CognitiveProfile, CommodityKind, ControlSource, EntityId,
        EventLog, EventPayload, ExecutionBudget, PendingEvent, PerceptionSource, Quantity,
        ReservationId, Seed, StateHash, Tick, TickRange, UniqueItemKind, VisibilitySpec,
        WitnessData, WorkstationTag, World, WorldTxn, build_prototype_world,
        test_utils::{
            sample_preference_profile, sample_route_experience, sample_source_reliability,
        },
    };

    fn state_hash(byte: u8) -> StateHash {
        StateHash([byte; 32])
    }

    fn new_txn(world: &mut World, tick: Tick, cause: CauseRef) -> WorldTxn<'_> {
        WorldTxn::new(
            world,
            tick,
            cause,
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        )
    }

    fn spawn_agent(
        world: &mut World,
        event_log: &mut EventLog,
        tick: Tick,
        name: &str,
    ) -> EntityId {
        let mut txn = new_txn(world, tick, CauseRef::Bootstrap);
        let agent = txn.create_agent(name, ControlSource::Ai).unwrap();
        let _ = txn.commit(event_log);
        agent
    }

    fn spawn_item_with_reservation(
        world: &mut World,
        event_log: &mut EventLog,
        reserver: EntityId,
    ) -> (EntityId, ReservationId) {
        let mut txn = new_txn(world, Tick(2), CauseRef::Bootstrap);
        let item = txn
            .create_item_lot(
                worldwake_core::CommodityKind::Bread,
                worldwake_core::Quantity(2),
            )
            .unwrap();
        let reservation = txn
            .try_reserve(item, reserver, TickRange::new(Tick(3), Tick(8)).unwrap())
            .unwrap();
        let _ = txn.commit(event_log);
        (item, reservation)
    }

    fn populated_recipe_registry() -> RecipeRegistry {
        let mut registry = RecipeRegistry::new();
        registry.register(RecipeDefinition {
            name: "Bake Bread".to_string(),
            inputs: vec![(CommodityKind::Grain, Quantity(2))],
            outputs: vec![(CommodityKind::Bread, Quantity(1))],
            work_ticks: std::num::NonZeroU32::new(3).unwrap(),
            required_workstation_tag: Some(WorkstationTag::Mill),
            required_tool_kinds: vec![UniqueItemKind::SimpleTool],
            body_cost_per_tick: BodyCostPerTick::zero(),
        });
        registry
    }

    fn believed_entity_state_with_activity(
        place: EntityId,
        observed_tick: Tick,
    ) -> BelievedEntityState {
        BelievedEntityState {
            last_known_place: Some(place),
            last_known_inventory: std::collections::BTreeMap::from([(
                CommodityKind::Apple,
                Quantity(3),
            )]),
            workstation_tag: Some(WorkstationTag::Mill),
            resource_source: None,
            alive: true,
            wounds: Vec::new(),
            last_known_courage: None,
            believed_activity: Some(BelievedActivity {
                action_domain: ActionDomain::Production,
                target: Some(place),
                observed_tick,
            }),
            believed_artifact: None,
            believed_contention: None,
            believed_evidence: None,
            observed_tick,
            source: PerceptionSource::DirectObservation,
        }
    }

    fn populated_state() -> (SimulationState, EntityId, EntityId, EntityId) {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let actor = spawn_agent(&mut world, &mut event_log, Tick(0), "save-actor");
        let target = spawn_agent(&mut world, &mut event_log, Tick(1), "save-target");
        let belief_place = world.topology().place_ids().next().unwrap();
        let (reserved_item, reservation) =
            spawn_item_with_reservation(&mut world, &mut event_log, actor);
        let mut beliefs = AgentBeliefStore::new();
        beliefs.update_entity(
            target,
            believed_entity_state_with_activity(belief_place, Tick(3)),
        );
        let mut belief_txn = new_txn(&mut world, Tick(3), CauseRef::Bootstrap);
        belief_txn
            .set_component_agent_belief_store(actor, beliefs)
            .unwrap();
        belief_txn
            .set_component_route_experience(actor, sample_route_experience())
            .unwrap();
        belief_txn
            .set_component_source_reliability(actor, sample_source_reliability())
            .unwrap();
        belief_txn
            .set_component_preference_profile(actor, sample_preference_profile())
            .unwrap();
        let _ = belief_txn.commit(&mut event_log);
        let _ = event_log.emit(PendingEvent::from_payload(EventPayload {
            tick: Tick(3),
            cause: CauseRef::SystemTick(Tick(3)),
            actor_id: None,
            action_name: None,
            target_ids: Vec::new(),
            evidence: Vec::new(),
            place_id: None,
            state_deltas: Vec::new(),
            observed_entities: std::collections::BTreeMap::new(),
            visibility: VisibilitySpec::Hidden,
            witness_data: WitnessData::default(),
            tags: std::collections::BTreeSet::from([worldwake_core::EventTag::System]),
        }));

        let mut scheduler = Scheduler::new_with_tick(Tick(3), SystemManifest::canonical());
        let _ = scheduler.input_queue_mut().enqueue(
            Tick(3),
            InputKind::SwitchControl {
                from: None,
                to: Some(actor),
            },
        );
        let _ = scheduler.input_queue_mut().enqueue(
            Tick(5),
            InputKind::RequestAction {
                actor,
                def_id: ActionDefId(9),
                targets: vec![target],
                payload_override: None,
                mode: crate::ActionRequestMode::Strict,
                provenance: crate::RequestProvenance::External,
            },
        );
        scheduler.insert_action(ActionInstance {
            instance_id: ActionInstanceId(7),
            def_id: ActionDefId(4),
            payload: ActionPayload::None,
            actor,
            targets: vec![target],
            start_tick: Tick(2),
            remaining_duration: ActionDuration::new(5),
            status: ActionStatus::Active,
            reservation_ids: vec![reservation],
            local_state: Some(ActionState::Empty),
            body_cost_override: None,
        });

        let mut rng = DeterministicRng::new(Seed([0x44; 32]));
        let _ = rng.next_u32();
        let _ = rng.next_u64();
        let recipe_registry = populated_recipe_registry();

        let initial_hash = SimulationState::replay_bootstrap_hash_parts(
            &world,
            &event_log,
            &scheduler,
            &recipe_registry,
            &ControllerState::with_entity(actor),
            &rng,
        )
        .unwrap();
        let mut replay_state = ReplayState::new(
            initial_hash,
            Seed([0x55; 32]),
            Tick(3),
            ReplayRecordingConfig::every(NonZeroU64::new(2).unwrap()),
        );
        replay_state
            .record_input(
                scheduler
                    .input_queue()
                    .iter_in_sequence_order()
                    .next()
                    .unwrap()
                    .clone(),
            )
            .unwrap();
        replay_state
            .record_checkpoint(ReplayCheckpoint {
                tick: Tick(4),
                event_log_hash: state_hash(0x12),
                world_state_hash: state_hash(0x34),
            })
            .unwrap();

        (
            SimulationState::new(
                world,
                event_log,
                scheduler,
                recipe_registry,
                replay_state,
                ControllerState::with_entity(actor),
                rng,
            ),
            actor,
            target,
            reserved_item,
        )
    }

    #[allow(clippy::needless_pass_by_value)]
    fn deterministic_system(context: SystemExecutionContext<'_>) -> Result<(), SystemError> {
        let roll = context.rng.next_u32();
        if context.system_id != SystemId::Needs || roll & 1 == 1 {
            return Ok(());
        }

        let mut txn = WorldTxn::new(
            context.world,
            context.tick,
            CauseRef::SystemTick(context.tick),
            None,
            None,
            VisibilitySpec::Hidden,
            WitnessData::default(),
        );
        txn.create_agent(
            &format!("{}-{}-{roll}", context.system_id.as_str(), context.tick.0),
            ControlSource::Ai,
        )
        .map_err(|error| SystemError::new(error.to_string()))?;
        let _ = txn.commit(context.event_log);
        Ok(())
    }

    fn deterministic_systems() -> SystemDispatchTable {
        SystemDispatchTable::from_handlers([deterministic_system; SystemId::ALL.len()])
    }

    fn services<'a>(
        action_defs: &'a ActionDefRegistry,
        action_handlers: &'a ActionHandlerRegistry,
        recipe_registry: &'a RecipeRegistry,
        systems: &'a SystemDispatchTable,
    ) -> TickStepServices<'a> {
        TickStepServices {
            action_defs,
            action_handlers,
            recipe_registry,
            systems,
            input_producer: None,
            action_trace: None,
            request_resolution_trace: None,
            politics_trace: None,
            perception_trace: None,
            institutional_knowledge_trace: None,
        }
    }

    fn continuation_state() -> SimulationState {
        let mut world = World::new(build_prototype_world()).unwrap();
        let mut event_log = EventLog::new();
        let first = spawn_agent(&mut world, &mut event_log, Tick(0), "continuation-a");
        let second = spawn_agent(&mut world, &mut event_log, Tick(1), "continuation-b");
        let mut scheduler = Scheduler::new_with_tick(Tick(2), SystemManifest::canonical());
        let first_input = scheduler
            .input_queue_mut()
            .enqueue(
                Tick(2),
                InputKind::SwitchControl {
                    from: Some(first),
                    to: Some(second),
                },
            )
            .clone();
        let second_input = scheduler
            .input_queue_mut()
            .enqueue(
                Tick(4),
                InputKind::SwitchControl {
                    from: Some(second),
                    to: Some(first),
                },
            )
            .clone();
        let mut rng = DeterministicRng::new(Seed([0x77; 32]));
        let _ = rng.next_u32();
        let controller = ControllerState::with_entity(first);
        let recipe_registry = populated_recipe_registry();
        let initial_hash = SimulationState::replay_bootstrap_hash_parts(
            &world,
            &event_log,
            &scheduler,
            &recipe_registry,
            &controller,
            &rng,
        )
        .unwrap();
        let mut replay_state = ReplayState::new(
            initial_hash,
            Seed([0x77; 32]),
            Tick(2),
            ReplayRecordingConfig::disabled(),
        );
        replay_state.record_input(first_input).unwrap();
        replay_state.record_input(second_input).unwrap();

        SimulationState::new(
            world,
            event_log,
            scheduler,
            recipe_registry,
            replay_state,
            controller,
            rng,
        )
    }

    fn advance_state(state: &mut SimulationState, ticks: u64) {
        let action_defs = ActionDefRegistry::new();
        let action_handlers = ActionHandlerRegistry::new();
        let systems = deterministic_systems();

        for _ in 0..ticks {
            let recipe_registry = state.recipe_registry().clone();
            let (world, event_log, scheduler, controller, rng) = state.runtime_parts_mut();
            step_tick(
                world,
                event_log,
                scheduler,
                controller,
                rng,
                services(&action_defs, &action_handlers, &recipe_registry, &systems),
            )
            .unwrap();
        }
    }

    fn temp_save_path(test_name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "worldwake-{test_name}-{}-{nanos}.bin",
            std::process::id()
        ))
    }

    fn legacy_v5_bytes(state: &SimulationState) -> Vec<u8> {
        let payload = bincode::serialize(state).unwrap();
        let mut bytes =
            Vec::with_capacity(SAVE_MAGIC.len() + std::mem::size_of::<u32>() + payload.len());
        bytes.extend_from_slice(&SAVE_MAGIC);
        bytes.extend_from_slice(&LEGACY_SAVE_FORMAT_VERSION.to_le_bytes());
        bytes.extend_from_slice(&payload);
        bytes
    }

    fn set_agent_profiles(
        state: &mut SimulationState,
        agent: EntityId,
        cognitive_profile: CognitiveProfile,
        execution_budget: ExecutionBudget,
    ) {
        let (world, event_log, ..) = state.runtime_parts_mut();
        let mut txn = new_txn(world, Tick(4), CauseRef::Bootstrap);
        txn.set_component_cognitive_profile(agent, cognitive_profile)
            .unwrap();
        txn.set_component_execution_budget(agent, execution_budget)
            .unwrap();
        let _ = txn.commit(event_log);
    }

    struct MockRuntime {
        bytes: Vec<u8>,
    }

    impl SaveableRuntime for MockRuntime {
        fn save_runtime_state(&self) -> Result<Vec<u8>, SaveError> {
            Ok(self.bytes.clone())
        }
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_full_nondefault_state() {
        let (state, actor, target, reserved_item) = populated_state();

        let bytes = save_to_bytes(&state, None).unwrap();
        let (restored, runtime) = load_from_bytes(&bytes).unwrap();

        assert_eq!(&bytes[..SAVE_MAGIC.len()], &SAVE_MAGIC);
        assert_eq!(
            u32::from_le_bytes(
                bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
                    .try_into()
                    .unwrap()
            ),
            SAVE_FORMAT_VERSION
        );
        assert_eq!(runtime, None);
        assert_eq!(restored, state);
        assert_eq!(restored.scheduler().active_actions().len(), 1);
        assert_eq!(restored.scheduler().input_queue().len(), 2);
        assert_eq!(restored.recipe_registry().len(), 1);
        assert_eq!(restored.replay_state().checkpoints().len(), 1);
        assert_eq!(restored.controller_state().controlled_entity(), Some(actor));
        assert!(!restored.world().reservations_for(reserved_item).is_empty());
        let restored_belief = restored
            .world()
            .get_component_agent_belief_store(actor)
            .and_then(|store| store.get_entity(&target))
            .unwrap();
        assert_eq!(
            restored_belief.believed_activity,
            Some(BelievedActivity {
                action_domain: ActionDomain::Production,
                target: restored_belief.last_known_place,
                observed_tick: Tick(3),
            })
        );
    }

    #[test]
    fn save_to_bytes_writes_current_format_version() {
        let (state, _, _, _) = populated_state();

        let bytes = save_to_bytes(&state, None).unwrap();
        let version_offset = SAVE_MAGIC.len();
        let version = u32::from_le_bytes(
            bytes[version_offset..version_offset + std::mem::size_of::<u32>()]
                .try_into()
                .unwrap(),
        );

        assert_eq!(version, SAVE_FORMAT_VERSION);
    }

    #[test]
    fn file_save_roundtrip_matches_in_memory_format() {
        let (state, _, _, _) = populated_state();
        let path = temp_save_path("roundtrip");
        let expected_bytes = save_to_bytes(&state, None).unwrap();

        save(&state, None, &path).unwrap();
        let file_bytes = std::fs::read(&path).unwrap();
        let (restored, runtime) = load(&path).unwrap();

        assert_eq!(file_bytes, expected_bytes);
        assert_eq!(runtime, None);
        assert_eq!(restored, state);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_to_bytes_roundtrip_preserves_runtime_payload() {
        let (state, _, _, _) = populated_state();
        let runtime = MockRuntime {
            bytes: vec![9, 8, 7, 6],
        };

        let bytes = save_to_bytes(&state, Some(&runtime)).unwrap();
        let (restored, runtime_payload) = load_from_bytes(&bytes).unwrap();

        assert_eq!(restored, state);
        assert_eq!(runtime_payload, Some(vec![9, 8, 7, 6]));
    }

    #[test]
    fn load_accepts_legacy_v5_format_and_returns_no_runtime_payload() {
        let (state, _, _, _) = populated_state();

        let (restored, runtime_payload) = load_from_bytes(&legacy_v5_bytes(&state)).unwrap();

        assert_eq!(restored, state);
        assert_eq!(runtime_payload, None);
    }

    #[test]
    fn load_migrates_coexistence_v24_format_to_split_profiles() {
        let (mut state, actor, _, _) = populated_state();
        let cognitive_profile = CognitiveProfile {
            max_candidates_to_plan: 5,
            max_plan_depth: 11,
            switch_margin: worldwake_core::Permille::new(210).unwrap(),
            transient_block_ticks: 17,
            unknown_block_ticks: 8,
            structural_block_ticks: 275,
            initial_cooldown_ticks: 9,
            max_cooldown_ticks: 120,
        };
        let execution_budget = ExecutionBudget {
            max_node_expansions: 640,
            beam_width: 13,
            snapshot_travel_horizon: 8,
            max_prerequisite_locations: 5,
        };
        set_agent_profiles(&mut state, actor, cognitive_profile, execution_budget);
        let runtime_bytes = [4, 3, 2, 1];

        let (restored, runtime_payload) =
            load_from_bytes(&legacy_v24::save_bytes(&state, Some(&runtime_bytes)).unwrap())
                .unwrap();

        assert_eq!(restored, state);
        assert_eq!(runtime_payload, Some(runtime_bytes.to_vec()));
        assert_eq!(
            restored.world().get_component_cognitive_profile(actor),
            Some(&cognitive_profile)
        );
        assert_eq!(
            restored.world().get_component_execution_budget(actor),
            Some(&execution_budget)
        );
    }

    #[test]
    fn load_rejects_wrong_magic() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes[..SAVE_MAGIC.len()].copy_from_slice(b"NOPE");

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(error, SaveError::InvalidMagic));
    }

    #[test]
    fn load_rejects_wrong_version() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
            .copy_from_slice(&(SAVE_FORMAT_VERSION + 1).to_le_bytes());

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(
            error,
            SaveError::UnsupportedVersion {
                found,
                expected: SAVE_FORMAT_VERSION
            } if found == SAVE_FORMAT_VERSION + 1
        ));
    }

    #[test]
    fn load_rejects_unsupported_intermediate_version_after_schema_bump() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        let unsupported_version = LEGACY_SAVE_FORMAT_VERSION + 1;
        bytes[SAVE_MAGIC.len()..SAVE_MAGIC.len() + std::mem::size_of::<u32>()]
            .copy_from_slice(&unsupported_version.to_le_bytes());

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(
            error,
            SaveError::UnsupportedVersion {
                found,
                expected: SAVE_FORMAT_VERSION
            } if found == unsupported_version
        ));
    }

    #[test]
    fn load_rejects_truncated_payload() {
        let (state, _, _, _) = populated_state();
        let bytes = save_to_bytes(&state, None).unwrap();

        let error = load_from_bytes(&bytes[..bytes.len() - 1]).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn load_rejects_trailing_bytes_after_runtime_payload() {
        let (state, _, _, _) = populated_state();
        let mut bytes = save_to_bytes(&state, None).unwrap();
        bytes.push(0xAA);

        let error = load_from_bytes(&bytes).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn load_rejects_truncated_header() {
        let error = load_from_bytes(&SAVE_MAGIC[..2]).unwrap_err();

        assert!(matches!(error, SaveError::Deserialization(_)));
    }

    #[test]
    fn loaded_state_continues_identically_to_uninterrupted_execution() {
        let mut uninterrupted = continuation_state();
        let (mut restored, runtime) =
            load_from_bytes(&save_to_bytes(&uninterrupted, None).unwrap()).unwrap();
        assert_eq!(runtime, None);

        advance_state(&mut uninterrupted, 4);
        advance_state(&mut restored, 4);

        assert_eq!(restored, uninterrupted);
    }
}
