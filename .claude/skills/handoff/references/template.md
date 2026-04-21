# Handoff Packet Template

The packet MUST use these sections in this exact order and with these exact headings. Sections with nothing to report say `None` or `Unknown` — do not omit them.

---

# RESUME PACKET — <short task label> — <YYYY-MM-DD>

## 1. Objective
<!-- One sentence. What the session is trying to accomplish. -->

## 2. Latest explicit user request
<!-- Verbatim quote of the most recent explicit user ask. Do not paraphrase. -->

## 3. Hard constraints
<!-- Bulleted. Session-level rules the new session must obey. Project-wide rules from root CLAUDE.md belong in auto-memory, not here — list only overrides or session-specific narrowings. -->

## 4. Current status
<!-- One short paragraph. Where the work stands right now. -->

## 5. Relevant files and symbols
<!-- Bulleted. `path/to/file.rs:42` or `module::Symbol`. No code excerpts. If no code in scope, write `None`. -->

## 6. Workspace state
<!-- Paste workspace_snapshot.sh output verbatim inside a fenced block. If the snapshot was unavailable, state so explicitly (e.g., `(snapshot unavailable — shell disabled)`). -->

## 7. Decisions already made
<!-- Bulleted. Choices the user or session has locked in. Include the reason when non-obvious. -->

## 8. Dead ends / do not retry
<!-- Bulleted. Approaches already tried and rejected, and why. Prevents rediscovery in the new session. -->

## 9. Evidence-backed facts
<!-- Bulleted. Things verified by running code, reading files, or explicit user confirmation. -->

## 10. Hypotheses / things to verify
<!-- Bulleted. Guesses, hunches, unconfirmed claims. If uncertain whether something is fact or guess, put it here. -->

## 11. Open blockers
<!-- Bulleted. Things stopping progress. `None` if none. -->

## 12. Tests / commands already run
<!-- Bulleted, one per line: `command → pass/fail/exit-N` with a terse note. `No tests run.` if none. -->

## 13. Things that will NOT survive a fresh session
<!-- Bulleted. In-memory findings, open tool results, unsaved plan state, transient context. -->

## 14. Reinvoke / reread on next session
<!-- Bulleted. Nested CLAUDE.md paths, path-scoped rules, skills to reinvoke (including ones invoked earlier this session), docs to re-read. -->

## 15. Ordered next steps
<!-- Numbered. Concrete actions the new session should take, in order. Each step should be directly executable (a command, a file edit target, a specific question to ask). -->

## 16. Paste into new session
<!-- The exact prompt the user should paste to continue. Usually one or two sentences plus a pointer to `.claude/handoffs/latest.md` or the full packet. -->
