# S42: Per-Agent Reasoning Style

## Summary

Replace the shared global `PlanningBudget` with a per-agent `ReasoningProfile` ECS component that governs planning depth, search patience, goal-switching reluctance, retry timing, and cooldown progression. Today all agents use identical reasoning parameters (224 expansions, 8 beam width, 100 permille switch margin, same cooldown curves). This makes agents that should behave very differently under uncertainty — cautious merchants vs reckless bandits — plan with identical thoroughness, switch goals at the same threshold, and retry at the same pace. The fix is straightforward: move existing `PlanningBudget` fields into a per-agent component registered on `EntityKind::Agent`, then read the agent's profile at each decision point instead of the driver-level default.

## Source

Derived from the ChatGPT architecture review (`brainstorming/improvements-to-ai-architecture.md`, Issue #10) validated against the actual codebase. All agents confirmed to share identical `PlanningBudget` constructed once in `AgentTickDriver::new()`. The existing per-agent profile pattern (`PerceptionProfile`, `TellProfile`, `UtilityProfile`, `CombatProfile`, `IntentionDispositionProfile`, `PursuitProfile`) is well established and should be extended to reasoning style.

## Phase

Phase 5: Architectural Substrates

## Crates

- `worldwake-core` (new `ReasoningProfile` component)
- `worldwake-ai` (consume per-agent profile instead of driver-level budget; remove `PlanningBudget` from `AgentTickDriver`)

## Dependencies

- None. All prerequisite infrastructure exists. Can be scheduled in parallel with any other spec.

## FOUNDATIONS Alignment

- **Principle 22, Agent Diversity Through Concrete Variation**: "Agents in the same role must differ in needs, skills, values, loyalties, courage, greed, patience, memory reliability, perception fidelity, and tolerance for risk or ambiguity. These differences come from concrete per-agent parameters." Reasoning style — how deeply an agent searches, how readily it switches goals, how long it waits before retrying — is a direct expression of patience, risk tolerance, and cognitive style. The current architecture violates this principle by forcing identical planning parameters on all agents.
- **Principle 3, Concrete State Over Abstract Scores**: The profile is a concrete per-agent struct stored as authoritative state, not a derived heuristic.
- **Principle 20, Resource-Bounded Practical Reasoning**: Agents with smaller search budgets reason more impulsively; agents with larger budgets reason more carefully. Both are lawful expressions of bounded rationality with different bounds.
- **Principle 28, No Backward Compatibility**: `PlanningBudget` is fully replaced, not shimmed. No `From` bridge or deprecated wrapper survives the migration. `AgentTickDriver` no longer stores a budget; all call sites are updated in a single pass.

## Design Goals

1. **Follow existing profile pattern**: `ReasoningProfile` is a `Component` registered on `EntityKind::Agent`, with `Default` providing current behavior. No behavioral change for agents without an explicit profile.
2. **No new planner architecture**: The existing `PlanningBudget` fields move into the profile. The driver resolves the profile from the world's component tables for each agent, falling back to `ReasoningProfile::default()`.
3. **Per-agent diversity, not per-tick variability**: The profile is stable authoritative state (survives save/load), not a per-decision random draw.
4. **Migration-safe defaults**: `ReasoningProfile::default()` must produce the exact same values as the current `PlanningBudget::default()` so all existing tests pass without modification.

## Deliverables

### 1. `ReasoningProfile` component (`worldwake-core`)

```rust
/// Per-agent reasoning style parameters controlling planning depth,
/// goal-switching reluctance, and retry timing.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReasoningProfile {
    /// Maximum goals to run plan search for per decision pass.
    pub max_candidates_to_plan: u8,
    /// Maximum steps in a single plan.
    pub max_plan_depth: u8,
    /// How many travel hops to include in the planning snapshot.
    pub snapshot_travel_horizon: u8,
    /// Maximum prerequisite locations to consider per search.
    pub max_prerequisite_locations: u8,
    /// Total node expansion budget for GOAP search.
    pub max_node_expansions: u16,
    /// Beam width for best-first search.
    pub beam_width: u8,
    /// Motive increase (in permille of current score) required before
    /// switching from the active goal to a same-class challenger when
    /// no intention frame is active. Higher = more stubborn. Lower = more
    /// flighty. When the agent has an active intention frame,
    /// `IntentionDispositionProfile.commitment_switch_margin` takes
    /// precedence (see Cross-System Interactions below).
    pub switch_margin: Permille,
    /// Ticks before retrying a transiently blocked goal.
    pub transient_block_ticks: u32,
    /// Ticks before retrying a goal blocked for unknown reasons.
    pub unknown_block_ticks: u32,
    /// Ticks before giving up on a structurally blocked goal.
    pub structural_block_ticks: u32,
    /// Initial cooldown ticks after exhaustion before retry.
    pub initial_cooldown_ticks: u32,
    /// Maximum cooldown ticks (caps exponential backoff).
    pub max_cooldown_ticks: u32,
}

impl Component for ReasoningProfile {}
```

Default values must exactly match current `PlanningBudget::default()`:

| Field | Default | Meaning |
|-------|---------|---------|
| `max_candidates_to_plan` | 2 | Plan for top 2 ranked goals |
| `max_plan_depth` | 8 | Max 8-step plans |
| `snapshot_travel_horizon` | 6 | Consider places within 6 hops |
| `max_prerequisite_locations` | 3 | Up to 3 prerequisite stops |
| `max_node_expansions` | 224 | Search budget |
| `beam_width` | 8 | Beam search width |
| `switch_margin` | 100 permille | 10% motive increase to switch (maps from `PlanningBudget.switch_margin_permille`) |
| `transient_block_ticks` | 20 | Retry transient blocks after 20 ticks |
| `unknown_block_ticks` | 5 | Retry unknown blocks after 5 ticks |
| `structural_block_ticks` | 200 | Give up after 200 ticks |
| `initial_cooldown_ticks` | 4 | Start cooldown at 4 ticks |
| `max_cooldown_ticks` | 64 | Cap cooldown at 64 ticks |

### 2. Component registration (`worldwake-core`)

Register `ReasoningProfile` in the component schema for `EntityKind::Agent`. Follow the existing macro-based pattern used by `PerceptionProfile`, `TellProfile`, `UtilityProfile`, etc. in `component_schema.rs` (the `with_component_schema_entries!` macro with `|kind| kind == EntityKind::Agent` filter predicate).

### 3. `AgentTickDriver` reads per-agent profile (`worldwake-ai`)

Remove the driver-level `PlanningBudget` field from `AgentTickDriver` entirely. Modify the per-agent decision path in `agent_tick` to:

1. Look up `ReasoningProfile` for the current agent from the world's component tables.
2. If present, use it. If absent, use `ReasoningProfile::default()`.
3. Pass the resolved profile to all downstream consumers: `search_plan()`, `compare_goal_switch()` (via `goal_switch_margin_details()`), `handle_plan_failure()`, `blocking_fact_ttl()`, cooldown computation (`record_budget_exhaustion()`).

This requires updating:
- `AgentTickDriver` struct and `AgentTickDriver::new()` — remove `budget` field and parameter.
- `AgentTickDriverState` (serialized state) — remove `budget` field. Existing saves with the old format will need a migration that discards the serialized budget (since per-agent profiles now live in the component store).
- `from_saved_runtime()` — no longer needs budget deserialization.
- All CLI call sites that construct `AgentTickDriver::new(PlanningBudget::default())`: `worldwake-cli/src/main.rs`, `worldwake-cli/src/handlers/tick.rs`, `worldwake-cli/src/handlers/actions.rs`, `worldwake-cli/tests/integration.rs`.
- All functions that currently take `&PlanningBudget` — update signatures to take `&ReasoningProfile`.

`PlanningBudget` itself is deleted from `worldwake-ai` (no shim, no re-export, no deprecated wrapper — Principle 28).

### 4. Save/load round-trip

`ReasoningProfile` is authoritative state and must survive save/load as a component in the world's component store. Bump `SAVE_FORMAT_VERSION` (currently 13 at `crates/worldwake-sim/src/save_load.rs:6`). Add a save/load round-trip test verifying profile preservation.

The `AgentTickDriverState` serialized format also changes (budget field removed). If existing save files contain `AgentTickDriverState` with a `budget` field, the migration must handle this gracefully (e.g., ignore the deserialized budget since all agents now read from their component).

### 5. Golden test: reasoning style diversity

At least one golden test must prove that two agents with different `ReasoningProfile` values produce observably different behavior from the same starting conditions:

- **Scenario**: Two agents at the same place, same needs, same beliefs. Agent A has `switch_margin: Permille(50)` (flighty), Agent B has `switch_margin: Permille(300)` (stubborn). A new higher-motive goal appears. Agent A switches; Agent B stays committed.
- **Variant**: Agent C has `max_node_expansions: 32` (impulsive), Agent D has default 224 (thorough). Present a goal that requires a 4-step plan. Agent D finds it; Agent C may not (or finds a shorter fallback).

Both scenarios with deterministic replay companions.

## Section H — FOUNDATIONS Analysis

### H.1 Information-path analysis

`ReasoningProfile` is read-only authoritative state on the agent entity. No information propagation needed — the profile is local to the agent and accessed directly by the AI pipeline. No perception, belief, or social transmission path is involved. The profile is never communicated to other agents (it represents internal cognitive style, not observable behavior).

### H.2 Positive-feedback analysis

No amplifying loops introduced. The profile is static per-agent (set at creation, modified only by explicit world processes if ever). Reasoning depth does not feed back into itself — a deeper search does not make future searches deeper.

One indirect loop to monitor: an agent with a large search budget may find better plans, succeed more often, and therefore accumulate more resources. But this is an intended consequence of agent diversity (smarter agents do better), and is dampened by the same physical-world mechanisms that limit all accumulation (inventory limits, need decay, time cost, competition).

### H.3 Concrete dampeners

- **Search budget**: `max_node_expansions` is a hard integer cap. Not a clamp on a runaway loop — it is an inherent resource bound on the agent's cognitive process. This is the FOUNDATIONS-approved form: a concrete capacity limit, not a numeric clamp on an output.
- **Cooldown cap**: `max_cooldown_ticks` prevents infinite backoff.
- **Accumulation from better planning**: Dampened by existing physical mechanisms — inventory weight limits, need decay, facility queues, travel time, competition from other agents.

### H.4 Stored state vs. derived read-model list

| Item | Classification |
|------|---------------|
| `ReasoningProfile` component | **Stored authoritative state** — persists in save/load, set at entity creation |
| Resolved profile for current decision | **Derived** — read from component or fallback each tick, never stored separately |
| `AgentTickDriverState` | **Stored** — serialized driver state; no longer contains budget (per-agent profiles live in the component store) |

## Cross-System Interactions (Principle 12)

`ReasoningProfile` interacts with other systems exclusively through state-mediated reads:

- **Candidate generation** reads `max_candidates_to_plan` to limit how many goals enter search.
- **Search** reads `max_node_expansions`, `beam_width`, `max_plan_depth`, `snapshot_travel_horizon`, `max_prerequisite_locations`.
- **Goal switching** reads `switch_margin` as the **fallback default** via `goal_switch_margin_details()` (`crates/worldwake-ai/src/agent_tick/active_action.rs`). **Precedence**: when the agent has an active intention frame AND an `IntentionDispositionProfile`, the profile's `commitment_switch_margin` takes priority. `ReasoningProfile.switch_margin` applies only when no frame is active or no `IntentionDispositionProfile` exists. This two-tier design is intentional: `commitment_switch_margin` governs stubbornness about an already-committed plan, while `switch_margin` governs baseline goal-switching reluctance in the uncommitted state.
- **Failure handling** reads `transient_block_ticks`, `unknown_block_ticks`, `structural_block_ticks`.
- **Cooldown** reads `initial_cooldown_ticks`, `max_cooldown_ticks`.

No system writes to `ReasoningProfile` as a side effect. Profile mutation (if ever needed) would be an explicit world process (e.g., learning, injury-induced cognitive degradation).

## Migration Path

1. Add `ReasoningProfile` to `worldwake-core` with `Component` impl and `Default`.
2. Register in component schema for `EntityKind::Agent`.
3. Remove `PlanningBudget` from `AgentTickDriver` and `AgentTickDriverState`. Update `AgentTickDriver::new()` to take no budget parameter.
4. Update all `agent_tick` functions to resolve `ReasoningProfile` from the world's component tables (with `Default` fallback). Replace all `&PlanningBudget` parameters with `&ReasoningProfile`.
5. Update all CLI call sites (`main.rs`, `handlers/tick.rs`, `handlers/actions.rs`, integration tests) that previously passed `PlanningBudget::default()`.
6. Delete `PlanningBudget` from `worldwake-ai` (`budget.rs`). No shim, no re-export.
7. Update test harness / golden test setup to optionally attach profiles.
8. Write golden test proving diversity.
9. Bump `SAVE_FORMAT_VERSION`.

## Verification

- `cargo test --workspace` passes with no behavioral change (all agents still use default values).
- Golden test proves two agents with different profiles diverge.
- Save/load round-trip preserves `ReasoningProfile`.
- `cargo clippy --workspace --all-targets -- -D warnings` clean.
- No remaining references to `PlanningBudget` in the codebase.
