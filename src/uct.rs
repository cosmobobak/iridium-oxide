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
    let exploitation = f64::from(q_value) / f64::from(visits);
    let exploration =
        ((f64::from(parent_visits)).ln() / f64::from(visits)).sqrt() * exp_factor * f64::from(WIN_SCORE);
    exploitation + exploration
}

#[inline]
pub fn best<G: Game>(nodes: &[Node<G>], parent_visits: u32, exp_factor: f64) -> usize {
    assert!(!nodes.is_empty());
    let first_node = unsafe { nodes.get_unchecked(0) };
    let mut max_idx = 0;
    let mut max_val = ucb1_value(parent_visits, first_node.wins(), first_node.visits(), exp_factor);

    for (idx, node) in nodes.iter().enumerate().skip(1) {
        let val = ucb1_value(parent_visits, node.wins(), node.visits(), exp_factor);
        if val > max_val {
            max_idx = idx;
            max_val = val;
        }
    }
    max_idx
}
