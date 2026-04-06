# Observer Binary & Skill Upgrades Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Upgrade the observer binary to produce richer dump data (full action timelines, belief summaries, inventory/resource snapshots) and update the simulation-observer skill to match.

**Architecture:** Add three new dump sections to `observer.rs`: Section 5 (Per-Agent Belief Summary), Section 6 (Inventory & Resource Snapshot), and expand the existing Action Trace Summary from tail-only to per-agent full timeline. Also update the skill's reading strategy, smell #8 framing, and report template.

**Tech Stack:** Rust (observer binary in `crates/worldwake-cli/src/bin/observer.rs`), Markdown (skill in `.claude/skills/simulation-observer/SKILL.md`)

---

### Task 1: Expand Action Trace Summary to Per-Agent Full Timeline

The current action trace shows only the last 50 events globally. This makes it impossible to see when behavioral transitions happen (e.g., when Guard Theron stopped acting). Replace with per-agent action timeline histograms binned by 100-tick windows.

**Files:**
- Modify: `crates/worldwake-cli/src/bin/observer.rs` (lines 641-651, the action trace summary formatting)

**Step 1: Replace tail-only action trace with per-agent timeline**

In `format_report()`, replace the current action trace summary block (lines 641-651) with per-agent action histograms. Keep the total count line and the tail-50 for raw inspection, but add a histogram above it.

The histogram should bin actions by 100-tick windows per agent, showing action counts per window. Format:

```
### Action Trace Summary

Total action trace events: 1306

#### Per-Agent Action Timeline (100-tick bins)

**Kael (e5g0)**

| Ticks | Actions |
|-------|---------|
| 0-99 | pick_up×3, eat×2, tell×5 |
| 100-199 | tell×8, sleep×10 |
| ... | ... |

**Merchant Vara (e6g0)**
...
```

Implementation: After the `detect_anomalies` call and before `format_report`, build a `BTreeMap<EntityId, BTreeMap<u64, BTreeMap<String, u32>>>` (agent -> bin -> action_name -> count) from `action_trace.events()`. Pass it to `format_report` as a new parameter.

Actually, simpler: build the histogram inside `format_report` since it already receives `action_trace` and `agents`.

In `format_report`, after writing "Total action trace events:", iterate `agents`, and for each agent iterate `action_trace.events_for(agent_id)`, bin by `event.tick.0 / 100`, collect action names + counts per bin, and format as a table.

Then keep the existing tail-50 block below for raw trace inspection.

**Step 2: Build and verify**

```bash
cargo build -p worldwake-cli --bin observer
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output /tmp/test-dump.md
```

Verify the new Section 4 has per-agent timeline tables and the tail-50 raw trace is still present.

**Step 3: Commit**

```bash
git add crates/worldwake-cli/src/bin/observer.rs
git commit -m "observer: expand action trace to per-agent timeline histograms"
```

---

### Task 2: Add Per-Agent Belief Summary (Section 5)

The dump currently has no belief data, making smell #8 (belief staleness) always INCONCLUSIVE. Add a Section 5 with per-agent belief summaries read from `AgentBeliefStore` at simulation end.

**Files:**
- Modify: `crates/worldwake-cli/src/bin/observer.rs` (add section after anomaly flags, before raw event sample; also pass `&World` to `format_report`)

**Step 1: Pass `&World` to `format_report`**

`format_report` currently doesn't receive the `World` reference. Add `world: &worldwake_core::World` as a parameter. Update the call site in `main()` to pass `sim.world()`.

**Step 2: Add Section 5 formatting**

After Section 3 (Anomaly Flags) and before Section 4 (Raw Event Sample), add a new section:

```markdown
## Section 5 — Per-Agent Belief Summary

### Kael

**Known entities**: 15 (3 agents, 2 places, 10 items)
**Entity beliefs by place**:
- Thornwall Village: 5 entities believed present
- Dusty Trail: 8 entities believed present

**Social observations**: 4
**Told beliefs**: 12 (from 2 counterparties)
**Heard beliefs**: 0
**Institutional beliefs**: 3

**Believed resource locations**:
- Food: [entities with believed inventory containing food commodities]
- Water: [entities with believed resource_source for water]
```

Implementation:
- For each agent, call `world.get_component_agent_belief_store(agent_id)`.
- Count `known_entities` total, broken down by entity kind (use `world.entity_kind()` on each key of `known_entities`).
- Group known entities by `last_known_place` to show "entity beliefs by place".
- Count `social_observations.len()`, `told_beliefs.len()`, `heard_beliefs.len()`, `institutional_beliefs.len()`.
- For resource locations: scan `known_entities` for entries with `resource_source.is_some()` or `last_known_inventory` containing food/water commodity kinds.

Use `entity_display_name(world, id)` for human-readable entity names where available.

**Step 3: Build and verify**

```bash
cargo build -p worldwake-cli --bin observer
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output /tmp/test-dump.md
```

Verify Section 5 exists with belief data for each agent.

**Step 4: Commit**

```bash
git add crates/worldwake-cli/src/bin/observer.rs
git commit -m "observer: add per-agent belief summary section"
```

---

### Task 3: Add Inventory & Resource Snapshot (Section 6)

Add a Section 6 showing what items each agent possesses and what resources exist at each place at simulation end. This directly addresses the "resource scarcity vs. planning inability" diagnostic gap.

**Files:**
- Modify: `crates/worldwake-cli/src/bin/observer.rs`

**Step 1: Add Section 6 formatting**

After Section 5, add:

```markdown
## Section 6 — End-State Inventory & Resources

### Agent Inventories

**Kael**: 2× Grain, 1× Waterskin
**Merchant Vara**: (empty)
...

### Place Contents

**Thornwall Village (e0g0)**: 5× Grain, Well (resource: Water), Campfire (workstation: Cooking)
**Dusty Trail (e2g0)**: (empty)
...
```

Implementation:
- For each agent: `world.possessions_of(agent_id)` → for each item, check `world.get_component_item_lot(item)` for commodity+quantity, or `entity_display_name` for other entities.
- For each place: `world.ground_entities_at(place_id)` → list items, workstations, resource sources. For items use `get_component_item_lot`. For workstations use `get_component_workstation_marker`. For resource sources check for `ResourceSource` component (if accessible).

**Step 2: Build and verify**

```bash
cargo build -p worldwake-cli --bin observer
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output /tmp/test-dump.md
```

Verify Section 6 shows inventories and place contents.

**Step 3: Commit**

```bash
git add crates/worldwake-cli/src/bin/observer.rs
git commit -m "observer: add end-state inventory and resource snapshot"
```

---

### Task 4: Run Clippy and Full Build Verification

**Step 1: Run clippy**

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Fix any warnings introduced by the new code.

**Step 2: Run tests**

```bash
cargo test --workspace
```

**Step 3: Run observer end-to-end**

```bash
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output /tmp/final-dump.md
```

Verify all 6 sections are present and well-formatted.

**Step 4: Commit any fixes**

---

### Task 5: Update Simulation-Observer Skill

Apply the remaining audit suggestions to the skill file.

**Files:**
- Modify: `.claude/skills/simulation-observer/SKILL.md`

**Step 1: Fix Step 3 reading strategy (Audit Issue #1)**

Change the >500 line reading order from:
> "read Section 1 (Run Metadata) and Section 3 (Anomaly Flags) first, then per-agent summaries for flagged agents"

To:
> "read Section 1 (Run Metadata) and Section 2 (Per-Agent Summaries) first, then Section 3 (Anomaly Flags), then Sections 4-6 (traces, beliefs, inventory)"

**Step 2: Reframe smell #8 with new belief data (Audit Improvement #1)**

Since Section 5 now provides belief summaries, update smell #8 from "always INCONCLUSIVE" to actionable analysis. Replace the note about missing belief snapshots with:

> **Belief staleness** -- Cross-reference the agent's belief summary (Section 5) with their action traces and perception traces. Check: does the agent believe resources exist at locations they haven't visited recently? Do their beliefs about entity locations match current placement? Are they acting on stale information when fresher data was available through perception?

Keep a fallback note: "If the belief summary is sparse (few known entities), note the limitation rather than speculating."

**Step 3: Add note about action trace (Audit Improvement #2)**

Under Step 3, add a note:
> "The Action Trace Summary now includes per-agent timeline histograms (100-tick bins) showing when behavioral transitions occur. Use these to identify the tick ranges where agent behavior shifts, then cross-reference with needs trajectory and anomaly tick ranges."

**Step 4: Specify NONE/INCONCLUSIVE handling (Audit Improvement #3)**

In Step 5, add:
> "Include all 10 smell categories in the report regardless of severity. NONE findings should be brief (1-2 sentences confirming no detection). INCONCLUSIVE findings should explain the data limitation."

**Step 5: Update section references for new dump structure**

Update all references to dump sections to reflect the new 6-section structure:
- Section 1: Run Metadata
- Section 2: Per-Agent Summary
- Section 3: Anomaly Flags
- Section 4: Raw Event Sample (with per-agent action timeline + tail traces)
- Section 5: Per-Agent Belief Summary (NEW)
- Section 6: End-State Inventory & Resources (NEW)

**Step 6: Commit**

```bash
git add .claude/skills/simulation-observer/SKILL.md
git commit -m "skill: update simulation-observer for expanded observer dump"
```

---

### Task 6: Verify End-to-End

Run the full skill workflow to confirm the observer binary + updated skill work together.

```bash
cargo run -p worldwake-cli --bin observer -- scenarios/cli-evaluation.ron --ticks 1440 --output reports/simulation-observer-dump.md
```

Spot-check the dump manually: all 6 sections present, belief data populated, inventory snapshot populated, action timeline bins populated.
