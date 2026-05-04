use crate::TargetSpec;
use serde::{Deserialize, Serialize};
use worldwake_core::{
    BeliefClaimKey, CommodityKind, Discrepancy, EntityId, EventTag, ExpectationId, Quantity,
    WoundCause,
};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectSchema {
    pub preconditions: Vec<EffectPrecondition>,
    pub steps: Vec<EffectStep>,
}

impl EffectSchema {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            preconditions: Vec::new(),
            steps: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectMode {
    Authoritative,
    Hypothetical,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectPrecondition {
    TargetMatchesSlot {
        slot_index: usize,
        shape: TargetSpec,
    },
    CoLocated {
        actor: EntityId,
        target: EntityId,
    },
    QuantityAvailable {
        source: EntityId,
        commodity: CommodityKind,
        min: Quantity,
    },
    CapacityFloor {
        container: EntityId,
        min_free: Quantity,
    },
    ContentionGrantHeld {
        actor: EntityId,
        affordance: EntityId,
    },
    BeliefHeld {
        agent: EntityId,
        claim: BeliefClaimKey,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectStep {
    Transfer {
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    Consume {
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    Produce {
        sink: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    ApplyWound {
        target: EntityId,
        cause: WoundCause,
    },
    EmitEvent {
        tag: EventTag,
    },
    AssertExpectationFulfilled {
        expectation: ExpectationId,
    },
    ConsumeContentionGrant {
        grant: EntityId,
    },
    PartialOnFailure {
        primary: Vec<EffectStep>,
        fallback: Vec<EffectStep>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum EffectFact {
    CommodityTransfer {
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    },
    PartialQuantity {
        requested: Quantity,
        delivered: Quantity,
    },
    WoundApplied {
        target: EntityId,
        cause: WoundCause,
    },
    ExpectationFulfilled {
        expectation: ExpectationId,
    },
    ContentionGrantConsumed {
        grant: EntityId,
    },
    EventEmitted {
        tag: EventTag,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EffectOutcome {
    pub facts: Vec<EffectFact>,
}

pub trait EffectSink {
    fn write_transfer(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    );

    fn write_consume(&mut self, source: EntityId, commodity: CommodityKind, quantity: Quantity);

    fn write_produce(&mut self, sink: EntityId, commodity: CommodityKind, quantity: Quantity);

    fn write_wound(&mut self, target: EntityId, cause: WoundCause);

    fn write_event(&mut self, tag: EventTag);

    fn assert_expectation_fulfilled(&mut self, expectation: ExpectationId);

    fn consume_grant(&mut self, grant: EntityId);
}

pub fn apply_effects(
    _schema: &EffectSchema,
    _actor: EntityId,
    _targets: &[EntityId],
    _sink: &mut dyn EffectSink,
    _mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> {
    Ok(EffectOutcome { facts: Vec::new() })
}

#[cfg(test)]
mod tests {
    use super::{EffectMode, EffectOutcome, EffectSchema, EffectSink, apply_effects};
    use worldwake_core::{CommodityKind, EntityId, EventTag, ExpectationId, Quantity, WoundCause};

    #[derive(Default)]
    struct NoopSink;

    impl EffectSink for NoopSink {
        fn write_transfer(
            &mut self,
            _source: EntityId,
            _dest: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) {
        }

        fn write_consume(
            &mut self,
            _source: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) {
        }

        fn write_produce(
            &mut self,
            _sink: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) {
        }

        fn write_wound(&mut self, _target: EntityId, _cause: WoundCause) {}

        fn write_event(&mut self, _tag: EventTag) {}

        fn assert_expectation_fulfilled(&mut self, _expectation: ExpectationId) {}

        fn consume_grant(&mut self, _grant: EntityId) {}
    }

    #[test]
    fn empty_schema_has_no_preconditions_or_steps() {
        let schema = EffectSchema::empty();

        assert!(schema.preconditions.is_empty());
        assert!(schema.steps.is_empty());
    }

    #[test]
    fn empty_schema_roundtrips_through_bincode() {
        let schema = EffectSchema::empty();

        let bytes = bincode::serialize(&schema).unwrap();
        let roundtrip: EffectSchema = bincode::deserialize(&bytes).unwrap();

        assert_eq!(roundtrip, schema);
    }

    #[test]
    fn apply_effects_returns_noop_outcome_for_empty_schema() {
        let schema = EffectSchema::empty();
        let mut sink = NoopSink;
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };

        let outcome =
            apply_effects(&schema, actor, &[], &mut sink, EffectMode::Hypothetical).unwrap();

        assert_eq!(outcome, EffectOutcome { facts: Vec::new() });
    }
}
