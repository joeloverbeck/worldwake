# Handoff Content Rules

Apply all of these when filling the template. If a rule and the user's ask conflict, flag the conflict in §10 and follow the user's ask — do not silently violate a rule.

## Fact vs. guess

- §9 (facts): only claims backed by session evidence — a file read, a command output, or an explicit user statement.
- §10 (hypotheses): anything inferred, plausible, or not yet checked.
- If in doubt, put it in §10. A fact with "probably" or "I think" in it is a hypothesis.
- Never fabricate file paths, symbol names, line numbers, or command outputs. If uncertain, write `Unknown`.

## Unknowns and empties

- If a section has no content, write `None` or `Unknown`. Do not omit the section.
- If `workspace_snapshot.sh` was unavailable (shell disabled, not in a repo, permission denied), §6 says so explicitly. Do not fabricate git state.
- If no tests or commands were run this session, §12 says `No tests run.` explicitly.
- If there are no code changes, §5 says `None` and §6 shows a clean working tree or `(not a git repo)`.

## Do NOT restate

- Root `CLAUDE.md` conventions the next session will reload automatically via auto-memory.
- Project-wide coding standards (FOUNDATIONS principles, crate layout, invariants) — reference by path, do not inline.
- Anything already in `docs/FOUNDATIONS.md`, `docs/debugging-traces.md`, or similar reference docs.

## DO restate

- User instructions given THIS session that override or narrow a default rule (§3).
- Nested `CLAUDE.md` paths, path-scoped rules, or skills invoked this session (§14).
- Decisions the user locked in (§7) and approaches the user rejected (§8).
- The exact next-step prompt for the new session (§16).

## Formatting

- No code excerpts beyond a single line for a tiny quote.
- No raw command logs. In §12, list cargo/behavioral test commands plus any explicit verification commands the user asked you to run — not routine Read/Grep tool calls or pre-apply anchor checks. Summarize each with `command → pass/fail/exit-N`.
- Prefer file paths, symbols, IDs, and commands over prose description.
- Keep each section under ~10 lines of content. If a section grows beyond that, the packet is becoming literary — tighten.
- Use fenced blocks only for §6 (workspace snapshot) and §16 (paste prompt).

## Anti-goals

- Do NOT turn the packet into a narrative recap of the session.
- Do NOT duplicate auto-memory content.
- Do NOT include secrets, tokens, or session-only credentials.
- Do NOT invent file paths, symbols, or command outputs.
- Do NOT add a "summary" or "completeness" section — the packet is operational, not a report.
- Do NOT rely on `/compact` or on the new session having access to this session's memory beyond what is in this packet.
