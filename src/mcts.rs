#![allow(clippy::cast_precision_loss)]

use rand::Rng;
use rayon::prelude::*;

use std::{
    fmt::Display,
    io::Write,
    time::{Duration, Instant},
};

use crate::{
    constants::{MAX_NODEPOOL_MEM, N_INF, ROOT_IDX},
    game::{Game, MoveBuffer},
    searchtree::SearchTree,
    treenode::Node,
    uct,
};

/// Determines whether we limit the search by time or by number of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Time(Duration),
    Rollouts(u32),
}

/// The policy to use when selecting moves during rollouts.
/// `Random` will select a random move from the available moves.
/// `Decisive` will try to choose an immediate win (if one exists), otherwise it will select a random move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RolloutPolicy {
    Random,
    Decisive,
    RandomQualityScaled,
    DecisiveQualityScaled,
    RandomCutoff,
}

/// A struct containing all configuration parameters for the MCTS algorithm.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Behaviour {
    pub debug: bool,
    pub readout: bool,
    pub limit: Limit,
    pub root_parallelism_count: usize,
    pub rollout_policy: RolloutPolicy,
    pub exp_factor: f64,
    pub training: bool,
}

impl Default for Behaviour {
    fn default() -> Self {
        use Limit::Rollouts;
        Self {
            debug: true,
            readout: true,
            limit: Rollouts(100_000),
            root_parallelism_count: 1,
            rollout_policy: RolloutPolicy::Random,
            exp_factor: 1.0,
            training: false,
        }
    }
}

impl Display for Behaviour {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "Behaviour {{ debug: {}, readout: {}, limit: {:?}, root_parallelism_count: {}, rollout_policy: {:?} }}",
            self.debug,
            self.readout,
            self.limit,
            self.root_parallelism_count,
            self.rollout_policy
        )
    }
}

/// A struct containing the results of an MCTS search.
pub struct SearchResults<G: Game> {
    pub rollout_distribution: Vec<u32>,
    pub new_node: G,
    pub new_node_idx: usize,
    pub rollouts: u32,
    pub win_rate: f64,
}

/// Information for the MCTS search, including both static config and particular search state.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SearchInfo {
    pub flags: Behaviour,
    pub side: i8,
    pub start_time: Option<Instant>,
}

/// The MCTS search engine.
/// Contains both the search tree(s) and the search state.
/// There may be multiple trees if the search is parallelised.
#[derive(Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct MCTS<G: Game> {
    search_info: SearchInfo,
    trees: Vec<SearchTree<G>>,
}

impl<G: Game> MCTS<G> {
    const NODEPOOL_SIZE: usize = MAX_NODEPOOL_MEM / std::mem::size_of::<Node<G>>();

    pub fn new(flags: Behaviour) -> Self {
        Self {
            search_info: SearchInfo {
                flags,
                side: 1,
                start_time: None,
            },
            trees: (0..flags.root_parallelism_count)
                .map(|_| {
                    SearchTree::with_capacity(Self::NODEPOOL_SIZE / flags.root_parallelism_count)
                })
                .collect(),
        }
    }

    pub fn get_trees(&self) -> &[SearchTree<G>] {
        &self.trees
    }

    fn limit_reached(search_info: &SearchInfo, rollouts: u32) -> bool {
        match search_info.flags.limit {
            Limit::Time(max_duration) => {
                let now = Instant::now();
                let elapsed = now.duration_since(search_info.start_time.unwrap());
                elapsed >= max_duration
            }
            Limit::Rollouts(max_rollouts) => rollouts >= max_rollouts,
        }
    }

    pub fn search(&mut self, board: &G) -> SearchResults<G> {
        self.search_info.start_time = Some(Instant::now());

        self.trees
            .iter_mut()
            .for_each(|tree| tree.setup(board.clone()));

        assert_eq!(
            self.trees.len(),
            self.search_info.flags.root_parallelism_count
        );
        self.trees
            .par_iter_mut()
            .enumerate()
            .for_each(|(id, tree)| {
                Self::do_treesearch(id, self.search_info, tree);
            });

        let rollout_distributions = self
            .trees
            .iter()
            .map(SearchTree::root_rollout_distribution)
            .collect::<Vec<_>>();

        if self.search_info.flags.debug {
            for dist in &rollout_distributions {
                println!("{:?}", dist);
            }
        }

        let mut fused_distribution = vec![0; rollout_distributions[0].len()];
        for dist in &rollout_distributions {
            for (i, &count) in dist.iter().enumerate() {
                fused_distribution[i] += count;
            }
        }

        assert!(self.trees.len() < 128);
        #[allow(clippy::cast_precision_loss)]
        let len_as_f64 = self.trees.len() as f64;
        let avg_win_rate = self
            .trees
            .iter()
            .map(|tree| tree.root().win_rate())
            .sum::<f64>()
            / len_as_f64;

        let total_rollouts = self.trees.iter().map(SearchTree::rollouts).sum::<u32>();
        if let Limit::Rollouts(x) = self.search_info.flags.limit {
            #[allow(clippy::cast_possible_truncation)]
            let expected_rollouts = x * self.search_info.flags.root_parallelism_count as u32;
            assert_eq!(total_rollouts, expected_rollouts);
        }

        let move_chosen = if self.search_info.flags.training {
            sample_move_index_from_rollouts(&fused_distribution)
        } else {
            let best = fused_distribution
                .iter()
                .enumerate()
                .max_by_key(|(_, &count)| count)
                .unwrap()
                .0;
            best
        };

        let first_tree = self.trees.first().unwrap();
        let root_children = first_tree.root().children();
        let new_node_idx = move_chosen + root_children.start;
        let new_node = first_tree[new_node_idx].state().clone();

        SearchResults {
            rollout_distribution: fused_distribution,
            new_node,
            new_node_idx,
            rollouts: total_rollouts,
            win_rate: avg_win_rate,
        }
    }

    pub fn best_next_board(&mut self, board: &G) -> G {
        let SearchResults {
            rollout_distribution,
            new_node,
            new_node_idx,
            rollouts,
            win_rate,
        } = self.search(board);

        if self.search_info.flags.readout {
            println!(
                "{} nodes processed in {}ms at {:.2} nodes per second.",
                rollouts,
                self.search_info.start_time.unwrap().elapsed().as_millis(),
                f64::from(rollouts) / self.search_info.start_time.unwrap().elapsed().as_secs_f64()
            );
            let p1_wr = (win_rate * 100.0).max(0.0).min(100.0);
            println!(
                "predicted outcome: {:.2}% chance of win.",
                if self.search_info.side == -1 {
                    p1_wr
                } else {
                    100.0 - p1_wr
                }
            );
        }
        if self.search_info.flags.debug {
            println!("{:?}", rollout_distribution);
            println!("{:?}", new_node_idx);
        }

        new_node
    }

    fn do_treesearch(id: usize, search_info: SearchInfo, tree: &mut SearchTree<G>) {
        while !Self::limit_reached(&search_info, tree.rollouts()) {
            if search_info.flags.debug && tree.rollouts() % 1_000 == 0 {
                print!("Search from tree {id}: ");
                print!("{}", tree.show_root_distribution().unwrap());
                println!(" rollouts: {}", tree.rollouts());
                std::io::stdout().flush().unwrap();
            }
            if search_info.flags.readout && tree.rollouts() % 1_000 == 0 {
                assert!(
                    tree.average_depth() >= 0.0,
                    "It's impossible to have searched any lines to a negative depth."
                );
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let avg_depth = tree.average_depth().round() as u64;
                println!(
                    "q: {:.3} eval: {:.3} depth: {}/{} pv: {}",
                    f64::from(tree.root().wins()) / f64::from(tree.rollouts()),
                    tree.eval(),
                    avg_depth,
                    tree.pv_depth(),
                    tree.pv_string()
                );
                std::io::stdout().flush().unwrap();
            }
            Self::select_expand_simulate_backpropagate(&search_info, tree);
            tree.inc_rollouts();
        }
    }

    fn select_expand_simulate_backpropagate(search_info: &SearchInfo, tree: &mut SearchTree<G>) {
        let promising_node_idx = Self::select(ROOT_IDX, tree, search_info);
        let promising_node = &tree[promising_node_idx];

        if !promising_node.state().is_terminal() {
            tree.expand(promising_node_idx);
        }

        let promising_node = unsafe { tree.get_unchecked(promising_node_idx) }; // makes borrowchk happy
        let node_to_explore = if promising_node.has_children() {
            promising_node.random_child()
        } else {
            promising_node_idx
        };

        let q = Self::simulate(search_info, node_to_explore, tree);

        Self::backprop(node_to_explore, q, tree);
    }

    fn backprop(node_idx: usize, q: f32, tree: &mut SearchTree<G>) {
        let mut node = tree.get_mut(node_idx).expect("called backprop on root");
        loop {
            node.update(q);
            if let Some(parent_idx) = node.parent() {
                node = &mut tree[parent_idx]; // this could be get_unchecked, but it feels dangerous
            } else {
                break;
            }
        }
    }

    fn simulate(search_info: &SearchInfo, node_idx: usize, tree: &mut SearchTree<G>) -> f32 {
        use RolloutPolicy::{
            Decisive, DecisiveQualityScaled, Random, RandomCutoff, RandomQualityScaled,
        };
        let node = &tree[node_idx];
        let playout_board = node.state().clone();

        // test for immediate loss
        let status = playout_board.evaluate();
        if status == -search_info.side {
            let parent_idx = node
                .parent()
                .expect("PANICKING: Immediate loss found in root node.");
            tree[parent_idx].set_win_score(N_INF as f32);
            return f32::from(status);
        }

        // playout
        match search_info.flags.rollout_policy {
            Random => Self::random_rollout(playout_board),
            Decisive => Self::decisive_rollout(playout_board),
            RandomQualityScaled => Self::random_rollout_qs(playout_board),
            DecisiveQualityScaled => Self::decisive_rollout_qs(playout_board),
            RandomCutoff => Self::random_rollout_cutoff(playout_board),
        }
    }

    fn select(root_idx: usize, tree: &mut SearchTree<G>, search_info: &SearchInfo) -> usize {
        let mut idx = root_idx;
        let mut node = &tree[idx];
        while node.has_children() {
            let children = node.children();
            idx = uct::best(
                &tree.nodes[children.clone()],
                node.visits(),
                search_info.flags.exp_factor,
            ) + children.start;
            node = &tree[idx];
        }
        idx
    }

    #[inline]
    fn random_rollout(playout_board: G) -> f32 {
        let mut board = playout_board;
        while !board.is_terminal() {
            board.push_random();
        }
        f32::from(board.evaluate())
    }

    #[inline]
    fn scale(q: f32, moves: f32) -> f32 {
        q * (-0.04 * moves).exp()
    }

    #[inline]
    fn random_rollout_qs(playout_board: G) -> f32 {
        let mut board = playout_board;
        let mut moves = 1;
        while !board.is_terminal() {
            board.push_random();
            moves += 1;
        }
        let q = f32::from(board.evaluate());
        Self::scale(q, moves as f32)
    }

    #[inline]
    fn decisive_rollout(playout_board: G) -> f32 {
        let mut board = playout_board;
        while !board.is_terminal() {
            let mut buffer = G::Buffer::default();
            board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut copy = board.clone();
                copy.push(m);
                let evaluation = copy.evaluate();
                if evaluation != 0 {
                    return f32::from(evaluation);
                }
            }
            board.push_random(); // can be optimised
        }
        f32::from(board.evaluate())
    }

    #[inline]
    fn decisive_rollout_qs(playout_board: G) -> f32 {
        let mut board = playout_board;
        let mut moves = 1;
        while !board.is_terminal() {
            let mut buffer = G::Buffer::default();
            board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut copy = board.clone();
                copy.push(m);
                let evaluation = copy.evaluate();
                if evaluation != 0 {
                    return f32::from(evaluation) / (moves as f32 + 10.0) * 10.0;
                }
            }
            board.push_random(); // can be optimised
            moves += 1;
        }
        let q = f32::from(board.evaluate());
        Self::scale(q, moves as f32)
    }

    #[inline]
    fn random_rollout_cutoff(playout_board: G) -> f32 {
        const MAX_ROLLOUT_LENGTH: usize = 20;
        let mut board = playout_board;
        let mut moves = 1;
        while !board.is_terminal() {
            board.push_random();
            if moves > MAX_ROLLOUT_LENGTH {
                return 0.0;
            }
            moves += 1;
        }
        f32::from(board.evaluate())
    }
}

fn sample_move_index_from_rollouts(fused_distribution: &[u32]) -> usize {
    let total_rollouts = fused_distribution.iter().sum::<u32>();
    let prob_vector = fused_distribution
        .iter()
        .map(|&count| {
            let raw_policy = f64::from(count) / f64::from(total_rollouts);
            // adjust to flatten the distribution
            #[allow(clippy::cast_precision_loss)]
            let uniform_val = 1.0 / fused_distribution.len() as f64;
            raw_policy.mul_add(0.7, uniform_val * 0.3)
        })
        .collect::<Vec<_>>();
    let rfloat = rand::thread_rng().gen_range(0.0..1.0);
    let mut cumulative_probability = 0.0;
    let mut choice = 0;
    for (i, &prob) in prob_vector.iter().enumerate() {
        cumulative_probability += prob;
        if rfloat <= cumulative_probability {
            choice = i;
            break;
        }
    }
    choice
}
