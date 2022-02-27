use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use crate::{
    constants::{MAX_NODEPOOL_SIZE, ROOT_IDX, TREE_PRINT_DEPTH},
    game::{Game, MoveBuffer},
    treenode::Node,
};

#[derive(Clone)]
pub struct SearchTree<G: Game> {
    pub nodes: Vec<Node<G>>,
    capacity: usize,
    rollouts: u32,
}

impl<G: Game> SearchTree<G> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(MAX_NODEPOOL_SIZE),
            capacity: MAX_NODEPOOL_SIZE,
            rollouts: 0,
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(capacity),
            capacity,
            rollouts: 0,
        }
    }

    pub fn root(&self) -> &Node<G> {
        self.nodes.get(ROOT_IDX).expect("SearchTree is empty")
    }

    pub fn inc_rollouts(&mut self) {
        self.rollouts += 1;
    }

    pub fn rollouts(&self) -> u32 {
        self.rollouts
    }

    pub fn root_rollout_distribution(&self) -> Vec<u32> {
        self
            .root()
            .children()
            .map(|idx| self.nodes[idx].visits())
            .collect::<Vec<_>>()
    }

    pub fn root_distribution(&self) -> Vec<f64> {
        let mut counts = self
            .root()
            .children()
            .map(|idx| f64::from(self.nodes[idx].visits()))
            .collect::<Vec<_>>();
        let total = counts.iter().sum::<f64>();
        counts.iter_mut().for_each(|count| *count /= total);
        counts
    }

    pub fn print_root_distribution(&self) {
        let counts = self.root_distribution();
        if counts.is_empty() {
            println!("No moves yet searched.");
            return;
        }
        let mut buffer = G::Buffer::default();
        self.root().state().generate_moves(&mut buffer);
        assert_eq!(buffer.len(), counts.len());
        print!("[");
        for (&m, &count) in buffer.iter().zip(counts.iter()) {
            print!("{}: {:.0}% ", m, count * 100.0);
        }
        println!("]");
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn setup(&mut self, root: G) {
        self.clear();
        self.nodes.push(Node::new(root, None));
        self.rollouts = 0;
    }

    pub fn best_child_by_visits(&self, idx: usize) -> usize {
        let children = self.nodes[idx].children();
        assert!(children.end <= self.nodes.len());
        // SAFETY: we know that the children are valid indices
        // because children.end <= self.nodes.len()
        children
            .max_by_key(|&i| unsafe { self.nodes.get_unchecked(i).visits() })
            .expect("Node has no children")
    }

    pub fn expand(&mut self, idx: usize) {
        let start = self.nodes.len();
        let node = self.nodes.get_mut(idx).expect("Node does not exist");
        assert!(!node.has_children(), "Node already has children");

        let mut move_buffer = G::Buffer::default();
        let board = *node.state();
        board.generate_moves(&mut move_buffer);
        for m in move_buffer.iter() {
            if self.nodes.len() == self.capacity {
                println!("{}", self);
                panic!("SearchTree full, aborting...");
            }
            let mut child_board = board;
            child_board.push(*m);
            self.nodes.push(Node::new(child_board, Some(idx)));
        }
        // SAFETY: we have already accessed this location in the vector
        // and we do not reduce the size of the vector between the accesses.
        // The only reason that we are re-accessing at all is to satisfy borrowchk.
        let node = unsafe { self.nodes.get_unchecked_mut(idx) };
        node.add_children(start, move_buffer.len());
    }

    fn write_tree(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        depth: usize,
        idx: usize,
    ) -> std::fmt::Result {
        if depth == 0 {
            return Ok(());
        }

        let node = self.nodes.get(idx).expect("Node does not exist");

        if node.visits() == 0 {
            return Ok(());
        }

        for i in 0..(TREE_PRINT_DEPTH - depth) {
            if i == (TREE_PRINT_DEPTH - depth) - 1 {
                write!(f, "├─")?;
            } else {
                write!(f, "| ")?;
            }
        }
        writeln!(
            f,
            "visits: {}, wins: {}, winrate: {:.2}, to_move: {}",
            node.visits(),
            node.wins(),
            node.win_rate(),
            node.to_move()
        )?;
        for child in node.children() {
            self.write_tree(f, depth - 1, child)?;
        }
        Ok(())
    }

    pub fn get(&self, idx: usize) -> Option<&Node<G>> {
        self.nodes.get(idx)
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut Node<G>> {
        self.nodes.get_mut(idx)
    }

    // fn mark_least_visited(&mut self, visit_threshold: u32) {
    //     let mut queue = VecDeque::new();
    //     queue.push_back(ROOT_IDX);
    //     while let Some(idx) = queue.pop_front() {
    //         let node = &mut self.nodes[idx];
    //         if node.visits() < visit_threshold {
    //             node.orphanise();
    //             for child in node.children() {
    //                 queue.push_back(child);
    //             }
    //         }
    //     }
    // }

    // pub fn prune_least_visited(&mut self, visit_threshold: u32) {

    // }
}

impl<G: Game> Display for SearchTree<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_tree(f, TREE_PRINT_DEPTH, ROOT_IDX)
    }
}

impl<G: Game> Index<usize> for SearchTree<G> {
    type Output = Node<G>;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.nodes[idx]
    }
}

impl<G: Game> IndexMut<usize> for SearchTree<G> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.nodes[idx]
    }
}
