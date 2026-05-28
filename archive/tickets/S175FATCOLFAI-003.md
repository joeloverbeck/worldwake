# S175FATCOLFAI-003: `exhaustion_collapse_observed` forensic flag on `CriticalWindowReport`

**Status**: ✅ COMPLETED
**Priority**: MEDIUM
**Effort**: Medium
**Engine Changes**: Yes — `worldwake-ai` survival forensics read-model
**Deps**: S175FATCOLFAI-001, S175FATCOLFAI-002

## Problem

The end-to-end collapse chain (failed rest → fatigue critical exposure → Exhaustion wound → wound-load death) is provable from authoritative state, but a forensic reader has no single downstream-facing signal to identify *which* critical windows ended in exhaustion collapse without iterating wound events. S175 D5 adds a derived `exhaustion_collapse_observed: bool` to `CriticalWindowReport` so golden tests and the observer can pair the flag with the per-frame `failed_rest_opportunities` records (from S174) to answer "why did this agent collapse from fatigue?".

## Assumption Reassessment (2026-05-28)

1. `CriticalWindowReport` (`crates/worldwake-ai/src/survival_forensics.rs:21`) derives `#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]` — **no `Default`**. Its fields are `agent, need, start_tick, end_tick, threshold, peak_value, frames: Vec<CriticalWindowFrame>`. It is constructed only via `WindowBuilder::flush()` (`survival_forensics.rs:274`, runtime) plus two literal construction sites in tests: `survival_forensics.rs:1343` and `crates/worldwake-cli/src/bin/observer.rs:7006` (`sample_critical_window_report`, inside the `#[cfg(test)]` block at `observer.rs:6098`). All three literal-construction sites must add the new field (3 sites, below the 15-site threshold — effort stays driven by the wiring, not the count). The new field carries `#[serde(default)]` so older serialized reports deserialize as `false`.
2. Spec D5 (`specs/S175-fatigue-collapse-and-failed-rest-traceability.md:150`) sets the flag `true` when the window ends with an Exhaustion wound creation event for the focal agent OR the agent dies with `DeathCause::NeedDeprivation { need: Fatigue }` during the window. `DeprivationKind::Exhaustion` (from 001) and the `Fatigue` death cause (from 002) are the upstream signals.
3. **Observation-surface gap (design decision for implementation)**: `SurvivalForensicExtractor::observe` (`survival_forensics.rs:194`) currently takes `(tick, &HomeostaticNeeds, &DriveThresholds, Option<&AgentDecisionTrace>, &ActionTraceSnapshot, &LocalSurvivalStateSummary)`. None of these carry wound-list or death state — `LocalSurvivalStateSummary` (`survival_forensics.rs:93`) holds only `place` + affordance-presence booleans. The extractor therefore needs a **new per-tick exhaustion-collapse signal**. Recommended approach: extend `observe()` with a small signal (e.g. `exhaustion_collapse_signal: bool` derived by the caller from the agent's `WoundList` gaining an `Exhaustion` wound this tick OR a `DeadAt { cause: NeedDeprivation { need: Fatigue } }`), threaded into the active `WindowBuilder` and surfaced in `flush()`. The exact signal shape is pinned during implementation; do not extend `LocalSurvivalStateSummary` unless that proves cleaner, since its `capture()` contract is affordance-presence, not wound/death.
4. Production caller: `observer.rs:5849` invokes `extractor.observe(...)` inside the per-tick loop (extractors created at `observer.rs:5683-5689`, finalized at `observer.rs:6011-6015`). The caller has `world` access (it already calls `LocalSurvivalStateSummary::capture(world, agent)`), so it can derive the exhaustion-collapse signal from `world.get_component_wound_list(agent)` / `world.get_component_dead_at(agent)`. Any golden-harness caller that constructs a `SurvivalForensicExtractor` and calls the 6-arg `observe` must be updated to pass the new argument.
5. FND-27 (derived views are caches, not truth): `exhaustion_collapse_observed` is a derived read-model field over the wound list + death event — it is never authoritative. The authoritative state remains `WoundList` + `DeadAt`; the flag is recomputable from them. Documented in spec's Stored-vs-Derived table.

## Architecture Check

1. The flag lives on the existing `CriticalWindowReport` read-model rather than introducing a new report type — it pairs naturally with the per-frame `failed_rest_opportunities` already on `CriticalWindowFrame` (S174). Threading a per-tick boolean signal into the existing `observe()` flow keeps the extractor's "fold per-tick observations into windows" shape intact (FND-26: the extractor reads authoritative state via the caller, writes only its derived report).
2. No backwards-compatibility shim: `#[serde(default)]` is the lawful save/replay-compat mechanism for an additive derived field (a boundary-normalization default, not a live-authority alias). The flag is recomputed on every fresh extraction, so an older report deserializing as `false` is corrected on recomputation.

## Verification Layers

1. Flag is `true` for a window that ends in an Exhaustion wound / fatigue death -> focused unit test on `WindowBuilder`/extractor (derived read-model state).
2. Flag is `false` for a window that recovers before any Exhaustion wound -> focused unit test (derived read-model state).
3. Older serialized report without the field deserializes as `false` -> focused serde-default unit test (mirrors the existing `critical_window_frame_deserializes_missing_failed_rest_as_empty` pattern, `survival_forensics.rs` test block at `:660`).
4. End-to-end flag-on-collapse / flag-off-recovery against real scenarios -> deferred to S175FATCOLFAI-004 goldens (this ticket proves the mechanism in isolation; the E2E proof is the golden ticket's surface).

## What to Change

### 1. Add the field to `CriticalWindowReport`

Add `#[serde(default)] pub exhaustion_collapse_observed: bool` to the struct (`survival_forensics.rs:21`).

### 2. Thread the exhaustion-collapse signal through `observe` → `WindowBuilder` → `flush`

Extend `SurvivalForensicExtractor::observe` with the new per-tick signal (item 3 of Assumption Reassessment), store it on the active `WindowBuilder` (set/latch when the signal fires during the window), and emit it from `WindowBuilder::flush()` into the new field. A latched boolean (once true, stays true for the window) matches the spec's "window ends with … OR dies during the window" semantics.

### 3. Update the production caller and test constructors

Update `observer.rs:5849` to derive the signal from the agent's `WoundList` (new `Exhaustion` wound this tick) and `DeadAt` (fatigue cause), and pass it to `observe()`. Add the new field to the two literal `CriticalWindowReport { … }` test constructions (`survival_forensics.rs:1343`, `observer.rs:7006`). Update any golden-harness caller of the 6-arg `observe` discovered during reassessment.

## Files to Touch

- `crates/worldwake-ai/src/survival_forensics.rs` (modify — struct field, `observe` signature, `WindowBuilder`, `flush`, `#[cfg(test)]` tests)
- `crates/worldwake-cli/src/bin/observer.rs` (modify — production `observe` caller at `:5849`, test sample constructor at `:7006`)
- `Likely:` golden test-harness helper that calls `SurvivalForensicExtractor::observe` — `grep -rn "SurvivalForensicExtractor" crates/worldwake-ai/tests` during reassessment to pin whether a harness wrapper passes through the 6-arg `observe`.

## Out of Scope

- Upstream wound/death production (S175FATCOLFAI-002).
- The `DeprivationKind::Exhaustion` variant (S175FATCOLFAI-001).
- E2E golden scenarios asserting the flag (S175FATCOLFAI-004).
- An incapacitation-without-death flag (spec Open Question 1 — deferred).
- Making the flag authoritative state — it remains a derived read-model field (FND-27).

## Acceptance Criteria

### Tests That Must Pass

1. A window whose frames span an Exhaustion-wound-creation tick (or a fatigue-death tick) flushes with `exhaustion_collapse_observed == true`.
2. A window that recovers before any Exhaustion wound flushes with `exhaustion_collapse_observed == false`.
3. A `CriticalWindowReport` serialized without the field deserializes with `exhaustion_collapse_observed == false`.
4. Existing suite: `cargo test -p worldwake-ai survival_forensics` and `cargo test -p worldwake-cli --bin observer`

### Invariants

1. `exhaustion_collapse_observed` is derived solely from wound-creation + death signals; deleting and recomputing it from `WoundList` + `DeadAt` yields the same value (FND-27).
2. The flag latches within a window (a transient mid-window signal is preserved through `flush`).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-ai/src/survival_forensics.rs` (`#[cfg(test)]`) — flag-true (collapse), flag-false (recovery), and serde-default tests. Rationale: D5 mechanism in isolation.

### Commands

1. `cargo test -p worldwake-ai survival_forensics`
2. `cargo test -p worldwake-cli --bin observer`
3. `cargo clippy -p worldwake-ai -p worldwake-cli --all-targets -- -D warnings`
4. `scripts/verify.sh`

## Outcome

**Completion date**: 2026-05-28

**What changed**:
- `crates/worldwake-ai/src/survival_forensics.rs`:
  - Added `#[serde(default)] pub exhaustion_collapse_observed: bool` to `CriticalWindowReport`.
  - Added a `exhaustion_collapse_observed: bool` field to the private `WindowBuilder` (init `false` in `new`, emitted in `flush`).
  - Extended `SurvivalForensicExtractor::observe` with a `exhaustion_collapse_signal: bool` param; after the per-need loop it latches the flag onto the **active fatigue window** (matching the established precedent that fatigue-specific forensics — `failed_rest_opportunities` — attach only to the fatigue window). The latch persists through `flush`.
  - Added a public free helper `exhaustion_collapse_signal(world, agent, tick) -> bool` that derives the per-tick signal from authoritative state: an `Exhaustion` deprivation wound with `inflicted_at == tick`, OR a `DeadAt` with `cause == NeedDeprivation { need: Fatigue }` and `tick == tick`. Both production and golden-harness callers reuse this helper (DRY).
- `crates/worldwake-cli/src/bin/observer.rs`: production caller derives the signal via the helper and passes it to `observe`; test sample constructor (`sample_critical_window_report`) sets the new field.
- `crates/worldwake-ai/tests/golden_harness/survival_forensics_assertions.rs`: `observe_critical_windows` derives the signal from `harness.world` and threads it through.
- Three integration-test callers (`forensic_determinism.rs`, `forensic_sleep_progress_barrier.rs`, `forensic_wash_vs_water_competition.rs`) pass `false` — those synthetic scenarios have no exhaustion collapse.

**Design decision (signal scope)**: Per the ticket's latitude (Assumption Reassessment item 3 — "exact signal shape pinned during implementation"), the flag latches onto the **Fatigue** window specifically rather than all active windows. Rationale: exhaustion is definitionally a fatigue consequence, and the codebase already scopes fatigue-specific forensics (`failed_rest_opportunities`, line 331) to the fatigue window only. On the wound-creation/death tick fatigue is always critical (the counter only reaches threshold while fatigue ≥ critical), so the fatigue window is guaranteed active when the signal fires.

**New tests** (`survival_forensics.rs` `#[cfg(test)]`): flag-true latch (collapse), flag-false (recovery), serde-default deserialization, and a helper test exercising the wound/non-exhaustion-wound/fatigue-death derivation directly against a real `World`.

**Verification**:
- `cargo test -p worldwake-ai --lib survival_forensics` — 18 passed (4 new).
- `cargo test -p worldwake-ai` — 1776 + 263 (69 ignored) + integration suites all green.
- `cargo test -p worldwake-cli --bin observer` — 106 passed.
- `cargo clippy -p worldwake-ai -p worldwake-cli --all-targets -- -D warnings` — clean (added `#[allow(clippy::too_many_arguments)]` to `observe`, now 8 args, matching the codebase pattern; collapsed the latch `if` into a let-chain).
