use crate::EntityId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum RightKind {
    PhysicalPossession,
    Ownership,
    FactionAuthority,
    OfficeAuthority,
    JurisdictionalAuthority,
    ContainerAccess,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectiveRight {
    pub kind: RightKind,
    pub via: Option<EntityId>,
}
