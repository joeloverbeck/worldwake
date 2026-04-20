use super::GoldenHarness;
use std::collections::BTreeMap;
use worldwake_ai::decision_trace::FrameTransitionKind;
use worldwake_ai::{AgentDecisionTrace, DecisionOutcome};
use worldwake_core::{
    CommodityKind, EntityId, FrameAssumption, FrameClearReason, FrameState, GoalKey,
    IntentionFrame, Tick, World,
};

pub fn commodity_assumption_falsification_probes_from_env(
) -> Option<CommodityAssumptionFalsificationProbes> {
    std::env::var("WORLDWAKE_FALSIFICATION_PROBES")
        .is_ok()
        .then(CommodityAssumptionFalsificationProbes::new)
}

#[derive(Default)]
pub struct CommodityAssumptionFalsificationProbes {
    frozen_frames: FrozenFrameTracker,
}

impl CommodityAssumptionFalsificationProbes {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe_tick(
        &mut self,
        harness: &GoldenHarness,
        agents: &BTreeMap<String, EntityId>,
        tick: Tick,
    ) -> Result<(), String> {
        let trace_sink = harness
            .driver
            .trace_sink()
            .ok_or_else(|| "decision tracing should be enabled for falsification probes".to_string())?;

        for (agent_name, agent) in agents {
            if let Some(frame) = harness.world.get_component_intention_frame(*agent) {
                validate_expected_commodity_assumption(agent_name, tick, frame)?;
            }

            if let Some(trace) = trace_sink.trace_at(*agent, tick) {
                validate_commodity_failure_transitions(agent_name, trace, &harness.world)?;
            }

            self.frozen_frames.observe_frame(
                agent_name,
                tick,
                observed_non_colocated_frame(&harness.world, *agent),
            )?;
        }

        Ok(())
    }

}

fn validate_expected_commodity_assumption(
    agent_name: &str,
    tick: Tick,
    frame: &IntentionFrame,
) -> Result<(), String> {
    let Some((commodity, place)) = frame.expected_commodity() else {
        return Ok(());
    };
    let expected = FrameAssumption::CommodityAvailableAt { commodity, place };
    if frame.assumptions.contains(&expected) {
        return Ok(());
    }

    Err(format!(
        "{agent_name} missing CommodityAvailableAt at tick {}: goal={:?}, commodity={commodity:?}, place={place}",
        tick.0, frame.goal.kind
    ))
}

fn validate_commodity_failure_transitions(
    agent_name: &str,
    trace: &AgentDecisionTrace,
    world: &World,
) -> Result<(), String> {
    let Some(transitions) = frame_transitions(trace) else {
        return Ok(());
    };

    for transition in transitions {
        let FrameTransitionKind::Cleared {
            reason: FrameClearReason::AssumptionFailed,
            failed_assumption:
                Some(FrameAssumption::CommodityAvailableAt { commodity, place }),
        } = transition
        else {
            continue;
        };

        validate_local_refutation(
            agent_name,
            trace.tick,
            world.effective_place(trace.agent),
            *commodity,
            *place,
            place_has_local_commodity_support(world, *place, *commodity),
        )?;
    }

    Ok(())
}

fn validate_local_refutation(
    agent_name: &str,
    tick: Tick,
    current_place: Option<EntityId>,
    commodity: CommodityKind,
    place: EntityId,
    local_support_present: bool,
) -> Result<(), String> {
    if current_place != Some(place) {
        return Err(format!(
            "{agent_name} cleared CommodityAvailableAt without co-location at tick {}: expected place={place}, actual={current_place:?}, commodity={commodity:?}",
            tick.0
        ));
    }

    if local_support_present {
        return Err(format!(
            "{agent_name} cleared CommodityAvailableAt despite local support at tick {}: place={place}, commodity={commodity:?}",
            tick.0
        ));
    }

    Ok(())
}

fn frame_transitions(trace: &AgentDecisionTrace) -> Option<&[FrameTransitionKind]> {
    let frame_transition = match &trace.outcome {
        DecisionOutcome::Planning(planning) => planning.frame_transition.as_ref(),
        DecisionOutcome::ActiveAction {
            frame_transition, ..
        } => frame_transition.as_ref(),
        DecisionOutcome::Dead => None,
    }?;

    Some(frame_transition.transitions.as_slice())
}

fn observed_non_colocated_frame(world: &World, agent: EntityId) -> Option<ObservedCommodityFrame> {
    let frame = world.get_component_intention_frame(agent)?;
    if frame.state != FrameState::Active {
        return None;
    }
    let (commodity, place) = frame.expected_commodity()?;
    if world.effective_place(agent) == Some(place) {
        return None;
    }

    Some(ObservedCommodityFrame {
        signature: CommodityFrameSignature {
            goal: frame.goal,
            commodity,
            place,
            established_at: frame.established_at,
            patience_limit: frame.patience_limit,
        },
        last_progress_tick: frame.last_progress_tick,
    })
}

fn place_has_local_commodity_support(world: &World, place: EntityId, commodity: CommodityKind) -> bool {
    let mut entities = vec![place];
    entities.extend(world.ground_entities_at(place));

    entities.into_iter().any(|entity| {
        world
            .get_component_item_lot(entity)
            .is_some_and(|lot| lot.commodity == commodity && lot.quantity.0 > 0)
            || world.get_component_resource_source(entity).is_some_and(|source| {
                source.commodity == commodity && source.available_quantity.0 > 0
            })
    })
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct CommodityFrameSignature {
    goal: GoalKey,
    commodity: CommodityKind,
    place: EntityId,
    established_at: Tick,
    patience_limit: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservedCommodityFrame {
    signature: CommodityFrameSignature,
    last_progress_tick: Option<Tick>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct FrozenFrameState {
    signature: CommodityFrameSignature,
    stagnant_ticks: u32,
    last_progress_tick: Option<Tick>,
}

#[derive(Default)]
struct FrozenFrameTracker {
    by_agent: BTreeMap<String, FrozenFrameState>,
}

impl FrozenFrameTracker {
    fn observe_frame(
        &mut self,
        agent_name: &str,
        tick: Tick,
        observed: Option<ObservedCommodityFrame>,
    ) -> Result<(), String> {
        let Some(observed) = observed else {
            self.by_agent.remove(agent_name);
            return Ok(());
        };

        match self.by_agent.get_mut(agent_name) {
            Some(state) if state.signature == observed.signature => {
                if state.last_progress_tick != observed.last_progress_tick {
                    state.last_progress_tick = observed.last_progress_tick;
                    state.stagnant_ticks = 0;
                    return Ok(());
                }

                state.stagnant_ticks += 1;
                if state.stagnant_ticks > state.signature.patience_limit {
                    return Err(format!(
                        "{agent_name} held frozen CommodityAvailableAt frame past patience at tick {}: commodity={:?}, place={}, stagnant_ticks={}, patience_limit={}",
                        tick.0,
                        state.signature.commodity,
                        state.signature.place,
                        state.stagnant_ticks,
                        state.signature.patience_limit
                    ));
                }
            }
            _ => {
                self.by_agent.insert(
                    agent_name.to_string(),
                    FrozenFrameState {
                        signature: observed.signature,
                        stagnant_ticks: 0,
                        last_progress_tick: observed.last_progress_tick,
                    },
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommodityFrameSignature, FrozenFrameTracker, ObservedCommodityFrame,
        validate_expected_commodity_assumption, validate_local_refutation,
    };
    use worldwake_ai::CommodityPurpose;
    use worldwake_core::{
        CommodityKind, EntityId, FrameAssumption, FrameState, GoalKey, GoalKind, IntentionDomain,
        IntentionFrame, Tick,
    };

    fn entity(slot: u32) -> EntityId {
        EntityId {
            slot,
            generation: 0,
        }
    }

    fn acquire_frame() -> IntentionFrame {
        IntentionFrame {
            goal: GoalKey::from(GoalKind::AcquireCommodity {
                commodity: CommodityKind::Apple,
                purpose: CommodityPurpose::SelfConsume,
            }),
            domain: IntentionDomain::Travel {
                destination: entity(5),
            },
            assumptions: vec![FrameAssumption::CommodityAvailableAt {
                commodity: CommodityKind::Apple,
                place: entity(5),
            }],
            state: FrameState::Active,
            established_at: Tick(3),
            last_progress_tick: None,
            stalled_ticks: 0,
            patience_limit: 2,
        }
    }

    fn observed_frame(last_progress_tick: Option<Tick>) -> ObservedCommodityFrame {
        ObservedCommodityFrame {
            signature: CommodityFrameSignature {
                goal: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Apple,
                    purpose: CommodityPurpose::SelfConsume,
                }),
                commodity: CommodityKind::Apple,
                place: entity(5),
                established_at: Tick(3),
                patience_limit: 2,
            },
            last_progress_tick,
        }
    }

    #[test]
    fn missing_expected_commodity_assumption_is_rejected() {
        let mut frame = acquire_frame();
        frame.assumptions.clear();

        let err = validate_expected_commodity_assumption("Agent A", Tick(7), &frame)
            .expect_err("missing CommodityAvailableAt should fail");
        assert!(err.contains("Agent A"));
        assert!(err.contains("tick 7"));
    }

    #[test]
    fn matching_expected_commodity_assumption_is_accepted() {
        validate_expected_commodity_assumption("Agent A", Tick(7), &acquire_frame())
            .expect("matching CommodityAvailableAt should pass");
    }

    #[test]
    fn commodity_failure_without_colocation_is_rejected() {
        let err = validate_local_refutation(
            "Agent A",
            Tick(4),
            Some(entity(2)),
            CommodityKind::Apple,
            entity(5),
            false,
        )
        .expect_err("non-colocated failure should be rejected");
        assert!(err.contains("without co-location"));
    }

    #[test]
    fn commodity_failure_with_local_support_is_rejected() {
        let err = validate_local_refutation(
            "Agent A",
            Tick(4),
            Some(entity(5)),
            CommodityKind::Apple,
            entity(5),
            true,
        )
        .expect_err("supported local failure should be rejected");
        assert!(err.contains("despite local support"));
    }

    #[test]
    fn colocated_local_absence_is_accepted() {
        validate_local_refutation(
            "Agent A",
            Tick(4),
            Some(entity(5)),
            CommodityKind::Apple,
            entity(5),
            false,
        )
        .expect("co-located local refutation should pass");
    }

    #[test]
    fn frozen_tracker_rejects_frame_past_patience() {
        let mut tracker = FrozenFrameTracker::default();

        tracker
            .observe_frame("Agent A", Tick(10), Some(observed_frame(None)))
            .expect("initial observation should pass");
        tracker
            .observe_frame("Agent A", Tick(11), Some(observed_frame(None)))
            .expect("first stagnant tick should pass");
        tracker
            .observe_frame("Agent A", Tick(12), Some(observed_frame(None)))
            .expect("second stagnant tick should pass");
        let err = tracker
            .observe_frame("Agent A", Tick(13), Some(observed_frame(None)))
            .expect_err("stagnant frame beyond patience should fail");
        assert!(err.contains("past patience"));
    }

    #[test]
    fn frozen_tracker_resets_on_progress_frame_change_and_clear() {
        let mut tracker = FrozenFrameTracker::default();

        tracker
            .observe_frame("Agent A", Tick(10), Some(observed_frame(None)))
            .expect("initial observation should pass");
        tracker
            .observe_frame("Agent A", Tick(11), Some(observed_frame(Some(Tick(11)))))
            .expect("progress should reset the tracker");
        tracker
            .observe_frame("Agent A", Tick(12), None)
            .expect("frame clear should reset the tracker");

        let replacement = ObservedCommodityFrame {
            signature: CommodityFrameSignature {
                goal: GoalKey::from(GoalKind::AcquireCommodity {
                    commodity: CommodityKind::Bread,
                    purpose: CommodityPurpose::SelfConsume,
                }),
                commodity: CommodityKind::Bread,
                place: entity(7),
                established_at: Tick(12),
                patience_limit: 2,
            },
            last_progress_tick: None,
        };
        tracker
            .observe_frame("Agent A", Tick(13), Some(replacement))
            .expect("replacement frame should start fresh");
    }
}
