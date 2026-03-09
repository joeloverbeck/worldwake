use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum SystemId {
    Needs,
    Production,
    Trade,
    Combat,
    Perception,
    Politics,
}

impl SystemId {
    pub const ALL: [Self; 6] = [
        Self::Needs,
        Self::Production,
        Self::Trade,
        Self::Combat,
        Self::Perception,
        Self::Politics,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Needs => "needs",
            Self::Production => "production",
            Self::Trade => "trade",
            Self::Combat => "combat",
            Self::Perception => "perception",
            Self::Politics => "politics",
        }
    }

    pub const fn ordinal(self) -> usize {
        match self {
            Self::Needs => 0,
            Self::Production => 1,
            Self::Trade => 2,
            Self::Combat => 3,
            Self::Perception => 4,
            Self::Politics => 5,
        }
    }
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

    pub fn canonical() -> Self {
        Self::new(SystemId::ALL)
        .expect("canonical system order must not contain duplicates")
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
    fn system_id_bincode_roundtrip() {
        let bytes = bincode::serialize(&SystemId::Combat).unwrap();
        let roundtrip: SystemId = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, SystemId::Combat);
    }

    #[test]
    fn manifest_rejects_duplicate_system_ids() {
        let err = SystemManifest::new([SystemId::Needs, SystemId::Trade, SystemId::Needs])
            .unwrap_err();

        assert_eq!(err, SystemManifestError::DuplicateSystemId(SystemId::Needs));
        assert_eq!(err.to_string(), "duplicate system id in manifest: needs");
    }

    #[test]
    fn manifest_preserves_insertion_order() {
        let manifest =
            SystemManifest::new([SystemId::Combat, SystemId::Needs, SystemId::Perception])
                .unwrap();

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
