#![allow(clippy::cast_precision_loss)]

use rand::Rng;

use std::{
    fmt::Display,
    io::Write,
    str::FromStr,
    sync::{mpsc, Mutex},
    time::{Duration, Instant},
};

use crate::{
    constants::{DEFAULT_EXP_FACTOR, MAX_NODEPOOL_MEM, N_INF, ROOT_IDX},
    game::{Game, MoveBuffer},
    searchtree::SearchTree,
    treenode::Node,
    ucb,
};

/// Determines whether we limit the search by time or by number of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Limit {
    Time(Duration),
    Rollouts(u32),
}

/// The policy to use when selecting moves during rollouts.
/// `Random` will select a random move from the available moves.
/// `Decisive` will try to choose an immediate win (if one exists), otherwise it will select a random move.
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RolloutPolicy {
    Random,
    Decisive,
    RandomQualityScaled,
    DecisiveQualityScaled,
    RandomCutoff { moves: usize },
    DecisiveCutoff { moves: usize },
    MetaAggregated { policy: Box<Self>, rollouts: usize },
}

impl FromStr for RolloutPolicy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "random" => Ok(Self::Random),
            "decisive" => Ok(Self::Decisive),
            "random_quality_scaled" => Ok(Self::RandomQualityScaled),
            "decisive_quality_scaled" => Ok(Self::DecisiveQualityScaled),
            s if s.starts_with("random_cutoff") => {
                let rest = s
                    .split_once('.')
                    .ok_or_else(|| {
                        format!("Invalid rollout policy, no dot separator after random_cutoff: {s}")
                    })?
                    .1;
                let moves = rest.parse::<usize>().map_err(|_| {
                    format!(
                        "Invalid rollout policy, could not parse moves after random_cutoff: {s}"
                    )
                })?;
                Ok(Self::RandomCutoff { moves })
            }
            s if s.starts_with("decisive_cutoff") => {
                let rest = s
                    .split_once('.')
                    .ok_or_else(|| {
                        format!(
                            "Invalid rollout policy, no dot separator after decisive_cutoff: {s}"
                        )
                    })?
                    .1;
                let moves = rest.parse::<usize>().map_err(|_| {
                    format!(
                        "Invalid rollout policy, could not parse moves after decisive_cutoff: {s}"
                    )
                })?;
                Ok(Self::DecisiveCutoff { moves })
            }
            s if s.starts_with("meta_aggregated") => {
                let rest = s
                    .split_once('.')
                    .ok_or_else(|| {
                        format!(
                            "Invalid rollout policy, no dot separator after meta_aggregated: {s}"
                        )
                    })?
                    .1;
                let policy = rest.split_once('.')
                    .ok_or_else(|| format!("Invalid rollout policy, no dot separator after meta_aggregated policy: {s}"))?
                    .0;
                let policy = policy.parse::<Self>().map_err(|_| {
                    format!(
                        "Invalid rollout policy, could not parse policy after meta_aggregated: {s}"
                    )
                })?;
                let rollouts = rest.split_once('.')
                    .ok_or_else(|| format!("Invalid rollout policy, no dot separator after meta_aggregated rollouts: {s}"))?
                    .1;
                let rollouts = rollouts.parse::<usize>()
                    .map_err(|_| format!("Invalid rollout policy, could not parse rollouts after meta_aggregated: {s}"))?;
                Ok(Self::MetaAggregated {
                    policy: Box::new(policy),
                    rollouts,
                })
            }
            _ => Err(format!("Invalid rollout policy: {s}")),
        }
    }
}

/// A struct containing all configuration parameters for the MCTS algorithm.
#[derive(Clone, Debug, PartialEq)]
pub struct Behaviour {
    pub debug: bool,
    pub readout: bool,
    pub log: bool,
    pub limit: Limit,
    pub root_parallelism_count: usize,
    pub rollout_policy: RolloutPolicy,
    pub exp_factor: f64,
    pub training: bool,
}

impl Default for Behaviour {
    fn default() -> Self {
        Self {
            debug: false,
            readout: true,
            log: false,
            limit: Limit::Time(Duration::from_millis(15_000)),
            root_parallelism_count: 1,
            rollout_policy: RolloutPolicy::Random,
            exp_factor: DEFAULT_EXP_FACTOR,
            training: false,
        }
    }
}

impl FromStr for Behaviour {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // debug, readout, rpc, and training are not configurable
        let mut behaviour = Self {
            debug: false,
            readout: false,
            log: false,
            limit: Limit::Rollouts(1),
            root_parallelism_count: 1,
            rollout_policy: RolloutPolicy::Random,
            exp_factor: DEFAULT_EXP_FACTOR,
            training: false,
        };
        // format is "limit=rollouts:50,rollout_policy=random_cutoff.10"
        // or        "limit=time:1000,rollout_policy=meta_aggregated.decisive.10"
        let (limit, rollout_policy) = s
            .split_once(',')
            .ok_or_else(|| format!("Invalid behaviour string, no comma separator: {s}"))?;
        let limit = limit.split_once('=').ok_or_else(|| {
            format!("Invalid behaviour string, no equals separator in limit: {s}")
        })?;
        let rollout_policy = rollout_policy.split_once('=').ok_or_else(|| {
            format!("Invalid behaviour string, no equals separator in rollout_policy: {s}")
        })?;
        if limit.0 != "limit" {
            return Err(format!("Invalid behaviour string, limit not first: {s}"));
        }
        if rollout_policy.0 != "rollout_policy" {
            return Err(format!(
                "Invalid behaviour string, rollout_policy not second: {s}"
            ));
        }
        let limit = limit
            .1
            .split_once(':')
            .ok_or_else(|| format!("Invalid behaviour string, no colon separator in limit: {s}"))?;
        let rollout_policy = rollout_policy.1;
        let limit = match limit.0 {
            "rollouts" => {
                let rollouts = limit.1.parse::<u32>().map_err(|_| {
                    format!("Invalid behaviour string, could not parse rollouts: {s}")
                })?;
                Limit::Rollouts(rollouts)
            }
            "time" => {
                let time = limit
                    .1
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid behaviour string, could not parse time: {s}"))?;
                Limit::Time(Duration::from_millis(time))
            }
            _ => return Err(format!("Invalid behaviour string, invalid limit type: {s}")),
        };
        let rollout_policy = rollout_policy.parse::<RolloutPolicy>().map_err(|err| {
            format!("Invalid behaviour string, could not parse rollout policy: {err}")
        })?;
        behaviour.limit = limit;
        behaviour.rollout_policy = rollout_policy;
        Ok(behaviour)
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

impl Behaviour {
    pub fn for_game<G: Game + MCTSExt>() -> Self {
        Self {
            rollout_policy: G::rollout_policy(),
            ..Self::default()
        }
    }
}

/// A struct containing the results of an MCTS search.
#[derive(Clone, Debug)]
pub struct SearchResults<G: Game> {
    pub rollout_distribution: Vec<u32>,
    pub new_node: G,
    pub new_node_idx: usize,
    pub rollouts: u32,
    pub win_rate: f64,
}

/// Information for the MCTS search, including both static config and particular search state.
#[derive(Clone, Debug)]
pub struct SearchInfo<'a> {
    pub quit: bool,
    pub flags: Behaviour,
    pub side: i8,
    pub start_time: Option<Instant>,
    /// A handle to a receiver for stdin.
    pub stdin_rx: Option<&'a Mutex<mpsc::Receiver<String>>>,
}

impl<'a> SearchInfo<'a> {
    #[allow(dead_code)]
    pub fn new(stdin_rx: &'a Mutex<mpsc::Receiver<String>>) -> Self {
        Self {
            quit: false,
            flags: Behaviour::default(),
            side: 1,
            start_time: None,
            stdin_rx: Some(stdin_rx),
        }
    }
    /// Returns true if the search should be terminated.
    fn limit_reached(&self, rollouts: u32) -> bool {
        match self.flags.limit {
            Limit::Time(max_duration) => {
                let now = Instant::now();
                let elapsed = now
                    .checked_duration_since(self.start_time.unwrap())
                    .unwrap_or_default();
                elapsed >= max_duration
            }
            Limit::Rollouts(max_rollouts) => rollouts >= max_rollouts,
        }
    }
    /// Check if we've run out of time or received a signal from stdin.
    #[allow(dead_code)]
    fn check_up(&self) -> bool {
        if let Some(rx) = self.stdin_rx {
            let rx = rx.lock().unwrap();
            if let Ok(msg) = rx.try_recv() {
                if msg == "stop" {
                    return true;
                }
            }
        }
        self.limit_reached(0)
    }
}

/// The MCTS search engine.
/// Contains both the search tree(s) and the search state.
/// There may be multiple trees if the search is parallelised.
#[derive(Clone)]
#[allow(clippy::upper_case_acronyms)]
pub struct MCTS<'a, G: Game> {
    search_info: SearchInfo<'a>,
    tree: SearchTree<G>,
    rng: fastrand::Rng,
}

pub trait MCTSExt: Game {
    fn rollout_cutoff_length() -> usize {
        100_000
    }
    fn rollout_policy() -> RolloutPolicy {
        RolloutPolicy::Random
    }
}

impl<'a, G: Game + MCTSExt> MCTS<'a, G> {
    const NODEPOOL_SIZE: usize = MAX_NODEPOOL_MEM / std::mem::size_of::<Node<G>>();

    pub fn new(flags: &Behaviour) -> Self {
        Self {
            search_info: SearchInfo {
                quit: false,
                flags: flags.clone(),
                side: 1,
                start_time: None,
                stdin_rx: None,
            },
            tree: SearchTree::with_capacity(Self::NODEPOOL_SIZE),
            rng: fastrand::Rng::new(),
        }
    }

    fn limit_reached(search_info: &SearchInfo, rollouts: u32) -> bool {
        search_info.limit_reached(rollouts)
    }

    pub fn search(&mut self, board: &G) -> SearchResults<G> {
        self.search_info.start_time = Some(Instant::now());

        self.tree.setup(board.clone());
        self.do_treesearch(board);

        let rollout_distribution = self.tree.root_rollout_distribution();

        let avg_win_rate = self.tree.root().win_rate();

        let total_rollouts = self.tree.rollouts();
        if let Limit::Rollouts(x) = self.search_info.flags.limit {
            #[allow(clippy::cast_possible_truncation)]
            let expected_rollouts = x * self.search_info.flags.root_parallelism_count as u32;
            assert_eq!(total_rollouts, expected_rollouts);
        }

        let move_chosen = if self.search_info.flags.training {
            sample_move_index_from_rollouts(&rollout_distribution)
        } else {
            let best = rollout_distribution
                .iter()
                .enumerate()
                .max_by_key(|(_, &count)| count)
                .unwrap()
                .0;
            best
        };

        let root_children = self.tree.root().children();
        let new_node_idx = move_chosen + root_children.start;
        let chosen_move = self.tree[new_node_idx].inbound_edge();
        let mut new_node = board.clone();
        new_node.push(chosen_move);

        SearchResults {
            rollout_distribution,
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
            let p1_wr = (win_rate * 100.0).clamp(0.0, 100.0);
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
            println!("{rollout_distribution:?}");
            println!("{new_node_idx:?}");
        }

        new_node
    }

    fn do_treesearch(&mut self, root: &G) {
        #![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let log_file = std::fs::File::create("log.txt").unwrap();
        let mut log_file = std::io::BufWriter::new(log_file);
        while !Self::limit_reached(&self.search_info, self.tree.rollouts()) {
            if self.search_info.flags.debug && self.tree.rollouts().is_power_of_two() {
                print!("{}", self.tree.show_root_distribution(root).unwrap());
                println!(" rollouts: {}", self.tree.rollouts());
                std::io::stdout().flush().unwrap();
            }
            if self.search_info.flags.readout && self.tree.rollouts().is_power_of_two() {
                assert!(
                    self.tree.average_depth() >= 0.0,
                    "It's impossible to have searched any lines to a negative depth."
                );
                #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
                let avg_depth = self.tree.average_depth().round() as u64;
                let q = if self.tree.root().to_move() == -1 {
                    f64::from(self.tree.root().wins()) / f64::from(self.tree.rollouts())
                } else {
                    1.0 - f64::from(self.tree.root().wins()) / f64::from(self.tree.rollouts())
                };
                print!(
                    "info depth {avg_depth} seldepth {} score wdl {:.3} nodes {} nps {} pv {}\r",
                    self.tree.pv_depth(),
                    q,
                    self.tree.rollouts(),
                    (f64::from(self.tree.rollouts())
                        / self.search_info.start_time.unwrap().elapsed().as_secs_f64())
                        as u64,
                    self.tree.pv_string()
                );
                std::io::stdout().flush().unwrap();
            } else if self.search_info.flags.log && self.tree.rollouts() % 512 == 0 {
                // print policy as an array
                let rdist = self.tree.root_rollout_distribution();
                let sum = rdist.iter().copied().map(u64::from).sum::<u64>();
                let policy = rdist
                    .iter()
                    .copied()
                    .map(u64::from)
                    .map(|x| x as f64 / sum as f64);
                for p in policy {
                    write!(log_file, "{p:.3}, ").unwrap();
                }
                writeln!(log_file).unwrap();
            }
            self.select_expand_simulate_backpropagate(root);
            self.tree.inc_rollouts();
        }
        if self.search_info.flags.readout {
            println!();
        }
    }

    /// The main search loop of the MCTS algorithm.
    ///
    /// This function has four stages:
    /// 1. Select a node to expand, based on the UCT formula.
    /// 2. Expand the selected node.
    /// 3. Simulate the game from the expanded node.
    /// 4. Backpropagate the result of the simulation up the tree.
    fn select_expand_simulate_backpropagate(&mut self, root: &G) {
        // Each time we perform SESB, we have to walk a position down the tree, making moves as we go.
        let mut traversing_state = root.clone();

        let promising_node_idx = Self::select(
            ROOT_IDX,
            &self.tree,
            &self.search_info,
            &mut traversing_state,
        );

        if !traversing_state.is_terminal() {
            self.tree.expand(promising_node_idx, &traversing_state);
        }

        let promising_node = self.tree.get(promising_node_idx).unwrap();
        let node_to_explore = if promising_node.has_children() {
            promising_node.random_child(&self.rng)
        } else {
            promising_node_idx
        };

        let q = self.simulate(node_to_explore, &mut traversing_state);

        Self::backprop(node_to_explore, q, &mut self.tree);
    }

    /// BACKPROPAGATE: Given a node and a Q-value, backpropagate the Q-value up the tree.
    fn backprop(node_idx: usize, q: f32, tree: &mut SearchTree<G>) {
        let mut node = tree.get_mut(node_idx).expect("called backprop on root");
        loop {
            node.update(q);
            if let Some(parent_idx) = node.parent() {
                node = tree.get_mut(parent_idx).unwrap();
            } else {
                break;
            }
        }
    }

    /// SIMULATE: Given a node, simulate the game from that node, and return the resulting Q-value.
    fn simulate(&mut self, node_idx: usize, rollout_board: &mut G) -> f32 {
        use RolloutPolicy::{
            Decisive, DecisiveCutoff, DecisiveQualityScaled, MetaAggregated, Random, RandomCutoff,
            RandomQualityScaled,
        };
        let node = &self.tree[node_idx];

        // test for immediate loss
        let status = rollout_board.evaluate();
        if status == -self.search_info.side {
            let parent_idx = node
                .parent()
                .expect("PANICKING: Immediate loss found in root node.");
            self.tree[parent_idx].set_win_score(N_INF as f32);
            return f32::from(status);
        }

        // playout
        match &self.search_info.flags.rollout_policy {
            Random => self.random_rollout(rollout_board),
            Decisive => self.decisive_rollout(rollout_board),
            RandomQualityScaled => self.random_rollout_qs(rollout_board),
            DecisiveQualityScaled => self.decisive_rollout_qs(rollout_board),
            RandomCutoff { moves } => self.random_rollout_cutoff(rollout_board, *moves),
            DecisiveCutoff { moves } => Self::decisive_rollout_cutoff(rollout_board, *moves),
            MetaAggregated { policy, rollouts } => {
                let rollouts = *rollouts;
                let f = match policy.as_ref() {
                    RolloutPolicy::Random => Self::random_rollout,
                    RolloutPolicy::Decisive => Self::decisive_rollout,
                    RolloutPolicy::RandomQualityScaled => Self::random_rollout_qs,
                    RolloutPolicy::DecisiveQualityScaled => Self::decisive_rollout_qs,
                    RolloutPolicy::RandomCutoff { .. } => {
                        panic!("MetaAggregated policy cannot be RandomCutoff")
                    }
                    RolloutPolicy::DecisiveCutoff { .. } => {
                        panic!("MetaAggregated policy cannot be DecisiveCutoff")
                    }
                    RolloutPolicy::MetaAggregated { .. } => {
                        panic!("MetaAggregated policy must be a RolloutPolicy")
                    }
                };
                let mut sum = 0.0;
                for _ in 0..rollouts {
                    let mut roller = rollout_board.clone();
                    sum += f(self, &mut roller);
                }
                sum / (rollouts as f32)
            }
        }
    }

    /// SELECT: we traverse the on-policy (in-memory) part of the tree, at each node we select the child
    /// with the highest UCB1 value. As we do not store states in the tree, we have to push
    /// moves as we go.
    fn select(
        root_idx: usize,
        tree: &SearchTree<G>,
        search_info: &SearchInfo,
        state: &mut G,
    ) -> usize {
        let mut idx = root_idx;
        let mut node = &tree[idx];
        while node.has_children() {
            let children = node.children();
            idx = ucb::best(
                state,
                &tree.nodes[children.clone()],
                node.visits(),
                search_info.flags.exp_factor,
            ) + children.start;
            node = &tree[idx];
            state.push(node.inbound_edge());
        }
        idx
    }

    /// The random rollout policy.
    /// Simply plays random moves until the game ends,
    /// then returns the result as 1.0 / 0.0 / -1.0.
    fn random_rollout(&mut self, playout_board: &mut G) -> f32 {
        while !playout_board.is_terminal() {
            playout_board.push_random(&mut self.rng);
        }
        f32::from(playout_board.evaluate())
    }

    /// A scaling function that allows for rollout results to be weighted by the quality of the
    /// rollout, where rollouts that end more quickly are considered better, as they should be
    /// more representative of the quality of the position they arose from.
    fn scale(q: f32, moves: f32) -> f32 {
        q * (-0.04 * moves).exp()
    }

    /// A quality-scaled version of [`random_rollout`](Self::random_rollout).
    fn random_rollout_qs(&mut self, playout_board: &mut G) -> f32 {
        let mut moves = 1;
        while !playout_board.is_terminal() {
            playout_board.push_random(&mut self.rng);
            moves += 1;
        }
        let q = f32::from(playout_board.evaluate());
        Self::scale(q, moves as f32)
    }

    /// The decisive rollout policy.
    /// In each position, if there is a move that wins on the spot, we play that move.
    /// Otherwise, we play a random move.
    fn decisive_rollout(&mut self, playout_board: &mut G) -> f32 {
        while !playout_board.is_terminal() {
            let mut buffer = G::Buffer::default();
            playout_board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut board_copy = playout_board.clone();
                board_copy.push(m);
                let evaluation = playout_board.evaluate();
                if evaluation != 0 {
                    return f32::from(evaluation);
                }
            }
            let idx = self.rng.usize(..buffer.len());
            playout_board.push(buffer[idx]);
        }
        f32::from(playout_board.evaluate())
    }

    /// A quality-scaled version of [`decisive_rollout`](Self::decisive_rollout).
    fn decisive_rollout_qs(&mut self, playout_board: &mut G) -> f32 {
        let mut moves = 1;
        while !playout_board.is_terminal() {
            let mut buffer = G::Buffer::default();
            playout_board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut board_copy = playout_board.clone();
                board_copy.push(m);
                let evaluation = playout_board.evaluate();
                if evaluation != 0 {
                    return f32::from(evaluation) / (moves as f32 + 10.0) * 10.0;
                }
            }
            let idx = self.rng.usize(..buffer.len());
            playout_board.push(buffer[idx]);
            moves += 1;
        }
        let q = f32::from(playout_board.evaluate());
        Self::scale(q, moves as f32)
    }

    /// A cutoff version of [`random_rollout`](Self::random_rollout).
    /// This policy will stop rollouts after a fixed number of moves,
    /// returning a Q-value of 0.0.
    fn random_rollout_cutoff(&mut self, playout_board: &mut G, moves: usize) -> f32 {
        let mut counter = 1;
        while !playout_board.is_terminal() {
            if counter > moves {
                return 0.0;
            }
            playout_board.push_random(&mut self.rng);
            counter += 1;
        }
        f32::from(playout_board.evaluate())
    }

    /// A cutoff version of [`decisive_rollout`](Self::decisive_rollout).
    /// This policy will stop rollouts after a fixed number of moves,
    /// returning a Q-value of 0.0.
    fn decisive_rollout_cutoff(playout_board: &mut G, moves: usize) -> f32 {
        let mut counter = 1;
        while !playout_board.is_terminal() {
            if counter > moves {
                return 0.0;
            }
            let mut buffer = G::Buffer::default();
            playout_board.generate_moves(&mut buffer);
            for &m in buffer.iter() {
                let mut board_copy = playout_board.clone();
                board_copy.push(m);
                let evaluation = playout_board.evaluate();
                if evaluation != 0 {
                    return f32::from(evaluation);
                }
            }
            let mut rng = rand::thread_rng();
            let idx = rng.gen_range(0..buffer.len());
            playout_board.push(buffer[idx]);
            counter += 1;
        }
        f32::from(playout_board.evaluate())
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
