# S139EPISENSUB-004: emit_ask_witness_candidates emitter

**Status**: COMPLETED
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` (`candidate_generation.rs`) and `worldwake-core` (`decision_event_payload.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, archive/tickets/S139EPISENSUB-002.md, archive/tickets/S139EPISENSUB-003.md

## Problem

Before this ticket, `GoalKind::AskWitness` (ticket 001), `EpistemicDispositionProfile.witness_recency_preference` (ticket 002), and the dispatch declaration (ticket 003) were in place, but the AI crate could not emit `AskWitness` candidates because no emitter existed. The planner could not adopt the goal unless candidate generation surfaced it. This ticket added `emit_ask_witness_candidates` to `crates/worldwake-ai/src/candidate_generation.rs` and wired it into the live candidate-generation pass.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `candidate_generation.rs` structures emitters as functions taking `(candidates: &mut Vec<GoalOffer>, diagnostics: &mut CandidateGenerationDiagnostics, ctx: &GenerationContext<'_>)`. `GenerationContext` exposes `view: &'a dyn GoalBeliefView`, `agent: EntityId`, `place: Option<EntityId>`, `blocked: &'a BlockerMemory`, and `current_tick: Tick`. Emitters call `emit_candidate_with_trace` to produce `GoalOffer` records. The per-target enumeration pattern that S139's emitter mirrors is `emit_engage_hostile_goals`.
2. After ticket 001, `GoalBeliefView::entity_beliefs_sourced_from_witness(agent, witness)` is the primary read for the emitter trigger. After ticket 002, `EpistemicDispositionProfile.witness_recency_preference` and `stale_evidence_barrier_threshold` are reachable through the existing `epistemic_disposition_profile(actor)` accessor at `belief_view.rs:548`.
3. Shared abstraction boundary under audit: candidate-generation phase entrypoint. The emitter is added directly in `generate_candidates_with_memories_with_travel_horizon_impl` at the same call-site tier as `emit_social_candidates`. The phase ordering does not affect correctness for this emitter because emission is filter-gated, not order-dependent.
4. Live `GoalKind` under test: `GoalKind::AskWitness` (added by ticket 001). The structural analog for the emitter pattern is `emit_engage_hostile_goals` (per-target enumeration with `Evidence::with_entity`).
5. Existing inline tests in `candidate_generation.rs`'s `#[cfg(test)]` block (line 6591) exercise emitter wiring through fixture `GoalBeliefView` impls. New tests for `emit_ask_witness_candidates` follow the same pattern.
6. Phase distinction (precision-rules Rule 1): this ticket lives in *candidate generation*. Ranking (priority class, motive_score) belongs to ticket 005. Suppression (`EPISTEMIC_SENSING_POLICY` at high stress) belongs to ticket 003's policy + ranking-side dispatch. The emitter's job is "should this candidate exist?" — the per-tick gate. The ranking layer decides relative priority among emitted candidates.

## Architecture Check

1. The emitter scans the belief envelope for testimony provenance (FND-14, FND-15). Co-located witnesses are enumerated from the actor's current place through `entities_at(place)` plus `EntityKind::Agent`, which is a same-tick physical observation surface under FND-14A; testimony subjects come from `view.entity_beliefs_sourced_from_witness`.
2. Per-topic emission cap `K = 3` prevents fan-out when many co-located witnesses match. The cap is enforced inside the emitter (in the candidate-generation phase), not at the ranking layer — this preserves the "emit if gate passes, rank emitted set" separation (precision-rules Rule 1).
3. Cooldown gate consumes the existing `AskWitnessMemory` substrate at `belief.rs:1763`. No new memory type; the gate reuses the same data that `epistemic_actions.rs::apply_ask_witness_commit:460-466` writes. Single source of truth.

## Verification Layers

1. Emitter fires for stale-belief case → focused unit test against a fixture `GoalBeliefView` impl with one belief entry whose `PerceptionSource::Report { from: witness, .. }` matches a co-located witness AND confidence is below `stale_evidence_barrier_threshold`. Expected: one `GoalOffer` with `GoalKind::AskWitness { witness, topic }` emitted.
2. Emitter does NOT fire when confidence ≥ threshold → focused unit test: same fixture but with confidence above threshold; emitter emits zero candidates.
3. Emitter does NOT fire when cooldown active → focused unit test: fixture with `AskWitnessMemory { asked_tick: current_tick - 1 }` (cooldown < `ask_memory_retention_ticks`); emitter emits zero candidates.
4. Per-tick emission cap → focused unit test: fixture with 10 co-located witnesses all matching the trigger gate; emitter emits exactly 3 candidates (ranked by witness recency × testimony freshness weighted by `witness_recency_preference`).
5. Candidate absence reasoning → focused candidate-generation diagnostics. `CandidateGenerationDiagnostics.ask_witness_gate_rejections` records high-confidence and cooldown gate-rejection reasons. Rendering those diagnostics in the external decision trace remains out of scope for this emitter ticket.

## What Changed

- Added `emit_ask_witness_candidates` in `crates/worldwake-ai/src/candidate_generation.rs` and invoked it from the live candidate-generation sequence immediately after `emit_social_candidates`.
- Added `CandidateGenerationDiagnostics.ask_witness_gate_rejections` plus `AskWitnessGateRejectionReason` for high-confidence and cooldown rejections.
- Added `EmitterTag::EpistemicSensing` and `EvidenceKindTag::TestimonyProvenance` in `crates/worldwake-core/src/decision_event_payload.rs`.
- Added focused candidate-generation tests for stale report emission, high-confidence suppression, cooldown suppression, and per-topic fan-out capping.
- Confirmed `GoalBeliefView::ask_witness_memory` already existed from prerequisite work, so `worldwake-sim/src/belief_view.rs` did not need changes.

## Files Touched

- `crates/worldwake-ai/src/candidate_generation.rs`
- `crates/worldwake-core/src/decision_event_payload.rs`

## Out of Scope

- Ranking and motive_score for emitted candidates — ticket 005 (this ticket emits with placeholder ranking; ticket 005 replaces it with the real formula).
- Cold-start emission (no prior testimony, witness co-located) — deferred to a potential follow-up if Scenario 2 of the goldens (ticket 006) requires it. Document the deferral here.
- Multi-witness topic-disagreement detection — `TellTopic::SocialObservation` and `InstitutionalClaim` topic shapes are not yet supported (per ticket 001's `build_payload_override`).
- Goldens — ticket 006.

## Acceptance Result

### Tests Passed

1. `candidate_generation::tests::ask_witness_emitter_emits_for_stale_report_from_local_witness` proves the stale-report positive case.
2. `candidate_generation::tests::ask_witness_emitter_skips_high_confidence_report` proves high-confidence suppression and records `ConfidenceAtOrAboveThreshold`.
3. `candidate_generation::tests::ask_witness_emitter_skips_active_cooldown` proves cooldown suppression and records `CooldownActive`.
4. `candidate_generation::tests::ask_witness_emitter_caps_witness_fanout_per_topic_by_salience` proves the per-topic cap and salience ordering.
5. `cargo test -p worldwake-ai` and `./scripts/verify.sh` passed.

### Invariants

1. Emitter topic discovery reads testimony-sourced belief state through `GoalBeliefView::entity_beliefs_sourced_from_witness`; co-located witness enumeration uses the existing same-tick local observation surface.
2. Per-topic emission cap `K = 3` is enforced before ranking.
3. Cooldown gate reuses the existing `AskWitnessMemory` substrate; no parallel cooldown state was introduced.
4. `BTreeMap`/`BTreeSet` ordering preserves deterministic iteration.

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` gained the four focused unit tests listed in `Acceptance Result`.

### Commands Run

1. `cargo test -p worldwake-ai --lib ask_witness_emitter -- --list`
2. `cargo test -p worldwake-ai --lib ask_witness_emitter`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
5. `./scripts/verify.sh`

## Outcome

Completed on 2026-05-13.

- `AskWitness` candidates now emit from testimony-sourced stale beliefs for co-located witnesses.
- High-confidence and active-cooldown cases now leave explicit candidate-generation diagnostic records.
- The live phase owner was `candidate_generation.rs`; the drafted `agent_tick/candidates.rs` path does not exist on this branch.
- The existing `GoalBeliefView::ask_witness_memory` accessor was already present, so no `worldwake-sim` patch was needed.

## Deviations

- External decision-trace rendering of the new gate-rejection diagnostics did not land here; this ticket records the reasons in `CandidateGenerationDiagnostics`, and ticket 006 remains the golden/E2E proof owner.
- Ranking remains intentionally staged at the ticket-005 placeholder, so this ticket proves candidate emission, not goal selection.

## Verification Result

- Passed `cargo test -p worldwake-ai --lib ask_witness_emitter -- --list`
- Passed `cargo test -p worldwake-ai --lib ask_witness_emitter`
- Passed `cargo test -p worldwake-ai`
- Passed `cargo clippy --workspace --all-targets -- -D warnings`
- Passed `./scripts/verify.sh`
