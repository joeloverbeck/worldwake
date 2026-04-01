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
2. **Extract all references** from the ticket: file paths, function names, type names, module references, class names
3. **Read the project's CLAUDE.md** if not already loaded, to understand project conventions

### Phase 2: Reassess Assumptions

4. **Grep/Glob for every referenced artifact** in the ticket:
   - File paths: do they exist? Are they at the stated location?
   - Functions/types/classes: do they exist? Are their signatures as described?
   - Module structures: does the code organization match what the ticket assumes?
   - Dependencies: are imported modules/packages available?
5. **Build a discrepancy list**: anything the ticket states that doesn't match reality

### Phase 3: Correct the Ticket (if needed)

6. If discrepancies were found:
   - **Present each discrepancy** to the user with what the ticket says vs. what the codebase actually has
   - **Propose corrections** to the ticket text
   - **Wait for user approval** before modifying the ticket file
   - **Edit the ticket file** with approved corrections
7. If no discrepancies: confirm the ticket is accurate and proceed

### Phase 4: Implement

8. **Invoke the `superpowers:executing-plans` skill** to implement the corrected ticket
   - The ticket serves as the implementation plan
   - Follow all project conventions (worktree discipline, immutability, TDD, etc.)
   - Run lint, typecheck, and tests before claiming completion (per Pre-Completion Verification rule)

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
