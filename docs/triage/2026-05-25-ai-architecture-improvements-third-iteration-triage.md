# Triage — AI Architecture Improvements, Third Iteration (2026-05-25)

**Source:** `reports/ai-architecture-improvements-third-iteration.md` (ChatGPT-Pro,
run against current `main` SHA `de0992f3` — the post-S168 state). Author
follows the same SHA-pinned current-tree pipeline used for the second
iteration.

## Verdict

The report's factual base is solid. Twelve of fourteen load-bearing claims
verified accurate against current code (the two non-confirmations were the
*absence* of pre-built abstractions like a `VerificationProvider` registry or
a unified "causal proof" assertion API — those non-existences are themselves
inputs to the proposal, not factual errors). The triage therefore turns on
benefit and scope. **2 of 5 proposals accepted** as new specs. **1
reaffirmed** as already-shipped. **2 dismissed** with cited rationale.

The accepted set is structurally narrow: extend the verification axis beyond
AskWitness (Proposal 1, the explicit follow-up the second-iteration triage
identified and deferred), and close the three confirmed learned-state
provenance gaps (Proposal 3, narrowed from the report's unified
`LearnedStateUpdate` ambition to the concrete code-level holes). The
dismissals reaffirm earlier triage decisions: diagnostics-as-CI-gate remains
premature (matches the 2026-05-22 dismissal of second-iteration Proposal 6),
and the HTN `RequiredActionLeaf` lint is already shipped by S160's negative
test.

## Claim verification

12 of 14 load-bearing claims CONFIRMED against current `main` SHA `de0992f3`.
Verifications recorded in the brainstorm transcript; representative citations:

- `plan_repair.rs:131-132` confirms `InsertVerification` succeeds only with a
  `RepairPlanCandidate` and fails as `NoEpistemicSubstrate` otherwise.
- `plan_repair.rs:452-491` confirms `append_insert_verification_candidate`
  exclusively constructs `ask_witness_verification_step()` — no
  `ConsultRecord`/`SearchPlace` paths.
- `agenda_manager.rs:309-327` confirms `information_barrier_companion_entry`
  hardcodes `GoalKind::AskWitness`.
- `consult_record_actions.rs:45` and `search_actions.rs:40` confirm both are
  real lawful actions with payload validators, duration, and same-place
  visibility.
- `htn/registry.rs:119` confirms a negative test already asserts no live
  method declares `RequiredActionLeaf` (S160 shipped this guard).
- `agent_tick/observation.rs:416-434` confirms `apply_pending_discrepancies`
  hardcodes `source_event: None`.
- `learned_opportunity_memory.rs:5-11` confirms `OpportunityEntry` has no
  `source_event` field.
- `route_preference.rs:85-95` confirms `record_safe` skips the
  `last_traversal_event` write that `record_dangerous` performs.
- `testimony_reliability.rs:20-62` confirms the ring-buffer provenance model
  (this surface is *not* part of the gap; it is the model the others should
  approximate).

Two claims refuted as "this pre-built mechanism does not exist":
- No `VerificationProvider` / `VerificationNeed` types or registry — the
  proposal builds it from scratch (correctly).
- No unified "causal proof" assertion API in the golden harness, though the
  ring-buffer-style `expect_testimony_reliability_update` is a related
  precedent.

## Accepted

- **`specs/S169-generalized-lawful-verification-substrate.md`** (Proposal 1)
  — extends `InsertVerification` to splice lawful repair candidates for
  three breach classes (stale entity belief → AskWitness; stale
  institutional claim → ConsultRecord; overdue expectation at place →
  SearchPlace) through a fixed three-provider registry at the revalidation
  seam. Extends `RepairApplied` with `provider_kind` + `target` for FND-29A
  history. Three new goldens (consult-record repair, search-place repair,
  negative omniscience) + S165 AskWitness parity. **Scope narrowings**: (1)
  the report's fourth "direct same-tick local observation" provider is
  dropped — it is the FND-14A belief-view layer's behaviour, not a planned
  action that repair can splice; (2) goal-level agenda companion extension
  (replacing the hardcoded `GoalKind::AskWitness` in
  `agenda_manager.rs:309`) is deferred — it requires new `GoalKind`
  variants with full candidate generation, ranking, and HTN compatibility,
  and that scope belongs to a follow-up spec. **FND-1/7/14/14A/14B/15/16/17/
  18/20/21/28/29/29A/31.**

- **`specs/S170-learned-state-provenance-hardening.md`** (Proposal 3,
  narrowed; absorbs Item E) — closes the three confirmed code-level
  provenance gaps: adds `source_event: EventId` to
  `LearnedOpportunityMemory::OpportunityEntry`; populates
  `RoutePreference`'s `last_traversal_event` on the safe-traversal branch
  (currently dangerous-only); replaces hardcoded `source_event: None` in
  `apply_pending_discrepancies` with an explicit `DiscrepancySource`
  enum (`Event(EventId)` vs. `ReadPhaseInference`). **Scope narrowing**:
  the report's unified `LearnedStateUpdate` trait/struct is explicitly
  rejected — it risks the "abstract learning sludge" the report itself
  warns against, and the concrete fix is three additive field changes per
  store, not a meta-abstraction. **FND-3/22A/26/28/29/29A.**

## Reaffirmed (no new deliverable)

- **Proposal 5 — HTN `RequiredActionLeaf` Lint** — already shipped. S160
  (`archive/specs/S160-htn-authority-honesty.md`, COMPLETED 2026-05-21) ships
  `test_no_method_declares_required_action_leaf` at
  `crates/worldwake-ai/src/htn/registry.rs:119`, asserting no live HTN
  method declares the variant. The 2026-05-22 second-iteration triage
  already captured this standing decision: *"matches the standing,
  repeatedly-reaffirmed 'no required leaves until a schema contract proves
  fallback invalid' decision"*. ChatGPT-Pro's third-iteration proposal
  re-raises it without observing the existing guard. No new work.

## Dismissed

- **Proposal 2 — Diagnostics-as-Proof Golden Contract (standalone)** —
  reaffirms the 2026-05-22 dismissal of second-iteration Proposal 6 ("Proposal
  6 (diagnostics as CI gates) — premature; the report itself says start
  non-failing"). FND-31 is already constitutional; a separate spec to
  enforce FND-31 globally would be process-not-architecture and risks
  overfitting to current trace shapes before a stable baseline exists. The
  *spirit* of Proposal 2 — that new architectural surfaces should land with
  causal trace assertions, not "looked plausible" goldens — is captured for
  the new surface by S169's D9 (verification provider selection/rejection
  in `DecisionTrace`) and D10 (goldens that assert provider kind, target,
  and rejection reasons). The third iteration adds no new evidence that
  warrants overriding the prior dismissal.

- **Proposal 4 — Candidate / Opportunity Convergence Contract** — already
  disciplined today. S138 fixed compiler/emitter duplicate-suppression;
  S166 derived `source_belief.status` from real belief and `required_
  actions` from `EffectSchemaIndex`. ChatGPT-Pro itself rates this rank 4,
  "Adopt narrowly", "Targeted hardening, not redesign", "premature
  unification could break working emitters." The report names no concrete
  failure mode; the proposal reduces to "add more parity tests." Reopen
  only if S169's provider work surfaces a parity gap. No new spec.

## Follow-ups identified, not actioned

- **Goal-level agenda-companion polymorphism.** Replacing the hardcoded
  `GoalKind::AskWitness` in `information_barrier_companion_entry`
  (`agenda_manager.rs:309-327`) with a polymorphic dispatch over
  `VerificationProviderKind` requires new `GoalKind::ConsultRecord` /
  `GoalKind::SearchPlace` variants with full candidate-generation, ranking,
  and HTN compatibility. Track as a likely successor to S169 if its
  provider abstraction surfaces enough pressure; not actioned this wave.

- **`DecisionTrace` decision-effect coupling for learned updates.** The
  third-iteration report's Proposal 3 includes a `decision_effect_trace`
  pointer per learned update. S170 explicitly excludes this — it requires
  wiring through `DecisionTrace` per update site and is genuine scope
  expansion. Track if/when an audit reveals a learned update whose
  decision-effect is hard to reconstruct.

- **Generated scenario coverage warnings classification** (Item D in the
  report; also flagged in the 2026-05-22 triage as a possible S167
  sibling). Classifying each unmapped field (`portfolio_weights_profile`,
  `expectation_store`, `last_seen_memory`, `social_observations`,
  `intention_disposition`, `risk_weight_profile`) as canonical feature,
  support field, fixture-only, or obsolete is a hygiene task. Track for a
  small ticket; not spec-worthy.
