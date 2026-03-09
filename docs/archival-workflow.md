# Archival Workflow

Use this as the canonical, single-source archival policy for tickets, specs, brainstorming docs, and reports.

## Required Steps

1. Edit the document to mark final status at the top:
   - `**Status**: ✅ COMPLETED` or `**Status**: COMPLETED`
   - `**Status**: ❌ REJECTED` or `**Status**: REJECTED`
   - `**Status**: ⏸️ DEFERRED` or `**Status**: DEFERRED`
   - `**Status**: 🚫 NOT IMPLEMENTED` or `**Status**: NOT IMPLEMENTED`
2. For completed items, add an `Outcome` section at the bottom with:
   - completion date
   - what actually changed
   - deviations from original plan
   - verification results
3. If implementation is refined after archival and the archived `Outcome` becomes stale, amend the archived document before merge/finalization so ownership, behavior, and verification facts remain accurate.
   - Add `Outcome amended: YYYY-MM-DD` inside `## Outcome` for each post-completion refinement update.
   - Policy effective date: `2026-03-05` (forward-only enforcement; no mandatory historical backfill before this date).
4. Ensure destination archive directory exists:
   - `archive/tickets/`
   - `archive/specs/`
   - `archive/brainstorming/`
   - `archive/reports/`
5. Move the ticket.
6. If there is a filename collision, pass an explicit non-colliding destination filename.
7. Confirm the original path no longer exists in its source folder (`tickets/`, `specs/`, `brainstorming/`, or `reports/`).
