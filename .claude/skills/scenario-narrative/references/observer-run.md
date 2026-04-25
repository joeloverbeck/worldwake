# Observer Run and Dump Reading (for Narrative Extraction)

This reference covers the build-and-run protocol and the dump-reading protocol when the goal is **narrative extraction** rather than anomaly detection. The observer binary and the dump format are shared with the `scenario-analysis` skill, but the reading strategy differs.

## Pre-flight Scan

Read the scenario `.ron` once and extract everything the report's framing needs:

- **Authored intent**: the leading `//` comment block at the top of the file, if present. Quote it verbatim into the report's Section A; do not paraphrase.
- **Seed**: from the scenario root (deterministic replay anchor — name it in Section A).
- **Agents**: each `AgentDef`'s name, control source, and which optional profiles are non-default. Collect: `metabolism_profile`, `drive_thresholds`, `drive_escalation_profile`, `perception_profile`, `exploration_profile`, `tell_profile`, `communication_profile`, `epistemic_disposition`, `theft_disposition`, `justice_disposition`, `violation_disposition`, `combat_profile`, `pursuit_profile`, `patrol_profile` (+ `patrol_route`), `merchandise_profile`, `trade_disposition`, `commodity_valuation`, `substitute_preferences`, `contention_disposition`, `disposal_profile`, `obligation_satiation_profile`, `diversification_profile`, `preference_profile`, `artifact_posting_profile`, `care_weight`, `intention_disposition`, `last_seen_memory`, `expectation_store`, `social_observations`, `cognitive_profile`. Profile *presence* drives feature activation per `docs/scenario-roadmap.md` Section 7; **record which profiles are present, which fields within them are non-default, and the concrete numeric values for the fields that will appear in the report**.
- **Places + topology**: each place's tags (Latrine / Outdoor / Wash-capable / etc.), edges with travel times, visibility/concealment fields, contention policies, facilities, resource sources (with capacity, available stock, and regeneration cadence).
- **Initial items, world state, scenario flags**: initial item lots, ownership, container contents, force-claim state, office holders, crime registers, bandit camps, posted artifacts, social artifacts.
- **Survival-health contract**: `max_authored_critical_run_ticks`, `max_idle_window_ticks_with_elevated_need`, required self-care families, per-need `critical_run_limits` overrides.

Record everything you extract — the report leans heavily on this material in Section A and Section B's "authored substrate" subsections.

## Build & Run

```bash
cargo build -p worldwake-cli --bin observer
```

**Hard gate**: if the build fails, stop and report the error to the user. Do not proceed.

```bash
cargo run -p worldwake-cli --bin observer -- <scenario_path> --ticks <N> --output reports/scenario-narrative-dump.md
```

- Use the scenario path and tick count from the user (default 1440; `--days N` is sugar for `N*1440`).
- The dump is written atomically at the end. If you background the run, wait for the process to exit; do not poll the file mid-write.
- If the binary exits non-zero:
  - **Scenario parse error**: stop and report. If it's schema drift caused by a recent spec, name the field.
  - **Runtime tick error** (e.g., `PreconditionFailed`, missing component): if it's a scenario data issue, stop and report. If it's a code/loader bug, fix it via the smallest narrow change, run the affected crate's tests, rebuild the observer, re-run. Note the fix in the report's Run Notes.
  - **Mid-simulation crash with no dump**: if the observer calls `std::process::exit(1)` before writing the dump, change the tick error handler in `observer.rs` so a partial dump is emitted (replace the exit with `break`). Note the crash tick, the error, and the observer change in Run Notes.
  - **Other errors** (permissions, I/O): stop and report.

**Hard gate**: if `reports/scenario-narrative-dump.md` does not exist or is empty, stop and report.

## Dump Structure

The dump contains 10 sections (older dumps may collapse some of the newer ones; verify against the actual file before pinning to a section number):

- **Section 1**: Run Metadata — scenario, seed, ticks, agents table, places table.
- **Section 2**: Per-Agent Summary — actions taken, perception activity, needs trajectory, location history, idle ticks, behavioral transitions, death tick/cause if any.
- **Section 3**: Decision History — chronological per-tick rows of `GoalOffered`, `GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `GoalSuppressed`, `GoalAbandoned`, `GoalSwitched` events with payload summaries. **This is the most useful section for anchoring belief and plan evolution to specific ticks.**
- **Section 4**: Anomaly Flags — mechanically detected oddities. **For narrative extraction, treat these as candidate inflection ticks worth investigating, not as defects to flag.**
- **Section 5**: Raw Event Sample — first 100 + last 100 events, plus a per-agent action timeline binned by 100 ticks. The first/last 100 event lists carry `Discovery`-tagged WorldMutation rows that anchor when each agent first perceived a place.
- **Section 6**: Per-Agent Belief Summary — known entities, believed locations, social/told/heard/institutional beliefs (item type names like "Apple", not EntityIds). End-state only; pair with Section 3 for evolution.
- **Section 7**: End-State Inventory & Resources — agent possessions, place contents.
- **Section 8**: Per-Agent Decision Summary — planning outcomes, goal selection, committed actions, failed plan attempts, blocked desires, affordances. **This is the spine of the per-agent narrative pass.**
- **Section 9**: Budget Exhaustion Snapshots — may be empty if no budget exhaustion occurred this run.
- **Section 10**: Critical Window Forensics — per-agent forensic entries for any authored-critical window entered. May be empty if no such windows were entered.

## Reading Protocol — Narrative Extraction Differences

Where `scenario-analysis` reads for *anomalies*, this skill reads for *story*. Concretely:

1. **Build the EntityId → name map first** from Section 1's agents and places tables. Use names everywhere in the report; translate EntityIds when quoting raw rows. Item EntityIds in Section 8 are not in the map — leave them as `eXgY` references when quoting.

2. **Section 2 is the chronological backbone**. Walk each agent's location history and action timeline together to anchor the narrative on real ticks. Behavioral transitions ("8→4 action types at tick 1400") are inflection-tick candidates — investigate before claiming them in the report.

3. **Section 6 is read for evolution, not just contents**. The report needs *changes* in beliefs (new entity learned at tick X, hearsay accepted at tick Y, contradiction registered at tick Z), not just final-state belief lists. Section 6 itself is end-state-only; reach for Section 3 (Decision History) and Section 5's first/last-100-events log (with `Discovery`-tagged WorldMutation rows) as the primary tick anchors for belief evolution. Only when neither section provides the needed timing should you consult `traceability-fix-protocol.md` for a cheap-fix candidate.

4. **Section 8 is dense**. Individual rows can exceed 5000 tokens. Never use the Read tool with `limit > 10` on Section 8. Instead:
   - `grep 'Tick breakdown'` and `grep 'Plan search outcomes'` for planning health baseline per agent.
   - `bash grep 'Goals selected' <dump>` for the goal-selection landmark list.
   - `grep 'Failed plan attempts' -A 30 <dump>` for failures and root causes — the report's decision-failure vocabulary maps these directly.
   - `grep 'Affordances available'`, `'Affordances after travel'`, `'Final affordances'` with `-A 15` for goal-context snapshots.
   - For specific decision rows: `bash sed -n 'Xp' <file> | head -c 3000` where X is a line number from prior grep hits.

5. **Cross-section anchoring**. Every claim in the per-agent narrative must be traceable to either Section 2 (location/action timeline), Section 3 (per-tick goal/plan/blocker events), Section 6 (belief), Section 7 (final inventory), or Section 8 (decisions). If you cannot anchor a claim, drop it or trigger the traceability protocol.

## ControlSource Heuristic

The dump does not list `ControlSource` per agent. If the scenario `.ron` is accessible (it is, during the pre-flight scan), read `AgentDef.control_source` directly. A human-controlled agent with no input will have minimal Section 8 planning activity — narrate this honestly as "no AI decisions to recount" rather than as agent failure.

## When Section 9 Is Present

Budget exhaustion snapshots are particularly useful as inflection-tick anchors for the per-agent narrative — they mark moments the planner ran out of search budget and what state the agent was in at the time. Quote them by tick and agent in the narrative; do not aggregate them into smell counts.
