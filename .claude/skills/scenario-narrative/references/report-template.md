# Report Template

This reference defines the exact structure of the timestamped narrative report. The skill composes the report by walking these sections in order. Sections A, B, C, and E are always present; Section D and the Run Notes appendix are conditional.

## File Header

The report begins with a YAML-free plain-text header, then a short framing block:

```markdown
# Scenario Narrative Report — <scenario stem>

**Scenario**: `<scenario path>`
**Run timestamp**: <YYYY-MM-DD HH:MM:SS>
**Ticks simulated**: <N>
**Seed**: <seed value>
**Skill version**: scenario-narrative

---
```

After the header, no further metadata block. Sections begin immediately.

## Section A — Run Identification & Authored Intent

Always present. Roughly 200–400 words.

Subsections:

1. **Authored intent** — the leading `//` comment block from the `.ron`, quoted verbatim under a level-3 heading. If absent, write a one-line note ("the scenario file does not carry an authored intent comment") and synthesize a brief framing from the survival-health contract and feature roster.
2. **Topology** — places, edges with travel times, any concealment or contention policies that matter. Plain prose, not a table.
3. **Cast** — agents, control sources, roles (principal vs. supporting). One paragraph.
4. **Survival-health contract** — `max_authored_critical_run_ticks`, idle-window bound, required self-care families, per-need critical-run overrides. State in plain English ("Guard Mira may not run any need above its critical threshold for more than 220 consecutive ticks, must keep idle windows under 28 ticks while a need is elevated, and must exercise eat, drink, sleep, relieve, and wash"). If no contract is authored, say so.
5. **What the run is meant to demonstrate** — one paragraph framing, drawn from the authored intent and the agent/world setup. No interpretation of outcomes yet.

## Section B — Gameplay Mechanics Exercised

Always present. The dominant section by length. Roughly one paragraph per exercised feature row.

Open with a one-paragraph orientation: how many feature rows fired in this run, which were unique to this scenario versus inherited from earlier survival rows, and which authored-but-inactive feature rows are listed at the bottom.

Then, one paragraph per **exercised** feature row, in roughly the order the catalog table presents them in `gameplay-feature-mapping.md` — basic needs first, social/epistemic next, economic next, institutional next, conflict last. Each paragraph follows the structure given in the mapping reference: what the mechanic *is*, what authored substrate enabled it (with concrete numeric values), what occurred during this run (counts and tick-anchored landmark events).

Close with an **Authored but inactive** subsection listing every feature row whose substrate is present but whose dump anchor never fired. State the cause when legible from the dump; otherwise list without speculation.

The Section B floor is roughly one paragraph per feature row. There is no ceiling — features that are central to this scenario's story warrant more space, and the per-paragraph length should reflect that.

## Section C — Per-Agent Narrative

Always present. Length scales with cast size. For each agent:

1. Level-3 heading with the agent's name.
2. The per-agent template from `agent-narrative-structure.md`, scaled by principal vs. supporting status.

Tracked principals first, supporting actors second. Within each tier, alphabetical by name.

Cross-reference: every committed-action mention in this section should map cleanly to a feature row in Section B without restating the mechanic. Section B is the *systems* layer; Section C is the *evidence* layer.

## Section D — Cross-Agent and Emergent Phenomena

Conditional. Include only when the run produced multi-agent interactions worth narrating. Triggers:

- Witness chains (one agent's perception or testimony altering another agent's plan).
- Contention episodes (queue-and-grant cycles at a facility, depleted resource sources forcing replanning).
- Trade exchanges (any committed `trade` action).
- Social transfers (any accepted `tell`, `ask_about_person`, `consult_record` with cross-agent belief change).
- Hostile encounters (any committed `attack`, `pursuit_*`, force-claim).
- Coordinated travel (escort handoffs, posse formation).

Open with a one-paragraph framing of which trigger categories apply this run, then one paragraph per concrete episode. Each episode should anchor on specific ticks and name the agents, the locations, and the systems involved. Tie each episode to the feature row(s) in Section B it provides evidence for.

If none of the triggers apply, omit Section D entirely.

## Section E — Realism, Resourcefulness, Resilience

Always present. Approximately 250–400 words. The user's stated framing for the external research audience.

Three short subsections:

1. **Realism** — moments where the run produced behavior that reflects genuine constraint propagation (cost of travel translating to a real motive shift; belief decay producing a real planning gap; an institution's authority producing a real branch-gating decision). Two to four specific moments, each anchored on a tick and an agent.
2. **Resourcefulness** — moments where an agent reached a goal through an unexpected substitute, a hearsay-bridged plan, a queue-and-grant detour, an exploration-discovered source, or a multi-step social pipeline. Two to four specific moments.
3. **Resilience** — moments where an agent absorbed a setback without collapsing — replan after a contradicted belief, recovery from a budget exhaustion, escalation override that broke a stuck loop, post-handoff reorientation. Two to four specific moments.

Every moment named here must already be substantiated earlier in the report. Section E synthesizes; it does not introduce new claims.

## Run Notes Appendix

Conditional. Include only when at least one cheap observer fix was applied or at least one traceability ticket was created during the run. Format defined in `traceability-fix-protocol.md`.

## Tone, Length, and Hygiene

- The full report typically runs 3,000–9,000 words, scaled to scenario complexity. Survival-baseline-class scenarios sit near the floor; final-integration-class scenarios sit near the ceiling.
- No code blocks except quoted commands or paths. No Rust types. No inline `snake_case` identifiers except action family names (e.g., `relieve_wilderness`, `queue_for_facility_use`) and recipe names that have no readable equivalent.
- All numeric values are unit-qualified ("permille", "ticks", "places", "lots", "items", "agents") on first use within a section.
- The first paragraph of every section orients the reader on what the section answers. Do not bury the lead.
- No bullet-list dumps where prose would serve. Bullets are reserved for Section A's topology cast list and the gameplay-feature catalog enumeration in Section B's "Authored but inactive" trailer if used.
- Every section's tick references must be reachable through the dump; the report does not invent ticks or quantities.

## Final-Write Hygiene

Before writing the file:

- Confirm the timestamped path is unique (no prior run at the exact same second; if collision somehow occurs, append `-2`, `-3` until unique).
- Confirm the report does not include any decision speculation that was not anchored in the dump.
- Confirm Section E's moments all appear earlier.
- Confirm the Run Notes appendix is present iff at least one fix or ticket was produced.

After writing the file, delete `reports/scenario-narrative-dump.md` and report the final path to the user along with any tickets created and any inline observer fix applied.
