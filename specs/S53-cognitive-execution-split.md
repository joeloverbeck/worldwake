# S53: Cognitive Profile vs Execution Budget

## Summary

Split `ReasoningProfile` into two distinct layers: `CognitiveProfile` (agent psychology — persisted, behavior-defining, per-agent diverse) and `ExecutionBudget` (engine compression — tunable for performance without changing agent identity). Golden test S97 proves that `max_node_expansions` changes agent behavior, meaning it is currently a cognitive parameter masquerading as a performance knob.

## Phase

Phase 6: Architectural Substrates II

## Status

Draft

## Crates

- `worldwake-core` (new profile types, component registration)
- `worldwake-ai` (consume split profiles in planner)
- `worldwake-cli` (scenario definition for new profiles)

## Dependencies

- S42 (per-agent reasoning style) — completed
- S44 (scenario profile completeness) — completed

## Design Goals

- Every field in current `ReasoningProfile` is classified as cognitive or engine
- Cognitive fields are per-agent, persisted, behavior-defining, and tested
- Engine fields are global or per-tier, tunable for performance, with validation that behavioral meaning is preserved within declared bounds
- The split is clean enough that a performance optimization pass can safely touch `ExecutionBudget` without golden test failures

## Non-Goals

- Adding new cognitive parameters beyond reclassifying existing ones
- Changing the planner algorithm
- Automatic budget adaptation based on system load

## FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| P12 (Compression ≠ Causality) | Core motivation — engine budget changes must not change world meaning |
| P20 (Practical Reasoning) | Cognitive parameters define who the agent is as a reasoner |
| P22 (Agent Diversity) | Per-agent cognitive variation preserved; engine budget is uniform |
| P29 (Debuggability) | Clear separation makes it obvious which knobs are safe to tune |
| P31 (Replay Fidelity) | Engine budget changes with behavioral validation prevent silent replay divergence |

## Deliverables

### Field Classification

Current `ReasoningProfile` fields, classified:

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
| `max_node_expansions` | **Engine** | Search budget — provably behavior-changing (S97), but intended as engine compression |
| `beam_width` | **Engine** | Search width — compression knob |
| `snapshot_travel_horizon` | **Engine** | Planning visibility — compression knob |
| `max_prerequisite_locations` | **Engine** | Prerequisite budget — compression knob |

### New Types

```rust
pub struct CognitiveProfile {
    pub max_candidates_to_plan: u8,
    pub max_plan_depth: u8,
    pub switch_margin: Permille,
    pub transient_block_ticks: u32,
    pub unknown_block_ticks: u32,
    pub structural_block_ticks: u32,
    pub initial_cooldown_ticks: u32,
    pub max_cooldown_ticks: u32,
}

pub struct ExecutionBudget {
    pub max_node_expansions: u16,
    pub beam_width: u8,
    pub snapshot_travel_horizon: u8,
    pub max_prerequisite_locations: u8,
}
```

### Migration

- Remove `ReasoningProfile` component
- Add `CognitiveProfile` (universal, per-agent, scenario-definable)
- Add `ExecutionBudget` (universal, per-agent or global default, scenario-overridable)
- `SAVE_FORMAT_VERSION` bump
- All planner code updated to read from both profiles
- `AgentDef` updated with both profile types
- `spawn_agent()` applies both with defaults

### Behavioral Validation Contract

`ExecutionBudget` changes are only safe if they preserve the **direction** of plan selection within the `CognitiveProfile` bounds. A conformance test should verify:
- For any `CognitiveProfile`, reducing `max_node_expansions` may degrade plan *quality* (shorter plans, fallback to barriers) but must not change goal *selection* if the cognitive parameters are unchanged
- If a budget change causes a goal selection change, that budget field should be reclassified as cognitive

## Cross-System Interactions

- **AI planner** reads `CognitiveProfile` for goal-selection parameters, `ExecutionBudget` for search bounds
- **Save/load** persists both profiles separately
- **Scenario system** configures both via `AgentDef`
- **Decision traces** label each parameter as cognitive or engine for debugging clarity

## Profile-Driven Parameters

Both `CognitiveProfile` and `ExecutionBudget` are per-agent. `CognitiveProfile` is the primary diversity axis. `ExecutionBudget` may have a global default that individual agents override.

## Component Registration

- `CognitiveProfile` on `EntityKind::Agent` (universal)
- `ExecutionBudget` on `EntityKind::Agent` (universal)
- Remove `ReasoningProfile`

## Section H — Causal Hooks

1. **Information path**: N/A — these are agent-internal parameters, not information.
2. **Positive feedback**: None.
3. **Dampeners**: N/A.
4. **Stored vs derived**: Both profiles are stored authoritative state. Plan selection is derived.
