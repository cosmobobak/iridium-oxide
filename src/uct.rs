use crate::{
    constants::{NODE_UNVISITED_VALUE, WIN_SCORE},
    game::Game,
    treenode::Node,
};

#[inline]
fn ucb1_value(parent_visits: u32, q_value: f32, visits: u32, exp_factor: f64) -> f64 {
    if visits == 0 {
        return NODE_UNVISITED_VALUE;
    }
    let exploitation = f64::from(q_value) / f64::from(visits) / f64::from(WIN_SCORE);
    let exploration = f64::sqrt(f64::ln(f64::from(parent_visits)) / f64::from(visits));
    exp_factor.mul_add(exploration, exploitation)
    // exploitation + exp_factor * exploration
}

#[inline]
pub fn best<G: Game>(nodes: &[Node<G>], parent_visits: u32, exp_factor: f64) -> usize {
    assert!(!nodes.is_empty());
    let mut nodes = nodes.iter().enumerate();
    let (mut max_idx, first_node) = unsafe { nodes.next().unwrap_unchecked() };
    let mut max_val = ucb1_value(parent_visits, first_node.wins(), first_node.visits(), exp_factor);

    for (idx, node) in nodes {
        let val = ucb1_value(parent_visits, node.wins(), node.visits(), exp_factor);
        if val > max_val {
            max_idx = idx;
            max_val = val;
        }
    }
    max_idx
}
