use crate::SystemId;
use rand_chacha::{
    rand_core::{RngCore, SeedableRng},
    ChaCha8Rng,
};
use serde::{Deserialize, Serialize};
use worldwake_core::{Seed, Tick};

const SUBSTREAM_SYSTEM_BITS: u32 = 4;
const SUBSTREAM_SYSTEM_MASK: u128 = (1u128 << SUBSTREAM_SYSTEM_BITS) - 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DeterministicRng {
    rng: ChaCha8Rng,
}

impl DeterministicRng {
    #[must_use]
    pub fn new(seed: Seed) -> Self {
        Self {
            rng: ChaCha8Rng::from_seed(seed.0),
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        self.rng.next_u32()
    }

    pub fn next_u64(&mut self) -> u64 {
        self.rng.next_u64()
    }

    pub fn next_range(&mut self, low: u32, high_exclusive: u32) -> u32 {
        assert!(
            low < high_exclusive,
            "invalid range: low ({low}) must be less than high_exclusive ({high_exclusive})"
        );
        low + self.next_below(high_exclusive - low)
    }

    #[must_use]
    pub fn substream(&self, tick: Tick, system_id: SystemId, seq: u64) -> Self {
        Self::new(self.derive_substream_seed(tick, system_id, seq))
    }

    #[must_use]
    pub fn seed(&self) -> Seed {
        Seed(self.rng.get_seed())
    }

    fn next_below(&mut self, upper_exclusive: u32) -> u32 {
        debug_assert!(upper_exclusive > 0);

        let span = u64::from(upper_exclusive);
        let total = u64::from(u32::MAX) + 1;
        let zone = total - (total % span);

        loop {
            let candidate = u64::from(self.next_u32());
            if candidate < zone {
                return (candidate % span) as u32;
            }
        }
    }

    fn derive_substream_seed(&self, tick: Tick, system_id: SystemId, seq: u64) -> Seed {
        let mut deriver = ChaCha8Rng::from_seed(self.seed().0);
        deriver.set_stream(seq);
        deriver.set_word_pos(substream_word_pos(tick, system_id));

        let mut seed = [0u8; 32];
        deriver.fill_bytes(&mut seed);
        Seed(seed)
    }
}

fn substream_word_pos(tick: Tick, system_id: SystemId) -> u128 {
    (u128::from(tick.0) << SUBSTREAM_SYSTEM_BITS)
        | (u128::from(system_id.ordinal() as u64) & SUBSTREAM_SYSTEM_MASK)
}

#[cfg(test)]
mod tests {
    use super::DeterministicRng;
    use crate::SystemId;
    use serde::{de::DeserializeOwned, Serialize};
    use worldwake_core::{Seed, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}

    fn seed(byte: u8) -> Seed {
        Seed([byte; 32])
    }

    #[test]
    fn deterministic_rng_satisfies_required_traits() {
        assert_traits::<DeterministicRng>();
    }

    #[test]
    fn same_seed_yields_same_first_hundred_values() {
        let mut left = DeterministicRng::new(seed(7));
        let mut right = DeterministicRng::new(seed(7));

        let left_values = (0..100).map(|_| left.next_u32()).collect::<Vec<_>>();
        let right_values = (0..100).map(|_| right.next_u32()).collect::<Vec<_>>();

        assert_eq!(left_values, right_values);
    }

    #[test]
    fn different_seeds_yield_different_sequences() {
        let mut left = DeterministicRng::new(seed(7));
        let mut right = DeterministicRng::new(seed(8));

        let left_values = (0..16).map(|_| left.next_u64()).collect::<Vec<_>>();
        let right_values = (0..16).map(|_| right.next_u64()).collect::<Vec<_>>();

        assert_ne!(left_values, right_values);
    }

    #[test]
    fn next_range_stays_within_bounds_and_handles_single_value_span() {
        let mut rng = DeterministicRng::new(seed(9));

        for _ in 0..128 {
            let value = rng.next_range(10, 17);
            assert!((10..17).contains(&value));
        }

        assert_eq!(rng.next_range(41, 42), 41);
    }

    #[test]
    #[should_panic(expected = "invalid range")]
    fn next_range_panics_for_empty_range() {
        let mut rng = DeterministicRng::new(seed(1));
        let _ = rng.next_range(5, 5);
    }

    #[test]
    fn substream_is_deterministic_for_identical_inputs() {
        let rng = DeterministicRng::new(seed(3));
        let mut left = rng.substream(Tick(12), SystemId::Trade, 4);
        let mut right = rng.substream(Tick(12), SystemId::Trade, 4);

        let left_values = (0..32).map(|_| left.next_u32()).collect::<Vec<_>>();
        let right_values = (0..32).map(|_| right.next_u32()).collect::<Vec<_>>();

        assert_eq!(left.seed(), right.seed());
        assert_eq!(left_values, right_values);
    }

    #[test]
    fn substream_changes_when_tick_system_or_sequence_changes() {
        let rng = DeterministicRng::new(seed(4));

        let same = rng.substream(Tick(20), SystemId::Needs, 1);
        let diff_tick = rng.substream(Tick(21), SystemId::Needs, 1);
        let diff_system = rng.substream(Tick(20), SystemId::Trade, 1);
        let diff_seq = rng.substream(Tick(20), SystemId::Needs, 2);

        assert_ne!(same.seed(), diff_tick.seed());
        assert_ne!(same.seed(), diff_system.seed());
        assert_ne!(same.seed(), diff_seq.seed());
    }

    #[test]
    fn substream_creation_does_not_advance_parent_state() {
        let rng = DeterministicRng::new(seed(5));
        let before = rng.clone();

        let _ = rng.substream(Tick(33), SystemId::Combat, 9);

        assert_eq!(rng, before);
    }

    #[test]
    fn bincode_roundtrip_preserves_exact_continuation_state() {
        let mut original = DeterministicRng::new(seed(11));
        let mut control = original.clone();

        assert_eq!(original.seed(), seed(11));
        assert_eq!(original.next_u32(), control.next_u32());
        assert_eq!(original.next_u64(), control.next_u64());
        assert_eq!(original.next_range(3, 97), control.next_range(3, 97));
        assert_eq!(original.next_u32(), control.next_u32());

        let bytes = bincode::serialize(&original).unwrap();
        let mut restored: DeterministicRng = bincode::deserialize(&bytes).unwrap();

        assert_eq!(restored, original);

        let restored_values = (0..64)
            .map(|_| (restored.next_u32(), restored.next_u64()))
            .collect::<Vec<_>>();
        let control_values = (0..64)
            .map(|_| (control.next_u32(), control.next_u64()))
            .collect::<Vec<_>>();

        assert_eq!(restored_values, control_values);
    }
}
