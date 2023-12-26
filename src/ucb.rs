use crate::{constants::NODE_UNVISITED_VALUE, game::Game, treenode::Node};

#[inline(never)]
fn puct(parent_visits: u32, q_value: f32, visits: u32, _exp_factor: f32, policy: f32) -> f32 {
    #![allow(clippy::cast_precision_loss)]

    if visits == 0 {
        return NODE_UNVISITED_VALUE;
    }

    // let exploitation = f32::from(q_value) / f32::from(visits);
    // let exploration = f32::sqrt(f32::ln(f32::from(parent_visits)) / f32::from(visits));
    // exp_factor.mul_add(exploration, exploitation)
    // // exploitation + exp_factor * exploration
    // ^^^ the normal UCB1 formula

    // pb_c = math.log((parent.visit_count + config.pb_c_base + 1) / config.pb_c_base) + config.pb_c_init
    let pb_c = fastapprox::faster::ln(((parent_visits as f32) + 1.8 + 1.0) / 1.8);
    // pb_c *= math.sqrt(parent.visit_count) / (child.visit_count + 1)
    let pb_c = pb_c * f32::sqrt(parent_visits as f32) / (visits as f32 + 1.0);

    // prior_score = pb_c * child.prior
    // value_score = child.value()
    // return prior_score + value_score

    let prior_score = pb_c * policy;
    let value_score = q_value / visits as f32;

    prior_score + value_score
}

#[inline(never)]
pub fn best<G: Game>(parent: &G, nodes: &[Node<G>], parent_visits: u32, exp_factor: f32) -> usize {
    assert!(!nodes.is_empty(), "ucb::best: nodes is empty");
    let mut best_value = f32::NEG_INFINITY;
    let mut best_index = 0;
    let mut policies = Vec::with_capacity(nodes.len());

    // compute policies
    for node in nodes {
        policies.push(parent.policy(node));
    }
    // normalise policies
    let sum: f32 = policies.iter().sum();
    policies.iter_mut().for_each(|p| *p /= sum);

    for (i, (node, policy)) in nodes.iter().zip(policies).enumerate() {
        let value = puct(parent_visits, node.wins(), node.visits(), exp_factor, policy);
        if value > best_value {
            best_value = value;
            best_index = i;
        }
    }
    best_index
}
