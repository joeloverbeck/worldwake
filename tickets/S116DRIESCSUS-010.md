# S116DRIESCSUS-010: Canonicalize the authoritative water-access contract for wash

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-systems` wash validation, matching AI/runtime fallout, focused action/planner coverage
**Deps**: specs/S116-drive-escalation-sustained-critical.md, archive/tickets/S116DRIESCSUS-009.md

## Problem

The immediate planner gap for Wash is that the AI cannot currently pursue water acquisition before wash. But even after that is repaired, the authoritative `wash` action still models washing as “consume a directly possessed water lot” rather than as access to a concrete local wash-water substrate. The live world already models wells, wash basins, and resource sources as separate concrete entities at places. If the project wants wash behavior to emerge around those substrates cleanly, it needs one canonical authoritative answer to “what physical state makes water available for washing?” instead of leaving basin-side washing coupled to portable inventory custody alone.

## Assumption Reassessment (2026-04-17)

1. The current authoritative start gate is `wash_preconditions()` in [crates/worldwake-systems/src/needs_actions.rs](/home/joeloverbeck/projects/worldwake/crates/worldwake-systems/src/needs_actions.rs). It requires the target to be a directly possessed `Water` item lot; basin presence, well presence, and co-located water source state do not participate in validation today.
2. The current world already has concrete wash-adjacent substrates: scenarios author `WorkstationTag::WashBasin`, `WorkstationTag::Well`, and `ResourceSource` entries at places. This means a redesigned wash contract can stay concrete and place-local rather than introducing an abstract permission or purpose flag.
3. The same gameplay fact currently has multiple nearby carriers with no single canonical contract: portable water lots, local water resource sources, and wash-basin facilities. This ticket must choose one authoritative carrier for “water available for wash” and remove any contradictory duplicate semantics from the live runtime path.
4. The immediate AI planning gap is distinct and owned by `S116DRIESCSUS-009`. This ticket is the adjacent authoritative-contract follow-up already anticipated by the S116 spec and reports; it should not be folded back into 009 unless implementation proves the two are inseparable.
5. The S116 spec already names this residual bottleneck explicitly in [specs/S116-drive-escalation-sustained-critical.md](/home/joeloverbeck/projects/worldwake/specs/S116-drive-escalation-sustained-critical.md): the direct-possession wash rule is the remaining architectural limit after motive escalation and planner repair.
6. Exact mixed-layer boundary under audit: authoritative `wash` start validation and execution semantics in `worldwake-systems` versus the corresponding planner-visible prerequisite contract in `worldwake-ai`. If the action contract changes, the AI must be updated to match it truthfully.
7. Intended invariant before implementation: wash should become available through one concrete, local, world-state-backed water-access contract that remains belief-driven on the AI side and authoritative on the action side. The fix must not introduce a second parallel “special wash water” lane or purpose-only bookkeeping.
8. Mismatch + correction: the active S116 golden ticket originally tried to prove repeated washing using only “basin + co-located water source” scenario authoring. Reassessment showed that authored setup is not sufficient under the current authoritative direct-possession rule. This ticket is the explicit owner for deciding whether that rule should remain canonical.

## Architecture Check

1. Choosing one concrete authoritative wash-water carrier is cleaner than layering more AI heuristics or purpose flags onto an unresolved action contract. FOUNDATIONS prefers concrete state over abstract intent, and locality prefers place-local substrate over omniscient exceptions.
2. This ticket should end with one canonical wash contract, not multiple lawful aliases such as “directly possessed water OR basin source OR special purpose water.” If the contract changes, the old path should be removed or intentionally preserved with clear reasoning, not left as accidental overlap.

## Verification Layers

1. Canonical wash start condition -> focused runtime/action coverage on `wash_preconditions()` and `wash` execution
2. AI prerequisite reasoning matches the new action contract -> focused planner/candidate/search coverage in `worldwake-ai`
3. Basin-side washing still respects belief locality -> decision trace / focused planning tests, not scenario narrative alone
4. Golden wash-cycle recovery under S116 remains downstream proof in `S116DRIESCSUS-006`, not the primary contract surface for this ticket

## What to Change

### 1. Choose and implement one canonical authoritative wash-water contract

Decide whether washing should remain portable-inventory-driven or become place-/facility-mediated around concrete local water access. Implement that choice in `wash_preconditions()` and any matching execution logic so wash uses one lawful world-state carrier for water access.

### 2. Thread the chosen contract through AI planning truthfully

Update the corresponding AI admission, feasibility, and/or search contract so the planner reasons against the same wash-water rule the authoritative action uses. Do not leave a split where the planner and runtime disagree about what makes washing possible.

### 3. Add focused cross-layer proof before reusing goldens

Add focused runtime and AI tests that prove the chosen contract directly before relying on S116 goldens to carry the architectural explanation.

## Files to Touch

- `crates/worldwake-systems/src/needs_actions.rs` (modify)
- `crates/worldwake-ai/src/candidate_generation.rs` (modify if action contract changes AI admission)
- `crates/worldwake-ai/src/goal_dispatch_decl.rs` or `crates/worldwake-ai/src/feasibility.rs` (modify if planner prerequisites change)
- `crates/worldwake-ai/tests/planner_conformance.rs` and/or focused AI tests (modify)
- `tickets/S116DRIESCSUS-006.md` (modify only if the golden dependency story changes again)

## Out of Scope

- The immediate planner-gap repair where Wash can already pursue lawful water acquisition under the current direct-possession rule — ticket `S116DRIESCSUS-009`
- Ranking/escalation multiplier changes from the landed S116 substrate
- Broad scenario retuning beyond what is needed to prove the new wash-water contract

## Acceptance Criteria

### Tests That Must Pass

1. Focused runtime regression proves the chosen wash-water contract at `wash_preconditions()` / execution level.
2. Focused AI regression proves the planner uses the same chosen contract truthfully.
3. Existing suite: `cargo test -p worldwake-ai`
4. Existing suite: `cargo test -p worldwake-systems needs`

### Invariants

1. There is exactly one canonical authoritative answer to “water is available for washing” after this ticket.
2. The planner and authoritative action layer agree on that answer without special-case shims or dual semantics.

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/needs_actions.rs` or nearby focused runtime coverage — proves the chosen authoritative wash-water contract
2. `crates/worldwake-ai/tests/planner_conformance.rs` and/or focused AI tests — proves planner/runtime parity on the new wash contract
3. `None — golden E2E coverage remains owned by S116DRIESCSUS-006 after the lower-layer contract is corrected.`

### Commands

1. `cargo test -p worldwake-ai conformance_wash -- --exact`
2. `cargo test -p worldwake-systems needs`
3. `cargo test -p worldwake-ai`
4. `cargo clippy --workspace --all-targets -- -D warnings`
