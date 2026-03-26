//! Knowledge-path diagnostic types for decision trace instrumentation.
//!
//! Records which specific beliefs motivated each goal candidate and how
//! they were acquired. See spec S28 for design rationale.

use worldwake_core::{
    CommodityKind, EntityId, HomeostaticNeedId, InstitutionalClaim, InstitutionalKnowledgeSource,
    PerceptionSource, Permille, Quantity, Tick, WorkstationTag,
};

/// Which aspect of a believed entity contributed to candidate generation.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BeliefAspect {
    /// Entity believed to be at a place (used for co-location evidence).
    LocationAt { place: EntityId },
    /// Entity believed to have commodity inventory (seller, resource source).
    HasCommodity { commodity: CommodityKind },
    /// Entity believed to have a workstation tag.
    HasWorkstation { tag: WorkstationTag },
    /// Entity believed to be a resource source for a commodity.
    IsResourceSource { commodity: CommodityKind },
    /// Entity believed to be alive.
    Alive,
    /// Entity believed to be dead (corpse evidence).
    Dead,
    /// Entity believed to have wounds (care target).
    Wounded,
    /// Entity believed to be hostile.
    Hostile,
}

/// One belief that motivated a goal candidate, with its acquisition provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BeliefProvenance {
    /// The entity this belief is about.
    pub subject: EntityId,
    /// What aspect of the entity motivated the candidate.
    pub aspect: BeliefAspect,
    /// How the agent acquired this belief.
    pub source: PerceptionSource,
    /// When the belief was last updated.
    pub observed_tick: Tick,
}

/// One institutional belief that motivated a goal candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InstitutionalBeliefProvenance {
    /// The institutional claim that motivated the candidate.
    pub claim: InstitutionalClaim,
    /// How the agent learned about this claim.
    pub source: InstitutionalKnowledgeSource,
    /// When the agent learned this.
    pub learned_tick: Tick,
    /// Where the agent learned this (place, if known).
    pub learned_at: Option<EntityId>,
}

/// Self-knowledge that motivated a goal candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelfKnowledgeProvenance {
    /// Homeostatic need level.
    NeedLevel {
        need: HomeostaticNeedId,
        permille: Permille,
    },
    /// Agent has wounds.
    OwnWounds { count: u16 },
    /// Agent possesses commodity.
    OwnCommodity {
        commodity: CommodityKind,
        quantity: Quantity,
    },
    /// Agent has merchandise profile (merchant identity).
    MerchantIdentity,
}

/// Complete knowledge path for one goal candidate.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KnowledgePath {
    /// Self-knowledge (needs, wounds, inventory) that motivated the candidate.
    pub self_knowledge: Vec<SelfKnowledgeProvenance>,
    /// Entity beliefs (with perception source) that motivated the candidate.
    pub entity_beliefs: Vec<BeliefProvenance>,
    /// Institutional beliefs that motivated the candidate.
    pub institutional_beliefs: Vec<InstitutionalBeliefProvenance>,
}

impl KnowledgePath {
    /// Returns `true` if all three provenance vectors are empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.self_knowledge.is_empty()
            && self.entity_beliefs.is_empty()
            && self.institutional_beliefs.is_empty()
    }
}
