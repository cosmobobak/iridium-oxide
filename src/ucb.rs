use crate::{constants::NODE_UNVISITED_VALUE, game::Game, treenode::Node};

fn puct(parent_visits: u32, q_value: f32, visits: u32, _exp_factor: f64, policy: f64) -> f64 {
    if visits == 0 {
        return NODE_UNVISITED_VALUE;
    }

    // let exploitation = f64::from(q_value) / f64::from(visits);
    // let exploration = f64::sqrt(f64::ln(f64::from(parent_visits)) / f64::from(visits));
    // exp_factor.mul_add(exploration, exploitation)
    // // exploitation + exp_factor * exploration
    // ^^^ the normal UCB1 formula

    // pb_c = math.log((parent.visit_count + config.pb_c_base + 1) / config.pb_c_base) + config.pb_c_init
    let pb_c = f64::ln((f64::from(parent_visits) + 1.8 + 1.0) / 1.8);
    // pb_c *= math.sqrt(parent.visit_count) / (child.visit_count + 1)
    let pb_c = pb_c * f64::sqrt(f64::from(parent_visits)) / (f64::from(visits) + 1.0);

    // prior_score = pb_c * child.prior
    // value_score = child.value()
    // return prior_score + value_score

    let prior_score = pb_c * policy;
    let value_score = f64::from(q_value) / f64::from(visits);

    prior_score + value_score
}

pub fn best<G: Game>(parent: &G, nodes: &[Node<G>], parent_visits: u32, exp_factor: f64) -> usize {
    assert!(!nodes.is_empty(), "ucb::best: nodes is empty");
    let mut best_value = f64::NEG_INFINITY;
    let mut best_index = 0;
    let mut policies = Vec::with_capacity(nodes.len());

    // compute policies
    for node in nodes {
        policies.push(parent.policy(node));
    }
    // normalise policies
    let sum: f64 = policies.iter().sum();
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
