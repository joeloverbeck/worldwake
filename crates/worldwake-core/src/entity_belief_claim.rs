use crate::{
    CommodityKind, EntityId, Permille, Quantity, Tick, WashBasinState, WorkstationTag, Wound,
    belief::{
        BelievedActivity, BelievedArtifactState, BelievedContentionState, BelievedEvidenceState,
        PerceptionSource,
    },
    production::ResourceSource,
};
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
)]
pub struct ClaimId(pub u64);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum EntityBeliefAspect {
    Location,
    Owner,
    Holder,
    Inventory(CommodityKind),
    Alive,
    Wounded,
    Activity,
    WorkstationPresent,
    ResourceAvailable(CommodityKind),
    ContentionState,
    WashBasinState,
    Artifact,
    Courage,
    Evidence,
    LotCondition,
}

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ClaimValue {
    Place(Option<EntityId>),
    Entity(Option<EntityId>),
    Quantity(Quantity),
    Bool(bool),
    Activity(Option<BelievedActivity>),
    WorkstationTag(Option<WorkstationTag>),
    ResourceSource(Option<ResourceSource>),
    ContentionState(Option<BelievedContentionState>),
    WashBasinState(Option<WashBasinState>),
    Artifact(Option<BelievedArtifactState>),
    Courage(Option<Permille>),
    WoundSnapshot(Vec<Wound>),
    EvidenceState(Option<BelievedEvidenceState>),
    LotCondition(Option<Permille>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntityBeliefClaim {
    pub claim_id: ClaimId,
    pub subject: EntityId,
    pub aspect: EntityBeliefAspect,
    pub value: ClaimValue,
    pub source: PerceptionSource,
    pub acquired_tick: Tick,
    pub claimed_event_tick: Option<Tick>,
    pub confidence: Permille,
    #[serde(default)]
    pub refuted_at_tick: Option<Tick>,
}
