# 2026-05-19 — `target/` Bloat Reduction (Hybrid: Defaults Now + Consolidation Spec)

## Brainstorm Context

**Original request** (2026-05-19): `cargo clean` had just removed 94,065 files
totalling 95.2 GiB from `target/`. The user reported being forced to
`cargo clean` after every spec implementation to avoid running out of disk
space on a WSL2 environment.

**No reference file.** Investigation was driven entirely from the codebase.

**Key interview insights**:

1. The user's request already included evidence and a hypothesis ("I have a
   hard time believing that we should truly store that much information"),
   pushing initial confidence to ~90% on the goal (structural fix preferred
   over workaround playbook).
2. Codebase investigation identified three drivers in descending order: 63
   integration-test binaries in `worldwake-ai/`, full debug info per binary,
   per-crate × per-profile incremental cache. The space-conscious wrapper at
   `scripts/verify-space-conscious.sh` already covered (2) and (3); promoting
   it to the default was the immediate win.
3. The user's follow-up — "blast radius" prompt — revealed the per-file
   `golden_*.rs` layout is consumed by `scripts/golden_inventory.py`,
   `scripts/test_golden_inventory.py`, `campaigns/golden-perf/harness.sh`, four
   `docs/generated/*` artifacts, two hand-authored docs, and five `.claude/skills/*`
   files. The "simple test refactor" framing was wrong — full consolidation
   touches ~80 files including two non-trivial Python tooling rewrites.
4. The user approved **Option 3 (hybrid)**: defaults land now for fast
   relief; the structural consolidation is captured as a forward-looking spec.

**Final confidence**: 95%. No remaining gaps.

## Deliverables

This plan documents two deliverables landing in this brainstorm session:

### Deliverable 1 — Immediate code changes (Option 1; pending Step 6 approval)

Five edits to land the space-conscious defaults globally. No tooling or doc
churn beyond the wrapper retirement.

| Edit | File | Change |
|---|---|---|
| 1.1 | `Cargo.toml` (workspace root) | Append `[profile.dev]` and `[profile.test]` with `debug = "line-tables-only"`. |
| 1.2 | `scripts/verify.sh` | Add `export CARGO_INCREMENTAL=0` after `set -euo pipefail`, with explanatory comment. |
| 1.3 | `scripts/verify-space-conscious.sh` | Delete. Subsumed by defaults. |
| 1.4 | `docs/cargo-artifact-hygiene.md` | Rewrite "Space-Conscious Verification" section: remove wrapper reference, note new defaults, add forward pointer to `specs/S154`. Keep "Check Disk Use" and "Cleanup Options" sections. |
| 1.5 | `CLAUDE.md` | Update "Pre-PR Verification" paragraph: remove `verify-space-conscious.sh` reference; note that `verify.sh` is space-conscious by default. |

**Expected impact**: `target/` after a clean `./scripts/verify.sh` should drop
from ~95 GiB to ~45–55 GiB. To be measured post-implementation and recorded
in this plan's Outcome section.

**Verification**:
1. After applying the edits, run `./scripts/verify.sh` once to a clean
   target.
2. Grep the repo for stale references to `verify-space-conscious.sh`; confirm
   only intentional historical references remain.
3. Record `du -sh target/` in this plan's Outcome section.

### Deliverable 2 — Spec for full test-binary consolidation (already written)

`specs/S154-test-binary-consolidation.md` captures the structural fix:
collapse the 63-binary fan-out in `crates/worldwake-ai/tests/` to two
entry-point binaries (`golden_ai.rs` + `integration_ai.rs`) with per-scenario
source files preserved under `tests/scenarios/` and `tests/integration/`
submodule directories.

The spec is decomposed into five tickets (T1–T5) covering tooling rewrite,
source moves, generated-doc regeneration, campaigns harness rework, and
hand-authored doc/skill sweep. Each ticket is independently landable and
verifiable.

**Implementation NOT performed in this session.** Spec is queued for later
decomposition via `/spec-to-tickets`.

### Deliverable 3 — implementation-order update

Add `### Adjunct Wave: Test-Binary Consolidation` under the existing
`## Developer Tooling` section of the then-active implementation-order artifact
(now archived at `archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md`),
citing this brainstorm and the 95 GiB pain point.

## Implementation Sequence

1. Write `specs/S154-test-binary-consolidation.md` ✅ (done in this session).
2. Update the then-active implementation-order artifact ✅ (done in this
   session; now archived at
   `archive/specs/IMPLEMENTATION-ORDER-final-2026-05-21.md`).
3. Apply Deliverable 1 edits (after Step 6 menu approval).
4. Run `./scripts/verify.sh`; record disk measurement in §Outcome.
5. Commit (when user requests).

## Out of Scope

- `worldwake-systems/tests/` and `worldwake-cli/tests/` consolidation — small
  fan-out; deferred (noted in S154 §Non-Goals).
- `worldwake-visualizer` workspace surgery — distinct concern, separate spec
  when prioritized.
- `cargo-sweep` integration — documented as user-installable tooling, not
  enforced.
- Test-binary consolidation itself — captured in S154; no implementation in
  this session.

## FOUNDATIONS Alignment

- **FND-29 (Debuggability is a product feature)**: defaults preserve
  backtraces and assertion line numbers (`line-tables-only` is sufficient for
  `RUST_BACKTRACE=1` and panic locations). Full DWARF (local variables in a
  debugger) is the trade-off; users needing it can locally override via
  environment variables for the duration of a debugger session.
- **All 28 simulation principles**: N/A — no simulation code is touched in
  Deliverable 1; S154 is also explicitly N/A (tooling-boundary).

## Outcome

To be populated after Deliverable 1 lands and `./scripts/verify.sh` is run.

- `du -sh target/` before defaults: 95.2 GiB (measured at cleanup).
- `du -sh target/` after defaults: TBD.
- `target/debug/deps/` binary count before: ~70. After: still ~70 (binary
  count unchanged by defaults; only sizes shrink).
- Workflow tax: zero — `verify.sh` interface unchanged from a user's
  perspective.
