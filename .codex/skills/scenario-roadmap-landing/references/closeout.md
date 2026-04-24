# Closeout Checklist and Report Format

## Closeout Checklist

Before finishing, verify which of these are true:

- exact roadmap row resolved and reassessed
- scenario file exists or was updated truthfully
- golden file exists or was updated truthfully
- the scenario-backed golden is ignored in ordinary lanes and wired into the correct CI family workflow
- golden proves survival-health contract from authored scenario data
- golden proves the mechanic's intended branch at the strongest honest surface
- deterministic replay coverage added or consciously justified
- generated scenario coverage refreshed
- golden inventory/docs refreshed
- any unrelated generated-doc blocker encountered during required refresh was either minimally fixed or explicitly reported
- roadmap sections updated to match the live outcome
- any sibling future rows that became structurally active through shared substrate were recorded as structural-only unless they were also behaviorally proven and intentionally landed
- blocker tickets created or updated when architecture prevented full landing
- for non-landed rows that keep CI workflow wiring, the final report explains why the retained seam is canonical and why the workflow entry is not premature
- markdown and generated-doc edits pass `git diff --check`
- final report states whether the row is now `Landed`, still `Drafting`/`In Progress`, or blocked behind named ticket(s)

## Report Format

Use a concise closeout shaped like this:

```markdown
# Scenario Roadmap Landing: <scenario-name>

## Reassessment
- <current row status, live overlap, and exact owned mechanics>

## Outcome
- <landed / in progress / blocked>
- <scenario, golden, production, and doc changes>

## Verification
- <exact commands actually run>
- <what each command proved>

## Follow-ups
- <tickets created or updated, if any>
```

If the row did not fully land, say that directly. Name the blocker and the owning ticket instead of implying success.
If the row did not fully land, also separate:

- what was implemented this session
- what remains blocked before `Landed`
