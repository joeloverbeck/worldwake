# Design Note: Emergent Situation Classes for a Non-Scripted Adventure Simulation

## Purpose

This note identifies the kinds of situations that make Bethesda-style adventure games and Dwarf Fortress / RimWorld compelling, then translates those situations into forms that are valid for this simulation.

The goal is **not** to recreate their scripted content patterns.

The goal is to recreate the **underlying causal shape** of the experiences they produce:

- traveling through a world that feels dangerous, legible, and alive,
- stumbling into opportunities and crises that were not authored for the player,
- seeing consequences accumulate in places, institutions, and people,
- and allowing memorable stories to arise from the interaction of persistent world state rather than from event scripts.

This note assumes a **medium-grain simulation** rather than Dwarf Fortress-style tile granularity. We do not need to simulate every pebble or every square meter. We do need to simulate the carriers of consequence with enough concreteness that events remain traceable, local, and inspectable.

---

## Core Thesis

The correct way to recreate the best parts of Bethesda, Dwarf Fortress, and RimWorld is **not**:

- encounter tables,
- raid directors,
- hidden event pacing,
- hand-placed perpetual bandit camps,
- dungeon reset logic,
- quest pipelines,
- or special-case "interesting content" spawners.

The correct way is to simulate a world containing:

- persistent places,
- moving agents,
- concrete inventories and containers,
- access rights and obligations,
- local perception,
- stale and conflicting beliefs,
- evidence and records,
- offices and institutions,
- transport and communication delay,
- competition over scarce opportunities,
- and aftermath-heavy actions.

If those primitives are correct, the memorable content should arise as a consequence.

---

## What We Are Actually Trying to Preserve

### From Bethesda

What is worth preserving from Morrowind / Oblivion / Skyrim is not their scripting. It is:

- the feeling that travel itself is an adventure,
- the existence of memorable sites with identity,
- the pleasure of hearing about something and deciding to go there,
- the feeling that trouble can interrupt ordinary plans,
- factional and institutional texture,
- and the sense that places hold traces of prior events.

### From Dwarf Fortress / RimWorld

What is worth preserving is not their event directors or storyteller randomness. It is:

- cascading consequence,
- local shortages and opportunism,
- attacks that matter because they damage real things,
- social and institutional failure under pressure,
- long-tail aftermath,
- unexpected interactions between independent systems,
- and the ability for a world to keep changing without direct player attention.

### What Must Be Rejected

The following are invalid for this project:

- "something interesting happens now" logic,
- any hidden drama dial,
- content authored as outcome rather than as world state,
- player-centric privilege where opportunities wait forever,
- offscreen freezing of meaningful simulation,
- omniscient institutions,
- and content that cannot answer "where did this come from?" and "who knows this and why?"

---

## Shared World Primitives Required

Most of the situation classes below depend on the same small set of world primitives. These should be treated as foundational infrastructure, not as one-off mechanics.

### 1. Place Graph

A medium-grain place model is sufficient, for example:

- region,
- route segment,
- settlement,
- site,
- sublocation,
- room,
- container.

The simulation does not need ultra-fine tile detail. It does need:

- adjacency,
- travel duration,
- line-of-approach,
- physical occupancy,
- visibility / audibility ranges where relevant,
- and concrete placement of actors, goods, traces, and records.

### 2. Persistent Sites

Sites must persist as actual places with identity. A ruin, mine, shrine, watchtower, inn, quarry, farm, bridge, warehouse, cave, or manor is not "content." It is a location with:

- access points,
- sublocations,
- containers,
- ownership or disputed ownership,
- occupancy,
- routes,
- and possible evidence.

### 3. Bodies, Inventories, Containers, and Transfer

Meaningful goods and tools must exist as concrete world state.

The simulation must support:

- items,
- stacks,
- containers,
- carried goods,
- stored goods,
- transfers,
- splitting and merging,
- destruction,
- spoilage if modeled,
- and derivation lineage where downstream systems care.

### 4. Ownership, Custody, Access, Obligation, and Jurisdiction

These must remain distinct.

A convincing world needs to represent cases like:

- the merchant owns the goods,
- the porter carries them,
- the caravan master can reassign them,
- the guard can inspect them,
- the warehouse can store them,
- the lender has a claim on them,
- and the town watch can adjudicate a dispute over them.

### 5. Belief, Memory, and Evidence

Ground truth is not enough. Agents need:

- beliefs,
- uncertainty,
- source attribution,
- freshness / age,
- confidence,
- contradiction,
- and memory decay or replacement.

Evidence must exist as world state:

- tracks,
- corpses,
- damage,
- blood,
- missing inventory,
- broken locks,
- witness testimony,
- notices,
- ledgers,
- accusations,
- letters,
- warrants,
- contracts.

### 6. Travel and Communication as World Processes

Movement and information transfer must take time and must happen through lawful channels.

There is no magical update layer.

If a settlement learns something, it learned it because:

- someone arrived,
- someone saw,
- someone told,
- something was posted,
- a record was inspected,
- or some other actual carrier reached the relevant place.

### 7. Institutions and Offices

Institutions must not exist as abstract manager code.

They must act through:

- office-holders,
- records,
- budgets,
- assets,
- jurisdiction,
- places of operation,
- duties,
- and succession rules.

### 8. Interruptible Action and Revisable Commitments

Plans must be stable enough to avoid thrashing, but never be rails.

Actions must have:

- preconditions,
- duration,
- cost,
- capacity occupancy,
- interruption rules,
- and lawful contention handling.

### 9. Contention Mechanics

Multiple actors must be able to chase the same opportunity.

The world needs explicit mechanisms for:

- races,
- queues,
- reservations,
- grants,
- locks,
- or contested access.

Planner intent alone must never secretly reserve future success.

### 10. Boundary Processes

Off-map influences must come through explicit channels with:

- source region,
- route or transmission medium,
- delay,
- capacity,
- observables,
- and failure modes.

---

## Situation Classes Worth Targeting

The following are the kinds of things that are meaningfully distinct, compelling, and strongly associated with Bethesda / Dwarf Fortress / RimWorld-style play. For each, the question is not "how do we script this?" but "what world representation makes this arise lawfully?"

---

## 1. Dangerous Road Travel

### Why it is compelling

Travel feels meaningful when it is not just a loading screen between authored content nodes.

The player remembers:
- taking the long road because rumors said the pass was unsafe,
- seeing a broken cart before seeing the attackers,
- arriving too late to help,
- or surviving because another group happened to be nearby.

### Required representation

To support this, the simulation needs:

- route segments with real traversal time,
- agents and caravans moving through them,
- patrol coverage,
- local hazards,
- visibility / warning signals where relevant,
- and overlapping occupancy in space and time.

A road is not dangerous because a `danger_score` is high.
It is dangerous because:
- predators use it,
- patrols no longer cover it,
- raiders observed traffic there,
- a bridge bottlenecks movement,
- or the route passes near hungry, territorial, or desperate actors.

### Canonical causal shape

1. Trade or travel creates predictable movement.
2. Threat actors identify or encounter that movement.
3. Attack, harassment, avoidance, escort, or flight occurs through local perception and movement.
4. Aftermath remains on the route.
5. Reports propagate unevenly.
6. Later travelers react based on what they know or fail to know.

### Design value

This is one of the most important situation classes because it turns movement itself into play.

---

## 2. Predator Displacement and Roaming Threats

### Why it is compelling

Players remember monsters when they feel like part of the world rather than content delivery devices.

A dangerous creature is interesting when it:
- leaves one area,
- appears somewhere it "shouldn't" be,
- pressures normal routines,
- and causes institutions and civilians to react imperfectly.

### Required representation

The simulation needs:

- territory or habitat preference,
- hunger / need pressure,
- prey availability,
- competition,
- injury,
- fear / aggression patterns,
- and movement rules.

A dragon-equivalent, giant predator, or supernatural threat should not appear because content pacing wants excitement. It should appear because local conditions made its movement or aggression lawful.

### Canonical causal shape

1. Local food or safety becomes insufficient.
2. The threat expands range, migrates, or turns to higher-risk prey.
3. It intersects normal travel or settlement activity.
4. Damage creates bodies, debris, rumor, fear, and institutional response.
5. New routes, hunts, warnings, or evacuations emerge.

### Design value

This preserves the emotional role of "monster content" without violating causal standards.

---

## 3. Emergent Outlawry and Hideouts

### Why it is compelling

Bethesda hand-places bandits. A stronger simulation lets outlawry emerge where predation is materially viable.

This is compelling because outlaw sites become:
- spatially legible,
- tied to trade and patrol patterns,
- and vulnerable to pressure, displacement, betrayal, and depletion.

### Required representation

The simulation needs:

- desperate or predatory actors,
- access to weapons and shelter,
- knowledge of traffic,
- ability to fence or consume stolen goods,
- patrol pressure,
- and hideout-worthy sites.

A hideout should form because:
- the site is remote enough,
- traffic passes nearby,
- institutional response is weak,
- and the location supports storage, rest, lookout, or retreat.

### Canonical causal shape

1. Vulnerable actors or factions adopt predation.
2. They select or occupy a viable site.
3. Raids or theft create inflow.
4. Stolen goods move through actual custody chains.
5. Rumors, tracks, absences, and retaliation expose the site over time.
6. The group relocates, fragments, is destroyed, negotiates, or evolves.

### Design value

This gives the world "bandit camps" without any bandit-camp system.

---

## 4. Persistent Ruins and Dungeons with Changing Occupancy

### Why it is compelling

One of Bethesda's strongest feelings is "there is a place out there worth entering."

The mistake is making that place a static authored content box.

The correct version is a site with:
- persistent identity,
- layered history,
- changing occupants,
- reusable material affordances,
- and discoverable traces.

### Required representation

A ruin, cave, tomb, watchtower, manor, crypt, or mine needs:

- sublocations,
- access constraints,
- containers,
- old records or remains,
- structural hazards if modeled,
- and occupancy rules.

Different actors can use the same site over time:
- squatters,
- smugglers,
- hunters,
- cultists,
- scavengers,
- fugitives,
- beasts,
- officials,
- refugees.

### Canonical causal shape

1. A site exists with resources, concealment, symbolism, or strategic value.
2. An actor or group occupies it.
3. Their use of it changes inventory, traces, defenses, and reputation.
4. Others hear of it, stumble on it, or investigate it.
5. Conflict, looting, abandonment, reuse, or contamination follows.
6. The site accumulates layered evidence of prior occupancy.

### Design value

This preserves "dungeon delving" while avoiding static scripted content.

---

## 5. Bounties, Notices, Contracts, and Warrants

### Why it is compelling

This is the lawful replacement for the quest system.

These objects are compelling because they:
- make social response visible,
- advertise opportunities,
- formalize institutional intent,
- and create competition among claimants.

### Required representation

A bounty, notice, contract, warrant, or accusation needs:

- issuer,
- authority basis,
- place of posting or delivery,
- conditions,
- proof requirements,
- reward source,
- expiry if any,
- and inspectable record state.

### Canonical causal shape

1. A report or need reaches an institution or sponsor.
2. The institution decides whether it has jurisdiction, resources, and motivation to act.
3. A social artifact is created.
4. Knowledge of that artifact spreads by seeing, reading, hearing, or being told.
5. Claimants decide whether to act.
6. Proof is delivered, contested, accepted, or rejected.
7. Reward or punishment follows from real assets and authority.

### Design value

This gives the world adventure hooks without ever creating a quest pipeline.

---

## 6. Theft, Missing Property, and Investigation

### Why it is compelling

This produces some of the strongest emergent stories because it links:
- ownership,
- mistaken belief,
- access,
- evidence,
- suspicion,
- and institutional response.

The emotional core is violated expectation.

### Required representation

The world needs:

- concrete stored goods,
- distinct ownership / custody / access,
- actual transfer or destruction paths,
- memory or belief about where the goods are,
- and inspectable traces of movement or tampering where applicable.

### Canonical causal shape

1. Goods are acquired and stored.
2. Another agent moves, steals, spends, destroys, or misrecords them.
3. The owner still believes them present.
4. Later inspection reveals mismatch.
5. Search, accusation, reporting, concealment, or resignation follows.
6. Evidence travels and may or may not convince others.

### Design value

This is a universal story generator and should be treated as core.

---

## 7. Rumor-Led Expeditions and Stale Information

### Why it is compelling

A huge part of adventure is acting on incomplete or wrong information.

The memorable version is not "there is content at the map marker."
It is:
- someone heard something,
- believed it enough to travel,
- arrived,
- and found reality had changed.

### Required representation

Beliefs about opportunities must carry:

- source,
- age,
- confidence,
- place,
- claimed event time,
- and possible distortion.

### Canonical causal shape

1. A claim enters circulation.
2. One or more actors plan around it.
3. The underlying state changes before they arrive.
4. Arrival produces mismatch.
5. Belief correction occurs locally.
6. Different actors update at different times.

### Design value

This preserves the pleasure of "I heard about a thing and went there" while staying faithful to partial observability.

---

## 8. Interrupted Errands and Plan Pivots

### Why it is compelling

Many memorable stories begin with an ordinary intention that gets derailed.

This is one of the cheapest and highest-value forms of emergent adventure:
- going to market,
- going home,
- heading to collect a debt,
- escorting goods,
- searching for medicine,
- and then something changes.

### Required representation

The world needs:

- duration-bearing travel and tasks,
- interruptible actions,
- revisable commitments,
- local perception of new threats or opportunities,
- and priorities that can change under pressure.

### Canonical causal shape

1. An actor adopts a plan based on current beliefs.
2. During execution, new evidence or danger appears.
3. Assumptions break.
4. The actor revises, delays, abandons, or replaces the plan.
5. The abandoned intention remains part of the causal story rather than vanishing.

### Design value

This makes the world feel alive even when no "major event" is happening.

---

## 9. Office Failure, Patrol Gaps, and Local Law Breakdown

### Why it is compelling

This is one of the richest sources of believable instability.

A settlement becomes dangerous not because the game toggled danger on, but because:
- the captain died,
- the steward is corrupt,
- the treasury is empty,
- succession is delayed,
- escorts are unpaid,
- records are lost,
- or jurisdiction is disputed.

### Required representation

Institutions need:

- office-holders,
- succession rules,
- duty lists,
- assets / budgets,
- delegated labor,
- and jurisdictional boundaries.

### Canonical causal shape

1. An office-holder is removed or impaired.
2. Duties are delayed or partially dropped.
3. Patrol or response capacity weakens.
4. Opportunists notice through evidence or routine change.
5. Predation increases.
6. Repair, succession, improvisation, or collapse follows.

### Design value

This is a superior replacement for abstract "crime increase" or "chaos event" systems.

---

## 10. Shortage, Failed Arrival, and Substitution

### Why it is compelling

Dwarf Fortress and RimWorld are at their best when pressure changes routine behavior across many actors.

Shortage is compelling because it reshapes:
- prices,
- theft,
- route choice,
- migration,
- rationing,
- labor allocation,
- social conflict,
- and institutional legitimacy.

### Required representation

The simulation needs:

- actual inventories,
- actual consumption,
- production or inflow channels,
- transport delay,
- failed deliveries,
- buyer and seller behavior,
- and substitute goods or fallback plans.

### Canonical causal shape

1. Expected inflow fails or declines.
2. Local stock depletes through ordinary use.
3. Different actors recognize the problem at different times.
4. Hoarding, substitution, queueing, theft, repricing, aid requests, or departure occurs.
5. Recovery requires real replenishment, not hidden normalization.

### Design value

This creates world-scale consequence without scripting a disaster event.

---

## 11. Competing Claimants and Lost Opportunities

### Why it is compelling

The world stops feeling fake the moment the player can lose an opportunity because someone else got there first.

This is essential.

### Required representation

The simulation needs explicit contention for:

- targets,
- goods,
- jobs,
- bounties,
- rescue attempts,
- access to a place,
- use of a facility,
- and social or institutional attention.

### Canonical causal shape

1. Multiple actors identify the same opportunity.
2. None secretly reserve it just by wanting it.
3. Access is decided by race, queue, negotiation, or explicit grant.
4. Losers react to the updated world state.
5. New frictions and second-order consequences emerge.

### Design value

This is one of the most powerful anti-script tools in the entire architecture.

---

## 12. Missing Persons, Search, and Rescue

### Why it is compelling

This combines emotion, uncertainty, travel, evidence, and timing.

It is compelling because:
- concern arises from violated expectation,
- search is inherently partial and local,
- and arriving late still produces meaningful aftermath.

### Required representation

The world needs:

- expected arrival or routine,
- recognition of absence,
- ability to form search intent,
- local evidence,
- survival / injury / captivity / movement states,
- and institutions or kin networks that may care.

### Canonical causal shape

1. Someone fails to appear.
2. Someone else notices because they expected them.
3. Search begins from partial information.
4. Clues and false leads compete.
5. Rescue, recovery, death discovery, ransom, or abandonment follows.

### Design value

This yields strong narrative without any bespoke story content.

---

## 13. False Rumor, Wrongful Accusation, and Imperfect Correction

### Why it is compelling

A believable world cannot route only true information.

This situation class creates:
- social damage,
- procedural mistakes,
- factional abuse,
- and delayed or incomplete correction.

### Required representation

The simulation needs:

- false testimony,
- rumor distortion,
- forged or mistaken records,
- confidence levels,
- institutional procedures,
- conflicting evidence,
- and actors with bias or incentives.

### Canonical causal shape

1. A false claim enters circulation.
2. Different actors receive and act on it.
3. Downstream consequences occur before truth is established.
4. New evidence appears.
5. Some actors update; others do not.
6. Exoneration, apology, retaliation, inertia, or miscarriage follows.

### Design value

This is crucial for a world that feels socially real rather than mechanically clean.

---

## 14. Boundary Shocks and Upstream Change

### Why it is compelling

A world feels larger than the simulated map when important changes can originate beyond the core region without cheating.

### Required representation

Boundary processes need:

- source regions,
- inflow types,
- delay,
- route,
- failure modes,
- and observables.

Examples:
- delayed convoy,
- refugee influx,
- border levy,
- upstream war,
- disease import,
- collapsed bridge on an external route,
- migrating herd,
- religious decree from outside authority.

### Canonical causal shape

1. Off-map conditions change.
2. Inflow or traffic is altered through lawful channels.
3. Local actors continue under stale expectations.
4. Evidence of disruption accumulates.
5. Local systems adapt through substitution, defense, migration, or collapse.

### Design value

This gives macro-scale change without violating locality.

---

## 15. Grudges, Loyalties, and Revenge Chains

### Why it is compelling

This is where the simulation begins to feel human rather than merely systemic.

Memorable stories often come from:
- insult,
- favoritism,
- betrayal,
- debt,
- loyalty,
- kinship,
- shame,
- and revenge.

### Required representation

The world needs:

- relationship state,
- memory of prior interactions,
- obligations,
- faction ties,
- reputational or testimonial consequences if modeled,
- and willingness to act outside formal systems.

### Canonical causal shape

1. A socially meaningful harm occurs.
2. It is remembered, interpreted, and retold differently by different actors.
3. Formal institutions may or may not address it.
4. Informal retaliation, cover-up, alliance, or protection emerges.
5. Later decisions are altered by that social history.

### Design value

This prevents agents from feeling like generic utility calculators.

---

## 16. Settlement Rise, Decline, and Abandonment

### Why it is compelling

A site becoming prosperous, hollow, dangerous, or abandoned is one of the strongest long-horizon signals that the world is truly running.

### Required representation

Settlements need:
- population,
- role distribution,
- institutions,
- inflow and outflow,
- local production,
- security conditions,
- and dependencies.

### Canonical causal shape

1. Some pressure or opportunity changes viability.
2. People adapt unevenly.
3. Functions degrade or strengthen.
4. Property changes hands or is left behind.
5. New ruins, squats, faction enclaves, or ghost infrastructure emerge.

### Design value

This lets the world generate its own future adventure sites.

---

## Synthesis: What These Situation Classes Have in Common

All of the situation classes above share the same structural properties:

1. **They begin from ordinary world state**, not authored spectacle.
2. **They depend on travel, delay, and locality.**
3. **They create aftermath**, not isolated event resolution.
4. **They produce evidence and rumor**, not just outcome flags.
5. **They involve competition**, contradiction, or imperfect institutional response.
6. **They stay legible after the fact** because the chain can be traced.
7. **They do not need a player to exist**, only a player to encounter them.

That is the real target.

---

## Practical Design Rule

When evaluating a proposed feature, ask:

> Does this add a new carrier of consequence, or does it merely add a new way for the game to declare content?

Good features add:
- a new record type,
- a new evidence type,
- a new social artifact,
- a new form of transfer,
- a new institution duty,
- a new kind of contention,
- a new form of local perception,
- or a new kind of aftermath.

Bad features add:
- a story trigger,
- an encounter chance,
- a director rule,
- a spawn event,
- a quest progression shortcut,
- or a convenience abstraction that silently skips causality.

---

## Recommended Near-Term Priorities

If the goal is to generate the target feeling as early as possible, the highest-value implementation priorities are:

1. **Place graph + travel + overlapping occupancy**
2. **Persistent sites with changing occupancy**
3. **Concrete inventory / container / transfer model**
4. **Belief, rumor, evidence, and stale information**
5. **Ownership / custody / access / jurisdiction separation**
6. **Interruptible plans and contention over scarce opportunities**
7. **Institutions, offices, notices, and bounties as real state**
8. **Aftermath-rich damage, theft, and loss**
9. **Boundary processes for off-map dependency**
10. **Long-horizon settlement change**

That stack will produce more authentic emergent adventure than any bespoke "content system."

---

## Final Rule

We should not try to simulate "quests," "raids," "encounters," or "dramatic events."

We should simulate:

- pressures,
- movement,
- perception,
- evidence,
- obligation,
- conflict,
- loss,
- records,
- delayed knowledge,
- and contested opportunity.

If those are concrete enough, the world will generate adventures on its own.