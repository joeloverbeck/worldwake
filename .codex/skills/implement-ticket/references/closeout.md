# Ticket Close-out

How to close out and finalize a ticket (Steps 7 and 8).

## Step 7: Close out the ticket honestly

After the owned implementation is fully verified:

1. Update the ticket's `Status` when the required verification surface has passed.
2. If reassessment, implementation, or broad verification exposed an adjacent but out-of-scope contradiction, create or update a follow-up ticket immediately (see mismatch-handling.md, Escalation decision tree).
3. If the owned invariant is proved and a broader rerun exposes a different unrelated blocker, close the current ticket honestly, record the broader blocker, and either link the existing owning ticket or create the follow-up immediately when no owner exists yet.
4. Give each follow-up explicit `Deps` links to the implemented ticket and any still-pending sibling tickets or active specs.
5. Distinguish clearly between:
   - bugs fixed inside the current ticket
   - compromises accepted to finish the current ticket safely
   - remaining work that needs its own ticket
6. Do not silently broaden the current ticket during close-out. If the remaining work has its own architectural boundary, capture it as a follow-up.
7. Keep scenario prose aligned with updated assertions so the documented contract stays traceable.
8. When the implemented ticket intentionally changes a contract still described in an active spec, update that active spec text in the same pass unless a named follow-up ticket explicitly owns the spec drift.
9. If reassessment proves the active ticket's core invariant false and no lawful implementation slice remains, close it as a rejection record instead of forcing `COMPLETED`: revert disproved code, set a factual terminal status such as `REJECTED`, record the reason in `Problem`/`Assumption Reassessment`/`Outcome`, create the successor ticket if work remains, and update the active spec or roadmap in the same pass.

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

When a staged ticket lands a new shared enum variant, dispatch key, or goal family ahead of later behavioral tickets, record in the ticket Outcome which branches are intentionally inert and which downstream ticket(s) are expected to make them live. This keeps the close-out honest when the type surface is complete but behavior is deliberately deferred.

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

For reassessment-only rejection:
- Set a factual non-completion status such as `REJECTED` when the live contract disproves the ticket's implementability claim.
- Keep the active ticket in place as the rejection record unless the user explicitly asks to archive it.
- Record the rejection cause, the reverted/discarded implementation attempt if one existed, and the successor ticket/spec link that now owns the remaining work.

### Before finishing

- Re-check `What to Change`, `Files to Touch`, `Verification Layers`, `Test Plan`, and `Out of Scope` against the actual landed diff. Remove reassessment-only fallout that did not become real edits. When drift changed the owned surface, update recorded scope, touched files/symbols, deviations, and stale exclusions so the ticket matches what really landed.
- If reassessment or verification changed the semantic contract, also re-check `Problem`, `Architecture Check`, and `Acceptance Criteria` so the ticket's narrative matches the landed behavior.
- Re-check inline code snippets, example signatures, or API sketches against the final landed shape.
- Re-check `Status`, `## Outcome`, and verification/command notes -- they should reflect commands that actually passed, not the pre-reassessment plan.
- When reassessment changed dependency status, fallback behavior, or whether a substrate is live vs. hypothetical, do one final ticket-wide search/read for the old dependency ID and old fallback wording before marking `COMPLETED`; remove stale references instead of leaving contradictory pre-reassessment prose behind.
- When broadened verification fails in a still-active downstream ticket or golden because the current ticket changed a live contract, update that active ticket's dependency and assumption text in the same pass instead of only recording the failure as generic fallout.
- When the ticket's motivating contradiction is fixed but the same broad verification command still fails for a newly distinguished root cause, create or update a dedicated follow-up ticket, narrow the current ticket to the proved invariant, and update dependent active tickets before marking the current ticket complete.
- If formatting was required in a dirty worktree, check for formatter spillover and call it out explicitly.
- Report tracked-vs-untracked status for the active ticket, any follow-up tickets created during the session, and any linked spec or planning drafts modified during reassessment/implementation (see SKILL.md Step 1 for tracking awareness).
- After golden scenario metadata changes, refresh the generated golden inventory/docs (see verification.md, Golden test verification). Inspect the generated diff footprint and call out whether broader generated-file churn is expected inventory/index fallout or unexpected. When a new `golden_*.rs` file introduces new scenario blocks, check whether the generated fallout includes a newly created `docs/generated/golden-scenario-details/<topic>.md` page alongside inventory/index/matrix files, and reflect that new file in closeout scope and outcome.
- If several cited golden scenarios were lawful but only a subset truthfully exposed the new trace contract, narrow the ticket to the surviving proof surface(s) and record the rejected candidates as reassessment deviations instead of forcing parallel assertions onto non-emitting scenarios.
- If the verification command that motivated the ticket remained the same but the final blocking culprit shifted during implementation, make sure the ticket's recorded reassessment/outcome explains that progression rather than implying the original blocker remained the only live cause throughout.

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
