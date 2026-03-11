use crate::InputEvent;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::num::NonZeroU64;
use worldwake_core::{Seed, StateHash, Tick};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayCheckpoint {
    pub tick: Tick,
    pub event_log_hash: StateHash,
    pub world_state_hash: StateHash,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayRecordingConfig {
    checkpoint_interval: Option<NonZeroU64>,
}

impl ReplayRecordingConfig {
    #[must_use]
    pub const fn disabled() -> Self {
        Self {
            checkpoint_interval: None,
        }
    }

    #[must_use]
    pub const fn every(interval: NonZeroU64) -> Self {
        Self {
            checkpoint_interval: Some(interval),
        }
    }

    #[must_use]
    pub const fn checkpoint_interval(&self) -> Option<NonZeroU64> {
        self.checkpoint_interval
    }
}

impl Default for ReplayRecordingConfig {
    fn default() -> Self {
        Self::disabled()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReplayState {
    initial_state_hash: StateHash,
    master_seed: Seed,
    terminal_tick: Tick,
    input_log: Vec<InputEvent>,
    checkpoints: Vec<ReplayCheckpoint>,
    config: ReplayRecordingConfig,
}

impl ReplayState {
    #[must_use]
    pub fn new(
        initial_hash: StateHash,
        seed: Seed,
        initial_terminal_tick: Tick,
        config: ReplayRecordingConfig,
    ) -> Self {
        Self {
            initial_state_hash: initial_hash,
            master_seed: seed,
            terminal_tick: initial_terminal_tick,
            input_log: Vec::new(),
            checkpoints: Vec::new(),
            config,
        }
    }

    pub fn record_input(&mut self, input: InputEvent) -> Result<(), ReplayStateError> {
        if let Some(previous) = self.input_log.last() {
            if input.sequence_no <= previous.sequence_no {
                return Err(ReplayStateError::NonMonotonicInputSequence {
                    previous_tick: previous.scheduled_tick,
                    previous_sequence_no: previous.sequence_no,
                    attempted_tick: input.scheduled_tick,
                    attempted_sequence_no: input.sequence_no,
                });
            }
        }

        self.input_log.push(input);
        Ok(())
    }

    pub fn record_checkpoint(
        &mut self,
        checkpoint: ReplayCheckpoint,
    ) -> Result<(), ReplayStateError> {
        if let Some(previous) = self.checkpoints.last() {
            if checkpoint.tick <= previous.tick {
                return Err(ReplayStateError::NonMonotonicCheckpoint {
                    previous_tick: previous.tick,
                    attempted_tick: checkpoint.tick,
                });
            }
        }

        self.checkpoints.push(checkpoint);
        Ok(())
    }

    pub fn set_terminal_tick(&mut self, terminal_tick: Tick) -> Result<(), ReplayStateError> {
        if terminal_tick < self.terminal_tick {
            return Err(ReplayStateError::TerminalTickRegression {
                current_terminal_tick: self.terminal_tick,
                attempted_terminal_tick: terminal_tick,
            });
        }

        self.terminal_tick = terminal_tick;
        Ok(())
    }

    #[must_use]
    pub fn should_checkpoint(&self, tick: Tick) -> bool {
        self.config
            .checkpoint_interval()
            .is_some_and(|interval| tick.0.is_multiple_of(interval.get()))
    }

    #[must_use]
    pub const fn initial_state_hash(&self) -> StateHash {
        self.initial_state_hash
    }

    #[must_use]
    pub const fn master_seed(&self) -> Seed {
        self.master_seed
    }

    #[must_use]
    pub const fn terminal_tick(&self) -> Tick {
        self.terminal_tick
    }

    #[must_use]
    pub const fn config(&self) -> &ReplayRecordingConfig {
        &self.config
    }

    #[must_use]
    pub fn input_log(&self) -> &[InputEvent] {
        &self.input_log
    }

    #[must_use]
    pub fn checkpoints(&self) -> &[ReplayCheckpoint] {
        &self.checkpoints
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayStateError {
    NonMonotonicInputSequence {
        previous_tick: Tick,
        previous_sequence_no: u64,
        attempted_tick: Tick,
        attempted_sequence_no: u64,
    },
    TerminalTickRegression {
        current_terminal_tick: Tick,
        attempted_terminal_tick: Tick,
    },
    NonMonotonicCheckpoint {
        previous_tick: Tick,
        attempted_tick: Tick,
    },
}

impl fmt::Display for ReplayStateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonMonotonicInputSequence {
                previous_tick,
                previous_sequence_no,
                attempted_tick,
                attempted_sequence_no,
            } => write!(
                f,
                "replay input sequence must increase strictly: previous=({previous_tick}, {previous_sequence_no}), attempted=({attempted_tick}, {attempted_sequence_no})"
            ),
            Self::TerminalTickRegression {
                current_terminal_tick,
                attempted_terminal_tick,
            } => write!(
                f,
                "replay terminal tick {attempted_terminal_tick} must not be earlier than current terminal tick {current_terminal_tick}"
            ),
            Self::NonMonotonicCheckpoint {
                previous_tick,
                attempted_tick,
            } => write!(
                f,
                "replay checkpoint tick {attempted_tick} must be greater than previous tick {previous_tick}"
            ),
        }
    }
}

impl std::error::Error for ReplayStateError {}

#[cfg(test)]
mod tests {
    use super::{ReplayCheckpoint, ReplayRecordingConfig, ReplayState, ReplayStateError};
    use crate::{ActionDefId, InputEvent, InputKind};
    use serde::{de::DeserializeOwned, Serialize};
    use std::num::NonZeroU64;
    use worldwake_core::{EntityId, Seed, StateHash, Tick};

    fn assert_traits<T: Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {}
    fn assert_copy_traits<T: Copy + Clone + Eq + std::fmt::Debug + Serialize + DeserializeOwned>() {
    }

    const fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 1,
        }
    }

    const fn hash(byte: u8) -> StateHash {
        StateHash([byte; 32])
    }

    fn input_event(tick: u64, sequence_no: u64, actor_slot: u32, target_slot: u32) -> InputEvent {
        InputEvent {
            scheduled_tick: Tick(tick),
            sequence_no,
            kind: InputKind::RequestAction {
                actor: entity(actor_slot),
                def_id: ActionDefId(7),
                targets: vec![entity(target_slot)],
                payload_override: None,
            },
        }
    }

    fn checkpoint(tick: u64, event_hash: u8, world_hash: u8) -> ReplayCheckpoint {
        ReplayCheckpoint {
            tick: Tick(tick),
            event_log_hash: hash(event_hash),
            world_state_hash: hash(world_hash),
        }
    }

    #[test]
    fn replay_state_satisfies_required_traits() {
        assert_traits::<ReplayState>();
    }

    #[test]
    fn replay_checkpoint_satisfies_required_traits() {
        assert_copy_traits::<ReplayCheckpoint>();
    }

    #[test]
    fn replay_recording_config_satisfies_required_traits() {
        assert_copy_traits::<ReplayRecordingConfig>();
    }

    #[test]
    fn new_stores_initial_hash_seed_and_config() {
        let config = ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap());
        let replay = ReplayState::new(hash(3), Seed([4; 32]), Tick(9), config);

        assert_eq!(replay.initial_state_hash(), hash(3));
        assert_eq!(replay.master_seed(), Seed([4; 32]));
        assert_eq!(replay.terminal_tick(), Tick(9));
        assert_eq!(replay.config(), &config);
        assert!(replay.input_log().is_empty());
        assert!(replay.checkpoints().is_empty());
    }

    #[test]
    fn record_input_preserves_insertion_order() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );
        let first = input_event(3, 0, 1, 2);
        let second = input_event(3, 1, 3, 4);
        let third = input_event(5, 2, 5, 6);

        replay.record_input(first.clone()).unwrap();
        replay.record_input(second.clone()).unwrap();
        replay.record_input(third.clone()).unwrap();

        assert_eq!(replay.input_log(), &[first, second, third]);
    }

    #[test]
    fn record_input_rejects_duplicate_or_earlier_sequence_numbers() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );
        replay.record_input(input_event(3, 4, 1, 2)).unwrap();

        let error = replay.record_input(input_event(5, 4, 3, 4)).unwrap_err();

        assert_eq!(
            error,
            ReplayStateError::NonMonotonicInputSequence {
                previous_tick: Tick(3),
                previous_sequence_no: 4,
                attempted_tick: Tick(5),
                attempted_sequence_no: 4,
            }
        );
    }

    #[test]
    fn record_checkpoint_preserves_insertion_order_for_increasing_ticks() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );
        let first = checkpoint(0, 10, 20);
        let second = checkpoint(5, 11, 21);
        let third = checkpoint(9, 12, 22);

        replay.record_checkpoint(first).unwrap();
        replay.record_checkpoint(second).unwrap();
        replay.record_checkpoint(third).unwrap();

        assert_eq!(replay.checkpoints(), &[first, second, third]);
    }

    #[test]
    fn record_checkpoint_rejects_duplicate_tick() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );
        replay.record_checkpoint(checkpoint(5, 10, 20)).unwrap();

        let error = replay.record_checkpoint(checkpoint(5, 11, 21)).unwrap_err();

        assert_eq!(
            error,
            ReplayStateError::NonMonotonicCheckpoint {
                previous_tick: Tick(5),
                attempted_tick: Tick(5),
            }
        );
    }

    #[test]
    fn record_checkpoint_rejects_earlier_tick() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );
        replay.record_checkpoint(checkpoint(8, 10, 20)).unwrap();

        let error = replay.record_checkpoint(checkpoint(3, 11, 21)).unwrap_err();

        assert_eq!(
            error,
            ReplayStateError::NonMonotonicCheckpoint {
                previous_tick: Tick(8),
                attempted_tick: Tick(3),
            }
        );
    }

    #[test]
    fn disabled_config_never_checkpoints() {
        let replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::disabled(),
        );

        assert!(!replay.should_checkpoint(Tick(0)));
        assert!(!replay.should_checkpoint(Tick(5)));
        assert!(!replay.should_checkpoint(Tick(10)));
    }

    #[test]
    fn interval_config_checkpoints_at_expected_ticks() {
        let replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap()),
        );

        assert!(replay.should_checkpoint(Tick(0)));
        assert!(!replay.should_checkpoint(Tick(4)));
        assert!(replay.should_checkpoint(Tick(5)));
        assert!(replay.should_checkpoint(Tick(10)));
        assert!(!replay.should_checkpoint(Tick(11)));
    }

    #[test]
    fn interval_one_checkpoints_every_tick() {
        let replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(0),
            ReplayRecordingConfig::every(NonZeroU64::new(1).unwrap()),
        );

        for tick in [0, 1, 2, 9, 42] {
            assert!(replay.should_checkpoint(Tick(tick)));
        }
    }

    #[test]
    fn config_accessors_expose_interval() {
        let disabled = ReplayRecordingConfig::disabled();
        let every_five = ReplayRecordingConfig::every(NonZeroU64::new(5).unwrap());

        assert_eq!(disabled.checkpoint_interval(), None);
        assert_eq!(
            every_five.checkpoint_interval(),
            Some(NonZeroU64::new(5).unwrap())
        );
    }

    #[test]
    fn replay_state_roundtrips_through_bincode() {
        let config = ReplayRecordingConfig::every(NonZeroU64::new(3).unwrap());
        let mut replay = ReplayState::new(hash(7), Seed([8; 32]), Tick(4), config);
        let first = input_event(2, 0, 1, 9);
        let second = input_event(4, 1, 2, 8);
        let checkpoint = checkpoint(6, 15, 16);
        replay.record_input(first.clone()).unwrap();
        replay.record_input(second.clone()).unwrap();
        replay.record_checkpoint(checkpoint).unwrap();

        let bytes = bincode::serialize(&replay).unwrap();
        let roundtrip: ReplayState = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, replay);
        assert_eq!(roundtrip.input_log(), &[first, second]);
        assert_eq!(roundtrip.checkpoints(), &[checkpoint]);
        assert_eq!(roundtrip.terminal_tick(), Tick(4));
        assert_eq!(roundtrip.config(), &config);
    }

    #[test]
    fn set_terminal_tick_advances_and_rejects_regression() {
        let mut replay = ReplayState::new(
            hash(1),
            Seed([2; 32]),
            Tick(3),
            ReplayRecordingConfig::disabled(),
        );

        replay.set_terminal_tick(Tick(5)).unwrap();
        assert_eq!(replay.terminal_tick(), Tick(5));

        let error = replay.set_terminal_tick(Tick(4)).unwrap_err();
        assert_eq!(
            error,
            ReplayStateError::TerminalTickRegression {
                current_terminal_tick: Tick(5),
                attempted_terminal_tick: Tick(4),
            }
        );
    }
}
