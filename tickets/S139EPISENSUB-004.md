# S139EPISENSUB-004: emit_ask_witness_candidates emitter

**Status**: PENDING
**Priority**: HIGH
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` (`candidate_generation.rs`, `agent_tick/candidates.rs`)
**Deps**: archive/tickets/S139EPISENSUB-001.md, tickets/S139EPISENSUB-002.md

## Problem

With `GoalKind::AskWitness` (ticket 001), `EpistemicDispositionProfile.witness_recency_preference` (ticket 002), and the dispatch declaration (ticket 003) all in place, the AI crate still cannot emit `AskWitness` candidates because no emitter exists. The planner cannot adopt the goal unless candidate generation surfaces it. This ticket adds `emit_ask_witness_candidates` to `crates/worldwake-ai/src/candidate_generation.rs` and wires it into the agent_tick candidate-generation phase.

## Assumption Reassessment (2026-05-13)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `candidate_generation.rs` structures emitters as functions taking `(candidates: &mut Vec<GoalOffer>, diagnostics: &mut CandidateGenerationDiagnostics, ctx: &GenerationContext<'_>)`. `GenerationContext` is defined at line 155-169 and exposes `view: &'a dyn GoalBeliefView`, `agent: EntityId`, `place: Option<EntityId>`, `blocked: &'a BlockerMemory`, `current_tick: Tick`. Emitters call `emit_candidate_with_trace` (line 5399) to produce `GroundedGoal` records. The per-target enumeration pattern that S139's emitter mirrors is `emit_engage_hostile_goals` at lines 2486-2565.
2. After ticket 001, `GoalBeliefView::entity_beliefs_sourced_from_witness(agent, witness)` is the primary read for the emitter trigger. After ticket 002, `EpistemicDispositionProfile.witness_recency_preference` and `stale_evidence_barrier_threshold` are reachable through the existing `epistemic_disposition_profile(actor)` accessor at `belief_view.rs:548`.
3. Shared abstraction boundary under audit: candidate-generation phase entrypoint. New emitter is added at the same call-site tier as `emit_social_candidates` (verify call ordering during implementation). The phase ordering does not affect correctness for this emitter because emission is filter-gated, not order-dependent.
4. Live `GoalKind` under test: `GoalKind::AskWitness` (added by ticket 001). The structural analog for the emitter pattern is `emit_engage_hostile_goals` (per-target enumeration with `Evidence::with_entity`).
5. Existing inline tests in `candidate_generation.rs`'s `#[cfg(test)]` block (line 6591) exercise emitter wiring through fixture `GoalBeliefView` impls. New tests for `emit_ask_witness_candidates` follow the same pattern.
6. Phase distinction (precision-rules Rule 1): this ticket lives in *candidate generation*. Ranking (priority class, motive_score) belongs to ticket 005. Suppression (`EPISTEMIC_SENSING_POLICY` at high stress) belongs to ticket 003's policy + ranking-side dispatch. The emitter's job is "should this candidate exist?" — the per-tick gate. The ranking layer decides relative priority among emitted candidates.

## Architecture Check

1. The emitter scans belief envelope only (FND-14, FND-15) — no global witness query, no world-state read. Co-location is filtered through `view.effective_place(witness) == ctx.place`. Belief provenance is read through `view.entity_beliefs_sourced_from_witness`, which the new accessor (ticket 001 D8) exposes.
2. Per-tick emission cap `K = 3` prevents fan-out when many co-located witnesses match. The cap is enforced inside the emitter (in the candidate-generation phase), not at the ranking layer — this preserves the "emit if gate passes, rank emitted set" separation (precision-rules Rule 1).
3. Cooldown gate consumes the existing `AskWitnessMemory` substrate at `belief.rs:1763`. No new memory type; the gate reuses the same data that `epistemic_actions.rs::apply_ask_witness_commit:460-466` writes. Single source of truth.

## Verification Layers

1. Emitter fires for stale-belief case → focused unit test against a fixture `GoalBeliefView` impl with one belief entry whose `PerceptionSource::Report { from: witness, .. }` matches a co-located witness AND confidence is below `stale_evidence_barrier_threshold`. Expected: one `GoalOffer` with `GoalKind::AskWitness { witness, topic }` emitted.
2. Emitter does NOT fire when confidence ≥ threshold → focused unit test: same fixture but with confidence above threshold; emitter emits zero candidates.
3. Emitter does NOT fire when cooldown active → focused unit test: fixture with `AskWitnessMemory { asked_tick: current_tick - 1 }` (cooldown < `ask_memory_retention_ticks`); emitter emits zero candidates.
4. Per-tick emission cap → focused unit test: fixture with 10 co-located witnesses all matching the trigger gate; emitter emits exactly 3 candidates (ranked by witness recency × testimony freshness weighted by `witness_recency_preference`).
5. Candidate absence reasoning → decision trace (per precision-rules Rule 6 decision-trace preference). The emitter's diagnostics (`CandidateGenerationDiagnostics`) record gate-rejection reasons so the trace explains why no candidate was emitted at high-confidence inputs.

## What to Change

### 1. Add `emit_ask_witness_candidates` function

In `crates/worldwake-ai/src/candidate_generation.rs` (alongside `emit_engage_hostile_goals` and `emit_social_candidates`):

```rust
fn emit_ask_witness_candidates(
    candidates: &mut Vec<GoalOffer>,
    diagnostics: &mut CandidateGenerationDiagnostics,
    ctx: &GenerationContext<'_>,
) {
    let Some(place) = ctx.place else { return; };
    let Some(profile) = ctx.view.epistemic_disposition_profile(ctx.agent) else { return; };
    let threshold = profile.stale_evidence_barrier_threshold;
    let cooldown = profile.ask_memory_retention_ticks;
    let recency_pref = profile.witness_recency_preference;

    // Phase 1: Find co-located known agents who might be asked.
    let local_entities = ctx.view.entities_at(place);
    let witnesses: BTreeSet<EntityId> = local_entities
        .into_iter()
        .filter(|e| ctx.view.entity_kind(*e) == Some(EntityKind::Agent) && *e != ctx.agent)
        .collect();

    // Phase 2: For each witness, gather candidate topics from belief envelope.
    // A topic is candidate when:
    //   - the belief on the topic's subject has confidence below `threshold`, AND
    //   - the belief was sourced from this witness (entity_beliefs_sourced_from_witness)
    //     OR the witness is co-located and we have no belief / cold-start belief on the subject
    //   - the (witness, topic) cooldown is not active

    let mut topic_emissions: BTreeMap<TellTopic, Vec<(EntityId, /* recency-weighted salience */ Permille)>>
        = BTreeMap::new();

    for witness in &witnesses {
        let sourced_beliefs = ctx.view.entity_beliefs_sourced_from_witness(ctx.agent, *witness);
        for (subject, believed_state) in sourced_beliefs {
            let topic = TellTopic::EntityBelief { subject };
            let confidence = compute_belief_confidence(&believed_state, ctx.current_tick);
            if confidence >= threshold { continue; }

            // Cooldown gate
            let memory_key = AskWitnessMemoryKey {
                counterparty: *witness,
                topic_entity: Some(subject),
                topic_commodity: None,
            };
            if let Some(memory) = ctx.view.ask_witness_memory(ctx.agent, &memory_key)
                && ctx.current_tick.0 - memory.asked_tick.0 < cooldown as u64
            {
                diagnostics.record_emitter_gate_rejection(EmitterTag::EpistemicSensing, "cooldown_active");
                continue;
            }

            let salience = compute_recency_weighted_salience(&believed_state, ctx.current_tick, recency_pref);
            topic_emissions.entry(topic).or_default().push((*witness, salience));
        }
    }

    // Phase 3: For each topic, rank witnesses by salience and emit up to K = 3.
    const K: usize = 3;
    for (topic, mut entries) in topic_emissions {
        entries.sort_by(|a, b| b.1.cmp(&a.1));  // descending salience
        for (witness, _salience) in entries.into_iter().take(K) {
            let mut evidence = Evidence::with_entity(witness);
            evidence.places.insert(place);
            let trace = build_evidence_trace_for_ask_witness(ctx, witness, &topic);

            emit_candidate_with_trace(
                candidates,
                diagnostics,
                EmitterTag::EpistemicSensing,
                single_evidence(EvidenceKindTag::TestimonyProvenance),
                GoalKind::AskWitness { witness, topic },
                OpportunityAnchor::Entity(witness),
                evidence,
                trace,
            );
        }
    }
}
```

The helpers `compute_belief_confidence`, `compute_recency_weighted_salience`, and `build_evidence_trace_for_ask_witness` are new module-private helpers in `candidate_generation.rs`. `EmitterTag::EpistemicSensing` and `EvidenceKindTag::TestimonyProvenance` are new variants (add in deterministic order in their respective enums; both are `Copy`).

A `ctx.view.ask_witness_memory(agent, key)` accessor is required on `GoalBeliefView`. The existing belief-view trait already exposes `AgentBeliefStore` indirectly; check whether `ask_witness_memory` already exists or needs adding. If absent, add it to `belief_view.rs:270+` alongside ticket 001's `entity_beliefs_sourced_from_witness` — this is a scope-extending mismatch that should be flagged here and resolved as part of this ticket (it cannot move to ticket 001 retroactively; the emitter needs it).

### 2. Wire emitter into agent_tick candidate-generation phase

In `crates/worldwake-ai/src/agent_tick/candidates.rs` (or wherever the candidate-generation phase orchestrates emitter calls — verify exact file during implementation), add a call to `emit_ask_witness_candidates(candidates, diagnostics, ctx)` at the same tier as `emit_social_candidates`.

### 3. Cold-start fallback (optional, deferred if scope grows)

The spec text mentions a cold-start case (no prior testimony, but co-located witness exists). The initial emitter handles the testimony-sourced case; cold-start can be added as a follow-up if scope grows. Document the limitation as Out of Scope and reference a potential follow-up ticket.

## Files to Touch

- `crates/worldwake-ai/src/candidate_generation.rs` (modify — new emitter function + helpers + EmitterTag/EvidenceKindTag variant additions)
- `crates/worldwake-ai/src/agent_tick/candidates.rs` (modify — wire emitter into phase)
- `crates/worldwake-sim/src/belief_view.rs` (modify — add `ask_witness_memory(agent, key)` accessor if absent; scope-extending from ticket 001's D8)
- Likely: `crates/worldwake-ai/src/decision_trace.rs` (modify — extend trace surface for EpistemicSensing emitter if existing trace categorizes by emitter)

## Out of Scope

- Ranking and motive_score for emitted candidates — ticket 005 (this ticket emits with placeholder ranking; ticket 005 replaces it with the real formula).
- Cold-start emission (no prior testimony, witness co-located) — deferred to a potential follow-up if Scenario 2 of the goldens (ticket 006) requires it. Document the deferral here.
- Multi-witness topic-disagreement detection — `TellTopic::SocialObservation` and `InstitutionalClaim` topic shapes are not yet supported (per ticket 001's `build_payload_override`).
- Goldens — ticket 006.

## Acceptance Criteria

### Tests That Must Pass

1. New focused unit test in `candidate_generation.rs`'s `#[cfg(test)]` block: stale-belief fixture (single witness, single topic, confidence below threshold, cooldown not active) → emitter emits exactly one `GoalKind::AskWitness` candidate with the expected witness + topic.
2. New focused unit test: high-confidence fixture (confidence ≥ threshold) → zero candidates.
3. New focused unit test: cooldown-active fixture (`AskWitnessMemory.asked_tick` within `ask_memory_retention_ticks`) → zero candidates; diagnostics record `cooldown_active`.
4. New focused unit test: fan-out fixture (10 witnesses matching the same topic) → exactly 3 candidates emitted, ranked by recency-weighted salience.
5. Existing suite: `cargo test -p worldwake-ai` passes.

### Invariants

1. Emitter reads belief state only — no world-state queries (verified by the fixture-based test using a stub `GoalBeliefView` that panics on world-state methods).
2. Per-tick emission cap `K = 3` is enforced regardless of witness count.
3. Cooldown gate is the single arbiter of "ask again or not"; the gate reuses the existing `AskWitnessMemory` substrate written by `epistemic_actions.rs:460-466`. No parallel cooldown state.
4. `BTreeMap` iteration order in candidate generation preserves determinism (CLAUDE.md Critical Invariants).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/candidate_generation.rs` (extend `#[cfg(test)]` block at line 6591) — add 4 new focused unit tests per Acceptance Criteria #1-#4.

### Commands

1. `cargo test -p worldwake-ai -- candidate_generation::tests` — targeted test run.
2. `cargo clippy -p worldwake-ai --all-targets -- -D warnings`.
3. `./scripts/verify.sh` — full pre-PR gate.
