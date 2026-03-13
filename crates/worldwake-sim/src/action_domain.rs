use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum ActionDomain {
    Generic,
    Needs,
    Production,
    Trade,
    Travel,
    Transport,
    Combat,
    Care,
    Corpse,
}

impl ActionDomain {
    #[must_use]
    pub const fn counts_as_combat_engagement(self) -> bool {
        matches!(self, Self::Combat)
    }
}

#[cfg(test)]
mod tests {
    use super::ActionDomain;
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<
        T: Copy + Clone + Eq + Ord + std::hash::Hash + std::fmt::Debug + Serialize + DeserializeOwned,
    >() {
    }

    const ALL_DOMAINS: [ActionDomain; 9] = [
        ActionDomain::Generic,
        ActionDomain::Needs,
        ActionDomain::Production,
        ActionDomain::Trade,
        ActionDomain::Travel,
        ActionDomain::Transport,
        ActionDomain::Combat,
        ActionDomain::Care,
        ActionDomain::Corpse,
    ];

    #[test]
    fn action_domain_satisfies_required_traits() {
        assert_traits::<ActionDomain>();
    }

    #[test]
    fn action_domain_roundtrips_through_bincode() {
        for domain in ALL_DOMAINS {
            let bytes = bincode::serialize(&domain).unwrap();
            let roundtrip: ActionDomain = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, domain);
        }
    }

    #[test]
    fn only_combat_domain_counts_as_combat_engagement() {
        for domain in ALL_DOMAINS {
            assert_eq!(
                domain.counts_as_combat_engagement(),
                domain == ActionDomain::Combat
            );
        }
    }
}
