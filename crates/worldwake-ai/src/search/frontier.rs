use super::SearchNode;
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone)]
pub(super) struct FrontierEntry<'snapshot> {
    node: SearchNode<'snapshot>,
}

impl<'snapshot> FrontierEntry<'snapshot> {
    pub(super) fn new(node: SearchNode<'snapshot>) -> Self {
        Self { node }
    }

    pub(super) fn into_node(self) -> SearchNode<'snapshot> {
        self.node
    }
}

impl PartialEq for FrontierEntry<'_> {
    fn eq(&self, other: &Self) -> bool {
        compare_search_nodes(&self.node, &other.node) == Ordering::Equal
    }
}

impl Eq for FrontierEntry<'_> {}

impl PartialOrd for FrontierEntry<'_> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FrontierEntry<'_> {
    fn cmp(&self, other: &Self) -> Ordering {
        compare_search_nodes(&other.node, &self.node)
    }
}

#[allow(dead_code)]
pub(super) struct DualFrontier<'snapshot> {
    regular: BinaryHeap<FrontierEntry<'snapshot>>,
    preferred: BinaryHeap<FrontierEntry<'snapshot>>,
    boost_remaining: u8,
    preferred_operator_boost: u8,
    use_preferred_next: bool,
}

#[allow(dead_code)]
impl<'snapshot> DualFrontier<'snapshot> {
    pub(super) fn new(preferred_operator_boost: u8) -> Self {
        Self {
            regular: BinaryHeap::new(),
            preferred: BinaryHeap::new(),
            boost_remaining: 0,
            preferred_operator_boost,
            use_preferred_next: true,
        }
    }

    pub(super) fn push_regular(&mut self, entry: FrontierEntry<'snapshot>) {
        self.regular.push(entry);
    }

    pub(super) fn push_preferred(&mut self, entry: FrontierEntry<'snapshot>) {
        self.preferred.push(entry);
    }

    pub(super) fn push_both(&mut self, entry: FrontierEntry<'snapshot>) {
        self.preferred.push(entry.clone());
        self.regular.push(entry);
    }

    pub(super) fn push(&mut self, entry: FrontierEntry<'snapshot>) {
        self.push_regular(entry);
    }

    pub(super) fn trigger_boost(&mut self) {
        self.boost_remaining = self.preferred_operator_boost;
    }

    pub(super) fn is_empty(&self) -> bool {
        self.preferred.is_empty() && self.regular.is_empty()
    }

    pub(super) fn pop(&mut self) -> Option<SearchNode<'snapshot>> {
        if self.is_empty() {
            return None;
        }

        let prefer_preferred = self.boost_remaining > 0 || self.use_preferred_next;
        let popped = if prefer_preferred {
            self.preferred.pop().or_else(|| self.regular.pop())
        } else {
            self.regular.pop().or_else(|| self.preferred.pop())
        };

        if popped.is_some() {
            if prefer_preferred && self.boost_remaining > 0 {
                self.boost_remaining = self.boost_remaining.saturating_sub(1);
            }
            self.use_preferred_next = !self.use_preferred_next;
        }

        popped.map(FrontierEntry::into_node)
    }
}

pub(super) fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let left_f = left.search_cost.saturating_add(left.heuristic_ticks);
    let right_f = right.search_cost.saturating_add(right.heuristic_ticks);
    left_f
        .cmp(&right_f)
        .then_with(|| left.search_cost.cmp(&right.search_cost))
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.as_slice().cmp(right.steps.as_slice()))
}

#[cfg(test)]
mod tests {
    use super::{DualFrontier, FrontierEntry};
    use crate::{GoalKey, GoalOffer, PlanningSnapshot, build_planning_snapshot};
    use std::collections::BTreeSet;
    use worldwake_core::{
        CauseRef, ControlSource, ExecutionBudget, GoalKind, OpportunityAnchor, Tick,
        VisibilitySpec, WitnessData, World, WorldTxn, build_prototype_world,
    };
    use worldwake_sim::{PerAgentBeliefView, RecipeRegistry};

    fn snapshot() -> PlanningSnapshot {
        let mut world = World::new(build_prototype_world()).unwrap();
        let place = world.topology().place_ids().next().unwrap();
        let agent = {
            let mut txn = WorldTxn::new(
                &mut world,
                Tick(1),
                CauseRef::Bootstrap,
                None,
                None,
                VisibilitySpec::SamePlace,
                WitnessData::default(),
            );
            let agent = txn
                .create_agent("Frontier Tester", ControlSource::Ai)
                .unwrap();
            txn.set_ground_location(agent, place).unwrap();
            let mut event_log = worldwake_core::EventLog::new();
            let _ = txn.commit(&mut event_log);
            agent
        };
        let view = PerAgentBeliefView::from_world(agent, &world);
        build_planning_snapshot(&view, agent, &BTreeSet::new(), &BTreeSet::new(), 0)
    }

    fn goal() -> GoalOffer {
        GoalOffer {
            key: GoalKey::from(GoalKind::ConsumeOwnedCommodity {
                commodity: worldwake_core::CommodityKind::Bread,
            }),
            anchor: OpportunityAnchor::None,
            evidence_entities: BTreeSet::new(),
            evidence_places: BTreeSet::new(),
            obligation_source: None,
            commitment_impact_if_ignored: worldwake_core::Permille::ZERO,
            required_information_gaps: Vec::new(),
            invalidators: Vec::new(),
            learned_expectation_refs: Vec::new(),
        }
    }

    fn node(
        search_cost: u32,
        heuristic_ticks: u32,
        total_estimated_ticks: u32,
        step_count: usize,
    ) -> crate::search::SearchNode<'static> {
        let snapshot = Box::leak(Box::new(snapshot()));
        let mut node = crate::search::heuristic::root_node(
            snapshot,
            &goal(),
            &RecipeRegistry::new(),
            &ExecutionBudget::default(),
        );
        node.search_cost = search_cost;
        node.heuristic_ticks = heuristic_ticks;
        node.total_estimated_ticks = total_estimated_ticks;
        debug_assert_eq!(
            step_count, 0,
            "frontier ordering tests do not require steps"
        );
        node
    }

    #[test]
    fn dual_frontier_alternates() {
        let mut frontier = DualFrontier::new(0);
        frontier.push_preferred(FrontierEntry::new(node(1, 1, 2, 0)));
        frontier.push_preferred(FrontierEntry::new(node(2, 1, 3, 0)));
        frontier.push_regular(FrontierEntry::new(node(5, 1, 6, 0)));
        frontier.push_regular(FrontierEntry::new(node(6, 1, 7, 0)));

        assert_eq!(frontier.pop().unwrap().search_cost, 1);
        assert_eq!(frontier.pop().unwrap().search_cost, 5);
        assert_eq!(frontier.pop().unwrap().search_cost, 2);
        assert_eq!(frontier.pop().unwrap().search_cost, 6);
    }

    #[test]
    fn dual_frontier_boost() {
        let mut frontier = DualFrontier::new(2);
        frontier.push_preferred(FrontierEntry::new(node(1, 1, 2, 0)));
        frontier.push_preferred(FrontierEntry::new(node(2, 1, 3, 0)));
        frontier.push_preferred(FrontierEntry::new(node(3, 1, 4, 0)));
        frontier.push_regular(FrontierEntry::new(node(9, 1, 10, 0)));
        frontier.trigger_boost();

        assert_eq!(frontier.pop().unwrap().search_cost, 1);
        assert_eq!(frontier.pop().unwrap().search_cost, 2);
        assert_eq!(frontier.pop().unwrap().search_cost, 3);
        assert_eq!(frontier.pop().unwrap().search_cost, 9);
    }

    #[test]
    fn dual_frontier_preferred_empty_falls_through() {
        let mut frontier = DualFrontier::new(1);
        frontier.push_regular(FrontierEntry::new(node(4, 1, 5, 0)));

        assert_eq!(frontier.pop().unwrap().search_cost, 4);
        assert!(frontier.pop().is_none());
    }

    #[test]
    fn dual_frontier_both_empty_returns_none() {
        let mut frontier = DualFrontier::new(1);

        assert!(frontier.pop().is_none());
        assert!(frontier.is_empty());
    }

    #[test]
    fn dual_frontier_ordering_preserved() {
        let mut frontier = DualFrontier::new(0);
        frontier.push_regular(FrontierEntry::new(node(5, 5, 10, 0))); // f = 10
        frontier.push_regular(FrontierEntry::new(node(3, 1, 4, 0))); // f = 4
        frontier.push_regular(FrontierEntry::new(node(2, 2, 5, 0))); // f = 4, lower cost

        assert_eq!(frontier.pop().unwrap().search_cost, 2);
        assert_eq!(frontier.pop().unwrap().search_cost, 3);
        assert_eq!(frontier.pop().unwrap().search_cost, 5);
    }

    #[test]
    fn dual_frontier_zero_boost() {
        let mut frontier = DualFrontier::new(0);
        frontier.push_preferred(FrontierEntry::new(node(1, 1, 2, 0)));
        frontier.push_preferred(FrontierEntry::new(node(2, 1, 3, 0)));
        frontier.push_regular(FrontierEntry::new(node(7, 1, 8, 0)));
        frontier.push_regular(FrontierEntry::new(node(8, 1, 9, 0)));
        frontier.trigger_boost();

        assert_eq!(frontier.pop().unwrap().search_cost, 1);
        assert_eq!(frontier.pop().unwrap().search_cost, 7);
        assert_eq!(frontier.pop().unwrap().search_cost, 2);
        assert_eq!(frontier.pop().unwrap().search_cost, 8);
    }
}
