# S158BELVIEWLEAK-004: Source-class contract documentation

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: None

## Problem

The belief-view source-class rule that S158 enforces in code (tickets 001–003)
must be codified as a durable contract so future belief-view accessors do not
re-introduce remote-truth leaks. Today `docs/planner-contracts.md` §2 documents
the entity-admission belief barrier and the control/rights gating, but not the
per-accessor source-class rule for economic/production/physical/contention reads,
and `docs/spec-drafting-rules.md` has no rule requiring new accessors to declare
their source class. S158 D2 + D3.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `docs/planner-contracts.md` §2 ("Entity admission and the belief barrier",
   lines ~76–96) exists and already covers entity admission and the
   `ControlBeliefView::can_control` co-located-unowned-item shortcut. This ticket
   adds a "Planner-visible fields are source-scoped" subsection; it does not alter
   the existing control/rights language (S158 leaves that path as S155 set it).
2. `docs/spec-drafting-rules.md` exists and currently has no rule on belief-view
   accessor source-class declaration; the new rule sits alongside the existing
   "Agent Profile Scenario Contract" / Section H rules.
3. Documentation-only ticket; no shared runtime boundary. The accessor list it
   documents is the exact set gated by tickets 001–003 and enumerated in
   `specs/S158-belief-view-remote-truth-leak-closure.md` (Source-Class Rule + D1).

## Architecture Check

1. Codifying the source-class rule in the two authoritative contract docs
   (planner-contracts for the live planner boundary, spec-drafting-rules for
   future spec authorship) prevents leak regressions at review time rather than
   relying on per-PR vigilance — the cheapest durable enforcement surface.
2. No backward-compatibility concern: additive documentation; the existing §2
   control/rights language is preserved unchanged, and the new subsection
   explicitly notes that stricter rights-value belief-backing is deferred.

## Verification Layers

1. Source-class rule is documented and consistent with the gated accessor set →
   manual cross-read against tickets 001–003 + S158 D1.
6. Single-layer (documentation) ticket; no runtime invariant to map — verification
   is review-based plus the workspace build/lint gate.

## What to Change

### 1. `docs/planner-contracts.md` §2 — add "Planner-visible fields are source-scoped"

Add a subsection codifying the S158 Source-Class Rule and listing the
economic (`has_sale_listing`, `seller_for_sale_lot`, `listed_sale_lots_at`),
production (`has_production_job`), physical (`carry_capacity`, `load_of_entity`),
and contention (`facility_queue_position`, `facility_grant`,
`extraction_slot_queue_position`, `actor_holds_extraction_slot_grant`,
`contention_queue_is_full`) accessors now under it. Note that the control/rights
*value* path is unchanged (governed by the existing §2 control language) and that
stricter value-belief-backing is a deferred future spec (new believed-rights
`EntityBeliefAspect`).

### 2. `docs/spec-drafting-rules.md` — add belief-view accessor source-class rule

Add a rule: every new belief-view accessor must declare its source class (self /
same-tick local physical / direct possession / belief-backed / public topology)
and its stale/unknown behavior before implementation. Social/relational facts are
belief-gated even when co-located (FND-14A).

## Files to Touch

- `docs/planner-contracts.md` (modify)
- `docs/spec-drafting-rules.md` (modify)

## Out of Scope

- Any code change to `PerAgentBeliefView` (tickets 001–003).
- Revising the existing §2 control/rights language or `can_control` behavior
  (S158 Non-Goals).

## Acceptance Criteria

### Tests That Must Pass

1. No tests (documentation-only). The documented accessor set matches the set
   gated by tickets 001–003.
2. Existing suite unaffected: `cargo build --workspace`

### Invariants

1. The documented source-class accessor list is consistent with the live gated
   accessors in `per_agent_belief_view.rs` (no documented-but-ungated accessor,
   no gated-but-undocumented accessor).
2. The existing §2 control/rights contract language is preserved unchanged.

## Test Plan

### New/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Commands

1. `cargo build --workspace` (docs do not affect build; confirms no broken
   intra-repo references if any doc-link checker runs)
2. Manual cross-read: documented accessor list vs. tickets 001–003 gated set.
3. `./scripts/verify.sh`
