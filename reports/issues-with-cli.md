# Issues with CLI

While using the CLI app of worldwake-cli (the focus of the skills .claude/skills/cli-improvement*), I've found issues that weren't found or evaluated through the cli-improvement* iterative pipeline.

1. declare_support issues

'declare_support — 1 ticks' appears as a valid action. However, when you choose it:

[tick 0] Kael @ Market Square > do 3
Requested: declare_support
[tick 0] Kael @ Market Square > tick
error: tick error: Action(PreconditionFailed("action def adef13 requires DeclareSupport payload"))

It's completely unclear what's failing here.

2. 'trace' menu action unclear

[tick 0] Kael @ Market Square > trace
error: the following required arguments were not provided:
  <ID>

Usage: trace <ID>

For more information, try '--help'.

What does the <ID> relate to? Events?

3. Insufficient decision/action info.

[tick 1] Kael @ Market Square > tick
--- Tick 1 --- (5 events) [actions: 1 started]
[tick 2] Kael @ Market Square > events
Events (9 of 9):
  [E8] tick 1 — System
  [E7] tick 1 — WorldMutation, System
  [E6] tick 1 — WorldMutation, System
  [E5] tick 1 — ActionStarted by Merchant Vara
  [E4] tick 1 — Decision by Merchant Vara
  [E3] tick 0 — System
  [E2] tick 0 — WorldMutation, System
  [E1] tick 0 — WorldMutation, System
  [E0] tick 0 — Internal

[tick 2] Kael @ Market Square > event 4
Event [E4]
  tick: 1
  tags: (none)
  cause: system tick 1
  actor: Merchant Vara
  place: (none)
  targets: (none)
  witnesses: (none)
  deltas (1):
    ActiveGoal: set on Merchant Vara

[tick 2] Kael @ Market Square > event 5
Event [E5]
  tick: 1
  tags: ActionStarted
  cause: external input 1
  actor: Merchant Vara
  place: Market Square
  targets: Kael
  witnesses: (none)
  deltas: (none)


[tick 3] Kael @ Market Square > events
Events (10 of 13):
  [E12] tick 2 — System
  [E11] tick 2 — WorldMutation, System
  [E10] tick 2 — WorldMutation, System
  [E9] tick 2 — WorldMutation, ActionCommitted, Social by Merchant Vara
  [E8] tick 1 — System
  [E7] tick 1 — WorldMutation, System
  [E6] tick 1 — WorldMutation, System
  [E5] tick 1 — ActionStarted by Merchant Vara
  [E4] tick 1 — Decision by Merchant Vara
  [E3] tick 0 — System
[tick 3] Kael @ Market Square > event 9
Event [E9]
  tick: 2
  tags: WorldMutation, ActionCommitted, Social
  cause: system tick 2
  actor: Merchant Vara
  place: Market Square
  targets: Kael
  witnesses: (none)
  deltas (2):
    AgentBeliefStore: set on Merchant Vara
    AgentBeliefStore: set on Kael

----

We get notice that ActiveGoal has been set, that ActionStarted, and that ActionCommitted. But we aren't told what's the ActiveGoal set, nor what action started regarding the ActionStarted event. In the ActionCommitted, we see that it's a Social action, possibly a Tell, and that it has changed something in the AgentBeliefStore, but we can't know what.
