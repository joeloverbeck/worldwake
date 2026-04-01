---
name: implement-ticket
description: "Ticket reassessment and implementation. Use when asked to implement a ticket (e.g., /implement-ticket tickets/LEGACTTOO-009*). Reads the ticket, reassesses assumptions against the codebase, corrects the ticket first if needed, then implements."
user-invocable: true
arguments:
  - name: ticket_path
    description: "Glob path to the ticket file(s) (e.g., tickets/LEGACTTOO-009*)"
    required: true
---

# Implement Ticket

Structured workflow for ticket reassessment and implementation. This eliminates the manual preamble of reading tickets, reassessing assumptions, correcting discrepancies, and then implementing.

## Workflow

### Phase 1: Read and Understand

1. **Read the ticket file(s)** matching the provided glob path
2. **Read any additional references** provided in the arguments (spec files, design docs, etc.)
3. **Extract all references** from the ticket: file paths, function names, type names, module references, class names
4. **Read the project's CLAUDE.md** if not already loaded, to understand project conventions

### Phase 2: Reassess Assumptions

5. **Check dependency tickets**: If the ticket has a `Deps` field, verify each dependency's changes are present on the current branch (grep for key artifacts those tickets introduced). If dependencies are missing, stop and report.
6. **Grep/Glob for every referenced artifact** in the ticket:
   - File paths: do they exist? Are they at the stated location?
   - Functions/types/classes: do they exist? Are their signatures as described?
   - Module structures: does the code organization match what the ticket assumes?
   - Dependencies: are imported modules/packages available?
   - Golden test coverage and harness setup: do existing tests cover the areas being changed?
   - If `/reassess-spec` and `/spec-to-tickets` were run on the parent spec in the current session and all findings were resolved, the codebase validation may be abbreviated to a spot-check of key references rather than a full re-validation.
7. **Build a discrepancy list**: anything the ticket states that doesn't match reality

### Phase 3: Correct the Ticket (if needed)

8. If discrepancies were found:
   - **Present each discrepancy** to the user with what the ticket says vs. what the codebase actually has
   - **Propose corrections** to the ticket text
   - **Wait for user approval** before modifying the ticket file
   - **Edit the ticket file** with approved corrections
9. If no discrepancies: confirm the ticket is accurate and proceed

### Phase 4: Extract Deliverables

10. **Extract deliverables** from the ticket's "What to Change" and "Acceptance Criteria" sections into a numbered task list. Exclude deliverables already marked as done during the reassessment phase (Phase 3).
11. **Present the task list** to the user for confirmation before starting implementation

### Phase 5: Implement

12. **Create a feature branch**. If the project uses worktrees (check CLAUDE.md), set one up before implementation. Verify a clean test baseline before starting work. If tests fail, report and stop.
13. **Execute in batches** of ~3 tasks:
    - Mark each task as `in_progress` before starting, `completed` when done
    - After each batch, report what was implemented and verification output
    - Wait for feedback before continuing to the next batch
    - If implementation reveals a blocking infrastructure gap (test failures due to missing planner wiring, unimplemented transitions, etc.), stop the batch. Diagnose the root cause, assess against FOUNDATIONS.md, present the 1-3-1 analysis, and fix the gap before resuming. Do not work around infrastructure gaps with manual injection or test mocking.
14. **Run lint, typecheck, and tests** before claiming completion (per Pre-Completion Verification rule)

### Phase 6: Archive

15. **Archive the ticket** per `docs/archival-workflow.md`:
    - If the ticket file does not exist in the worktree (e.g., it was created on main after the branch point), copy it from main before archiving.
    - Mark status as `✅ COMPLETED`
    - Add an `## Outcome` section (completion date, what changed, deviations, verification results)
    - Move to `archive/tickets/`
    - If the ticket is only partially complete (user-approved scope reduction), mark status as `✅ COMPLETED (partial)` with a note about what was deferred. Create a follow-up ticket for the remaining work before archiving.
    - **Before returning to main from a worktree**, run `git status` on main to check for uncommitted changes. Skills like `/spec-to-tickets`, `/reassess-spec`, and `/skill-audit` may have left uncommitted work on main before the worktree was created. Use `git pull` to sync — never `git reset --hard` — or commit/stash local changes first.

## Rules

- **Never adapt tests to match bugs** — fix the code
- **Never silently skip deliverables** — if something seems wrong, present options (1-3-1 rule)
- **Worktree discipline**: if working in a worktree, ALL file operations use the worktree root path
- **Correct the ticket, not the code** when assumptions are wrong — the ticket is the source of truth for intent, the codebase is the source of truth for current state
- **Ticket fidelity**: every deliverable listed in the ticket must be addressed — either implemented, or flagged as blocked with the 1-3-1 rule

## Example Usage

```
/implement-ticket tickets/LEGACTTOO-009*
/implement-ticket tickets/FITLSEC7RULGAP-001*
/implement-ticket .claude/worktrees/my-feature/tickets/FOO-003*
```
