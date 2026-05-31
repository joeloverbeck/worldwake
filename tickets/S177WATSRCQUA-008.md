# S177WATSRCQUA-008: CLI player-POV gating for source quality

**Status**: PENDING
**Priority**: MEDIUM
**Effort**: Small
**Engine Changes**: No — observer/CLI rendering only, no simulation state mutation
**Deps**: `archive/tickets/S177WATSRCQUA-001.md`, `archive/tickets/S177WATSRCQUA-004.md`

## Problem

The spec's D8 deliverable surfaces water-source quality to the player via the CLI, with FND-19 agent-symmetry: a controlled agent sees only what they would lawfully perceive (co-located → direct read; remote → belief-mediated with freshness annotation). Today the observer at `crates/worldwake-cli/src/bin/observer.rs:2400-2404` surfaces only `available_quantity > 0` as a boolean presence check — no quality and no magnitude. Without this ticket, the player has no inspectable surface for the new quality axis and cannot verify their agent's water-quality decisions during play.

## Assumption Reassessment (2026-05-31)

1. Observer source-rendering site at `crates/worldwake-cli/src/bin/observer.rs:2400-2404`:
   ```rust
   let water_source_present = world.query_resource_source().any(|(entity, source)| {
       world.effective_place(entity) == Some(place)
           && source.commodity == CommodityKind::Water
           && source.available_quantity > Quantity(0)
   });
   ```
   This is the (likely) only player-facing water-source check. Verify via grep for additional sites during implementation.
2. After ticket 001: `ResourceSource.quality: Option<WaterQuality>` is readable from the same `query_resource_source()` iterator. After ticket 004: `SourceReliability.sources[key].last_observed_quality + .last_observed_quality_tick` carry the agent's per-source belief.
3. FND-14A direct read: when the controlled agent is co-located with a source, the observer may read `ResourceSource.quality` directly (perception would deliver the same fact same-tick). For non-co-located sources, the observer reads only the agent's belief — `SourceReliability.sources[key].last_observed_quality` — annotating freshness as "(observed N ticks ago)".
4. FND-19 agent symmetry: the gating logic for human-controlled agents must be identical to what an AI-controlled agent's belief-view exposes. The observer reuses the existing `resource_source(entity)` accessor flow (which after ticket 001 returns the quality field for co-located, and reads belief for remote per the existing `per_agent_belief_view.rs` impl).
5. Shared abstraction boundary: the observer's place-rendering surface. No new accessor needed — the observer consumes the existing `GoalBeliefView::resource_source` flow (or queries world authoritative state directly for co-located, depending on the observer's existing pattern). Verify the existing observer's read style at line 2400 — it currently reads `world.query_resource_source()` directly (authoritative iteration) which is OK because the observer is a debug/inspection tool, but for the player-POV-gated branch the observer must route through belief-view to honor FND-19.
6. Existing tests: the observer is the kind of code that's typically not extensively unit-tested; verify whether `crates/worldwake-cli/src/bin/observer.rs` carries `#[cfg(test)]` and what surfaces are tested. If none, the new gating logic ships with a small focused test covering the belief-view-vs-direct-read split. The compile-and-render smoke test is the primary verification surface.
7. Adjacent contradictions: the existing line 2400 check reads `world.query_resource_source()` authoritatively. If the observer always renders all places (including non-co-located ones), the current code surfaces `water_source_present` for places the controlled agent has never observed — an FND-14B violation. The fix is in scope here: route the player-POV branch through belief-view.
8. No SAVE_FORMAT_VERSION bump — no serialized state changes.

## Architecture Check

1. Player-POV gating via belief-view (vs. authoritative read with role check) is the FND-19 + FND-14B-compliant choice — the same belief-view that drives AI candidate generation drives the player's information surface. No special-case "human can see more" code.
2. Freshness annotation (vs. binary stale/fresh) is the FND-15 / FND-22A inspectability choice — the player needs to know whether a stale "muddy" belief might be wrong now.
3. Co-located direct-read is lawful (FND-14A) and matches what the AI side does — the observer code reuses the same `effective_place` check.

## Verification Layers

1. Compile and render — workspace builds with the new observer rendering.
2. Co-located rendering: focused test (or manual verification through observer execution against a known scenario) confirms the controlled agent's place shows the source's actual quality when co-located.
3. Remote rendering: focused test confirms the controlled agent's place-list shows belief quality for non-co-located sources, with "(observed N ticks ago)" annotation.
4. Unobserved-source rendering: focused test confirms the controlled agent's place-list does NOT show any source quality for sources the agent has never observed.
5. AI symmetry: the same rendering applied to a non-human-controlled agent produces identical output (modulo the agent identity) — FND-19 verification.

## What to Change

### 1. Refactor the observer's source-rendering branch

`crates/worldwake-cli/src/bin/observer.rs:2400-2404`: extract the source-rendering into a helper fn that accepts the controlled agent's ID and routes through belief-view:

```rust
fn render_water_source_for_controlled_agent(
    world: &World,
    controlled_agent: EntityId,
    place: EntityId,
) -> Option<WaterSourceRenderInfo> {
    let agent_place = world.effective_place(controlled_agent);
    let sources_at_place: Vec<_> = world
        .query_resource_source()
        .filter(|(source_id, state)| {
            world.effective_place(*source_id) == Some(place)
                && state.commodity == CommodityKind::Water
        })
        .collect();

    if agent_place == Some(place) {
        // FND-14A: co-located direct read of authoritative state.
        sources_at_place
            .iter()
            .find(|(_, state)| state.available_quantity > Quantity(0))
            .map(|(_, state)| WaterSourceRenderInfo {
                present: true,
                quality: state.quality,
                freshness: FreshnessTag::Direct,
            })
    } else {
        // FND-14B: remote sources via belief.
        let reliability = world.get_component_source_reliability(controlled_agent);
        sources_at_place
            .iter()
            .find_map(|(source_id, state)| {
                reliability
                    .and_then(|r| r.sources.get(&SourceKey {
                        entity: *source_id,
                        commodity: state.commodity,
                    }))
                    .map(|record| WaterSourceRenderInfo {
                        present: record.last_observed_capacity > 0,
                        quality: record.last_observed_quality,
                        freshness: FreshnessTag::Stale {
                            ticks_ago: current_tick.0
                                .saturating_sub(record.last_observed_quality_tick.0),
                        },
                    })
            })
    }
}
```

The exact function name, `WaterSourceRenderInfo` shape, and `FreshnessTag` enum should be pinned during implementation by reading the existing observer's data structures.

### 2. Update the call site to route through the new helper

Replace the direct `water_source_present` boolean check with the helper call. Update the rendering string to include quality + freshness:

```text
Riverside Camp: water source [Clean, observed now]
Backup Camp: water source [Muddy, observed 200 ticks ago]
Unknown Camp: (no water-source belief)
```

The exact rendering format should match the observer's existing place-rendering style.

### 3. Verify symmetry

Add a focused test (or extend an existing observer test if any) that runs the rendering for an AI-controlled agent vs. a human-controlled agent and confirms identical output (FND-19 agent symmetry).

## Files to Touch

- `crates/worldwake-cli/src/bin/observer.rs` (modify — extract source-rendering helper; route through belief-view; new rendering string)

## Out of Scope

- Rendering source quantity magnitude — the spec's D8 focuses on quality; quantity magnitude is a separate concern and currently only `available_quantity > 0` is surfaced. Adding magnitude is a YAGNI extension.
- Rendering forensic `SourceAcquisitionFailure` records — out of scope; that's a separate debug surface, not player-POV.
- New TUI/CLI commands for filtering by quality — out of scope; rendering changes only.
- Tests that render to specific golden output strings — out of scope; the rendering format is non-load-bearing and may need to evolve with the broader observer UX. Tests focus on data correctness, not string format.

## Acceptance Criteria

### Tests That Must Pass

1. New: `observer_renders_colocated_source_quality_directly` — controlled agent at the source's place sees actual `quality`.
2. New: `observer_renders_remote_source_quality_from_belief_with_freshness` — controlled agent not co-located; belief age annotated.
3. New: `observer_omits_unobserved_remote_source` — no belief, no rendering.
4. New: `observer_renders_identically_for_human_and_ai_controlled_agents` — FND-19 agent symmetry.
5. Existing: `cargo test --workspace` passes.

### Invariants

1. The observer never reveals a non-co-located source's quality to the controlled agent unless the agent's `SourceReliability` carries the belief.
2. Co-located rendering uses FND-14A direct authoritative read; remote rendering uses belief-view.
3. The same rendering function produces identical output for AI and human-controlled agents (FND-19).

## Test Plan

### New/Modified Tests

1. `crates/worldwake-cli/src/bin/observer.rs` (test module extension or new sibling test file) — 4 new focused tests on the helper fn.

### Commands

1. `cargo test -p worldwake-cli observer_renders` — targeted observer tests.
2. `./scripts/verify.sh` — full workspace.
