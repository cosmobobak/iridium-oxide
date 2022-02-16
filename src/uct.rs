use crate::{constants::EXP_FACTOR, game::Game, treenode::Node};

#[derive(Debug, Clone, PartialEq)]
struct Sortablef64(f64);

impl Eq for Sortablef64 {}

impl PartialOrd for Sortablef64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl Ord for Sortablef64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .expect("Sortablef64::cmp found NaN")
    }
}

fn ucb1_value(parent_visits: u32, win_count: i32, visits: u32) -> Sortablef64 {
    if visits == 0 {
        return Sortablef64(f64::MAX);
    }
    let exploitation = f64::from(win_count) / f64::from(visits);
    let exploration = f64::sqrt(f64::ln(f64::from(parent_visits)) / f64::from(visits)) * EXP_FACTOR;
    Sortablef64(exploitation + exploration)
}

pub fn best<G: Game>(nodes: &[Node<G>], parent_visits: u32, first_index: usize) -> usize {
    nodes
        .iter()
        .enumerate()
        .max_by_key(|(_, node)| ucb1_value(parent_visits, node.wins(), node.visits()))
        .map(|(idx, _)| idx)
        .unwrap()
        + first_index
}
