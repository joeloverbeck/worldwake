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
            /// - `Combat` runs before `Perception` so observers can react to the current tick's outcomes.
            /// - `Perception` runs before `Politics` so social systems consume freshly propagated local information.
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
    (Perception, "perception"),
    (Politics, "politics"),
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
    /// This must stay aligned with [`SystemId::ALL`]. Reordering it changes the
    /// simulation's causal sequencing and should only happen with an explicit
    /// architecture decision.
    pub fn canonical() -> Self {
        Self::new(SystemId::ALL).expect("canonical system order must not contain duplicates")
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
    use serde::{de::DeserializeOwned, Serialize};

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
        assert_eq!(SystemId::Perception.to_string(), "perception");
        assert_eq!(SystemId::Politics.to_string(), "politics");
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
                SystemId::Perception,
                SystemId::Politics,
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
                SystemId::Perception,
                SystemId::Politics,
            ]
        );
    }

    #[test]
    fn manifest_bincode_roundtrip() {
        let manifest = SystemManifest::canonical();
        let bytes = bincode::serialize(&manifest).unwrap();
        let roundtrip: SystemManifest = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, manifest);
    }
}
