use super::SearchNode;
use std::cmp::Ordering;

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

pub(super) fn compare_search_nodes(left: &SearchNode<'_>, right: &SearchNode<'_>) -> Ordering {
    let left_f = left
        .total_estimated_ticks
        .saturating_add(left.heuristic_ticks);
    let right_f = right
        .total_estimated_ticks
        .saturating_add(right.heuristic_ticks);
    left_f
        .cmp(&right_f)
        .then_with(|| left.total_estimated_ticks.cmp(&right.total_estimated_ticks))
        .then_with(|| left.steps.len().cmp(&right.steps.len()))
        .then_with(|| left.steps.cmp(&right.steps))
}
