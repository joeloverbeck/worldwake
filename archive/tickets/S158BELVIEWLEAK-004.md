# S158BELVIEWLEAK-004: Source-class contract documentation

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: None
**Deps**: archive/tickets/S158BELVIEWLEAK-001.md, archive/tickets/S158BELVIEWLEAK-002.md, archive/tickets/S158BELVIEWLEAK-003.md

## Problem

Before this ticket, the belief-view source-class rule that S158 enforced in code
through tickets 001-003 was not codified as a durable contract. `docs/planner-contracts.md`
§2 documented the entity-admission belief barrier and the control/rights gating,
but not the per-accessor source-class rule for economic, production, physical,
and contention reads, and `docs/spec-drafting-rules.md` had no rule requiring new
accessors to declare their source class. S158 D2 and D3 owned that documentation
gap.

## Assumption Reassessment (2026-05-21)

<!-- Apply all domain-specific precision rules from docs/precision-rules.md -->

1. `docs/planner-contracts.md` §2 ("Entity admission and the belief barrier")
   existed and already covered entity admission plus the
   `ControlBeliefView::can_control` co-located-unowned-item shortcut. This ticket
   added a "Planner-visible fields are source-scoped" subsection without altering
   the existing control/rights language; S158 leaves that path as S155 set it.
2. `docs/spec-drafting-rules.md` existed and had no rule on belief-view accessor
   source-class declaration. This ticket added that rule alongside the existing
   Section H, HTN, and agent-profile drafting rules.
3. Documentation-only ticket; no shared runtime boundary. The documented
   accessor list matches the S158 set gated by archived tickets 001-003 and
   enumerated in `specs/S158-belief-view-remote-truth-leak-closure.md`
   (Source-Class Rule + D1).

## Architecture Check

1. Codifying the source-class rule in the two authoritative contract docs
   (planner-contracts for the live planner boundary, spec-drafting-rules for
   future spec authorship) prevents leak regressions at review time rather than
   relying on per-PR vigilance — the cheapest durable enforcement surface.
2. No backward-compatibility concern: additive documentation; the existing §2
   control/rights language is preserved unchanged, and the new subsection
   explicitly notes that stricter rights-value belief-backing is deferred.

## Verified Layers

1. Source-class rule documented and consistent with the gated accessor set ->
   manual cross-read against archived tickets 001-003 plus S158 D1.
2. Single-layer documentation ticket; no runtime invariant to map. Verification
   is review-based plus `cargo build --workspace`.

## Landed Changes

### 1. `docs/planner-contracts.md` §2 — "Planner-visible fields are source-scoped"

Added a subsection codifying the S158 Source-Class Rule and listing the economic
(`has_sale_listing`, `seller_for_sale_lot`, `listed_sale_lots_at`), production
(`has_production_job`), physical (`carry_capacity`, `load_of_entity`), and
contention (`facility_queue_position`, `facility_grant`,
`extraction_slot_queue_position`, `actor_holds_extraction_slot_grant`,
`contention_queue_is_full`) accessors now under it. The subsection records that
the control/rights value path is unchanged and stricter value-belief-backing
requires a future believed-rights or jurisdiction aspect.

### 2. `docs/spec-drafting-rules.md` — belief-view accessor source-class rule

Added a rule requiring every new belief-view accessor to declare its source class
(self, same-tick local physical, direct possession, belief-backed, or public
topology) and stale/unknown behavior before implementation. The rule also states
that social and relational facts remain belief-gated even when co-located.

## Landed Files

- `docs/planner-contracts.md`
- `docs/spec-drafting-rules.md`
- `archive/tickets/S158BELVIEWLEAK-004.md`

## Out of Scope

- Any code change to `PerAgentBeliefView` (tickets 001–003).
- Revising the existing §2 control/rights language or `can_control` behavior
  (S158 Non-Goals).

## Acceptance Result

### Proof That Passed

1. No new tests were needed because this was documentation-only. The documented
   accessor set was cross-read against S158 and archived tickets 001-003.
2. Existing workspace build was unaffected: `cargo build --workspace`.

### Invariants

1. The documented source-class accessor list is consistent with the live gated
   accessors in `per_agent_belief_view.rs` (no documented-but-ungated accessor,
   no gated-but-undocumented accessor).
2. The existing §2 control/rights contract language is preserved unchanged.

## Test Plan Result

### Added/Modified Tests

1. `None — documentation-only ticket; verification is command-based and existing runtime coverage is named in Assumption Reassessment.`

### Proof Commands

1. Passed `cargo build --workspace`.
2. Passed manual cross-read: documented accessor list vs. S158 and archived
   tickets 001-003 gated set.
3. Waived `./scripts/verify.sh` for this ticket iteration because the
   harness-owned pre-push phase runs the full wrapper after the final S158 spec
   archive; the only implementation diff here is non-generated Markdown.

## Outcome

Completed on 2026-05-21.

- Added the source-scoped planner-visible field contract to
  `docs/planner-contracts.md`, including the exact S158 economic, production,
  physical, and contention accessor set.
- Added the belief-view accessor source-class drafting rule to
  `docs/spec-drafting-rules.md`.
- Corrected this ticket's dependency record to point at the archived S158
  implementation tickets that delivered the code contract.

## Deviations

- The ticket remained documentation-only. No code, generated docs, or tests
  changed.
- `./scripts/verify.sh` was left to the harness pre-push phase rather than run
  for this single Markdown-only ticket iteration.

## Verification Result

- Passed `cargo build --workspace`
- Passed manual cross-read of the documented accessor list against
  `specs/S158-belief-view-remote-truth-leak-closure.md` and
  `archive/tickets/S158BELVIEWLEAK-001.md`,
  `archive/tickets/S158BELVIEWLEAK-002.md`, and
  `archive/tickets/S158BELVIEWLEAK-003.md`
- Passed `git diff --check -- docs/planner-contracts.md docs/spec-drafting-rules.md tickets/S158BELVIEWLEAK-004.md`
  before archival, then rechecked the archived path after the move
- Waived `./scripts/verify.sh` for this ticket iteration because it is the
  harness-owned final pre-push gate after S158 spec archival
