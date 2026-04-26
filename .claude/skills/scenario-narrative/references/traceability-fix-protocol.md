# Hybrid Traceability Fix Protocol

When the skill is composing a section of the report and discovers it cannot write the section honestly because the observer dump does not surface the necessary data, the Hybrid traceability protocol decides whether to fix the gap inline or fall back to a ticket. This reference is the decision tree.

## When To Trigger This Protocol

Trigger on any of:

- The narrative needs an *event tick* (when a belief was acquired, when a contradiction registered, when a goal first appeared) but the dump only emits end-state summaries.
- The narrative needs the *reason* the planner ranked goal A above goal B at a specific tick, and Section 8's goal-selection rows are aggregate-only for that agent.
- The narrative wants to attribute a failure to a specific decision-failure category from `agent-narrative-structure.md` but the dump does not surface enough provenance to do so honestly.
- A Section B feature is *active by substrate* but no dump anchor exists to confirm activation, even though the feature is widely understood to have fired.

Do **not** trigger this protocol for:

- Information that simply wasn't in this scenario (e.g., no theft occurred → no theft narrative). Mark the feature inactive in Section B and move on.
- Subjective calls about agent intent. The skill never speculates on intent.
- Things the engine genuinely does not compute. If the data does not exist anywhere in the engine, neither inline fix nor a small ticket is the right move; flag this as a meta-observation in Section E (Realism / Resourcefulness / Resilience) for the external researcher to consider, and proceed.

## Decision Tree

```
Gap detected.
  |
  v
Is the data already computed somewhere in the engine,
and the only missing step is exposing it in the dump?
  |
  +-- YES -->  Is the change ≲ 30 lines of observer.rs,
  |             AND can it be added to an existing
  |             dump section (no new section/protocol),
  |             AND has no inline fix already been
  |             applied this invocation?
  |              |
  |              +-- YES --> CHEAP FIX. Apply, rebuild
  |              |             observer, re-run, re-read
  |              |             the affected section, note
  |              |             in Run Notes appendix.
  |              |
  |              +-- NO  --> STRUCTURAL. Ticket route.
  |
  +-- NO  -->  STRUCTURAL. Ticket route.
```

### Cheap-Fix Definition (precise)

All of the following must hold:

1. The data already exists in an in-memory structure the observer touches.
2. The fix is a new field on an existing dumped struct, or a new line within an existing dump section, or a small added counter/timestamp emission. **No new dump section. No new event type. No new component read. No cross-system coordination.**
3. The patch fits in roughly 30 lines or fewer in `observer.rs`.
4. **No inline fix has been applied earlier this invocation.** The hard cap is one cheap fix per run. The second discovered gap, even if it would also qualify as cheap, is reclassified as structural — repeated patches in one pass mean the skill is overreaching its scope and the gaps need to be planned, not patched.

If any condition fails, the gap is structural.

### Cheap-Fix Procedure

1. State the gap and the planned change to the user in one sentence before editing.
2. Apply the change to `observer.rs`.
3. Run `cargo build -p worldwake-cli --bin observer`. If it fails, revert and reclassify as structural.
4. Re-run the observer with the same arguments and output path.
5. Re-read the affected dump section.
6. Continue composing.
7. In the report's Run Notes appendix, name the file and a one-line description of what was added (e.g., "Added belief-acquisition tick to Section 6 per-agent belief summary so the per-agent narrative could anchor discovery moments to specific ticks.").

The cheap-fix is a runtime convenience, not a design change. It must not alter simulation behavior, scenario loading, or any non-observer module. If the smallest viable fix would touch anything outside `observer.rs`, treat it as structural.

### Structural Procedure

1. Stop composing the affected section.
2. Note the gap in a working list.
3. Continue composing the rest of the report. Where the affected section is, write a "Data limitation" note inline in the report (one or two sentences, plain English) that names exactly what could not be told and what was inferred or omitted instead.
4. After the report is composed and written, draft a ticket in `tickets/` using the template at `tickets/_TEMPLATE.md`. The ticket should:
   - Name a clear `<PREFIX-NNN>` ID. Use `OBSTRACE` (observer traceability) as the prefix unless the user has set a different convention.
   - Title: short imperative ("Surface belief-acquisition tick per entry in observer Section 6").
   - **Status**: PENDING.
   - **Priority**: MEDIUM unless the gap blocked a feature row entirely (HIGH).
   - **Effort**: Small for fields/lines, Medium for new sections, Large for new event types or component reads.
   - **Engine Changes**: state which crate(s) and module(s) the fix would touch.
   - **Problem**: name the narrative section that could not be written and what the report instead said.
   - **Assumption Reassessment**: include the relevant numbered items from the template — at minimum #1 (current code/test state), #2 (specs/docs reference), and any of #5–#13 that apply. Skip the irrelevant items rather than padding.
   - **Verification Layers**: describe how the fix's correctness would be verified (re-running the observer on this same scenario and confirming the missing field appears).
   - **What to Change** and **Files to Touch**: concrete edits, paths.
5. List the ticket in the report's Run Notes appendix with its path.

## Run Notes Appendix Format

Only present in the report if at least one cheap fix was applied or at least one ticket was created. Format:

```markdown
## Run Notes

### Inline observer fixes applied this run

- `crates/worldwake-cli/src/bin/observer.rs`: <one-line description>

### Traceability tickets created this run

- `tickets/<PREFIX-NNN>.md` — <ticket title>
```

If neither applies, omit the appendix entirely.

## Why This Cap Exists

The skill's primary deliverable is the narrative report. Inline patching is a convenience to keep cheap, obvious gaps from rotting in the dump and to keep the skill's quality compounding over time. It is not a license to refactor the observer mid-run. The one-fix cap forces every additional gap discovered in a single pass to become a ticket — visible, reviewed, and integrated through the normal planning loop instead of accumulating silently in `observer.rs` across many invocations.
