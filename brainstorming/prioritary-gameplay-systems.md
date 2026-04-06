# Priority Report: What Gameplay Systems Worldwake Should Implement Next

## Executive judgment

Right now the project already proves a strong village / institution / logistics simulator.

It does **not** yet fully prove the broader **adventure-world generator** described in `design-note.md`.

That distinction matters.

The current golden suite already shows that the world can:
- run without the player,
- produce needs-driven autonomous behavior,
- route travel through belief and danger,
- create supply disruption and recovery,
- handle theft, accusation, punishment, offices, patrols, notices, and bounties,
- keep aftermath and contention real,
- and preserve meaning through save/load.

So the next priorities should **not** be “more content” inside already-proven loops.

They should be the **missing carriers of consequence** that widen the causal graph and create whole new classes of adventure.

Bluntly: **stop deepening the village square. Start giving the world places to go, people to miss, beasts to hunt, lies to believe, and outside shocks to absorb.**

---

## How I am prioritizing

I am ranking candidates by five questions:

1. Does this add a **new carrier of consequence** rather than a content wrapper?
2. Does it unlock **several situation classes at once**?
3. Does it heavily reuse systems you have **already proven**?
4. Does it fill a clearly missing hole in `FOUNDATIONS.md` and `design-note.md`?
5. Does it increase **player-facing adventure density** without violating anti-script rules?

---

## What FOUNDATIONS imply for prioritization

The foundations push you toward systems that:

- create new inspectable world state,
- move knowledge through lawful carriers,
- preserve aftermath,
- separate truth from belief,
- resolve contention explicitly,
- and remain legible in debug, replay, and save/load.

That means the wrong next move is anything like:

- “add a quest system,”
- “spawn encounters on roads,”
- “make dungeons refresh,”
- “add a global crime or danger score,”
- or “make something interesting happen every so often.”

The right next move is anything that adds:

- new persistent places,
- new expectation / absence records,
- new evidence types,
- new institution-facing records,
- new physical or social routes for consequence to travel,
- or new long-tail aftermath.

---

## What the golden suite already proves well

You are already strong in these areas:

### 1. Needs, acquisition, production, and fallback behavior
The game already proves hunger / thirst / fatigue pressure, local and remote acquisition, production fallback, and multi-step plan materialization chains.

### 2. Travel as a real process
The suite already proves multi-hop travel, route memory, danger-aware rerouting, travel interruption, and death or failure mid-route.

### 3. Trade, merchant behavior, and supply movement
You already have real stock, sale listings, home markets, restocking, delivery to facilities, rejection recovery, and supply-chain disruption.

### 4. Combat, wounds, corpses, looting, burial, and recovery
This is already a real consequence stack, not a decorative combat layer.

### 5. Local knowledge, rumor, contradiction, and evidence
You already prove direct observation vs report, social tell, stale or contradictory claims, evidence persistence and decay, and locality of institutional knowledge.

### 6. Institutions, succession, patrol, justice, notices, and bounties
You already have office claims, loyalty, threats, succession delays, patrol adaptation, accusation and punishment, social artifact lifecycle, and autonomous institutional posting.

### 7. Explicit contention and deterministic authority
You already prove queues, grants, race loss, plan invalidation, save/load persistence, and deterministic replay.

This matters because it changes the roadmap.

You do **not** need more isolated mechanics inside these areas yet.

You need the missing structures that make these existing systems generate **new families of situations**.

---

## Gap map against the design note

### Already largely covered
- dangerous road travel
- emergent outlawry in the bandit-camp sense
- bounties / notices
- theft / missing property / investigation
- interrupted errands
- office failure / patrol gaps / local law breakdown
- competing claimants and lost opportunities

### Partial but still thin
- rumor-led expeditions
- shortage behavior beyond stockout/restock
- hideouts and changing occupancy beyond bandit-specific cases
- place identity beyond route nodes, markets, halls, farms, and camps

### Clearly underpowered or missing
- missing persons, search, and rescue
- persistent ruins / dungeons / watchtowers / caves with layered occupancy
- predator displacement and roaming nonhuman threats
- false rumor -> wrongful accusation -> contested correction
- boundary shocks and upstream dependency
- long-horizon settlement decline, abandonment, and reoccupation
- social aftermath like grudges, debt, loyalty-protection, revenge

A useful smell test from `FOUNDATIONS.md`:

The canonical regression families **C, D, E, and F** are already substantially represented in the current suite.

**A, B, G, and H** are not.

That alone should push:
- predators,
- interrupted travel under extraordinary threat,
- wrongful accusation,
- and remote-shock logistics

high on the list.

---

## Priority order

## 1. Expectation, obligations, and missing-person search / rescue

This is my top pick.

### Why it should come first
- It activates Principle 17 directly: **surprise comes from violated expectation**.
- It reuses systems you already have: travel, local perception, rumor, contradiction, investigation, care, patrol, corpse handling, bounty-like artifacts, and route-local evidence.
- It unlocks a huge number of high-value situations at once: missing couriers, overdue merchants, failed escorts, family search, rescue-too-late, corpse recovery, ransom suspicion, false leads, and search parties.

### The key design insight
Do **not** start with “search” as a special activity.

Start with **expectation records**.

The substrate you actually need is:
- expected arrivals,
- duties,
- patrol check-ins,
- escort obligations,
- delivery promises,
- household routines,
- “should have been back by now” beliefs.

From there, search and rescue become lawful downstream behavior.

### Concrete state to add
- expectation / commitment records with subject, expected place, time window, basis, and owner
- overdue state and who knows it
- last-seen and lead records with provenance and freshness
- search intents and search assignments
- outcome records: found safe, found wounded, found dead, not found

### Actions / processes to add
- `report_missing`
- `ask_about_person` / `share_last_seen`
- `search_route` / `search_site`
- `identify_body`
- `escort_wounded` / `carry_home` / `report_found`

### What it unlocks immediately
- “Courier never arrived.”
- “Hunter went out and did not return.”
- “Guard expected at checkpoint is missing.”
- “Child left for the orchard and family panics.”
- “Search begins from rumor and gets it wrong.”
- “A body is found before the institution learns who it is.”

### Golden scenarios I would add first
- courier overdue -> employer notices -> guard searches last-seen route -> courier found wounded -> escort to healer
- traveler missing -> stale rumor sends searcher to wrong place -> local correction -> re-search
- merchant fails to return -> corpse recovery triggers ownership and inheritance / debt consequences

### Failure modes to avoid
- no global missing-person detector
- no free omniscient search radius
- no event-script “search mission appears”
- no instant institutional awareness without a carrier

---

## 2. Persistent sites with changing occupancy, access, and layered traces

If priority 1 creates **people to miss**, this creates **places worth going to**.

### Why this is next
- The design note is explicit that persistent sites are foundational infrastructure, not content.
- Right now you have many functional places, but not enough memorable, reusable adventure sites with layered history.
- This is the cleanest path to the Bethesda feeling without cheating.

The point is not “add dungeons.”

The point is to generalize place identity so sites can persist, be entered, be used, be cleared, be reoccupied, and retain traces.

### Concrete state to add
- site entities with identity
- sublocations / rooms / approaches
- access points, doors, locks, and barriers
- containers, stash spots, and defendable chokepoints
- occupancy claims
- site-local evidence and prior-use traces
- site reputation / rumor as belief, not truth

### Actions / processes to add
- `occupy_site`
- `claim_room` / `secure_door` / `stash_goods`
- `search_site` / `inspect_room`
- `clear_site` / `abandon_site` / `reuse_site`

### What it unlocks immediately
- ruins that become hideouts
- caves reused by predators or fugitives
- watchtowers that change hands after patrol failure
- shrines or crypts that acquire layered traces over time
- stolen goods hidden in a cellar
- rescue or pursuit targets with spatial depth

### Golden scenarios I would add first
- abandoned watchtower -> outlaw occupation after patrol gap -> stolen goods cached there -> eventual discovery
- guards clear a cave -> traces remain -> scavengers reuse it later
- rumor of occupied ruin -> traveler arrives -> finds different occupants than rumor claimed

### Strong opinion
Do **not** build static dungeon content boxes and do **not** build reset logic.

That would be directly against your own design note.

Also, do **not** wait for tile-level interiors. A medium-grain site model with two to eight meaningful sublocations is enough to start generating real adventure.

---

## 3. Predator ecology, dens, and roaming nonhuman threats

This is the missing **beasts to hunt** layer.

### Why it is this high
- `FOUNDATIONS.md` canonical regressions A and B are basically shouting for this.
- Right now danger is rich on the human/institutional side but thin on the ecological/nonhuman side.
- Without this, the world risks feeling like a great social sim with bandits, but not like a living frontier.

### Concrete state to add
- predator agents or factions with territory / habitat preference
- hunger, prey preference, injury, fear, aggression, and retreat thresholds
- dens / lairs as persistent sites
- carcass and track evidence
- route-local fear / warning beliefs carried socially, not globally

### Actions / processes to add
- `roam` / `hunt` / `scavenge` / `retreat_to_den`
- territory expansion under food pressure
- lair occupation / abandonment
- institutional or private hunt response through existing notice / bounty channels

### What it unlocks immediately
- caravan attacks that are not bandit-only
- road fear caused by real roaming threats
- hunts, warnings, evacuations, and route avoidance
- “monster” content without encounter tables
- ecological displacement after depletion or competition

### Golden scenarios I would add first
- prey shortage -> beast range expansion -> caravan attack -> survivors report -> bounty posted -> hunter tracks and claims reward
- hungry traveler going to market -> predator sighting invalidates safety assumption -> retreat or reroute
- cleared den -> nearby route becomes safer until another threat lawfully occupies it

### Failure modes to avoid
- no encounter spawning
- no roaming threat director
- no global danger meter
- no monster-specific quest pipeline

---

## 4. Boundary processes and remote shocks

This is the missing **the world is larger than the map** layer.

### Why it matters
- `FOUNDATIONS.md` and the design note both explicitly call this out.
- It is the clean way to get upstream war, delayed convoy, refugee flow, levy, external shortage, migrating herd, or border closure without cheating.
- It gives you macro-pressure while preserving locality and provenance.

### Concrete state to add
- source regions
- boundary channels / routes
- scheduled or expected inflows
- manifests or commitments for incoming goods / people / messages
- failure, delay, reroute, and capacity states
- observables for non-arrival and degraded arrival

### Actions / processes to add
- convoy arrival through boundary node
- delayed / canceled / reduced inflow
- external message or decree arrival
- refugee or migrant arrival
- local detection of failed expectation

### What it unlocks immediately
- shortages that begin outside the local simulation core
- settlement adaptation to outside pressure
- dependency chains that feel real instead of scripted
- geopolitical flavor without omniscient manager code

### Golden scenarios I would add first
- off-map grain convoy delayed -> local market continues under stale expectation -> shortage emerges -> ration / theft / substitution follow
- refugee party arrives with war report -> patrol priorities and office behavior change locally
- external bridge collapse reduces inflow until repaired or rerouted

### Strong opinion
This system is more important than a fancy economy model.

Without boundary processes, your world will tend to feel causally **closed** even if local systems are excellent.

---

## 5. Contested evidence, wrongful accusation, warrants, and correction

This is the missing **the world can be socially wrong** layer.

### Why it is high priority
- Current justice seems strong where theft is real and witnessed.
- But the design note and `FOUNDATIONS.md` want imperfect institutions, false rumor, and delayed correction.
- Canonical regression G is one of the clearest architectural gaps.

### Concrete state to add
- accusation, suspicion, warrant, detention, and case records as separate states
- evidence bundles with provenance, freshness, and conflict
- alibi records
- correction / exoneration records that append rather than overwrite
- office procedures for acting under uncertainty

### Actions / processes to add
- `issue_warrant`
- `detain` / `release`
- `present_evidence` / `contest_evidence`
- `record_alibi`
- `revise_case` / `close_case`

### What it unlocks immediately
- mistaken pursuit
- wrongful detention
- split institutional response
- later exoneration that does not erase damage already done
- political abuse through selective evidence handling
- downstream grudges and legitimacy effects

### Golden scenarios I would add first
- false testimony -> warrant issued -> suspect detained -> later alibi arrives -> one office updates, another does not
- conflicting witness claims -> magistrate acts on incomplete evidence -> correction propagates unevenly
- innocent fugitive behavior becomes rational because the institution is acting on a wrong belief

### Failure modes to avoid
- no omniscient judge correction
- no single guilt boolean
- no retroactive history rewrite
- no “institution always eventually gets truth for free”

---

## 6. Scarcity response: substitution, rationing, debt, and household triage

This is how you turn existing logistics into social pressure.

### Why it is not higher
- You already have stockout, restock, supply disruption, and merchant behavior.
- What is missing is deeper downstream human response once supply gets tight.
- It becomes much stronger once boundary shocks exist, which is why I rank it after them.

### Concrete state to add
- substitute-good preferences
- household and institution stock commitments
- debt / credit / obligation records
- ration orders and priority access rules
- queue pressure and aid requests
- listing changes driven by concrete stock and demand, not price scripts

### Actions / processes to add
- `ration_distribution`
- `borrow` / `lend` / `repay`
- `substitute_purchase` / `substitute_consumption`
- `request_aid`
- `refuse_sale` / `prioritize_locals` / `hoard`

### What it unlocks immediately
- shortages that reshape routine behavior rather than just empty shelves
- debt-driven future behavior
- aid and favoritism
- theft as a pressure response rather than a generic bad action
- class differences in coping behavior

### Golden scenarios I would add first
- expected bread inflow fails -> baker shifts to barley substitute -> some households buy, some borrow, some steal
- office treasury grain rationed to guards and sick first
- merchant prefers trusted debtors or locals under shortage

### Strong opinion
Do **not** jump straight to a global price simulation.

If you add “price dynamics” before substitution, rationing, and debt, you will get pretty numbers with thin consequences.

---

## 7. Social aftermath memory: grudges, debts, kin-protection, revenge

This is how agents stop feeling merely rational and start feeling human.

### Why it is important but not earlier
- It is high-value flavor and consequence, but it depends on other systems having more interesting events to remember.
- Search/rescue, wrongful accusation, and shortage all become much richer once this exists.

### Concrete state to add
- relationship edges with provenance
- obligation and debt memory
- grudge memory from theft, harm, humiliation, or loss
- kin / protector / patron relations
- willingness to help, hide, testify, retaliate, or refuse

### Actions / processes to add
- `seek_revenge`
- `protect_kin`
- `shelter_fugitive`
- `collect_debt` / `forgive_debt`
- `refuse_help` / `give_priority_aid`

### What it unlocks immediately
- revenge chains
- selective witness cooperation
- favoritism and informal protection
- institutions being bypassed by social reality
- repeated world consequences from past harm

### Golden scenarios I would add first
- healer saves wounded traveler -> later receives preferential aid or protection
- thief is legally punished but sibling later retaliates
- wrongfully accused agent is exonerated, but not everyone forgives or updates

### Failure modes to avoid
- no floating friendship score with no event source
- no universal revenge behavior
- no social “mood layer” detached from concrete events

---

## 8. Settlement decline, abandonment, and reoccupation

This is the macro synthesis system, not the next immediate one.

### Why it is last in this stack
- It is extremely valuable, but it wants the earlier systems to feed it.
- Done too early, it will collapse into fake settlement health bars.
- Done after sites, boundary shocks, scarcity, and social fallout, it becomes a natural emergent outcome.

### Concrete state to add
- household departure / business closure
- vacant and abandoned buildings
- suspended offices or degraded institutions
- reoccupation rights and squatting
- migration plans and local labor loss

### Actions / processes to add
- `leave_settlement`
- `close_shop` / `abandon_home`
- `occupy_abandoned_site`
- `scavenge_from_ruin`
- `resettle` / `re-found`

### What it unlocks immediately
- villages becoming thinner, more dangerous, and more interesting over time
- organic creation of new ruins and squats
- long-horizon world self-authorship
- a world that changes meaningfully even without the player nearby

### Golden scenarios I would add first
- repeated convoy failure + patrol weakness -> shop closure -> household departure -> abandoned building later reused by squatters
- recovered route + new inflow -> partial reoccupation of a declining site

### Strong opinion
Absolutely do **not** model this as a single settlement prosperity or security score.

It has to emerge from population, assets, duties, inflow, and fear.

---

## What I would explicitly not prioritize yet

### 1. More commodities, recipes, and crafting branches
You already have enough proof that concrete goods, production, and fallback behavior work. More content here mostly deepens an existing branch.

### 2. More office types or law variants
You already prove the institutional pattern. More variants are lower leverage than better failure, misinformation, and downstream consequence.

### 3. More bounty / notice variants in isolation
The artifact system is already paying off. The higher-value move is to add more real causes for those artifacts to exist.

### 4. More combat verb variety
Combat is already serving the simulation. The missing value is better reasons for conflict, better aftermath, and richer spatial context.

### 5. Any quest-like wrapper
If you feel tempted to add “adventure content generation,” that is almost certainly a sign that one of the systems above is still missing.

---

## The practical sequence I would actually follow

If I were sequencing implementation for maximum payoff:

1. expectation / overdue / missing-person substrate
2. persistent sites with medium-grain sublocations and occupancy turnover
3. one end-to-end predator family plus den behavior
4. boundary processes for one concrete dependency class, probably staple food inflow
5. wrongful accusation / warrant / correction layer
6. shortage response on top of the new boundary pressure
7. relationship aftermath
8. settlement decline / reuse

That sequence gives you the biggest increase in situation-class coverage with the least violation risk.

---

## Final assessment

The current game already proves that local causality works.

What it does **not** yet fully prove is the thing your design note is really aiming for: a world that continuously manufactures adventure situations out of ordinary state.

The fastest route there is **not** more authored content and **not** more detail inside already-solved systems.

It is the missing carrier stack:

- expectation and absence,
- persistent site identity,
- ecological threat motion,
- remote dependency,
- contested institutional truth,
- richer scarcity coping,
- personal social aftermath,
- and settlement transformation.

In one sentence:

You already have the beginnings of a living polity-and-logistics simulator. The next step is to turn it into a self-authored adventure topology generator.