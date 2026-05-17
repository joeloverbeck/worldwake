# S151TESRELROU-007: Ranking damping + candidate emission suppression for unreliable witnesses

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — threads runtime `TestimonyReliability` into candidate generation/ranking, adds `CandidateDampingReason::TestimonySourceUnreliable`, adds `TestimonyOmissionReason`, extends `extract_ask_witness_candidates`, and populates suppression payload testimony context
**Deps**: archive/tickets/S151TESRELROU-001.md, archive/tickets/S151TESRELROU-002.md, archive/tickets/S151TESRELROU-003.md, archive/tickets/S151TESRELROU-004.md, archive/tickets/S151TESRELROU-006.md

## Problem

S151's D8 wires `TestimonyReliability` into the planner's ranking pipeline two ways: (1) soft damping of AskWitness candidates whose witness trust falls between `trust_threshold` and a hard floor, and (2) hard suppression of candidates whose witness trust falls below the floor. Per Q2 delegated resolution (FND-3 + FND-26 + FND-28 + FND-29 + FND-30 favored option (a)), the suppression reason lives in a new domain-specific `TestimonyOmissionReason` enum parallel to the existing `PoliticalCandidateOmissionReason`, `BanditCandidateOmissionReason`, and `ViolationDetectionOmissionReason`.

## Assumption Reassessment (2026-05-17)

1. `apply_ask_witness_learned_damping` was the existing AskWitness learned-opportunity damping site. Live implementation keeps that path intact and composes testimony damping in `ask_witness_motive` before learned-opportunity damping. `CandidateDampingEntry` production remains in the ranking pass through a sibling testimony damping entry helper.
2. `CandidateDampingReason` at `crates/worldwake-ai/src/decision_trace.rs:416` currently carries a single variant `SurveyMemoryNegative { place, hypothesis, confidence }`. Extending with `TestimonySourceUnreliable { source, topic, trust, threshold }` adds an arm with the same Permille-sized payload shape — no derive issues.
3. `PoliticalCandidateOmissionReason` at `decision_trace.rs:545` (9 variants), `BanditCandidateOmissionReason` at line 575, `ViolationDetectionOmissionReason` at line 600 — the precedent enums for new `TestimonyOmissionReason`. All recorded through the relevant emitter's `CandidateGenerationDiagnostics` surface.
4. `extract_ask_witness_candidates` is the AskWitness extractor. The live change records unreliable-source omissions on a new `diagnostics.omitted_testimony` field and records a `CandidateSuppressionDiagnostic` with `GoalRejectionReason::SuppressedByUnreliableTestimony`, keeping stale-confidence/cooldown `ask_witness_gate_rejections` distinct.
5. Per the Authoritative-to-AI Impact Rule (AGENTS.md): this ticket modifies candidate emission (#2) and ranking damping (#3 affects rank ordering). The remaining checklist points (#1, #4-#7) need explicit consideration during implementation — no preconditions change, no validate_* functions change, no affordance enumeration changes; the omission is pre-rank so plan failure handling is not affected by it.

## Architecture Check

1. Per FND-3: the new `TestimonyOmissionReason` variant carries concrete state (`source`, `topic`, `trust`, `threshold`) — every suppression is fully attributable.
2. Per FND-26: ranking reads `TestimonyReliability` (from `AgentDecisionRuntime` directly, since it's per-agent runtime state) and `TestimonyTrustProfile` (via `GoalBeliefView` accessor from ticket 004). No cross-system command paths.
3. Per FND-28: net-new enum + net-new variant. No deprecated fallthrough — `extract_ask_witness_candidates` either emits a candidate or records an explicit omission reason; no third "silently dropped" path.
4. Per FND-29: every suppression and damping decision is inspectable through the existing decision trace surface.
5. Damping uses the existing ranking damping trace infrastructure while keeping trust computation in a shared `testimony_trust` helper used by generation and ranking.

## Completion Notes

Implemented on 2026-05-17:

1. Added shared testimony trust helpers in `crates/worldwake-ai/src/testimony_trust.rs` for trust summaries, suppression floor calculation, and proportional damping factors.
2. Threaded `AgentDecisionRuntime.testimony_reliability` through read-phase candidate generation and ranking while preserving empty defaults for public/test-only generation and ranking entry points.
3. Added hard AskWitness suppression below `trust_threshold * 500/1000`, with `TestimonyOmissionReason::SourceUnreliable`, `GoalRejectionReason::SuppressedByUnreliableTestimony`, `GoalSuppressedPayload.testimony_trust_context`, and `CandidateTrace.omitted_testimony`.
4. Added soft AskWitness damping for trust between the suppression floor and `TestimonyTrustProfile.trust_threshold`, with `CandidateDampingReason::TestimonySourceUnreliable`.
5. Updated trace/fixture initializers in `worldwake-ai`, `worldwake-cli`, and `worldwake-visualizer` for the new trace field.

No authoritative precondition, validation, affordance, action-start, or plan-failure behavior changed; the owned seam is candidate emission plus ranking only.

## Verification

Passed:

1. `cargo fmt --all`
2. `cargo test -p worldwake-ai ask_witness_emitter_suppresses_unreliable_witness`
3. `cargo test -p worldwake-ai ask_witness_motive_score_is_damped_by_unreliable_testimony_source`
4. `cargo test -p worldwake-ai goal_suppressed_event_preserves_testimony_trust_context`
5. `cargo test -p worldwake-ai agent_tick`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`

## Verification Layers

1. Candidate emission gate → focused unit test in `candidate_generation.rs#[cfg(test)]` — agent with `TestimonyReliability` entry below trust_threshold * suppression_floor for a witness; `extract_ask_witness_candidates` records a `TestimonyOmissionReason::SourceUnreliable` and does NOT emit the candidate.
2. Ranking damping → extend `ranking.rs:7664 ask_witness_motive_score_is_damped_by_learned_opportunity_memory` and add a sibling test asserting damping fires when trust falls between threshold and suppression-floor; `CandidateDampingEntry` records `TestimonySourceUnreliable`.
3. Above `minimum_observations` boundary → no damping or suppression when `TestimonyReliabilityEntry.observations < TestimonyTrustProfile.minimum_observations` (the no-signal case).
4. Decision trace → suppression and damping decisions appear in the per-tick trace with the correct reason variant; verified via decision-trace assertion test.

## What to Change

### 1. Extend `CandidateDampingReason` (`crates/worldwake-ai/src/decision_trace.rs:416`)

Add the new variant alongside `SurveyMemoryNegative`:

```rust
pub enum CandidateDampingReason {
    SurveyMemoryNegative { place: EntityId, hypothesis: ExplorationHypothesisKey, confidence: Permille },
    TestimonySourceUnreliable { source: EntityId, topic: TopicScope, trust: Permille, threshold: Permille },
}
```

### 2. Add `TestimonyOmissionReason` enum (`crates/worldwake-ai/src/decision_trace.rs`)

Co-locate with the other domain-omission enums (around line 545):

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TestimonyOmissionReason {
    SourceUnreliable {
        source: EntityId,
        topic: TopicScope,
        trust: Permille,
        threshold: Permille,
    },
}
```

### 3. Extend `extract_ask_witness_candidates` (`crates/worldwake-ai/src/candidate_generation.rs:2877-3045`)

For each candidate witness-topic pair before `emit_candidate_with_trace` at line 3033:

```rust
let entry = runtime.testimony_reliability.get(&TestimonyReliabilityKey { source: witness, topic: mapped_topic });
let profile = belief_view.testimony_trust_profile(agent);
if let Some(entry) = entry {
    if u32::from(profile.minimum_observations) <= entry.total_observations() {
        let trust = entry.trust(profile, mapped_topic);
        let suppression_floor = profile.trust_threshold.saturating_sub_pct(suppression_floor_offset);  // exact formula determined during implementation
        if trust < suppression_floor {
            diagnostics.testimony_omissions.push(TestimonyOmissionReason::SourceUnreliable { source: witness, topic: mapped_topic, trust, threshold: suppression_floor });
            continue;  // skip emission
        }
    }
}
emit_candidate_with_trace(/* ... */);
```

The `diagnostics.testimony_omissions: Vec<TestimonyOmissionReason>` is a new field on `CandidateGenerationDiagnostics` (verify its current shape during implementation — likely needs the new field added alongside `ask_witness_gate_rejections`).

### 4. Extend `apply_ask_witness_learned_damping` (`crates/worldwake-ai/src/ranking.rs:1494-1517`)

Within the existing function body, add a second-pass check on `TestimonyReliability` after the existing `learned_opportunity_memory` check:

```rust
fn apply_ask_witness_learned_damping(candidate, context, base) -> ... {
    // existing learned_opportunity_memory damping ...
    let damped = /* existing path */;

    // S151 testimony damping (only fires when minimum_observations met but trust below threshold and above suppression floor)
    if let Some(entry) = context.testimony_reliability.get(&key) {
        if u32::from(profile.minimum_observations) <= entry.total_observations() {
            let trust = entry.trust(profile, key.topic);
            if trust < profile.trust_threshold {
                let damping_factor = compute_testimony_damping_factor(trust, profile.trust_threshold);
                let damped_after_testimony = scale_motive_by_confidence(damped, damping_factor);
                damping_trace.push(CandidateDampingEntry {
                    goal_key: candidate.goal_key,
                    reason: CandidateDampingReason::TestimonySourceUnreliable {
                        source: key.source, topic: key.topic, trust, threshold: profile.trust_threshold,
                    },
                });
                return damped_after_testimony;
            }
        }
    }
    damped
}
```

The `compute_testimony_damping_factor` helper produces the Permille factor proportional to `(trust_threshold - trust)` — closed-form determined during implementation.

### 5. Populate `GoalSuppressedPayload.testimony_trust_context`

When `extract_ask_witness_candidates` suppresses a candidate via `TestimonyOmissionReason`, the corresponding `GoalSuppressed` decision-event payload (emitted at the planner's suppression path) populates `testimony_trust_context: Vec<TestimonyTrustSummary>` with the relevant witness summary. Reuse the snapshot helper introduced by ticket 006.

## Files to Touch

- `crates/worldwake-ai/src/decision_trace.rs` (modify — `CandidateDampingReason` variant + new `TestimonyOmissionReason` enum + likely new `Vec<TestimonyOmissionReason>` field on `CandidateGenerationDiagnostics`)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify — extend `extract_ask_witness_candidates`)
- `crates/worldwake-ai/src/ranking.rs` (modify — extend `apply_ask_witness_learned_damping`)
- `crates/worldwake-ai/src/testimony_trust.rs` (new — shared trust summary/floor/damping helpers)
- `crates/worldwake-ai/src/agent_tick/observation.rs` (modify — pass runtime testimony reliability through read phase)
- `crates/worldwake-ai/src/agent_tick/mod.rs` (modify — `GoalSuppressed` payload context population and trace field propagation)
- `crates/worldwake-core/src/decision_event_payload.rs` (modify — add `GoalRejectionReason::SuppressedByUnreliableTestimony`)
- Trace fixture fallout in `worldwake-ai`, `worldwake-cli`, and `worldwake-visualizer` for the new `CandidateTrace.omitted_testimony` field.

## Out of Scope

- Travel-cost integration with `RoutePreference` — ticket 008 (separate D8 site)
- Observation-phase hook that populates `TestimonyReliability` — ticket 006
- Diagnostics aggregator that reads emitted omission events — ticket 009
- `SAVE_FORMAT_VERSION` bump — ticket 010

## Acceptance Criteria

### Tests That Must Pass

1. AskWitness candidate with witness trust below `trust_threshold * suppression_floor` is suppressed; `TestimonyOmissionReason::SourceUnreliable` recorded in `CandidateGenerationDiagnostics`.
2. AskWitness candidate with witness trust between `trust_threshold * suppression_floor` and `trust_threshold` is damped; `CandidateDampingReason::TestimonySourceUnreliable` recorded in `CandidateDampingEntry`.
3. AskWitness candidate with witness observations below `minimum_observations` passes through unchanged (no signal, no damping, no suppression).
4. Existing `ask_witness_motive_score_is_damped_by_learned_opportunity_memory` (ranking.rs:7664) continues to pass — the new path composes with, not replaces, the existing `learned_opportunity_memory` damping.
5. `GoalSuppressedPayload.testimony_trust_context` is populated for the suppression case.
6. Existing suite: `cargo test --workspace`.

### Invariants

1. Trust threshold is per-agent via `TestimonyTrustProfile.trust_threshold` — no global threshold.
2. Damping never inverts ordering between candidates that share identical inputs (deterministic).
3. Suppression is recorded with full `(source, topic, trust, threshold)` attribution — no opaque drops.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs#[cfg(test)]` — new test for testimony suppression (below floor → omission recorded, candidate not emitted).
2. `crates/worldwake-ai/src/ranking.rs#[cfg(test)]` — new test for testimony damping (between floor and threshold → damped); extend `ask_witness_motive_score_is_damped_by_learned_opportunity_memory` to verify composition with the new path.
3. Decision trace assertion in an integration test verifying the suppression / damping decision appears with the correct reason variant.

### Commands

1. `cargo test -p worldwake-ai ask_witness_emitter_suppresses_unreliable_witness`
2. `cargo test -p worldwake-ai ask_witness_motive_score_is_damped_by_unreliable_testimony_source`
3. `cargo test -p worldwake-ai goal_suppressed_event_preserves_testimony_trust_context`
4. `cargo test -p worldwake-ai agent_tick`
5. `cargo clippy --workspace --all-targets -- -D warnings`
6. `cargo test --workspace`

## Outcome

Completed on 2026-05-17.

What changed:

1. Added shared testimony trust helpers for `TestimonyReliability` summary lookup, suppression-floor calculation, and proportional damping.
2. Threaded runtime testimony reliability into the read-phase candidate-generation and ranking paths without changing authoritative validation, affordances, action start, or plan-failure handling.
3. Added hard AskWitness suppression below `trust_threshold * 500/1000`, with `TestimonyOmissionReason::SourceUnreliable`, `GoalRejectionReason::SuppressedByUnreliableTestimony`, `GoalSuppressedPayload.testimony_trust_context`, and `CandidateTrace.omitted_testimony`.
4. Added soft AskWitness ranking damping between the suppression floor and `TestimonyTrustProfile.trust_threshold`, with `CandidateDampingReason::TestimonySourceUnreliable`.
5. Updated downstream trace fixtures in `worldwake-ai`, `worldwake-cli`, and `worldwake-visualizer` for the new trace field.

Deviation from original plan:

1. The landed code uses `TellTopic` in AI trace diagnostics and maps to `TopicScope` only for `TestimonyReliabilityKey` / `TestimonyTrustSummary`; this matches the live AskWitness goal key and avoids lossy reverse mapping in candidate/ranking traces.
2. Goal suppression payload context is populated in `agent_tick/mod.rs` via `CandidateSuppressionDiagnostic`, not `agent_tick/planning.rs`.

Verification result:

1. `cargo fmt --all`
2. `cargo test -p worldwake-ai ask_witness_emitter_suppresses_unreliable_witness`
3. `cargo test -p worldwake-ai ask_witness_motive_score_is_damped_by_unreliable_testimony_source`
4. `cargo test -p worldwake-ai goal_suppressed_event_preserves_testimony_trust_context`
5. `cargo test -p worldwake-ai agent_tick`
6. `cargo clippy --workspace --all-targets -- -D warnings`
7. `cargo test --workspace`
