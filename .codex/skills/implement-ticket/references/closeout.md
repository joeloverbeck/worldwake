# Ticket Close-out

How to close out and finalize a ticket (Steps 7 and 8).

## Step 7: Close out the ticket honestly

After the owned implementation is fully verified:

1. Update the ticket's `Status` when the required verification surface has passed.
2. If reassessment, implementation, or broad verification exposed an adjacent but out-of-scope contradiction, create or update a follow-up ticket immediately (see mismatch-handling.md, Escalation decision tree).
3. If the owned invariant is proved and a broader rerun exposes a different unrelated blocker, close the current ticket honestly, record the broader blocker, and create the follow-up immediately.
4. Give each follow-up explicit `Deps` links to the implemented ticket and any still-pending sibling tickets or active specs.
5. Distinguish clearly between:
   - bugs fixed inside the current ticket
   - compromises accepted to finish the current ticket safely
   - remaining work that needs its own ticket
6. Do not silently broaden the current ticket during close-out. If the remaining work has its own architectural boundary, capture it as a follow-up.
7. Keep scenario prose aligned with updated assertions so the documented contract stays traceable.
8. When the implemented ticket intentionally changes a contract still described in an active spec, update that active spec text in the same pass unless a named follow-up ticket explicitly owns the spec drift.

### Planner and AI proof

- Prove behavior at the strongest available layer, not a weaker downstream proxy.
- When adding start-failure aftermath before action instantiation, check whether the surrounding path normally abandons empty transactions. Preserve that contract.
- When a ticket claims cross-layer valuation agreement, check whether the shared scorer computes marginal value over the actor's current accessible stock.
- When a ticket changes action availability, include at least one proof through real affordance enumeration, not just direct action construction.
- For exact-bound planner-root candidates, do not treat target binding as the whole contract when operator legality depends on intermediate goal state.
- When making a goal family live, verify its ranking entry in `compute_motive` returns a nonzero motive. A stub `=> 0` ranking silently prevents goal selection. When the new goal shares a signal or motive helper with existing goals, verify the shared helper's filtering criteria match the new goal's expected state.
- When the operator can be contention-managed (`Harvest`, `Craft`, `Loot`, `Heal`, or similar), verify direct affordance admission and queue-action expansion. Check the affordance-filter layer explicitly so a newly live direct operator path cannot bypass queue/grant contention.

### Staged scaffolding

When a ticket lands pure scaffolding ahead of downstream integration, wire immediate call sites or mark the temporary unused surface deliberately. Do not let staged work fail later CI clippy passes.

## Step 8: Close the loop on the ticket

If the user asked for full ticket completion, archive per [docs/archival-workflow.md](../../../../docs/archival-workflow.md):
- Mark completion status accurately
- Add an `Outcome` section (what changed, how verified)
- Note approved partial completion; create follow-up tickets when required

If the user asked only for implementation or analysis, do not archive. Default assumption: unless the user explicitly asks to archive, treat the task as implementation-only.

For implementation-only completion:
- Set `Status: COMPLETED` on the active ticket once the required verification surface has passed.
- Append factual close-out notes: `## Outcome`, `## Verification Result`, and any explicit deviations.
- If the active ticket is short-form or pre-template, add only the minimum missing sections: `## Assumption Reassessment`, `## Outcome`, optional `## Deviations`, and `## Verification Result`.

### Before finishing

- Re-check `What to Change`, `Files to Touch`, `Verification Layers`, and `Test Plan` against the actual landed diff. Remove reassessment-only fallout that did not become real edits.
- If reassessment or verification changed the semantic contract, also re-check `Problem`, `Architecture Check`, and `Acceptance Criteria` so the ticket's narrative matches the landed behavior.
- Re-check inline code snippets, example signatures, or API sketches against the final landed shape.
- Re-check `Status`, `## Outcome`, and verification/command notes -- they should reflect commands that actually passed, not the pre-reassessment plan.
- If formatting was required in a dirty worktree, check for formatter spillover and call it out explicitly.
- Report tracked-vs-untracked status for the active ticket and any follow-up tickets created during the session (see Section 1 for tracking awareness).
- After golden scenario metadata changes, refresh the generated golden inventory/docs (see verification.md, Golden test verification). Inspect the generated diff footprint and call out whether broader generated-file churn is expected inventory/index fallout or unexpected.

### Minimal active-ticket close-out shape

```markdown
## Outcome

Completed on YYYY-MM-DD.

- What changed
- Any bounded deviation from the original ticket wording

## Deviations

- Optional: semantic or scope correction accepted during reassessment/verification

## Verification Result

- Passed `<command 1>`
- Passed `<command 2>`
```
