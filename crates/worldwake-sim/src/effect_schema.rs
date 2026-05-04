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
    fn check_precondition(
        &self,
        precondition: &EffectPrecondition,
        actor: EntityId,
        targets: &[EntityId],
    ) -> Result<(), Discrepancy>;

    fn checkpoint(&mut self) -> usize;

    fn restore(&mut self, checkpoint: usize) -> Result<(), Discrepancy>;

    fn write_transfer(
        &mut self,
        source: EntityId,
        dest: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_consume(
        &mut self,
        source: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_produce(
        &mut self,
        sink: EntityId,
        commodity: CommodityKind,
        quantity: Quantity,
    ) -> Result<(), Discrepancy>;

    fn write_wound(&mut self, target: EntityId, cause: WoundCause) -> Result<(), Discrepancy>;

    fn write_event(&mut self, tag: EventTag) -> Result<(), Discrepancy>;

    fn assert_expectation_fulfilled(
        &mut self,
        expectation: ExpectationId,
    ) -> Result<(), Discrepancy>;

    fn consume_grant(&mut self, grant: EntityId) -> Result<(), Discrepancy>;
}

pub fn apply_effects(
    schema: &EffectSchema,
    actor: EntityId,
    targets: &[EntityId],
    sink: &mut dyn EffectSink,
    mode: EffectMode,
) -> Result<EffectOutcome, Discrepancy> {
    match mode {
        EffectMode::Authoritative | EffectMode::Hypothetical => {}
    }

    for precondition in &schema.preconditions {
        sink.check_precondition(precondition, actor, targets)?;
    }

    let mut facts = Vec::new();
    apply_steps(&schema.steps, sink, &mut facts)?;
    Ok(EffectOutcome { facts })
}

fn apply_steps(
    steps: &[EffectStep],
    sink: &mut dyn EffectSink,
    facts: &mut Vec<EffectFact>,
) -> Result<(), Discrepancy> {
    for step in steps {
        apply_step(step, sink, facts)?;
    }
    Ok(())
}

fn apply_step(
    step: &EffectStep,
    sink: &mut dyn EffectSink,
    facts: &mut Vec<EffectFact>,
) -> Result<(), Discrepancy> {
    match step {
        EffectStep::Transfer {
            source,
            dest,
            commodity,
            quantity,
        } => {
            sink.write_transfer(*source, *dest, *commodity, *quantity)?;
            facts.push(EffectFact::CommodityTransfer {
                source: *source,
                dest: *dest,
                commodity: *commodity,
                quantity: *quantity,
            });
        }
        EffectStep::Consume {
            source,
            commodity,
            quantity,
        } => {
            sink.write_consume(*source, *commodity, *quantity)?;
        }
        EffectStep::Produce {
            sink: sink_entity,
            commodity,
            quantity,
        } => {
            sink.write_produce(*sink_entity, *commodity, *quantity)?;
        }
        EffectStep::ApplyWound { target, cause } => {
            sink.write_wound(*target, *cause)?;
            facts.push(EffectFact::WoundApplied {
                target: *target,
                cause: *cause,
            });
        }
        EffectStep::EmitEvent { tag } => {
            sink.write_event(*tag)?;
            facts.push(EffectFact::EventEmitted { tag: *tag });
        }
        EffectStep::AssertExpectationFulfilled { expectation } => {
            sink.assert_expectation_fulfilled(*expectation)?;
            facts.push(EffectFact::ExpectationFulfilled {
                expectation: *expectation,
            });
        }
        EffectStep::ConsumeContentionGrant { grant } => {
            sink.consume_grant(*grant)?;
            facts.push(EffectFact::ContentionGrantConsumed { grant: *grant });
        }
        EffectStep::PartialOnFailure { primary, fallback } => {
            let checkpoint = sink.checkpoint();
            let fact_checkpoint = facts.len();
            if apply_steps(primary, sink, facts).is_err() {
                facts.truncate(fact_checkpoint);
                sink.restore(checkpoint)?;
                apply_steps(fallback, sink, facts)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        EffectFact, EffectMode, EffectOutcome, EffectPrecondition, EffectSchema, EffectSink,
        EffectStep, apply_effects,
    };
    use worldwake_core::{CommodityKind, EntityId, EventTag, ExpectationId, Quantity, WoundCause};

    #[derive(Clone, Default)]
    struct NoopSink {
        calls: Vec<&'static str>,
        fail_next_consume: bool,
        checkpoints: Vec<(Vec<&'static str>, bool)>,
    }

    impl EffectSink for NoopSink {
        fn check_precondition(
            &self,
            _precondition: &EffectPrecondition,
            _actor: EntityId,
            _targets: &[EntityId],
        ) -> Result<(), worldwake_core::Discrepancy> {
            Ok(())
        }

        fn checkpoint(&mut self) -> usize {
            let id = self.checkpoints.len();
            self.checkpoints
                .push((self.calls.clone(), self.fail_next_consume));
            id
        }

        fn restore(&mut self, checkpoint: usize) -> Result<(), worldwake_core::Discrepancy> {
            let (calls, fail_next_consume) = self
                .checkpoints
                .get(checkpoint)
                .cloned()
                .ok_or(worldwake_core::Discrepancy::ImproperPlanningState)?;
            self.calls = calls;
            self.fail_next_consume = fail_next_consume;
            Ok(())
        }

        fn write_transfer(
            &mut self,
            _source: EntityId,
            _dest: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("transfer");
            Ok(())
        }

        fn write_consume(
            &mut self,
            _source: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("consume");
            if self.fail_next_consume {
                self.fail_next_consume = false;
                return Err(worldwake_core::Discrepancy::PartialExecutionDrift);
            }
            Ok(())
        }

        fn write_produce(
            &mut self,
            _sink: EntityId,
            _commodity: CommodityKind,
            _quantity: Quantity,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("produce");
            Ok(())
        }

        fn write_wound(
            &mut self,
            _target: EntityId,
            _cause: WoundCause,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("wound");
            Ok(())
        }

        fn write_event(&mut self, _tag: EventTag) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("event");
            Ok(())
        }

        fn assert_expectation_fulfilled(
            &mut self,
            _expectation: ExpectationId,
        ) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("expectation");
            Ok(())
        }

        fn consume_grant(&mut self, _grant: EntityId) -> Result<(), worldwake_core::Discrepancy> {
            self.calls.push("grant");
            Ok(())
        }
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
        let mut sink = NoopSink::default();
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };

        let outcome =
            apply_effects(&schema, actor, &[], &mut sink, EffectMode::Hypothetical).unwrap();

        assert_eq!(outcome, EffectOutcome { facts: Vec::new() });
    }

    #[test]
    fn apply_effects_dispatches_steps_and_returns_facts() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let target = EntityId {
            slot: 2,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![
                EffectStep::Transfer {
                    source: actor,
                    dest: target,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(3),
                },
                EffectStep::EmitEvent {
                    tag: EventTag::Transfer,
                },
                EffectStep::AssertExpectationFulfilled {
                    expectation: ExpectationId(7),
                },
                EffectStep::ConsumeContentionGrant { grant: target },
            ],
        };
        let mut sink = NoopSink::default();

        let outcome = apply_effects(
            &schema,
            actor,
            &[target],
            &mut sink,
            EffectMode::Authoritative,
        )
        .unwrap();

        assert_eq!(
            sink.calls,
            vec!["transfer", "event", "expectation", "grant"]
        );
        assert_eq!(
            outcome.facts,
            vec![
                EffectFact::CommodityTransfer {
                    source: actor,
                    dest: target,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(3),
                },
                EffectFact::EventEmitted {
                    tag: EventTag::Transfer,
                },
                EffectFact::ExpectationFulfilled {
                    expectation: ExpectationId(7),
                },
                EffectFact::ContentionGrantConsumed { grant: target },
            ]
        );
    }

    #[test]
    fn partial_on_failure_restores_sink_and_runs_fallback() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![EffectStep::PartialOnFailure {
                primary: vec![
                    EffectStep::Produce {
                        sink: actor,
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(1),
                    },
                    EffectStep::Consume {
                        source: actor,
                        commodity: CommodityKind::Bread,
                        quantity: Quantity(2),
                    },
                ],
                fallback: vec![EffectStep::EmitEvent {
                    tag: EventTag::ExpectationMismatch,
                }],
            }],
        };
        let mut sink = NoopSink {
            calls: Vec::new(),
            fail_next_consume: true,
            checkpoints: Vec::new(),
        };

        let outcome =
            apply_effects(&schema, actor, &[], &mut sink, EffectMode::Authoritative).unwrap();

        assert_eq!(sink.calls, vec!["event"]);
        assert_eq!(
            outcome.facts,
            vec![EffectFact::EventEmitted {
                tag: EventTag::ExpectationMismatch,
            }]
        );
    }

    #[test]
    fn authoritative_and_hypothetical_modes_share_interpretation() {
        let actor = EntityId {
            slot: 1,
            generation: 0,
        };
        let target = EntityId {
            slot: 2,
            generation: 0,
        };
        let schema = EffectSchema {
            preconditions: Vec::new(),
            steps: vec![
                EffectStep::Transfer {
                    source: actor,
                    dest: target,
                    commodity: CommodityKind::Bread,
                    quantity: Quantity(1),
                },
                EffectStep::EmitEvent {
                    tag: EventTag::Transfer,
                },
            ],
        };
        let mut authoritative_sink = NoopSink::default();
        let mut hypothetical_sink = NoopSink::default();

        let authoritative = apply_effects(
            &schema,
            actor,
            &[target],
            &mut authoritative_sink,
            EffectMode::Authoritative,
        )
        .unwrap();
        let hypothetical = apply_effects(
            &schema,
            actor,
            &[target],
            &mut hypothetical_sink,
            EffectMode::Hypothetical,
        )
        .unwrap();

        assert_eq!(authoritative, hypothetical);
        assert_eq!(authoritative_sink.calls, hypothetical_sink.calls);
    }
}
