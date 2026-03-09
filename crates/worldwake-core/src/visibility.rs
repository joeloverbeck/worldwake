//! Visibility classes for event perception and witness resolution.

use serde::{Deserialize, Serialize};

/// Graph-oriented visibility semantics used by event records.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum VisibilitySpec {
    ParticipantsOnly,
    SamePlace,
    AdjacentPlaces { max_hops: u8 },
    PublicRecord,
    Hidden,
}

#[cfg(test)]
mod tests {
    use super::VisibilitySpec;
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    const ALL_VISIBILITY_SPECS: [VisibilitySpec; 5] = [
        VisibilitySpec::ParticipantsOnly,
        VisibilitySpec::SamePlace,
        VisibilitySpec::AdjacentPlaces { max_hops: 2 },
        VisibilitySpec::PublicRecord,
        VisibilitySpec::Hidden,
    ];

    #[test]
    fn visibility_spec_satisfies_required_traits() {
        assert_traits::<VisibilitySpec>();
    }

    #[test]
    fn adjacent_places_stores_hop_count() {
        let spec = VisibilitySpec::AdjacentPlaces { max_hops: 2 };

        assert!(matches!(
            spec,
            VisibilitySpec::AdjacentPlaces { max_hops: 2 }
        ));
    }

    #[test]
    fn visibility_spec_order_is_declaration_stable() {
        let mut specs = ALL_VISIBILITY_SPECS;
        specs.reverse();
        specs.sort_unstable();

        assert_eq!(specs, ALL_VISIBILITY_SPECS);
    }

    #[test]
    fn visibility_spec_bincode_roundtrip_covers_every_variant() {
        for spec in ALL_VISIBILITY_SPECS {
            let bytes = bincode::serialize(&spec).unwrap();
            let roundtrip: VisibilitySpec = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, spec);
        }
    }
}
