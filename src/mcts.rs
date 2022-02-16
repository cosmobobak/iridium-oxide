use std::time::{Duration, Instant};

use crate::{
    constants::{N_INF, ROOT_IDX},
    game::Game,
    searchtree::SearchTree,
    uct,
};

pub enum Limit {
    Time(Duration),
    Rollouts(u32),
}

pub struct Behaviour {
    pub debug: bool,
    pub readout: bool,
    pub limit: Limit,
}

impl Default for Behaviour {
    fn default() -> Self {
        use Limit::Rollouts;
        Self {
            debug: true,
            readout: true,
            limit: Rollouts(10_000),
        }
    }
}

pub struct MonteCarloTreeSearcher<G: Game> {
    flags: Behaviour,
    side: i8,
    rollouts: u32,
    start_time: Option<std::time::Instant>,
    tree: SearchTree<G>,
}

impl<G: Game> MonteCarloTreeSearcher<G> {
    pub fn new(flags: Behaviour) -> Self {
        Self {
            flags,
            side: 1,
            rollouts: 0,
            start_time: None,
            tree: SearchTree::new(),
        }
    }

    fn rollouts(&self) -> u32 {
        self.rollouts
    }

    pub fn get_tree(&self) -> &SearchTree<G> {
        &self.tree
    }

    #[inline]
    fn limit_reached(&self) -> bool {
        match self.flags.limit {
            Limit::Time(duration) => {
                let now = std::time::Instant::now();
                let elapsed = now.duration_since(self.start_time.unwrap());
                elapsed >= duration
            }
            Limit::Rollouts(rollouts) => self.rollouts >= rollouts,
        }
    }

    pub fn best_next_board(&mut self, board: G) -> G {
        self.tree.setup(board);

        self.rollouts = 0;
        self.start_time = Some(Instant::now());

        loop {
            if self.flags.debug {
                println!("looping in SESB, rollouts: {}", self.rollouts);
            }
            self.select_expand_simulate_backpropagate();
            self.rollouts += 1;
            if self.limit_reached() {
                break;
            }
        }

        let root = self.tree.root();
        let children = root.children();
        let best_child = uct::best(
            &self.tree.nodes[children.clone()],
            root.visits(),
            children.start,
        );

        if self.flags.readout {
            println!(
                "{} nodes processed in {}ms at {:.2} nodes per second.",
                self.rollouts,
                self.start_time.unwrap().elapsed().as_millis(),
                f64::from(self.rollouts) / self.start_time.unwrap().elapsed().as_secs_f64()
            );
            let p1_wr = (root.win_rate() * 100.0).max(0.0).min(100.0);
            println!(
                "predicted outcome: {:.2}% chance of win.",
                if self.side == -1 {
                    p1_wr
                } else {
                    100.0 - p1_wr
                }
            );
        }

        *self.tree[best_child].state()
    }

    fn select_expand_simulate_backpropagate(&mut self) {
        if self.flags.debug {
            println!("Selecting!");
        }
        let promising_node_idx = self.select(ROOT_IDX);
        let promising_node = &self.tree[promising_node_idx];

        if self.flags.debug {
            println!("Expanding!");
        }
        if !promising_node.state().is_terminal() {
            self.tree.expand(promising_node_idx);
        }

        let promising_node = &self.tree[promising_node_idx]; // makes borrowchk happy
        let node_to_explore = if promising_node.has_children() {
            promising_node.random_child()
        } else {
            promising_node_idx
        };

        if self.flags.debug {
            println!("Simulating!");
        }
        let winner = self.simulate(node_to_explore);

        if self.flags.debug {
            println!("Backpropagating!");
        }
        self.backprop(node_to_explore, winner);
    }

    fn backprop(&mut self, node_idx: usize, winner: i8) {
        let mut node = self.tree.get_mut(node_idx);
        while node.is_some() {
            // SAFETY: node is some
            let inner_node = unsafe { node.unwrap_unchecked() };
            inner_node.update(winner);
            let parent_idx = inner_node.parent();
            node = parent_idx.and_then(|idx| self.tree.get_mut(idx));
        }
    }

    fn simulate(&mut self, node_idx: usize) -> i8 {
        let node = &self.tree[node_idx];
        let playout_board = *node.state();

        // test for immediate loss
        let status = playout_board.evaluate();
        if status == -self.side {
            let parent_idx = node
                .parent()
                .expect("PANICKING: Immediate loss found in root node.");
            self.tree[parent_idx].set_win_score(N_INF);
            return status;
        }

        // playout
        let mut board = playout_board;
        while !board.is_terminal() {
            board.push_random();
        }

        board.evaluate()
    }

    fn select(&mut self, root_idx: usize) -> usize {
        let mut idx = root_idx;
        let mut node = &self.tree[idx];
        while node.has_children() {
            let children = node.children();
            idx = uct::best(
                &self.tree.nodes[children.clone()],
                node.visits(),
                children.start,
            );
            node = &self.tree[idx];
        }
        idx
    }
}
