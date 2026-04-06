# S53: Cognitive Profile vs Execution Budget

## Summary

Split `ReasoningProfile` into two distinct layers: `CognitiveProfile` (agent psychology — persisted, behavior-defining, per-agent diverse) and `ExecutionBudget` (engine compression — tunable for performance without changing agent identity). Conformance work now proves that both `max_node_expansions` and `snapshot_travel_horizon` are behavior-changing and therefore belong on `CognitiveProfile`, leaving only `beam_width` and `max_prerequisite_locations` in `ExecutionBudget`.

## Phase

Phase 6: Architectural Substrates II

## Status

COMPLETED

## Crates

- `worldwake-core` (new profile types, component registration)
- `worldwake-ai` (consume split profiles in planner)
- `worldwake-cli` (scenario definition for new profiles)

## Dependencies

- S42 (per-agent reasoning style) — completed (`archive/specs/S42-per-agent-reasoning-style.md`)
- S44 (scenario profile completeness) — completed (`archive/specs/S44-scenario-profile-completeness.md`)

## Design Goals

- Every field in current `ReasoningProfile` is classified as cognitive or engine
- Cognitive fields are per-agent, persisted, behavior-defining, and tested
- Engine fields are global or per-tier, tunable for performance, with validation that behavioral meaning is preserved within declared bounds
- The split is clean enough that a performance optimization pass can safely touch the surviving `ExecutionBudget` fields without changing goal selection

## Non-Goals

- Adding new cognitive parameters beyond reclassifying existing ones
- Changing the planner algorithm
- Automatic budget adaptation based on system load

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Performance May Compress Computation, Never Causality) | Core motivation — engine budget changes must not change world meaning. This also covers replay fidelity: budget changes must not silently break deterministic replay. |
| P20 (Resource-Bounded Practical Reasoning) | Cognitive parameters define who the agent is as a reasoner |
| P22 (Agent Diversity) | Per-agent cognitive variation preserved; engine budget is uniform |
| P29 (Debuggability) | Clear separation makes it obvious which knobs are safe to tune |
| P31 (Validation and Falsification) | The Behavioral Validation Contract defines explicit falsification criteria: if any budget change causes goal-selection divergence, the field must be reclassified |

## Deliverables

### Field Classification

Current `ReasoningProfile` fields (at `crates/worldwake-core/src/reasoning_profile.rs:8-21`), classified:

| Field | Classification | Rationale |
|-------|---------------|-----------|
| `max_candidates_to_plan` | **Cognitive** | How many options the agent considers — bounded foresight |
| `max_plan_depth` | **Cognitive** | How far ahead the agent thinks — planning horizon |
| `switch_margin` | **Cognitive** | Goal-switching reluctance — temperament |
| `transient_block_ticks` | **Cognitive** | Retry patience for transient blocks |
| `unknown_block_ticks` | **Cognitive** | Retry patience for unknown blocks |
| `structural_block_ticks` | **Cognitive** | Give-up threshold — persistence |
| `initial_cooldown_ticks` | **Cognitive** | Retry timing — impulsiveness |
| `max_cooldown_ticks` | **Cognitive** | Maximum backoff — patience ceiling |
| `max_node_expansions` | **Cognitive** | Search budget — conformance proved it changes goal selection under unchanged remaining engine fields. |
| `beam_width` | **Engine** | Search width — compression knob |
| `snapshot_travel_horizon` | **Cognitive** | Planning visibility — conformance proved it changes goal selection. |
| `max_prerequisite_locations` | **Engine** | Prerequisite budget — compression knob |

### New Types

```rust
pub struct CognitiveProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub snapshot_travel_horizon: u8,
    pub max_node_expansions: u16,
    pub switch_margin: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}

pub struct ExecutionBudget {
    pub beam_width: u8,
    pub max_prerequisite_locations: u8,
}
```

### Migration

- Remove `ReasoningProfile` component (P28 — no backward compatibility)
- Add `CognitiveProfile` (universal, per-agent, scenario-definable)
- Add `ExecutionBudget` (universal, per-agent or global default, scenario-overridable)
- `SAVE_FORMAT_VERSION` bump
- All planner code updated to read from both profiles
- `AgentDef` updated with both profile types (per `docs/spec-drafting-rules.md` section 5 — both are universal profiles with Default impls)
- `spawn_agent()` applies both with `unwrap_or_default()` and `expect()` for runtime access

### Behavioral Validation Contract

`ExecutionBudget` changes are only safe if they preserve the **direction** of plan selection within the `CognitiveProfile` bounds. The conformance test outcome is now:

1. `max_node_expansions` and `snapshot_travel_horizon` failed the conformance boundary and were reclassified as cognitive.
2. `beam_width` and `max_prerequisite_locations` remain the surviving execution-budget fields owned by the engine-compression contract.
3. Future execution-budget tuning is only safe if it preserves goal selection with the corrected split.

## Cross-System Interactions (Principle 26)

- **AI planner** reads `CognitiveProfile` for behavior-defining search horizon and expansion limits, and `ExecutionBudget` for the surviving compression bounds
- **Save/load** persists both profiles separately
- **Scenario system** configures both via `AgentDef`
- **Decision traces** label each parameter as cognitive or engine for debugging clarity

All interaction through state. No cross-system direct calls.

## Profile-Driven Parameters

Both `CognitiveProfile` and `ExecutionBudget` are per-agent. `CognitiveProfile` is the primary diversity axis. `ExecutionBudget` may have a global default that individual agents override.

Per `docs/spec-drafting-rules.md` section 5:
- Both are universal profiles — all agents need them for reasoning
- Both require Default impls
- Both added to `AgentDef` in scenario types
- `spawn_agent()` applies both with `unwrap_or_default()`
- Runtime access uses `expect()`

## Component Registration

- `CognitiveProfile` on `EntityKind::Agent` (universal)
- `ExecutionBudget` on `EntityKind::Agent` (universal)
- Remove `ReasoningProfile` registration

## Section H — Causal Hooks

### H.1 Information path
N/A — these are agent-internal parameters, not information. No perception, transmission, or belief updates involved.

### H.2 Positive feedback
None — profile parameters are static per-agent configuration, not dynamic state.

### H.3 Dampeners
N/A — no feedback loops to dampen.

### H.4 Stored vs derived

| Item | Classification |
|------|---------------|
| `CognitiveProfile` on agent | **Stored authoritative state** |
| `ExecutionBudget` on agent | **Stored authoritative state** |
| Plan selection output | **Derived** — recomputed per decision cycle from profiles + beliefs |

### H.5-H.10 (N/A)
No contention, partial failures, belief staleness, temporal resolution, derived views, or error correction applicable — these are static per-agent configuration profiles.

### H.11 Scheduling
Profile reads happen at the start of each AI decision cycle (`agent_tick`). Profile values are stable within a tick. No simultaneity concerns.

### H.13 Invariants and regression
- `CognitiveProfile` changes MUST change golden test behavior (they are behavior-defining)
- `ExecutionBudget` changes with cognitive parameters held constant SHOULD NOT change goal selection
- Conformance now treats `max_node_expansions` and `snapshot_travel_horizon` as cognitive regression surfaces rather than execution-budget knobs
- All existing golden tests must pass after the split with equivalent parameter values

### H.14 Save/load
Old `ReasoningProfile` saves must migrate to `CognitiveProfile` + `ExecutionBudget` at load time. Migration function splits fields by classification table. `SAVE_FORMAT_VERSION` bumped. No backward compatibility — old format is read-migrated, not aliased.

## Verification

### Conformance test: ExecutionBudget behavioral validation

Validate only the surviving execution-budget fields (`beam_width`, `max_prerequisite_locations`) against representative conformance scenarios. If a remaining execution-budget field changes goal selection, it must be reclassified as Cognitive.

### Migration test: ReasoningProfile → CognitiveProfile + ExecutionBudget

Verify that a world saved with `ReasoningProfile` loads correctly with the split profiles and produces identical behavior with equivalent parameter values.

## Outcome

- Completed: 2026-04-05
- Landed the full S53 ticket chain:
  - `S53COGEXE-001`: introduced `CognitiveProfile` and `ExecutionBudget` as universal persisted agent components
  - `S53COGEXE-002`: migrated live AI consumers onto the split carriers
  - `S53COGEXE-003`: removed `ReasoningProfile` entirely and migrated saves to split-only state
  - `S53COGEXE-004`: added focused conformance coverage for the execution-budget boundary
  - `S53COGEXE-005`: reclassified `max_node_expansions` and `snapshot_travel_horizon` from `ExecutionBudget` to `CognitiveProfile` after conformance proved both are behavior-changing
- What changed in the live code:
  - `CognitiveProfile` now owns all behavior-defining reasoning parameters, including search horizon and node-expansion limits
  - `ExecutionBudget` now owns only the surviving compression fields: `beam_width` and `max_prerequisite_locations`
  - CLI/scenario/save surfaces were updated to author and persist the corrected split
  - save migration now covers legacy `ReasoningProfile`, coexistence `24 -> 26`, and prior split-only `25 -> 26`
- Deviation from the original draft:
  - the draft initially treated `max_node_expansions` and `snapshot_travel_horizon` as provisional engine fields; conformance falsified that classification and the final implementation moved both to `CognitiveProfile`
  - the honest proof surface for the execution-budget contract is the focused conformance suite, not an “all goldens” meta-run
- Verification:
  - `cargo test -p worldwake-core`
  - `cargo test -p worldwake-cli`
  - `cargo test -p worldwake-ai --test conformance_execution_budget -- --nocapture`
  - `cargo test -p worldwake-sim -- save`
  - `cargo run -p worldwake-cli -- scenarios/cli-evaluation.ron --exec quit`
  - `cargo test --workspace`
  - `cargo clippy --workspace --all-targets -- -D warnings`
