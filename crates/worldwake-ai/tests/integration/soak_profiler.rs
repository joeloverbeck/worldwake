//! Soak performance profiler.
//!
//! Runs the T30 soak world for a configurable number of ticks, recording
//! per-tick wall-clock timing and per-agent belief store metrics.
//!
//! Output: a comprehensive report written to `reports/soak-performance-profile.md`.
//!
//! Run with: `cargo test -p worldwake-ai --features soak --test soak_profiler -- --nocapture`
#![cfg(feature = "soak")]

use crate::golden_harness::soak_world::build_t30_world;
use std::collections::BTreeMap;
use std::fmt::Write as FmtWrite;
use std::time::Instant;
use worldwake_core::{EntityId, EntityKind, Seed};

/// How many ticks to profile. 500 is enough to see growth trends without
/// taking too long (~1-5 min depending on severity of regression).
const PROFILE_TICKS: u64 = 500;

/// Sample detailed per-agent metrics every N ticks.
const DETAIL_SAMPLE_INTERVAL: u64 = 50;

struct TickTiming {
    tick: u64,
    wall_ms: f64,
    event_log_len: usize,
    total_known_entities: usize,
    total_entity_claims: usize,
    total_social_observations: usize,
    total_told_beliefs: usize,
    total_institutional_beliefs: usize,
}

struct AgentSnapshot {
    name: String,
    entity_id: EntityId,
    known_entities: usize,
    entity_claims: usize,
    total_claim_records: usize,
    social_observations: usize,
    told_beliefs: usize,
    heard_beliefs: usize,
    institutional_beliefs: usize,
    place: Option<EntityId>,
    is_dead: bool,
}

struct DetailedSample {
    tick: u64,
    agents: Vec<AgentSnapshot>,
}

#[test]
fn profile_t30_soak_performance() {
    let seed = {
        let mut bytes = [0u8; 32];
        bytes[0] = 0;
        bytes[31] = 0;
        Seed(bytes)
    };

    let (mut h, all_agents, _ruling_faction, _bandit_faction, _office) = build_t30_world(seed);

    let mut tick_timings: Vec<TickTiming> = Vec::with_capacity(PROFILE_TICKS as usize);
    let mut detailed_samples: Vec<DetailedSample> = Vec::new();

    // Collect agent names once
    let agent_names: BTreeMap<EntityId, String> = all_agents
        .iter()
        .map(|&agent| {
            let name = h
                .world
                .get_component_name(agent)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| format!("{:?}", agent));
            (agent, name)
        })
        .collect();

    // Collect place names once
    let place_names: BTreeMap<EntityId, String> = h
        .world
        .topology()
        .place_ids()
        .filter_map(|id| {
            h.world
                .topology()
                .place(id)
                .map(|place| (id, place.name.clone()))
        })
        .collect();

    let total_start = Instant::now();

    for tick_idx in 0..PROFILE_TICKS {
        let tick_start = Instant::now();
        h.step_once();
        let wall_ms = tick_start.elapsed().as_secs_f64() * 1000.0;

        // Aggregate belief store metrics
        let mut total_known = 0usize;
        let mut total_claims = 0usize;
        let mut total_social = 0usize;
        let mut total_told = 0usize;
        let mut total_institutional = 0usize;

        for &agent in &all_agents {
            if let Some(store) = h.world.get_component_agent_belief_store(agent) {
                total_known += store.known_entities.len();
                total_claims += store.entity_claims.values().map(|v| v.len()).sum::<usize>();
                total_social += store.social_observations.len();
                total_told += store.told_beliefs.len();
                total_institutional += store
                    .institutional_beliefs
                    .values()
                    .map(|v| v.len())
                    .sum::<usize>();
            }
        }

        tick_timings.push(TickTiming {
            tick: tick_idx,
            wall_ms,
            event_log_len: h.event_log.len(),
            total_known_entities: total_known,
            total_entity_claims: total_claims,
            total_social_observations: total_social,
            total_told_beliefs: total_told,
            total_institutional_beliefs: total_institutional,
        });

        // Detailed per-agent snapshot at intervals
        if tick_idx % DETAIL_SAMPLE_INTERVAL == 0 || tick_idx == PROFILE_TICKS - 1 {
            let mut agents_snapshot = Vec::new();
            for &agent in &all_agents {
                let store = h.world.get_component_agent_belief_store(agent);
                let is_dead = h.world.get_component_dead_at(agent).is_some();
                let place = h.world.effective_place(agent);
                let name = agent_names.get(&agent).cloned().unwrap_or_default();

                let (known, claims, total_claim_recs, social, told, heard, institutional) =
                    if let Some(s) = store {
                        (
                            s.known_entities.len(),
                            s.entity_claims.len(),
                            s.entity_claims.values().map(|v| v.len()).sum::<usize>(),
                            s.social_observations.len(),
                            s.told_beliefs.len(),
                            s.heard_beliefs.len(),
                            s.institutional_beliefs
                                .values()
                                .map(|v| v.len())
                                .sum::<usize>(),
                        )
                    } else {
                        (0, 0, 0, 0, 0, 0, 0)
                    };

                agents_snapshot.push(AgentSnapshot {
                    name,
                    entity_id: agent,
                    known_entities: known,
                    entity_claims: claims,
                    total_claim_records: total_claim_recs,
                    social_observations: social,
                    told_beliefs: told,
                    heard_beliefs: heard,
                    institutional_beliefs: institutional,
                    place,
                    is_dead,
                });
            }
            detailed_samples.push(DetailedSample {
                tick: tick_idx,
                agents: agents_snapshot,
            });
        }
    }

    let total_elapsed = total_start.elapsed();

    // === Build report ===
    let mut report = String::new();

    writeln!(report, "# Soak Performance Profile Report").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "**Date**: {}", "2026-04-14").unwrap();
    writeln!(report, "**Branch**: implemented-spec-S102").unwrap();
    writeln!(report, "**Seed**: 0").unwrap();
    writeln!(report, "**Ticks profiled**: {}", PROFILE_TICKS).unwrap();
    writeln!(
        report,
        "**Total wall time**: {:.2}s",
        total_elapsed.as_secs_f64()
    )
    .unwrap();
    writeln!(report, "**Agents**: {}", all_agents.len()).unwrap();
    writeln!(
        report,
        "**Places**: {}",
        h.world.topology().place_ids().count()
    )
    .unwrap();
    writeln!(report).unwrap();

    // Summary statistics
    let timings_ms: Vec<f64> = tick_timings.iter().map(|t| t.wall_ms).collect();
    let avg_ms = timings_ms.iter().sum::<f64>() / timings_ms.len() as f64;
    let max_ms = timings_ms.iter().cloned().fold(0.0f64, f64::max);
    let min_ms = timings_ms.iter().cloned().fold(f64::MAX, f64::min);
    let p50 = percentile(&timings_ms, 50);
    let p90 = percentile(&timings_ms, 90);
    let p99 = percentile(&timings_ms, 99);

    writeln!(report, "## Tick Timing Summary").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Metric | Value |").unwrap();
    writeln!(report, "|--------|-------|").unwrap();
    writeln!(report, "| Avg ms/tick | {avg_ms:.2} |").unwrap();
    writeln!(report, "| Min ms/tick | {min_ms:.2} |").unwrap();
    writeln!(report, "| Max ms/tick | {max_ms:.2} |").unwrap();
    writeln!(report, "| P50 ms/tick | {p50:.2} |").unwrap();
    writeln!(report, "| P90 ms/tick | {p90:.2} |").unwrap();
    writeln!(report, "| P99 ms/tick | {p99:.2} |").unwrap();
    writeln!(report).unwrap();

    // Check for growth: compare first 50 ticks avg vs last 50 ticks avg
    let early_avg = tick_timings[..50].iter().map(|t| t.wall_ms).sum::<f64>() / 50.0;
    let late_avg = tick_timings[(PROFILE_TICKS as usize - 50)..]
        .iter()
        .map(|t| t.wall_ms)
        .sum::<f64>()
        / 50.0;
    let growth_ratio = late_avg / early_avg;

    writeln!(report, "### Growth Analysis").unwrap();
    writeln!(report).unwrap();
    writeln!(report, "| Period | Avg ms/tick |").unwrap();
    writeln!(report, "|--------|-------------|").unwrap();
    writeln!(report, "| First 50 ticks | {early_avg:.2} |").unwrap();
    writeln!(report, "| Last 50 ticks | {late_avg:.2} |").unwrap();
    writeln!(report, "| Growth ratio | {growth_ratio:.2}x |").unwrap();
    writeln!(report).unwrap();
    if growth_ratio > 2.0 {
        writeln!(
            report,
            "**WARNING**: Tick cost grew {growth_ratio:.1}x over {PROFILE_TICKS} ticks. \
             This suggests O(n) or worse scaling with simulation age."
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    // Per-tick timing table (sampled every 10 ticks)
    writeln!(report, "## Per-Tick Timing (sampled every 10 ticks)").unwrap();
    writeln!(report).unwrap();
    writeln!(
        report,
        "| Tick | ms | EventLog | KnownEntities(all) | Claims(all) | Social(all) | Told(all) | Institutional(all) |"
    )
    .unwrap();
    writeln!(
        report,
        "|------|-----|----------|---------------------|-------------|-------------|-----------|---------------------|"
    )
    .unwrap();
    for t in tick_timings
        .iter()
        .filter(|t| t.tick % 10 == 0 || t.tick == PROFILE_TICKS - 1)
    {
        writeln!(
            report,
            "| {} | {:.1} | {} | {} | {} | {} | {} | {} |",
            t.tick,
            t.wall_ms,
            t.event_log_len,
            t.total_known_entities,
            t.total_entity_claims,
            t.total_social_observations,
            t.total_told_beliefs,
            t.total_institutional_beliefs,
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    // Detailed per-agent snapshots
    writeln!(report, "## Detailed Per-Agent Snapshots").unwrap();
    writeln!(report).unwrap();

    for sample in &detailed_samples {
        writeln!(report, "### Tick {}", sample.tick).unwrap();
        writeln!(report).unwrap();
        writeln!(
            report,
            "| Agent | Known | ClaimEntities | ClaimRecords | Social | Told | Heard | Institutional | Place | Dead |"
        )
        .unwrap();
        writeln!(
            report,
            "|-------|-------|---------------|--------------|--------|------|-------|---------------|-------|------|"
        )
        .unwrap();
        for a in &sample.agents {
            let place_name = a
                .place
                .and_then(|p| place_names.get(&p))
                .map(|s: &String| s.as_str())
                .unwrap_or("???");
            writeln!(
                report,
                "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
                a.name,
                a.known_entities,
                a.entity_claims,
                a.total_claim_records,
                a.social_observations,
                a.told_beliefs,
                a.heard_beliefs,
                a.institutional_beliefs,
                place_name,
                if a.is_dead { "YES" } else { "" },
            )
            .unwrap();
        }
        writeln!(report).unwrap();
    }

    // Belief store growth by entity kind (at last sample)
    if let Some(last_sample) = detailed_samples.last() {
        writeln!(report, "## Belief Store Composition (Final Tick)").unwrap();
        writeln!(report).unwrap();

        for a in &last_sample.agents {
            if let Some(store) = h.world.get_component_agent_belief_store(a.entity_id) {
                let mut by_kind: BTreeMap<Option<EntityKind>, usize> = BTreeMap::new();
                for (_, state) in &store.known_entities {
                    *by_kind.entry(state.believed_kind).or_default() += 1;
                }
                writeln!(report, "### {}", a.name).unwrap();
                writeln!(report).unwrap();
                writeln!(report, "| EntityKind | Count |").unwrap();
                writeln!(report, "|------------|-------|").unwrap();
                for (kind, count) in &by_kind {
                    let kind_str = match kind {
                        Some(EntityKind::Agent) => "Agent",
                        Some(EntityKind::Place) => "Place",
                        Some(EntityKind::ItemLot) => "ItemLot",
                        Some(EntityKind::Faction) => "Faction",
                        Some(EntityKind::Office) => "Office",
                        None => "Unknown",
                        _ => "Other",
                    };
                    writeln!(report, "| {kind_str} | {count} |").unwrap();
                }
                writeln!(report).unwrap();
            }
        }
    }

    // Slowest ticks breakdown
    writeln!(report, "## Top 20 Slowest Ticks").unwrap();
    writeln!(report).unwrap();
    let mut sorted_by_time: Vec<&TickTiming> = tick_timings.iter().collect();
    sorted_by_time.sort_by(|a, b| b.wall_ms.partial_cmp(&a.wall_ms).unwrap());
    writeln!(report, "| Tick | ms | KnownEntities | Claims |").unwrap();
    writeln!(report, "|------|----|---------------|--------|").unwrap();
    for t in sorted_by_time.iter().take(20) {
        writeln!(
            report,
            "| {} | {:.1} | {} | {} |",
            t.tick, t.wall_ms, t.total_known_entities, t.total_entity_claims,
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    // Extrapolation to 10080 ticks
    writeln!(report, "## Extrapolation to Full Soak (10080 ticks)").unwrap();
    writeln!(report).unwrap();
    if growth_ratio > 1.1 {
        // Linear growth model: ms_per_tick = a + b*tick
        let n = tick_timings.len() as f64;
        let sum_x: f64 = tick_timings.iter().map(|t| t.tick as f64).sum();
        let sum_y: f64 = tick_timings.iter().map(|t| t.wall_ms).sum();
        let sum_xy: f64 = tick_timings.iter().map(|t| t.tick as f64 * t.wall_ms).sum();
        let sum_x2: f64 = tick_timings.iter().map(|t| (t.tick as f64).powi(2)).sum();
        let denom = n * sum_x2 - sum_x * sum_x;
        if denom.abs() > 1e-10 {
            let b = (n * sum_xy - sum_x * sum_y) / denom;
            let a = (sum_y - b * sum_x) / n;
            let est_10080_ms = a + b * 10080.0;
            let est_total_s: f64 =
                (0..10080).map(|t| (a + b * t as f64).max(0.0)).sum::<f64>() / 1000.0;

            writeln!(
                report,
                "Linear regression: ms/tick = {a:.2} + {b:.4} * tick"
            )
            .unwrap();
            writeln!(report, "Estimated ms/tick at tick 10080: {est_10080_ms:.1}").unwrap();
            writeln!(
                report,
                "Estimated total time for 10080 ticks: {est_total_s:.0}s ({:.1} min)",
                est_total_s / 60.0
            )
            .unwrap();
        }
    } else {
        let est_total_s = avg_ms * 10080.0 / 1000.0;
        writeln!(
            report,
            "Growth ratio is flat ({growth_ratio:.2}x). Estimated total: {est_total_s:.0}s ({:.1} min)",
            est_total_s / 60.0
        )
        .unwrap();
    }
    writeln!(report).unwrap();

    // Write report
    let report_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("reports")
        .join("soak-performance-profile.md");
    std::fs::write(&report_path, &report).expect("failed to write report");

    eprintln!("Report written to: {}", report_path.display());
    eprintln!("Total time: {:.2}s", total_elapsed.as_secs_f64());
    eprintln!("Avg tick: {avg_ms:.2}ms, Growth: {growth_ratio:.2}x");
}

fn percentile(data: &[f64], pct: usize) -> f64 {
    let mut sorted = data.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let idx = (pct * sorted.len() / 100).min(sorted.len() - 1);
    sorted[idx]
}
