# Mixed Outcome: Narrow Fix Landed, Broader Golden Still False

Use this branch whenever focused live proof confirms a real narrow production fix inside the ticket's domain, but the drafted higher-level golden/E2E ending still does not hold afterward.

1. Land the narrow production fix at the strongest honest owning seam.
2. Stop before adding more exploratory end-to-end proof edits for the drafted broader ending.
   Short-lived diagnostic probes are allowed only when needed to isolate the remaining owner or failure seam after the narrow fix lands. Do not keep those probes as part of the final proof surface unless they become the truthful owned seam.
3. Rewrite the active ticket/spec immediately to the newly proved boundary.
4. Record the exact still-false higher-level premise and the focused evidence that disproved it.
5. Create or update the follow-up ticket that owns the deferred broader seam before broader verification and closeout.
   If the disproved remainder is still in the same scenario/domain but now clearly belongs to a later authoritative boundary, create a new follow-up ticket for that later boundary instead of stretching the current ticket back upward.
   If focused proof lands candidate/goal emission or another earlier planner seam, but same-family ranking, selected-branch, or scenario-row proof still stays false, close the current ticket to the earlier landed seam and open the follow-up specifically on the later selection/proof boundary rather than treating the emitted candidate itself as unproven.
   If diagnosis reveals multiple non-overlapping remaining contradictions, create one follow-up ticket per causal owner / proof seam rather than one umbrella successor ticket.
   If the first broadened repro after a narrow fix still fails and reveals another earlier causal owner, stop scenario/golden tuning immediately, split or land that earlier owner first, and only then return to the broader scenario proof.
6. During closeout, record the split explicitly: landed narrow boundary, focused and broadened commands, concrete reason the broader premise stayed false, and the follow-up owner.
   For multi-scenario golden suites, label each scenario's proof seam separately in the active ticket/spec and generated docs: which scenarios now use source-backed events or traces, which remain explicit fixture/lifecycle proof, and which follow-up owns each remaining explicit or missing source-backed branch.
7. If you added a temporary exploratory golden/test only to prove or disprove the stronger end state, remove or rewrite it before final verification when that stronger contract remains false.
   Apply the same cleanup rule to temporary exploratory scenario, fixture, or config edits used only for diagnosis.
   When the narrower seam survives in the same golden/test file, rename or narrow the remaining test names, assertions, metadata comments, and nearby roadmap prose so they describe only the landed seam rather than the disproved broader ending.
8. If the user later approves the split rather than asking for more implementation on the disproved broader seam, immediately formalize it in the repo: update the active ticket's status/scope/outcome to the landed narrow boundary, create the follow-up from `tickets/_TEMPLATE.md`, and update any live roadmap or blocker docs that still point at the old combined ownership. Do not rerun unchanged code verification in that follow-up docs/ticket pass unless implementation changed again.

If the active ticket is already the follow-up / remainder owner and focused reassessment shows no truthful implementation slice remains inside its current claim, stop further implementation attempts immediately. Remove temporary probes, keep the strongest focused evidence, and treat the ticket as a rejection-and-split boundary rather than continuing local churn. Use 1-3-1 when more than one plausible causal continuation remains (for example two different punishment paths or two different scenario owners); otherwise rewrite the active ticket to `REJECTED`, create the narrower successor ticket, and update live roadmap/blocker docs in the same pass.
