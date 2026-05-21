# Triage: AI Architecture Consolidation (2026-05-20)

**Source.** `archive/reports/ai-architecture-consolidation-first-iteration.md` — hostile AI-architecture
audit by ChatGPT-Pro (verdict: keep core, tighten authority boundaries; recommended Option B).
Every load-bearing code claim was re-verified against the codebase before acceptance. The audit
was treated as a set of hypotheses, not facts.

## Accepted (specs written)

- **S155 — Belief-View Boundary Correctness** (`archive/specs/S155-belief-view-boundary-correctness.md`).
  Findings #1 (CONFIRMED, Critical), #7 and #12 (CONFIRMED). `effective_place()` falls back to
  authoritative `world.effective_place()` for non-self entities known only via institutional
  belief or last-seen memory (FND-14A leak); `can_control()` lacks the belief gate its sibling
  `believed_rights()` has, yet is called from planning/affordance paths. Completed on 2026-05-20.
- **S156 — HTN Authority Honesty** (`archive/specs/S156-htn-authority-honesty.md`). Findings #3, #4, #5,
  #9, #10 (all CONFIRMED). `GoalSchema.methods` fossil (FND-28); no-op `AgentRole` precondition;
  two dead methods (`investigate_on_scene`, `escort_to_office`) via always-false
  `EntityCriterion` variants; three unenforced `MethodSchema` fields; implicit untraced fallback;
  shallow method traces. Strip (do not build new capability); make fallback explicit + traced.
  Completed on 2026-05-20.

## Accepted but deferred (spec written, NOT in active order)

- **S157 — Planner Snapshot Admission Provenance**
  (`archive/specs/S157-planner-snapshot-admission-provenance.md`). Findings #2, #13 (CONFIRMED);
  completed as the bounded admission-source, strategic-scan restriction, and decision-trace
  provenance slice.
  Snapshot entities carry no admission source; strategic search scans the entity map directly.
  Defense-in-depth/provenance (FND-15, FND-29), not a correctness fix — S155 closes the leak at
  the source. Bounded alternative to the audit's `PlannerVisible<T>` + four-trait + "never pass
  `&World`" proposal, which is not adopted. Depends on S155.

## Dismissed (verification refuted or overstated)

- **Finding #6 — candidate generation "mixes desire emission with memory writes."** REFUTED.
  Generation is already side-effect-free and returns typed `pending_*` records; a separate write
  phase in `agent_tick`/`observation.rs` applies them. Auditor self-hedged this. No spec.
- **Finding #8 — ranking "abstract-score creep / magic numbers."** REFUTED. Tunable weights come
  from `UtilityProfile`; the few hard constants (`HYGIENE_FACTOR_FLOOR`, ask-witness gap weight)
  are documented, empirically-justified architectural constants, not FND-2 magic numbers. No spec.
  (Portfolio slots = a feature, not a fix; deferred.)
- **Finding #7-goal — goal semantics fragmented across 7+ files.** Real fragmentation, but a
  smell, not a correctness bug; consolidating `is_satisfied` for 30+ goal kinds is the audit's own
  Option C, which it calls premature until the belief-boundary bug is fixed. Deferred (no spec).
- **Finding #11 — player-POV / CLI symmetry.** Suspected-only (CLI not inspected by auditor). The
  bounded control-source-swap symmetry check is folded into S155's test plan (D3); a broader CLI
  affordance audit is left as a follow-up, not a spec.
- **Finding #15 — "HTN bypasses action causality."** Auditor's own false-alarm; confirmed not a
  problem. No action.

## Follow-up identified, not actioned

- Broader CLI/player-POV affordance audit (verify the human action UI consumes the same
  belief/affordance surface as AI, debug truth fully separated) — beyond S155's single swap golden.
- Goal-semantics consolidation — revisit only if S155/S156 reveal the candidate/ranking/schema
  seams cannot be stabilized (the audit's Option C trigger).
