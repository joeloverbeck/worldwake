use serde::{Deserialize, Serialize};

/// Resolved runtime duration for an active action instance.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ActionDuration {
    Finite(u32),
    Indefinite,
}

impl ActionDuration {
    #[must_use]
    pub const fn fixed_ticks(self) -> Option<u32> {
        match self {
            Self::Finite(ticks) => Some(ticks),
            Self::Indefinite => None,
        }
    }

    #[must_use]
    pub const fn advance(&mut self) -> bool {
        match self {
            Self::Finite(remaining_ticks) => {
                if *remaining_ticks > 0 {
                    *remaining_ticks -= 1;
                }
                *remaining_ticks == 0
            }
            Self::Indefinite => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ActionDuration;
    use serde::{de::DeserializeOwned, Serialize};

    fn assert_traits<T: Copy + Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    #[test]
    fn action_duration_satisfies_required_traits() {
        assert_traits::<ActionDuration>();
    }

    #[test]
    fn finite_duration_exposes_ticks_and_counts_down_to_completion() {
        let mut duration = ActionDuration::Finite(2);
        assert_eq!(duration.fixed_ticks(), Some(2));
        assert!(!duration.advance());
        assert_eq!(duration, ActionDuration::Finite(1));
        assert!(duration.advance());
        assert_eq!(duration, ActionDuration::Finite(0));
        assert!(duration.advance());
    }

    #[test]
    fn indefinite_duration_never_auto_completes() {
        let mut duration = ActionDuration::Indefinite;
        assert_eq!(duration.fixed_ticks(), None);
        assert!(!duration.advance());
        assert_eq!(duration, ActionDuration::Indefinite);
    }

    #[test]
    fn action_duration_roundtrips_through_bincode() {
        for duration in [ActionDuration::Finite(3), ActionDuration::Indefinite] {
            let bytes = bincode::serialize(&duration).unwrap();
            let roundtrip: ActionDuration = bincode::deserialize(&bytes).unwrap();
            assert_eq!(roundtrip, duration);
        }
    }
}
