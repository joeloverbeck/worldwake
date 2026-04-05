use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

macro_rules! define_system_ids {
    ($(($variant:ident, $name:literal)),+ $(,)?) => {
        #[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
        #[repr(u8)]
        pub enum SystemId {
            $($variant),+
        }

        impl SystemId {
            /// Authoritative tick order for the closed system set.
            ///
            /// The ordering is load-bearing:
            /// - `Needs` runs first so deprivation and wound pressure are visible before economic systems act.
            /// - `Production` runs before `Trade` so newly created goods exist before market exchange.
            /// - `Trade` runs before `Combat` so economic resolution happens before violence mutates the world.
            /// - `Combat` runs before `BanditCamp` so combat deaths can contribute to same-tick camp abandonment.
            /// - `BanditCamp` runs before `Contention` so abandonment is visible before later world-state systems.
            /// - `Contention` runs before `Politics` so completed exclusive actions can free resources before political resolution.
            /// - `Politics` runs before `Perception` so institutional state changes (`OfficeController`, contested state)
            ///   are visible to co-located observers in the same tick via `force_control_claims_for_event()`.
            ///   Without this ordering, `Perception` cannot project institutional beliefs from political events
            ///   (violates Principle 7: locality of information).
            /// - `Perception` runs before `EvidenceDecay` so same-tick observers can still
            ///   perceive fresh scene evidence before cleanup runs.
            /// - `EvidenceDecay` runs before `Patrol` so authoritative route adaptation only sees
            ///   scene evidence that remains live after the tick's decay boundary.
            ///
            /// Do not reorder this list casually. Any change here changes the simulation's causal sequencing.
            pub const ALL: [Self; define_system_ids!(@count $($variant),+)] = [$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $name),+
                }
            }

            pub const fn ordinal(self) -> usize {
                self as usize
            }
        }
    };
    (@count $($variant:ident),+ $(,)?) => {
        <[()]>::len(&[$(define_system_ids!(@unit $variant)),+])
    };
    (@unit $variant:ident) => {
        ()
    };
}

define_system_ids! {
    (Needs, "needs"),
    (Production, "production"),
    (Trade, "trade"),
    (Combat, "combat"),
    (ArtifactLifecycle, "artifact_lifecycle"),
    (Contention, "contention"),
    (Politics, "politics"),
    (Perception, "perception"),
    (BanditCamp, "bandit_camp"),
    (Patrol, "patrol"),
    (EvidenceDecay, "evidence_decay"),
}

impl fmt::Display for SystemId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str((*self).as_str())
    }
}

#[derive(Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub struct SystemManifest {
    ordered_ids: Box<[SystemId]>,
}

impl SystemManifest {
    pub fn new(ids: impl Into<Vec<SystemId>>) -> Result<Self, SystemManifestError> {
        let ids = ids.into();
        let mut seen = BTreeSet::new();

        for id in &ids {
            if !seen.insert(*id) {
                return Err(SystemManifestError::DuplicateSystemId(*id));
            }
        }

        Ok(Self {
            ordered_ids: ids.into_boxed_slice(),
        })
    }

    /// Returns the authoritative per-tick system order.
    ///
    /// This is the authoritative per-tick execution order. It may differ from
    /// [`SystemId::ALL`] when execution order must change without renumbering
    /// existing system ordinals.
    pub fn canonical() -> Self {
        Self::new([
            SystemId::Needs,
            SystemId::Production,
            SystemId::Trade,
            SystemId::Combat,
            SystemId::BanditCamp,
            SystemId::Contention,
            SystemId::Politics,
            SystemId::Perception,
            SystemId::EvidenceDecay,
            SystemId::Patrol,
        ])
        .expect("canonical system order must not contain duplicates")
    }

    /// Returns the authoritative pre-action system order.
    ///
    /// These systems run before input drain and action admission for the tick.
    /// They exist for world-state transitions whose timing must be authoritative
    /// before any same-tick action can begin.
    pub fn pre_action() -> Self {
        Self::new([SystemId::ArtifactLifecycle])
            .expect("pre-action system order must not contain duplicates")
    }

    pub fn ordered_ids(&self) -> &[SystemId] {
        &self.ordered_ids
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Serialize, Deserialize)]
pub enum SystemManifestError {
    DuplicateSystemId(SystemId),
}

impl fmt::Display for SystemManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSystemId(id) => write!(f, "duplicate system id in manifest: {id}"),
        }
    }
}

impl std::error::Error for SystemManifestError {}

#[cfg(test)]
mod tests {
    use super::{SystemId, SystemManifest, SystemManifestError};
    use serde::{Serialize, de::DeserializeOwned};

    fn assert_traits<
        T: Copy
            + Clone
            + Eq
            + Ord
            + std::hash::Hash
            + std::fmt::Debug
            + std::fmt::Display
            + Serialize
            + DeserializeOwned,
    >() {
    }

    #[test]
    fn system_id_satisfies_required_traits() {
        assert_traits::<SystemId>();
    }

    #[test]
    fn system_id_display_is_stable() {
        assert_eq!(SystemId::Needs.to_string(), "needs");
        assert_eq!(SystemId::Production.to_string(), "production");
        assert_eq!(SystemId::Trade.to_string(), "trade");
        assert_eq!(SystemId::Combat.to_string(), "combat");
        assert_eq!(
            SystemId::ArtifactLifecycle.to_string(),
            "artifact_lifecycle"
        );
        assert_eq!(SystemId::BanditCamp.to_string(), "bandit_camp");
        assert_eq!(SystemId::Contention.to_string(), "contention");
        assert_eq!(SystemId::Perception.to_string(), "perception");
        assert_eq!(SystemId::EvidenceDecay.to_string(), "evidence_decay");
        assert_eq!(SystemId::Politics.to_string(), "politics");
        assert_eq!(SystemId::Patrol.to_string(), "patrol");
    }

    #[test]
    fn system_id_all_matches_canonical_variant_order() {
        assert_eq!(
            SystemId::ALL,
            [
                SystemId::Needs,
                SystemId::Production,
                SystemId::Trade,
                SystemId::Combat,
                SystemId::ArtifactLifecycle,
                SystemId::Contention,
                SystemId::Politics,
                SystemId::Perception,
                SystemId::BanditCamp,
                SystemId::Patrol,
                SystemId::EvidenceDecay,
            ]
        );
    }

    #[test]
    fn system_id_ordinals_match_declaration_order() {
        for (expected, system_id) in SystemId::ALL.into_iter().enumerate() {
            assert_eq!(system_id.ordinal(), expected);
        }
    }

    #[test]
    fn system_id_ordinals_cover_dense_dispatch_range() {
        let mut covered_slots = [false; SystemId::ALL.len()];

        for system_id in SystemId::ALL {
            covered_slots[system_id.ordinal()] = true;
        }

        assert!(covered_slots.into_iter().all(std::convert::identity));
    }

    #[test]
    fn system_id_bincode_roundtrip() {
        let bytes = bincode::serialize(&SystemId::Combat).unwrap();
        let roundtrip: SystemId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, SystemId::Combat);
    }

    #[test]
    fn manifest_rejects_duplicate_system_ids() {
        let err =
            SystemManifest::new([SystemId::Needs, SystemId::Trade, SystemId::Needs]).unwrap_err();

        assert_eq!(err, SystemManifestError::DuplicateSystemId(SystemId::Needs));
        assert_eq!(err.to_string(), "duplicate system id in manifest: needs");
    }

    #[test]
    fn manifest_preserves_insertion_order() {
        let manifest =
            SystemManifest::new([SystemId::Combat, SystemId::Needs, SystemId::Perception]).unwrap();

        assert_eq!(
            manifest.ordered_ids(),
            &[SystemId::Combat, SystemId::Needs, SystemId::Perception]
        );
    }

    #[test]
    fn canonical_manifest_matches_fixed_scheduler_order() {
        let manifest = SystemManifest::canonical();

        assert_eq!(
            manifest.ordered_ids(),
            &[
                SystemId::Needs,
                SystemId::Production,
                SystemId::Trade,
                SystemId::Combat,
                SystemId::BanditCamp,
                SystemId::Contention,
                SystemId::Politics,
                SystemId::Perception,
                SystemId::EvidenceDecay,
                SystemId::Patrol,
            ]
        );
    }

    #[test]
    fn pre_action_manifest_matches_fixed_scheduler_order() {
        let manifest = SystemManifest::pre_action();

        assert_eq!(manifest.ordered_ids(), &[SystemId::ArtifactLifecycle]);
    }

    #[test]
    fn manifest_bincode_roundtrip() {
        let manifest = SystemManifest::canonical();
        let bytes = bincode::serialize(&manifest).unwrap();
        let roundtrip: SystemManifest = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, manifest);
    }
}
