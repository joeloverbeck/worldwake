use serde::{Deserialize, Serialize};

/// Resolved runtime duration for an active action instance.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ActionDuration(u32);

impl ActionDuration {
    #[must_use]
    pub const fn new(ticks: u32) -> Self {
        Self(ticks)
    }

    #[must_use]
    pub const fn ticks(self) -> u32 {
        self.0
    }

    #[must_use]
    pub const fn advance(&mut self) -> bool {
        if self.0 > 0 {
            self.0 -= 1;
        }
        self.0 == 0
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
        let mut duration = ActionDuration::new(2);
        assert_eq!(duration.ticks(), 2);
        assert!(!duration.advance());
        assert_eq!(duration, ActionDuration::new(1));
        assert!(duration.advance());
        assert_eq!(duration, ActionDuration::new(0));
        assert!(duration.advance());
    }

    #[test]
    fn action_duration_roundtrips_through_bincode() {
        let duration = ActionDuration::new(3);
        let bytes = bincode::serialize(&duration).unwrap();
        let roundtrip: ActionDuration = bincode::deserialize(&bytes).unwrap();
        assert_eq!(roundtrip, duration);
    }
}
