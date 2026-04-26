# Observer Run and Dump Reading (Steps 0–2)

## Step 0: Pre-flight Scan

Read the scenario `.ron` file once and extract everything needed for downstream steps:

- **Scenario purpose**: The first `//` comment block at the top of the file, if present. Include in report Run Summary as **Scenario purpose**; in the conclusion, note whether the run achieved it.
- **Agent profiles**: Which optional profiles each agent has — especially `exploration_profile`, `obligation_satiation_profile`, `cognitive_profile`, `metabolism_profile`, `perception_profile`. Used in Layer 2 classification.
- **Place topology**: Edges and travel times.
- **Facilities, resource sources, initial items**.

**Survival gap checks** (report findings as "Pre-flight Warnings" in Run Summary; informational only — do not gate the observer run):

- **Agents without food recipes**: Any AI agent whose `known_recipes` contains no food-producing recipe (e.g., only "Harvest Water") will be unable to produce food.
- **Agents without `perception_profile`**: Severely limited observation; effectively blind to ground items.
- **Locations without reachable food/water**: For each place with agents, trace travel edges (2–3 hop radius) to check food/water reachability. Flag isolated locations.
- **Agents without water access for washing**: Wash requires possessed Water (not a facility). Flag agents with no reachable water source (Well, River, or other water-producing facility) within 2–3 travel hops.
- **Agents with disabled social profiles**: If `tell_profile.max_tell_candidates` is zero for all agents, note: "Social interaction disabled — smell 9 (Social Isolation) is expected by design." Prevents false reporting in Layer 1.

## Step 1: Build & Run Observer

```bash
cargo build -p worldwake-cli --bin observer
```

**Hard gate**: If the build fails, stop and report the error.

```bash
cargo run -p worldwake-cli --bin observer -- <scenario_path> --ticks <N> --output reports/scenario-analysis-dump.md
```

- Use the scenario path and tick count from the user (default 1440).
- The dump is written atomically at the end; the final write phase is CPU-intensive for large simulations. If backgrounding, wait for the process to exit rather than polling the output file.
- If the binary exits non-zero, diagnose:
  - **Scenario parse error** (missing field, wrong type): stop and report. If it's schema drift (field renamed/added by a recent spec), note which field.
  - **Runtime tick error** (e.g., `PreconditionFailed`, missing component): decide whether it's (a) a scenario data issue → stop and report, or (b) a code/loader bug → fix, run the affected crate's tests, rebuild the observer, re-run. Note the fix in Run Summary.
  - **Mid-simulation crash with no dump**: if the observer calls `std::process::exit(1)` before writing the dump, change the tick error handler in `observer.rs` to `break` so a partial dump is emitted. Note crash tick, error message, and any observer fix in Observer Notes.
  - **Other errors** (permissions, I/O): stop and report.

**Hard gate**: If `reports/scenario-analysis-dump.md` does not exist or is empty, stop and report.

## Step 2: Read the Dump

Read `reports/scenario-analysis-dump.md`. If it exceeds 500 lines, read section by section using headers (`## Section N`) with offset-based reads. Build an entity-name mapping from Section 1 (agents + places tables) — all subsequent sections reference entities by EntityId (e.g., `e5g0`). Use names in the report; translate EntityIds when quoting raw dump data. Section 1 only maps agents and places; item EntityIds in failed plan attempts and blocked desires cannot be translated — leave as-is.

**ControlSource heuristic**: The dump doesn't list ControlSource per agent. If the scenario file is accessible, check `AgentDef.control_source: Human`. Otherwise, treat no planning activity in Section 8 as evidence of human control. Matters for smell 3 (stuck agents): a human-controlled agent with no input will always appear stuck — expected behavior.

**The dump has 10 sections** (older dumps may collapse some of the newer ones; verify against the actual file before pinning to a section number):

- **Section 1**: Run Metadata (scenario, seed, ticks, agents, places)
- **Section 2**: Per-Agent Summary (actions, perception, needs, locations, idle ticks, behavioral transitions, death tick/cause)
- **Section 3**: Decision History — chronological per-tick rows of `GoalOffered`, `GoalCommitted`, `PlanAdopted`, `BlockerRecorded`, `ReplanTriggered`, `GoalSuppressed`, `GoalAbandoned`, `GoalSwitched` events with payload summaries.
- **Section 4**: Anomaly Flags (mechanically detected smells)
- **Section 5**: Raw Event Sample (first/last 100 events plus a per-agent action timeline binned by 100 ticks)
- **Section 6**: Per-Agent Belief Summary (known entities, believed locations, social/told/heard/institutional beliefs; uses item type names e.g., "Waste", "Apple", not EntityIds)
- **Section 7**: End-State Inventory & Resources (agent possessions, place contents). Places with 500+ SocialArtifacts from post_notice/tell spam appear as extremely long single lines — note the pollution count and skip enumeration.
- **Section 8**: Per-Agent Decision Summary (planning outcomes, goal selection, failed plans, blocked desires, affordances)
- **Section 9**: Budget Exhaustion Snapshots — may be empty if no budget exhaustion occurred this run.
- **Section 10**: Critical Window Forensics — per-agent forensic entries for any authored-critical window entered. May be empty if no such windows were entered.

**Section 8 reading protocol** (lines are extremely dense — individual rows can exceed 5000 tokens; never use Read with `limit` > 10 on Section 8). For each agent, extract in this order:

1. Grep `Tick breakdown` and `Plan search outcomes` — planning health baseline.
2. `bash grep 'Goals selected' <dump>` — goal types (too long for the Grep tool).
3. Grep `Failed plan attempts` with `-A 30` — failures and root causes.
4. Grep `Blocked desires` with `-A 10` — may be absent; skip if not found.
5. Grep `Affordances available`, `Affordances after travel`, and `Final affordances` with `-A 15`.
6. For specific decision timeline rows: `bash sed -n 'Xp' <file> | head -c 3000` where X is a line number from prior Grep hits.
