# MCP Observer + Trace Tooling — Design

## Brainstorm Context

**Original request:** Given what you know of this repository (rely on `docs/FOUNDATIONS.md`), figure out, researching in-depth online as necessary, if there are MCPs we should add to our repertoire to make the day-to-day working on this repository much smoother and capable.

**Reference file:** `docs/FOUNDATIONS.md` (principles guidance).

**Key interview insights that shaped the design:**

1. **Research-narrowed solution space.** An external-research pass (April 2026) found that the MCP ecosystem offers little off-the-shelf value for this project:
   - Rust-specific MCPs (`rust-analyzer-mcp`, `rust-mcp-server`) heavily overlap Serena + direct `cargo` usage — redundant.
   - Markdown knowledge-graph servers (`markdown-vault-mcp`) require frontmatter standardization and duplicate what project skills already handle (e.g., `post-ticket-review`).
   - No mature Rust static-analysis, dead-code, or simulation-trace-diff MCPs exist.
   - GitHub MCP (official) was the only broadly-applicable candidate — useful but low priority for this user, who only occasionally needs GitHub introspection for failed PR workflows.

2. **User-identified primary pain:** Reading/querying large observer/trace output. Loading full reports into context is wasteful; a scoped query surface would be "fantastic."

3. **Cross-spec consistency is already handled** by `.claude/skills/` and `.codex/skills/` (notably `post-ticket-review`) — not a gap an MCP should fill.

4. **Scope-discipline concern raised by the user:** Adding trace dumping to in-test code must not slow the normal golden-test lanes. Investigation confirmed tracing is opt-in per-test (163 `enable_tracing` calls across 14 files, all explicit) — so a purely opt-in `write_json` method on sinks is safe, but automatic trace dumping was explicitly de-scoped.

5. **Data surface decision:** Observer markdown reports already exist and are structured. The user chose "observer reports + JSON dump mode" as the core deliverable. Trace-dump support was reinstated as Phase 2 after clarifying that opt-in dumping costs nothing on unrelated lanes.

**Final confidence:** 95%. Phased approach approved.

**Assumptions carried forward:**
- `rmcp` v1.5.0 (April 2026) remains the actively maintained Rust MCP SDK.
- Observer data is aggregative enough to serialize without restructuring authoritative sim types.
- CI pipeline has the Rust toolchain available; no extra build infra needed.

---

## 1. Overview

Ship a small Rust MCP server alongside the workspace that makes `observer` output (and later, opt-in trace dumps) queryable during investigation and debugging sessions. The goal is replacing "paste a 3000-line markdown report into context" with structured, scoped queries ("show budget-exhaustion snapshots for agent X" / "list all anomaly flags of kind Y").

**Two phases:**

- **Phase 1:** Observer gains a `--json <path>` sidecar emitter. A new `worldwake-mcp` crate implements an MCP server using the official `rmcp` SDK, with tools that run the observer or query a cached JSON report.
- **Phase 2 (only after Phase 1 pays off):** Opt-in `TraceSink::write_json(path)` and `ActionTraceSink::write_json(path)` methods, plus MCP tools that query those dumps. Regular golden lanes untouched.

**Non-goals:**
- Not changing simulation behavior.
- Not auto-dumping traces from any test that doesn't already opt in.
- Not replacing the markdown report (it stays the human-readable primary artifact).
- Not overlapping Serena's code-navigation surface.

---

## 2. Architecture / Structure

**New workspace member:** `crates/worldwake-mcp/`

```
crates/
  worldwake-mcp/
    Cargo.toml          # depends on rmcp, serde, serde_json, tokio, anyhow
    src/
      main.rs           # server bootstrap (stdio transport)
      server.rs         # rmcp ServerHandler impl
      tools/
        mod.rs
        run_observer.rs # Phase 1: spawns observer, caches JSON
        report_query.rs # Phase 1: queries cached observer JSON
        trace_query.rs  # Phase 2: queries trace dump JSON (stub in P1)
      cache.rs          # scenario+seed+ticks -> parsed JSON cache (in-memory)
      schema/
        mod.rs          # shared serde types matching observer JSON dump
        observer.rs     # Phase 1 schema
        trace.rs        # Phase 2 schema (stub in P1)
```

**Dependency direction:**

- `worldwake-mcp` depends on `serde_json` and `rmcp`; **not** on other worldwake crates. It reads observer output as opaque JSON following a versioned schema.
- This deliberately keeps the MCP crate decoupled from simulation internals — it stays a pure consumer of already-emitted artifacts. Changing an internal trace type in `worldwake-sim` does not force an MCP rebuild unless the JSON schema changes.
- The **schema is the contract**, owned by a small shared types module (initially duplicated in `worldwake-mcp/src/schema/` and `worldwake-cli/src/bin/observer.rs` sidecar emission; unified into a `worldwake-mcp-schema` crate only if drift becomes a real problem — avoiding premature extraction).

**Observer change (Phase 1):**

- New CLI flag `--json <path>` on `observer.rs`.
- After the existing markdown emission, serialize the same structured data (per-agent snapshots, anomaly flags, budget exhaustion records, scenario summary) to JSON.
- Markdown emission path unchanged.

**Transport:** stdio (standard for local MCP servers launched by Claude Code).

---

## 3. Key Decisions

**D1. Use `rmcp` (official Rust MCP SDK), not a Python/TypeScript MCP server.**
Keeps the stack Rust-native. Workspace members already use `serde`. Honest tradeoff: `rmcp` is newer than the Python SDK, mitigated by being the officially maintained reference (v1.5.0 as of April 2026).

**D2. Observer JSON schema is versioned and additive.**
Top-level `{ "schema_version": "1.0", ... }`. Bumped when a field is removed or renamed; additions don't bump. Aligns with FND-28 (no backward-compat shims in live authority paths): JSON is a derived view (FND-27), so breaking changes are permitted provided old cached JSON files are rejected rather than silently accepted.

**D3. Cache is in-memory, keyed by `(scenario_path, seed, ticks)`.**
No disk cache in v1. Process lifetime only. Re-running the same `(scenario, seed, ticks)` reuses the parsed JSON without re-spawning the observer. Same key → same result (simulation is deterministic per project invariants).

**D4. Observer spawning uses `cargo run --bin observer --release`, working dir = workspace root.**
Release profile to match CI/bench conditions. Env-var override: `WORLDWAKE_MCP_OBSERVER_PROFILE=dev` for faster iteration. MCP tool returns stderr on non-zero exit; does not swallow build errors.

**D5. Tool surface is query-shaped, not grep-shaped.**
Tools named by question (`budget_exhaustion_for`, `anomalies_of_kind`, `agent_tick_summary`), not by data structure. Each tool does one focused thing.

**D6. Phase 2 trace dump is gated behind explicit opt-in, schema-separate from observer JSON.**
`TraceSink::write_json(path)` is a method on the sink; tests call it deliberately. Optional env-var wrapper (`WORLDWAKE_DUMP_TRACES=<dir>`) handled at harness level only, not inside per-test code. Separate schema file (`schema/trace.rs`) — different domain (per-agent-per-tick) from observer (per-scenario aggregate).

**D7. MCP registration is project-level via `.mcp.json` at repo root, checked in.**
Discoverable and shared across contributors. Global user-level registration is opt-in, not default.

---

## 4. Data Flow

**Phase 1 — run-and-query path:**

```
Claude → MCP tool: run_observer(scenario="scenarios/default.ron", seed=42, ticks=500)
  → MCP spawns: cargo run --release --bin observer -- --scenario ... --seed 42 --ticks 500 --json /tmp/ww-mcp-<hash>.json
  → Observer writes markdown (unchanged) AND json sidecar
  → MCP reads+parses json, stores in in-memory cache keyed by (scenario, seed, ticks)
  → Returns summary header: { scenario, total_agents, tick_count, anomaly_count, cache_key }

Claude → MCP tool: anomalies_of_kind(cache_key, kind="BudgetExhaustion")
  → MCP reads from cache
  → Returns filtered list (no re-run)

Claude → MCP tool: agent_tick_summary(cache_key, agent_id=12, tick_range=[50, 60])
  → MCP reads from cache, slices, returns compact record
```

**Phase 1 — query-existing-json path:**

```
Claude → MCP tool: load_report(path="reports/scenario-analysis-report.json")
  → MCP parses, caches under (path_hash)
  → Returns cache_key + summary header
```

Both paths converge on the same cache and query tools.

**Phase 2 — trace dump path (additive):**

```
Test code (explicit opt-in):
  h.driver.enable_tracing();
  for _ in 0..20 { h.step_once(); }
  h.driver.trace_sink().unwrap().write_json("target/trace-dumps/<test>.json");

Claude → MCP tool: load_trace_dump(path)
  → MCP parses trace JSON, caches separately from observer cache
  → Returns cache_key

Claude → MCP tool: trace_agent_decisions(cache_key, agent_id, tick_range)
  → Returns per-tick decision records (candidates, plan attempts, selection, outcome)
```

**Error handling:**

- Observer spawn failure → structured error with stderr tail; does not crash.
- JSON parse failure → schema-version diagnostic (expected N, got M) rather than raw serde error.
- Cache miss on `cache_key` → "no such report cached; call `run_observer` or `load_report` first."

---

## 5. Phase 1 Scope — Observer JSON + MCP Core

**Observer changes (`crates/worldwake-cli/src/bin/observer.rs`):**

- Add `--json <path>` CLI flag.
- After markdown emission, build a structured `ObserverReport` struct with the same data, serialize via `serde_json` (pretty-printed).
- Markdown output: unchanged. No behavior difference when flag absent.

**JSON schema (v1.0) minimum fields:**

- `schema_version: "1.0"`
- `scenario: { path, seed, ticks }`
- `agents: [{ id, name, final_state, anomaly_flags, needs_trajectory_summary }]`
- `sections: [{ name, entries: [...] }]` — one entry per markdown section
- `budget_exhaustion_snapshots: [{ agent_id, goal_debug, tick, expansions_used, max_depth_reached, planner_cfg, ... }]`
- `anomalies: [{ agent_id, tick, kind, detail }]`

**MCP crate tools (Phase 1):**

- `run_observer(scenario, seed, ticks) -> { cache_key, summary }`
- `load_report(path) -> { cache_key, summary }`
- `list_cached_reports() -> [cache_key]`
- `report_summary(cache_key) -> ObserverSummary`
- `agents(cache_key, filters?) -> [AgentHeader]`
- `agent_detail(cache_key, agent_id) -> AgentRecord`
- `anomalies_of_kind(cache_key, kind, agent_filter?) -> [Anomaly]`
- `budget_exhaustion_for(cache_key, agent_filter?) -> [Snapshot]`
- `section(cache_key, section_name) -> SectionContent`
- `agent_tick_summary(cache_key, agent_id, tick_range) -> [TickRecord]` (deferred if tick-level data not in report)

**Configuration:**

- `.mcp.json` at workspace root:
  ```json
  { "mcpServers": { "worldwake": { "command": "cargo", "args": ["run", "--release", "-p", "worldwake-mcp"] } } }
  ```
- Document in `docs/debugging-traces.md` (new subsection).

**Out of Phase 1 scope:** trace-sink JSON dumping, disk-backed cache, report diffing, running goldens.

---

## 6. Phase 2 Scope — Opt-in Trace Dump

**Trigger to start Phase 2:** Observer MCP has been used for at least one real investigation cycle and a concrete "I wish I could query the decision trace for this failing golden" moment has been identified. If Phase 1 doesn't surface that friction, Phase 2 stays deferred.

**Sim changes:**

- `worldwake-sim/src/trace/`: add `write_json(path)` to `TraceSink` and `ActionTraceSink`. Pure opt-in; no caller changes required.
- Serialization only activates when `write_json` is called. Normal tracing (in-memory collection) is unchanged.
- Harness-level env-var hook in `crates/worldwake-ai/tests/golden_harness/mod.rs`: if `WORLDWAKE_DUMP_TRACES` is set AND tracing was enabled on the harness, dump sinks to `$WORLDWAKE_DUMP_TRACES/<test_name>.json` on drop. Zero effect when unset.

**Trace JSON schema (v1.0):**

- `schema_version`, `scenario_or_test`, `tick_range`
- `decision_traces: [{ agent_id, tick, outcome_kind, candidates, plan_attempts, selection, interrupt? }]`
- `action_traces: [{ agent_id, tick, action_kind, lifecycle, abort_reason? }]`

**MCP additions (Phase 2):**

- `load_trace_dump(path) -> cache_key`
- `trace_agent_decisions(cache_key, agent_id, tick_range?) -> [DecisionTrace]`
- `trace_plan_failures(cache_key, reason_filter?) -> [PlanAttempt]`
- `trace_interrupts(cache_key, agent_id?) -> [InterruptRecord]`
- `trace_action_lifecycle(cache_key, agent_id, tick_range?) -> [ActionEvent]`

**Explicit non-additions:**

- No automatic trace dumping in tests that don't already enable tracing.
- No `write_json` calls added to existing tests unless a specific test has a documented debugging need.

---

## 7. Testing Strategy

**Phase 1 tests:**

- **Observer JSON schema fidelity:** Unit test in `worldwake-cli/tests/` runs the observer on `scenarios/default.ron` with both `--json out.json` and markdown output, parses the JSON, asserts: (1) every markdown section has a corresponding JSON section, (2) agent counts match, (3) `schema_version` is present and equal to `"1.0"`, (4) every `budget_exhaustion_snapshot` has all required fields populated.
- **Schema regression test:** Snapshot test — observer JSON for a fixed `(scenario, seed, ticks)` tuple is committed under `crates/worldwake-cli/tests/snapshots/`. Any schema-shape change forces an explicit snapshot update (FND-28 discipline).
- **MCP crate unit tests:** In `crates/worldwake-mcp/tests/`, feed canned JSON fixtures into the cache and assert each tool returns correct filtered output. No observer spawn — fast and decoupled.
- **MCP integration test:** One end-to-end test that spawns the `worldwake-mcp` binary, sends a `run_observer` request over stdio, verifies the response. Gated behind `#[ignore]` by default; runnable via `cargo test -p worldwake-mcp -- --ignored`.
- **CI policy:** Schema snapshot + MCP unit tests run in normal CI. Integration test runs in a separate job.

**Phase 2 tests (deferred until Phase 2 lands):**

- `TraceSink::write_json` round-trip test.
- Harness env-var hook test: set/unset `WORLDWAKE_DUMP_TRACES`, assert file presence/absence.
- MCP trace-query fixture tests parallel to Phase 1 pattern.

**Out of testing scope:** re-testing simulation correctness (existing goldens cover that); testing every `rmcp` protocol edge case (trust the SDK).

---

## 8. FOUNDATIONS Alignment

| Principle | Alignment |
|-----------|-----------|
| **FND-12** (Performance may compress computation, never causality) | Cache keyed by `(scenario, seed, ticks)` respects determinism — same key, same answer. JSON is an encoding of the same data the observer already computes. |
| **FND-27** (Derived summaries are caches, never truth) | Observer JSON and trace JSON are derived views over authoritative simulation state. Deleting a cached JSON and regenerating produces identical output. Neither is a source of truth. |
| **FND-28** (No backward compatibility in live authority paths) | MCP is not on the live authority path. Observer JSON is versioned; old versions are rejected, not accommodated via shims. |
| **FND-29** (Debuggability is a product feature) | Direct enablement — the MCP exists specifically to let investigators ask "why did this agent do that?" and get scoped answers without loading entire reports. |
| **FND-29A** (Causal history append-only and queryable) | JSON dumps are append-only artifacts of a completed simulation run — they never rewrite history. Multiple dumps from different seeds coexist without contradiction. |

**Principles deliberately not invoked:**

- FND-1–11 (Causal Standard, World Dynamics): dev tooling does not alter simulation causality.
- FND-19 (Agent Symmetry): MCP is a dev-facing channel, not in-world information flow; does not create an omniscient side channel for agents.
- FND-26 (Systems interact through state): MCP crate intentionally does not depend on sim internals. Interaction is through JSON artifacts on disk.

**Risk flags to watch during implementation:**

- If Phase 2 trace dump ever grows a "live" query path that reaches into the running simulation, stop — that would cross from dev tool into omniscient side channel (FND-19 risk).
- If the JSON schema ever becomes authoritative for any simulation decision, stop — that promotes a cache to truth (FND-27 violation).
