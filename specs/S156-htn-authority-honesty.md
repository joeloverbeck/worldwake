# S156: HTN Authority Honesty

## Summary

The live HTN layer advertises more semantic authority than it enforces, and carries fossil and
fake declarations that FND-28 forbids. This spec strips the dishonest parts so the layer is
exactly what it actually is — **strategic, fallback-legal method guidance over ordinary
`ActionDef` affordances** — and makes the strategic fallback path explicit and traced. It
does **not** build new HTN capability (role state, witness/evidence/ledger location
resolution); per the consolidation triage that capability is dropped now and may return later
with real enforcement and tests.

Concretely it removes: (1) `GoalSchema.methods`, a deliberately-empty field shadowing
`MethodRegistry`'s real authority; (2) `MethodPrecondition::AgentRole`, which always evaluates
`true`; (3) `EntityCriterion::Witness/ViolationEvidence/Ledger`, which always evaluate `false`,
and the two methods dead because of them; (4) `MethodSchema` fields `expected_artifacts`,
`required_claims`, `failure_modes`, which have zero consumers. It then makes the implicit
"fallback always allowed" behavior explicit and trace-visible, and folds an HTN method
drafting checklist into `docs/spec-drafting-rules.md`.

## Phase

AI Architecture Consolidation (Adjunct Wave — derived from `reports/ai-architecture-consolidation-first-iteration.md`)

## Status

DRAFT

## Crates

- `worldwake-ai` (`goal_schema.rs`, `htn/method_schema.rs`, `htn/methods.rs`, `htn/selector.rs`,
  `htn/registry.rs`, `search/strategic.rs`, decision-trace types, integration tests)

## Dependencies

- E06 / two-phase planning pipeline (S88–S90) — completed
- Independent of S155; may be implemented in parallel (different files; the strategic fallback
  trace work does not depend on the belief-view fix).

## Problem Statement

### Motivation and evidence

`reports/ai-architecture-consolidation-first-iteration.md` (Findings #3, #4, #5, #9, #10, all
rated *High* or *Medium/High*) argued the HTN layer is "overnamed" and carries fossil seams.
Direct code verification confirmed every load-bearing claim:

- **`GoalSchema.methods` fossil (FND-28).** `goal_schema.rs` declares
  `methods: &'static [MethodSchemaId]`, and `tests/integration/goal_schema_methods.rs` asserts
  every live `GoalDispatchKey` declaration leaves it **empty** because "method assignment
  belongs to the method registry." `MethodRegistry` (`htn/registry.rs`,
  `by_goal_kind: BTreeMap<GoalKindDiscriminant, Vec<MethodSchemaId>>`, `insert`/`methods_for`)
  is the real, sole authority. Two live-looking authorities for one concept.
- **Fake `AgentRole` precondition.** `selector.rs:77`: `MethodPrecondition::AgentRole(_) => true`.
  The agent context profile has no role field to check. Only `fulfill_bounty_group_hunt`
  (`methods.rs:182`) uses `AgentRole(RoleTag::Hunter)`; the gate filters nothing.
- **Dead `LocationKnown` criteria + dead methods.** `selector.rs:97-99`:
  `EntityCriterion::Witness | ViolationEvidence | Ledger => false`. The only methods using these
  **as preconditions** are `investigate_on_scene` (`methods.rs:388`, `ViolationEvidence`) and
  `escort_to_office` (`methods.rs:507`, `Ledger`) — both permanently unselectable.
- **Declared-but-unenforced schema fields.** `method_schema.rs`: `expected_artifacts`,
  `required_claims`, `failure_modes` have **zero consumers** anywhere (verified by workspace
  grep); `search/strategic.rs` never reads them.
- **Implicit fallback.** `search/strategic.rs` falls back to missing-commodities/goal-places
  with `method_trace: None` when no method produces stages; no method-required/fallback policy
  exists at the goal-schema boundary.
- **Shallow method traces.** `method_trace()` records only the selected method id, subgoals as
  `Pending`, and motive score — `failure_mode: None`, no rejected methods, no fallback reason.

### Verified blast radius (precise)

| Method | Status after strip | Reason |
|--------|--------------------|--------|
| `investigate_on_scene` (id 9) | **DELETE** | precondition `LocationKnown(ViolationEvidence)` always false |
| `escort_to_office` (id 13) | **DELETE** | precondition `LocationKnown(Ledger)` always false |
| `fulfill_bounty_group_hunt` (id 3) | **KEEP**, remove `AgentRole(Hunter)` precondition | gate is a no-op; method still has real preconditions (`TargetBelievedDangerous`, `AllyOrBountyOfficeAvailable`) and real subgoals (`DeclareSupport`→`TravelTo`→`Attack`) |
| `fulfill_bounty_investigation` (id 2), `investigate_by_ledger` (id 11) | **KEEP unchanged** | use `ViolationEvidence`/`Ledger` only via `InspectArtifact(ArtifactTemplate::…)` in **subgoals** — a different enum (`ArtifactTemplate`), not gated by the selector |
| All other methods | unchanged | no fake/dead preconditions |

Variant deletion touches `methods.rs`, `selector.rs`, `search/strategic.rs` (a fallthrough arm
returning `goal.evidence_places`), and `method_schema.rs` test code. `RoleTag` becomes orphaned
if `AgentRole` was its only user — remove it if so.

### Why this matters

Per FND-20, "any planner formalism may encode only reusable lawful affordances, decomposition
knowledge, or search control" and "a method-required goal needs an explicit schema contract and
tests proving fallback would be semantically invalid." Today the schema *looks* like it encodes
artifacts/claims/role gating it does not enforce, creating false confidence and exactly the
"two live authorities for one concept" FND-28 prohibits. Dead methods and no-op gates are
fossilized logic (FND preamble: "clean meaning no dead paths or fossilized logic"). Honest
naming + explicit fallback + deeper traces make HTN debuggable (FND-29) and prevent a future
contributor from accidentally relying on unenforced semantics.

### Key interview decisions

- **Strip, do not build** (user decision): delete fossils/fakes now; do not add role state or
  witness/evidence/ledger location resolution. Capability can return later *with* enforcement.
- **No speculative `enforcement_level`/`fallback_policy` schema field now.** Since no current
  method is method-required, such a field would be unused — itself a fossil-in-waiting,
  contradicting the strip. Instead make the *behavior* (fallback-always-legal) explicit and
  **traced**, and require future method-required goals to add the policy *with* enforcement.
- Doc updates folded into this spec.

## Design Goals

- `MethodRegistry` is the single, unambiguous method-assignment authority; no shadow field.
- Every `MethodPrecondition` variant that survives evaluates to a real, state-dependent result.
- Every `MethodSchema` field that survives has at least one live consumer.
- The strategic fallback (no method produced stages) is explicit and recorded in the method
  trace with a reason, not silently `None`.
- Method traces answer "why this method, why not the others, and did fallback happen?" —
  recording rejected methods with their failing precondition, and the fallback reason.
- `docs/spec-drafting-rules.md` gains an HTN method checklist that prevents reintroducing
  declared-but-unenforced method semantics.

## Non-Goals

- Building role-gated, witness-, evidence-, or ledger-resolved methods (dropped per triage).
- Enforcing method leaves / making any goal method-required (explicitly deferred; FND-20
  permits flat GOAP fallback as the default).
- Adding portfolio slots, consolidating goal-satisfaction semantics, or changing ranking —
  out of scope per the consolidation triage.
- Changing tactical GOAP search internals.

## FOUNDATIONS Alignment

| Principle | How this spec satisfies it |
|-----------|----------------------------|
| FND-20 (Resource-bounded reasoning over scripts) | HTN remains lawful search-control with legal flat-GOAP fallback; no goal is silently method-required; surviving methods express how an agent pursues a world condition, not a scripted beat. |
| FND-28 (No backward compat / no fossil authority) | `GoalSchema.methods` shadow field, no-op `AgentRole`, dead criteria/methods, and unenforced schema fields are removed, not wrapped. One authority (`MethodRegistry`) for method assignment. |
| FND-29 (Debuggability) | Method trace records selected + rejected methods, failing preconditions, and explicit fallback reason — answering contrastive "why not?" questions. |
| FND-31 (Validation/falsification) | New integration tests prove single method authority, that `group_hunt` stays selectable, that deleted methods are gone, and that traces record rejection/fallback. |
| FND-26 (Systems through state) | Method selection continues to read belief/runtime state and motives; no new cross-system call introduced. |

## Section H — Causal Hooks Declaration

### H.1 Information-path analysis
No new information path. Method selection still reads the same belief/motive/runtime surface;
this spec removes declarations, not inputs. The added trace data is debug output, not an
in-world information carrier.

### H.2 Positive-feedback analysis
None. No amplifying loop created or removed.

### H.3 Concrete dampeners
N/A (per H.2).

### H.4 Stored state vs. derived read-model
Removes static schema fields (`GoalSchema.methods`; `MethodSchema.expected_artifacts/
required_claims/failure_modes`) that were neither authoritative world state nor consumed
derived reads — pure dead declarations. The method trace is a derived, transient debug
read-model (not authoritative state), consistent with FND-29A (authoritative causal history is
separate from debug traces).

### H.5 Planner-formalism analysis
This is the core of the spec. Post-strip the layer is **HTN method decomposition over existing
`ActionDef` affordances, with legal flat-GOAP fallback** — not method-required. Surviving
methods each encode a reusable multi-stage lawful pursuit pattern (acquire-before-craft,
gather-before-craft, purchase-before-craft, restock, investigate-by-witness/by-ledger,
escort-to-home, bounty direct/investigation/group-hunt). No method bypasses
`ActionDef` preconditions/cost/duration/contention/dispatch. The drafting-rule update (D6)
codifies that any *future* method-required goal must name the explicit schema contract,
enforced leaves/artifacts, and bypass-impossibility tests required by FND-20.

### Agent Profile Scenario Contract
N/A — no new ECS component on `EntityKind::Agent`. (Note: this spec *removes* the latent
`RoleTag` coupling rather than adding role state.) No `Permille` or numeric profile parameter
introduced.

## Deliverables

### D1 — Remove `GoalSchema.methods` fossil; single method authority
Delete the `methods` field from `GoalSchema` and from all `GoalDispatchKey` declarations.
Rewrite `tests/integration/goal_schema_methods.rs` from "all dispatch declarations expose empty
method anchors" to assertions that `MethodRegistry` is the sole method-assignment authority
(e.g. every goal kind's methods come from `MethodRegistry::methods_for`, and no second surface
declares method assignment).

### D2 — Remove fake `AgentRole` precondition
Delete `MethodPrecondition::AgentRole` and its always-`true` selector arm; remove the
`AgentRole(RoleTag::Hunter)` precondition from `fulfill_bounty_group_hunt` (the method
otherwise unchanged and still selectable). Remove `RoleTag` if it becomes unused workspace-wide.

### D3 — Remove dead criteria + dead methods
Delete `EntityCriterion::Witness`, `EntityCriterion::ViolationEvidence`, `EntityCriterion::Ledger`
and their always-`false` selector arms and the `search/strategic.rs` fallthrough arm. Delete the
two dead methods `investigate_on_scene` and `escort_to_office` and any registry entries for them.
Verify `fulfill_bounty_investigation` and `investigate_by_ledger` are untouched (their
`InspectArtifact` subgoals use `ArtifactTemplate`, a different enum).

### D4 — Remove unenforced `MethodSchema` fields
Delete `expected_artifacts`, `required_claims`, and `failure_modes` from `MethodSchema` and all
constructors/tests, since they have no consumers. (They may be reintroduced *with* enforcement
when a method-required goal is actually built — see D6.)

### D5 — Explicit, traced strategic fallback + deeper method traces
Make the strategic fallback path explicit: when no selected method produces stages,
`search/strategic.rs` records in the method trace that fallback occurred and why (e.g.
`no_viable_method` / `method_produced_no_stages`) rather than emitting `method_trace: None`.
Extend `MethodPlanAttemptTrace` (and `method_trace()`) to record **rejected** candidate methods
for the goal kind with the precondition that failed, alongside the selected method. Per the
Authoritative-to-AI Impact Rule, run the full decision-cycle trace and the golden suite.

### D6 — Doc updates (folded in)
- `docs/spec-drafting-rules.md`: add an HTN method drafting checklist — each method must declare
  the reusable pursuit pattern, why flat GOAP is insufficient, whether flat-GOAP fallback is
  allowed/forbidden/allowed-after-traced-failure, every belief/record/observation it reads, and
  the golden tests proving selection, rejection, fallback, and trace. State that any field
  expressing required artifacts/claims/failure modes must be *enforced* when declared (no
  re-creation of dead schema), and that a method-required goal is invalid unless the schema
  proves fallback would satisfy the wrong semantic condition.
- `docs/planner-contracts.md`: document the method-trace fallback/rejection contract.

### D7 — Tests
Integration/golden tests: single method-assignment authority (D1); `fulfill_bounty_group_hunt`
still selectable for a qualifying agent after the `AgentRole` removal; the two deleted methods
are absent from the registry; method trace records at least one rejected method with its failing
precondition and records the fallback reason when no method applies.

## Test Plan

1. Focused: `cargo test -p worldwake-ai --test goal_schema_methods` and HTN selector/strategic
   unit tests.
2. Golden: `cargo test -p worldwake-ai --test golden_ai` (HTN/strategic scenarios) — verify no
   world-outcome regressions (trace changes expected, behavior changes not).
3. Full AI suite: `cargo test -p worldwake-ai`.
4. `cargo clippy --workspace --all-targets -- -D warnings` (variant/field removals must leave no
   dead code or unused-import warnings).
5. `./scripts/verify.sh` before PR.
