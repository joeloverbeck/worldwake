use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub enum WaterQuality {
    Clean,
    Stale,
    Muddy,
}

#[cfg(test)]
mod tests {
    use super::WaterQuality;
    use std::collections::BTreeSet;

    #[test]
    fn water_quality_ordering_is_deterministic() {
        let qualities = BTreeSet::from([
            WaterQuality::Muddy,
            WaterQuality::Clean,
            WaterQuality::Stale,
        ]);
        let ordered: Vec<_> = qualities.into_iter().collect();

        assert_eq!(
            ordered,
            vec![
                WaterQuality::Clean,
                WaterQuality::Stale,
                WaterQuality::Muddy
            ]
        );
    }

    #[test]
    fn water_quality_serialization_roundtrip() {
        for quality in [
            WaterQuality::Clean,
            WaterQuality::Stale,
            WaterQuality::Muddy,
        ] {
            let bytes = bincode::serialize(&quality).unwrap();
            let roundtrip: WaterQuality = bincode::deserialize(&bytes).unwrap();

            assert_eq!(roundtrip, quality);
        }
    }
}
