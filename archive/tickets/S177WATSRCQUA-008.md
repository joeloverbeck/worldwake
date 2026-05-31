# S177WATSRCQUA-008: CLI player-POV gating for source quality

**Status**: COMPLETED
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — observer/CLI rendering only, no simulation state mutation
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-004.md`

## Problem

The spec's D8 deliverable surfaces water-source quality to the player via the CLI, with FND-19 agent-symmetry: a controlled agent sees only what they would lawfully perceive (co-located → direct read; remote → belief-mediated with freshness annotation). Before this ticket, the observer did not render water quality in its player-facing local survival summary, so the player had no inspectable surface for the quality axis while reviewing an agent's water-quality decisions.

## Assumption Reassessment (2026-05-31)

1. Initial reassessment targeted `crates/worldwake-cli/src/bin/observer.rs:2400-2404`:
   ```rust
   let water_source_present = world.query_resource_source().any(|(entity, source)| {
       world.effective_place(entity) == Some(place)
           && source.commodity == CommodityKind::Water
           && source.available_quantity > Quantity(0)
   });
   ```
   Implementation reassessment corrected this: that helper is an anomaly-support check, while the rendered player-facing seam is the critical-window local summary.
2. After ticket 001: `ResourceSource.quality: Option<WaterQuality>` is readable from the same `query_resource_source()` iterator. After ticket 004: `SourceReliability.sources[key].last_observed_quality + .last_observed_quality_tick` carry the agent's per-source belief.
3. FND-14A direct read: when the controlled agent is co-located with a source, the observer may read `ResourceSource.quality` directly (perception would deliver the same fact same-tick). For non-co-located sources, the observer reads only the agent's belief — `SourceReliability.sources[key].last_observed_quality` — annotating freshness as "(observed N ticks ago)".
4. FND-19 agent symmetry: the gating logic for human-controlled agents must be identical to the gating for AI-controlled agents.
5. Shared abstraction boundary: the observer's place-rendering surface. No new accessor was needed; the landed helper explicitly separates co-located direct reads from remote `SourceReliability` reads.
6. Existing tests: `crates/worldwake-cli/src/bin/observer.rs` already carries a `#[cfg(test)]` module, and this ticket extended it with focused helper tests for direct, remote-belief, omitted, and symmetric branches. The compile-and-render smoke coverage is provided by `cargo test -p worldwake-cli`.
7. Adjacent contradictions: the existing line 2400 check reads `world.query_resource_source()` authoritatively. If the observer always renders all places (including non-co-located ones), the current code surfaces `water_source_present` for places the controlled agent has never observed — an FND-14B violation. The fix is in scope here: route the player-POV branch through belief-view.
8. No SAVE_FORMAT_VERSION bump — no serialized state changes.

## Implementation Reassessment (2026-05-31)

1. The drafted `crates/worldwake-cli/src/bin/observer.rs:2400-2404` site is an anomaly-support helper, not the main rendered player-facing line. The rendered output seam for this ticket is the critical-window local summary table built by `format_critical_window_forensics`, which now calls `format_local_survival_state_summary_for_agent`.
2. The landed helper is `water_source_render_info_for_agent(world, agent, place, current_tick)`. It uses FND-14A direct authoritative reads only when `world.effective_place(agent) == Some(place)`, and it uses the agent's `SourceReliability.sources[SourceKey { entity, Water }]` record for non-co-located places.
3. The direct critical-window output currently renders co-located quality because the table's `LocalSurvivalStateSummary` is captured for the frame agent's local place. Remote belief rendering is proved at the helper seam and can be used by future observer place-list output without adding another information path.
4. No `GoalBeliefView` change landed. The existing observer binary already has direct access to `World`; this ticket keeps the legal source split explicit in the helper instead of introducing a new observer-only accessor.

## Architecture Check

1. Player-POV gating via belief-view (vs. authoritative read with role check) is the FND-19 + FND-14B-compliant choice — the same belief-view that drives AI candidate generation drives the player's information surface. No special-case "human can see more" code.
2. Freshness annotation (vs. binary stale/fresh) is the FND-15 / FND-22A inspectability choice — the player needs to know whether a stale "muddy" belief might be wrong now.
3. Co-located direct-read is lawful (FND-14A) and matches what the AI side does — the observer code reuses the same `effective_place` check.

## Verified Layers

1. Compile and render -> `cargo test -p worldwake-cli` passed with the observer rendering.
2. Co-located rendering -> `observer_renders_colocated_source_quality_directly` confirms the controlled agent's place shows the source's actual quality when co-located.
3. Remote rendering -> `observer_renders_remote_source_quality_from_belief_with_freshness` confirms belief quality for non-co-located sources, with "(observed N ticks ago)" annotation.
4. Unobserved-source rendering -> `observer_omits_unobserved_remote_source` confirms no remote source quality renders without a belief.
5. AI symmetry -> `observer_renders_identically_for_human_and_ai_controlled_agents` confirms identical helper output for human- and AI-controlled agents with the same belief.

## Landed Changes

### 1. Refactored observer source rendering

`crates/worldwake-cli/src/bin/observer.rs` now has `water_source_render_info_for_agent`, `WaterSourceRenderInfo`, and `WaterSourceFreshness`. The helper accepts the frame/controlled agent, place, and current tick:

```rust
fn water_source_render_info_for_agent(
    world: &worldwake_core::World,
    agent: EntityId,
    place: EntityId,
    current_tick: Tick,
) -> Option<WaterSourceRenderInfo>
```

When the agent is co-located with the place, the helper reads the live water source directly and returns `WaterSourceFreshness::Direct`. For remote places, it only returns information when the agent has a matching `SourceReliability` record; freshness is `current_tick - last_observed_quality_tick`.

### 2. Updated the rendered critical-window local summary

The critical-window table now calls `format_local_survival_state_summary_for_agent`, appending the water source quality and freshness when the helper can lawfully render it:

```text
Village Square: water=yes, wash=no, sleep=yes, food=no, water_source=yes, water_quality=Clean (observed now)
```

The legacy test-only `format_local_survival_state_summary` path remains for existing formatter tests and does not render agent-specific water quality.

### 3. Verified symmetry

The helper has a focused test proving identical output for `ControlSource::Human` and `ControlSource::Ai` agents with identical `SourceReliability` records.

## Landed Files

- `crates/worldwake-cli/src/bin/observer.rs` (modified — helper, critical-window rendering call, focused tests)
- `crates/worldwake-ai/src/survival_forensics.rs` (modified — verification-hygiene clippy allowance for already-landed same-family helper)
- `archive/tickets/S177WATSRCQUA-008.md` (modified — closeout truthing and archival)

## Out of Scope

- Rendering source quantity magnitude — the spec's D8 focuses on quality; quantity magnitude is a separate concern and currently only `available_quantity > 0` is surfaced. Adding magnitude is a YAGNI extension.
- Rendering forensic `SourceAcquisitionFailure` records — out of scope; that's a separate debug surface, not player-POV.
- New TUI/CLI commands for filtering by quality — out of scope; rendering changes only.
- Tests that render to specific golden output strings — out of scope; the rendering format is non-load-bearing and may need to evolve with the broader observer UX. Tests focus on data correctness, not string format.

## Acceptance Result

### Tests Passed

1. Passed `observer_renders_colocated_source_quality_directly` — controlled agent at the source's place sees actual `quality`.
2. Passed `observer_renders_remote_source_quality_from_belief_with_freshness` — controlled agent not co-located; belief age annotated.
3. Passed `observer_omits_unobserved_remote_source` — no belief, no rendering.
4. Passed `observer_renders_identically_for_human_and_ai_controlled_agents` — FND-19 agent symmetry.
5. Waived `cargo test --workspace` at per-ticket closeout because `cargo test -p worldwake-cli` covers this CLI-only diff and the `implement-spec-tickets` harness final branch phase owns full pre-push verification.

### Invariants

1. The observer never reveals a non-co-located source's quality to the controlled agent unless the agent's `SourceReliability` carries the belief.
2. Co-located rendering uses FND-14A direct authoritative read; remote rendering uses belief-view.
3. The same rendering function produces identical output for AI and human-controlled agents (FND-19).

## Test Plan Result

### Added/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (test module extension or new sibling test file) — 4 new focused tests on the helper fn.

### Commands Run

1. Passed `cargo test -p worldwake-cli --bin observer observer_renders` — targeted observer tests for co-located, remote-belief, and ControlSource-symmetric rendering.
2. Passed `cargo test -p worldwake-cli --bin observer observer_omits_unobserved_remote_source` — targeted omitted-remote-source proof.
3. Passed `cargo test -p worldwake-cli` — affected crate suite.
4. Waived `./scripts/verify.sh` at per-ticket closeout because the `implement-spec-tickets` harness final branch phase owns full pre-push verification.

## Outcome

Completed on 2026-05-31.

Outcome amended: 2026-05-31.

- Added player-POV water-source quality rendering for observer critical-window local summaries through an agent-aware helper.
- Preserved co-located direct reads under FND-14A and remote reads through `SourceReliability` only.
- Added four focused bin-local tests for direct quality, remote belief freshness, omission of unobserved remote sources, and ControlSource symmetry.
- During clippy verification, added a behavior-neutral `#[allow(clippy::too_many_arguments)]` to the already-landed S177 survival-forensics `build_frame` helper so the all-target warning gate remains green.

## Deviations

- The drafted line-number target was an anomaly-support helper rather than the rendered output seam. The landed rendered output seam is the critical-window local summary table, with the remote-belief branch proved at the reusable helper seam.
- No new `GoalBeliefView` observer accessor landed; the observer binary keeps the legal source split explicit inside its own helper.

## Verification Result

- Passed `cargo test -p worldwake-cli --bin observer observer_renders`
- Passed `cargo test -p worldwake-cli --bin observer observer_omits_unobserved_remote_source`
- Passed `cargo test -p worldwake-cli`
- Passed `cargo clippy -p worldwake-cli --all-targets -- -D warnings`
- Passed `cargo test -p worldwake-ai survival_forensics`
- Waived `./scripts/verify.sh` for this per-ticket closeout because the harness final branch phase owns the full pre-push gate.
