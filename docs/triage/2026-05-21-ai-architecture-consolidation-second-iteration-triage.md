# Triage — AI Architecture Consolidation (Second Iteration)

**Date:** 2026-05-21
**Source:** `reports/ai-architecture-consolidation-second-iteration.md` (ChatGPT-Pro
hostile AI-architecture audit, second iteration; follows S155–S157 from the first).
**Method:** Every load-bearing claim re-verified against current code before
acceptance. Audit recommended Option B (Moderate Consolidation); accepted in
narrowed form, with the audit's `Sourced<T>`/per-field-snapshot static-typing
(its own Option C) rejected for now.

## Accepted (specs written)

- **S158 — Belief-View Remote-Truth Leak Closure** —
  `archive/specs/S158-belief-view-remote-truth-leak-closure.md`. Economic,
  production, physical, and contention `PerAgentBeliefView` accessors were
  confirmed to leak current world truth for remote entities
  (`has_sale_listing`, `seller_for_sale_lot`, `listed_sale_lots_at`,
  `has_production_job`, `carry_capacity`, `load_of_entity`, plus ungated
  contention reads). `can_control` / `believed_rights` value-backing was
  deliberately deferred after S155/S158 reassessment. FND-7/14/14A/19. Completed.
- **S159 — Candidate-Generation Schema-Owned Extractor Authority** —
  `specs/S159-candidate-generation-schema-owned-extractor-authority.md`.
  `LEGACY_EXTRACTOR_ORDER` fossil + out-of-band blocked-self-care emitter confirmed.
  FND-28. Behavior-preserving cleanup.
- **S160 — HTN Authority Honesty** —
  `specs/S160-htn-authority-honesty.md`. Confirmed: subgoals are unenforced stage
  hints, `fulfill_bounty_group_hunt` fakes coordination (no recruit leaf, solo
  attack), `ActionDefId(u32::MAX)` escort sentinel. FND-20/29. No method-required.

## Dismissed (claim refuted or out of scope)

- **"Critical: container/possessor leak"** — REFUTED. `direct_container` /
  `direct_possessor` already gate on `knows_entity || local visibility || owned`.
  Fixed by S155.
- **"Per-field snapshot provenance is missing" / "per-entity AdmissionSource is too
  weak"** — OVERSTATED. `build_snapshot_entity` already resolves fields belief-first
  and gates `direct_container` on co-location; the leak is the leaky *view*
  fallbacks (→ S158), not absent per-field handling. Per-field source *tags* are a
  diagnostics nicety, not a safety requirement once the view is fixed.
- **`Sourced<T>` / `FieldSource` source-typed view APIs + source-class trait split**
  — DEFERRED (Option C). Large migration the audit itself flags as risky; the
  FND-14 win is captured by S158's behavioral gating. Revisit only if S158 goldens
  reveal pervasive leaks that cannot be localized.
- **`LEGACY_EXTRACTOR_ORDER` "controls authority"** — PARTIALLY REFUTED. The schema
  already decides *which* extractors run; the const only controls *order*. A
  naming/authority smell (folded into S159), not a behavioral leak.
- **"Candidate generation mixes emission with anomaly/discrepancy detection"** —
  OUT OF SCOPE. Real responsibility-breadth smell, but the result is documented
  side-effect-free (caller applies in write phase). Moving observation/anomaly
  interpretation out of candidate emission is a larger perception-architecture
  change, not actioned here.
- **Snapshot all-pairs shortest-path scaling / 100-agent soak** — NOT ACTIONED.
  Performance concern (FND-20), not a correctness/leak issue; no behavior bug
  confirmed. Defer to a dedicated scaling pass.

## Follow-up identified, not actioned

- Perception-architecture refactor to split observation/anomaly/discrepancy
  detection out of `candidate_generation.rs` (audit §7). Candidate for a future
  spec if the responsibility breadth causes a concrete defect.
- Snapshot/planning scaling pass (Floyd-Warshall cost + per-tick snapshots) for
  hundreds of agents/places.
- Decision-trace/visualizer "debug-as-gameplay-oracle" separation (audit §10).
  No live leak found; revisit when a player-facing "why" view is built.

See `specs/IMPLEMENTATION-ORDER.md` → "Adjunct Wave: AI Architecture Consolidation
— Second Iteration" for sequencing.
