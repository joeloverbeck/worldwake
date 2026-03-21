# E16CINSBELRECCON-008: Tell Integration — Institutional Claims via HeardBeliefMemory

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — extend tell action handler in worldwake-systems
**Deps**: E16CINSBELRECCON-001, E16CINSBELRECCON-003, E16CINSBELRECCON-004

## Problem

When an agent Tells another about an institutional fact, the claim must flow through the existing E15c `HeardBeliefMemory` system before projecting into the listener's `institutional_beliefs`. This unifies all social information flow through one channel and preserves provenance, chain length, and resend-suppression logic. Without this, institutional claims via Tell would bypass E15c and create a parallel propagation path.

## Assumption Reassessment (2026-03-21)

1. `tell_actions.rs` in worldwake-systems handles the Tell action. It already writes into `HeardBeliefMemory` using `TellMemoryKey` for entity beliefs.
2. E15c's `ToldBeliefMemory` / `HeardBeliefMemory` / `TellMemoryKey` are in `belief.rs` in worldwake-core. The current `TellMemoryKey` may need extension to cover institutional claim identity (or institutional claims map to existing keys).
3. Chain length degradation: direct witness → Report(chain_len=1) → Report(chain_len=2) per spec §E15c Integration.
4. N/A — not a planner ticket.
5. N/A — no ordering.
6. N/A — no heuristic removal.
7. N/A.
8. N/A.
9. N/A.
10. N/A.
11. No mismatch.
12. N/A.

## Architecture Check

1. Routing institutional Tell through existing HeardBeliefMemory is cleaner than a separate institutional-only propagation path — follows Principle 24 (systems interact through state) and reuses E15c's resend-suppression.
2. No backward-compatibility shims.

## Verification Layers

1. Tell about office holder → listener gains institutional belief with `Report` source → belief store inspection
2. Chain length increments through relay → two-hop Tell test
3. Resend suppression prevents redundant institutional claims → TellMemoryKey check
4. Institutional claims enter HeardBeliefMemory before projecting to institutional_beliefs → intermediate state check

## What to Change

### 1. Extend `TellMemoryKey` for institutional claims

If `TellMemoryKey` does not already cover institutional claim identity, add a variant or field that distinguishes institutional claims by their `InstitutionalBeliefKey`. This ensures resend-suppression works correctly for institutional topics.

### 2. Extend Tell handler in `tell_actions.rs`

When an agent Tells an institutional claim:
- Write the claim into the listener's `HeardBeliefMemory` with appropriate `TellMemoryKey`
- After HeardBeliefMemory write, project the institutional claim into `institutional_beliefs` using `WorldTxn::project_institutional_belief()` with `InstitutionalKnowledgeSource::Report { from: speaker, chain_len }`
- Chain length = speaker's own chain_len + 1 (0 if speaker was direct witness)

### 3. Extend ToldBeliefMemory for institutional claims

When an agent decides to Tell an institutional claim, record it in `ToldBeliefMemory` so the agent doesn't redundantly re-tell the same claim to the same listener.

## Files to Touch

- `crates/worldwake-core/src/belief.rs` (modify — extend `TellMemoryKey` if needed for institutional claim identity)
- `crates/worldwake-systems/src/tell_actions.rs` (modify — handle institutional claims in Tell commit, project beliefs)

## Out of Scope

- AI deciding WHAT institutional claims to Tell (Phase B2 / candidate generation)
- ConsultRecord action (ticket -005)
- Perception projection for witnesses (ticket -006)
- Record mutation (ticket -007)
- Non-institutional Tell subjects (already handled by E15c)

## Acceptance Criteria

### Tests That Must Pass

1. Agent A Tells Agent B about an office holder → B gains `InstitutionalClaim::OfficeHolder` belief with `Report { from: A, chain_len: 1 }` source
2. Agent B relays to Agent C → C gains belief with `Report { from: B, chain_len: 2 }`
3. Resend suppression: A telling B the same institutional claim twice does not duplicate the belief entry
4. Institutional claims flow through `HeardBeliefMemory` before reaching `institutional_beliefs`
5. `ToldBeliefMemory` records the institutional tell to prevent redundant re-telling
6. Existing suite: `cargo test -p worldwake-systems`

### Invariants

1. All social information flows through HeardBeliefMemory (no parallel path bypassing E15c)
2. Chain length monotonically increases through relays
3. Resend-suppression prevents duplicate institutional claims from same speaker→listener pair
4. Provenance is always traceable (source agent + chain length)

## Test Plan

### New/Modified Tests

1. `crates/worldwake-systems/src/tell_actions.rs` — institutional tell projection, chain length degradation, resend suppression, two-hop relay

### Commands

1. `cargo test -p worldwake-systems tell_actions`
2. `cargo clippy --workspace && cargo test --workspace`
