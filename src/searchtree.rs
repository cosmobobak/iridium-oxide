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
}

impl<G: Game> SearchTree<G> {
    pub fn new() -> Self {
        Self {
            nodes: Vec::with_capacity(MAX_NODEPOOL_SIZE),
        }
    }

    pub fn root(&self) -> &Node<G> {
        self.nodes.get(ROOT_IDX).expect("SearchTree is empty")
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
    }

    pub fn best_child_of(&self, idx: usize) -> usize {
        self.nodes[idx]
            .children()
            .max_by_key(|&i| self.nodes[i].visits())
            .expect("Node has no children")
    }

    pub fn expand(&mut self, idx: usize) {
        let start = self.nodes.len();
        let node = &mut self.nodes[idx];
        assert!(!node.has_children(), "Node already has children");
        let mut moves = G::Buffer::default();
        let board = *node.state();
        board.generate_moves(&mut moves);
        for m in moves.iter() {
            if self.nodes.len() == MAX_NODEPOOL_SIZE {
                println!("{}", self);
                panic!("SearchTree full, aborting...");
            }
            let mut child_board = board;
            child_board.push(*m);
            self.nodes.push(Node::new(child_board, Some(idx)));
        }
        let node = &mut self.nodes[idx]; // makes borrowchk happy
        node.add_children(start, moves.len());
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
        if self.nodes[idx].visits() == 0 {
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
            self.nodes[idx].visits(),
            self.nodes[idx].wins(),
            self.nodes[idx].win_rate(),
            self.nodes[idx].to_move()
        )?;
        for child in self.nodes[idx].children() {
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
