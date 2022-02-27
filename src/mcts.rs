use rayon::prelude::*;

use std::{time::{Duration, Instant}, fmt::Display};

use rayon::iter::IntoParallelRefMutIterator;

use crate::{
    constants::{N_INF, ROOT_IDX, MAX_NODEPOOL_SIZE},
    game::{Game, MoveBuffer},
    searchtree::SearchTree,
    uct,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limit {
    Time(Duration),
    Rollouts(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RolloutPolicy {
    Random,
    Decisive
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Behaviour {
    pub debug: bool,
    pub readout: bool,
    pub limit: Limit,
    pub root_parallelism_count: usize,
    pub rollout_policy: RolloutPolicy,
}

impl Default for Behaviour {
    fn default() -> Self {
        use Limit::Rollouts;
        Self {
            debug: true,
            readout: true,
            limit: Rollouts(10_000),
            root_parallelism_count: 1,
            rollout_policy: RolloutPolicy::Random,
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

pub struct SearchResults<G: Game> {
    pub rollout_distribution: Vec<u32>,
    pub new_node: G,
    pub new_node_idx: usize,
    pub rollouts: u32,
    pub win_rate: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchInfo {
    pub flags: Behaviour,
    pub side: i8,
    pub start_time: Option<Instant>,
}

#[derive(Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct MCTS<G: Game> {
    search_info: SearchInfo,
    trees: Vec<SearchTree<G>>,
}

impl<G: Game> MCTS<G> {
    pub fn new(flags: Behaviour) -> Self {
        Self {
            search_info: SearchInfo {
                flags,
                side: 1,
                start_time: None,
            },
            trees: vec![SearchTree::with_capacity(MAX_NODEPOOL_SIZE / flags.root_parallelism_count); flags.root_parallelism_count],
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

    pub fn search(&mut self, board: G) -> SearchResults<G> {
        self.search_info.start_time = Some(Instant::now());

        self.trees.iter_mut().for_each(|tree| tree.setup(board));

        assert_eq!(self.trees.len(), self.search_info.flags.root_parallelism_count);
        self.trees.par_iter_mut().for_each(|tree| {
            Self::do_treesearch(self.search_info, tree);
        });

        let rollout_distributions = self.trees.iter().map(SearchTree::root_rollout_distribution).collect::<Vec<_>>();

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

        let best = fused_distribution
            .iter()
            .enumerate()
            .max_by_key(|(_, &count)| count)
            .unwrap().0;

        assert!(self.trees.len() < 128);
        #[allow(clippy::cast_precision_loss)]
        let len_as_f64 = self.trees.len() as f64;
        let avg_win_rate = self.trees.iter().map(|tree| tree.root().win_rate()).sum::<f64>() / len_as_f64;

        let total_rollouts = self.trees.iter().map(SearchTree::rollouts).sum::<u32>();
        if let Limit::Rollouts(x) = self.search_info.flags.limit {
            #[allow(clippy::cast_possible_truncation)]
            let expected_rollouts = x * self.search_info.flags.root_parallelism_count as u32;
            assert_eq!(total_rollouts, expected_rollouts);
        }

        let first_tree = self.trees.first().unwrap();
        let root_children = first_tree.root().children();
        let new_node_idx = best + root_children.start;
        let new_node = *first_tree[new_node_idx].state();

        SearchResults {
            rollout_distribution: fused_distribution,
            new_node,
            new_node_idx,
            rollouts: total_rollouts,
            win_rate: avg_win_rate,
        }
    }

    pub fn best_next_board(&mut self, board: G) -> G {
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

    fn do_treesearch(search_info: SearchInfo, tree: &mut SearchTree<G>) {
        loop {
            // if self.flags.readout && self.rollouts % 100_000 == 0 {
            //     tree.print_root_distribution();
            // }
            // if self.flags.debug {
            //     println!("looping in SESB, rollouts: {}", self.rollouts);
            // }
            Self::select_expand_simulate_backpropagate(&search_info, tree);
            tree.inc_rollouts();
            if Self::limit_reached(&search_info, tree.rollouts()) {
                break;
            }
        }
    }

    fn select_expand_simulate_backpropagate(search_info: &SearchInfo, tree: &mut SearchTree<G>) {
        let promising_node_idx = Self::select(ROOT_IDX, tree);
        let promising_node = &tree[promising_node_idx];

        if !promising_node.state().is_terminal() {
            tree.expand(promising_node_idx);
        }

        let promising_node = &tree[promising_node_idx]; // makes borrowchk happy
        let node_to_explore = if promising_node.has_children() {
            promising_node.random_child()
        } else {
            promising_node_idx
        };

        let winner = Self::simulate(search_info, node_to_explore, tree);

        Self::backprop(node_to_explore, winner, tree);
    }

    fn backprop(node_idx: usize, winner: i8, tree: &mut SearchTree<G>) {
        let mut node = tree.get_mut(node_idx);
        while let Some(inner) = node {
            inner.update(winner);
            let parent_idx = inner.parent();
            node = parent_idx.and_then(|idx| tree.get_mut(idx));
        }
    }

    fn simulate(search_info: &SearchInfo, node_idx: usize, tree: &mut SearchTree<G>) -> i8 {
        let node = &tree[node_idx];
        let playout_board = *node.state();

        // test for immediate loss
        let status = playout_board.evaluate();
        if status == -search_info.side {
            let parent_idx = node
                .parent()
                .expect("PANICKING: Immediate loss found in root node.");
            tree[parent_idx].set_win_score(N_INF);
            return status;
        }

        // playout
        match search_info.flags.rollout_policy {
            RolloutPolicy::Random => {
                Self::random_rollout(playout_board)
            }
            RolloutPolicy::Decisive => {
                Self::decisive_rollout(playout_board)
            }
        }
    }

    fn select(root_idx: usize, tree: &mut SearchTree<G>) -> usize {
        let mut idx = root_idx;
        let mut node = &tree[idx];
        while node.has_children() {
            let children = node.children();
            idx = uct::best(
                &tree.nodes[children.clone()],
                node.visits(),
                children.start,
            );
            node = &tree[idx];
        }
        idx
    }

    #[inline]
    fn random_rollout(playout_board: G) -> i8 {
        let mut board = playout_board;
        while !board.is_terminal() {
            board.push_random();
        }
        board.evaluate()
    }

    #[inline]
    fn decisive_rollout(playout_board: G) -> i8 {
        let mut board = playout_board;
        while !board.is_terminal() {
            let mut buffer = G::Buffer::default();
            board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut copy = board;
                copy.push(m);
                let evaluation = copy.evaluate();
                if evaluation != 0 {
                    return evaluation;
                }
            }
            board.push_random(); // can be optimised
        }
        board.evaluate()
    }
}
