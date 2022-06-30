use crate::{
    constants::NODE_UNVISITED_VALUE,
    game::Game,
    treenode::Node,
};

fn ucb1_value(parent_visits: u32, q_value: f32, visits: u32, exp_factor: f64) -> f64 {
    if visits == 0 {
        return NODE_UNVISITED_VALUE;
    }

    let exploitation = f64::from(q_value) / f64::from(visits);
    let exploration = f64::sqrt(f64::ln(f64::from(parent_visits)) / f64::from(visits));
    exp_factor.mul_add(exploration, exploitation)
    // exploitation + exp_factor * exploration
}

pub fn best<G: Game>(nodes: &[Node<G>], parent_visits: u32, exp_factor: f64) -> usize {
    assert!(!nodes.is_empty(), "ucb::best: nodes is empty");
    let max = nodes
        .iter()
        .map(|node| ucb1_value(parent_visits, node.wins(), node.visits(), exp_factor))
        .enumerate()
        .max_by(|(_, a), (_, b)| a.total_cmp(b));
    unsafe { max.unwrap_unchecked().0 }
}
