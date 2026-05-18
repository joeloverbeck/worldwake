use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
)]
pub enum OperatingMode {
    Emergency,
    #[default]
    Normal,
    Idle,
}

#[cfg(test)]
mod tests {
    use super::OperatingMode;

    #[test]
    fn operating_mode_default_is_normal() {
        assert_eq!(OperatingMode::default(), OperatingMode::Normal);
    }

    #[test]
    fn operating_mode_round_trips_through_bincode() {
        for mode in [
            OperatingMode::Emergency,
            OperatingMode::Normal,
            OperatingMode::Idle,
        ] {
            let bytes = bincode::serialize(&mode).expect("mode should serialize");
            let decoded: OperatingMode =
                bincode::deserialize(&bytes).expect("mode should deserialize");
            assert_eq!(decoded, mode);
        }
    }
}
