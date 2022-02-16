use std::{
    fmt::Display,
    ops::{Index, IndexMut},
};

use crate::{
    constants::{MAX_NODEPOOL_SIZE, ROOT_IDX, TREE_PRINT_DEPTH},
    game::{Game, MoveBuffer},
    treenode::Node,
};

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
        assert!(!self.nodes.is_empty(), "SearchTree is empty");
        // SAFETY:
        // - self.nodes is not empty, i.e. index 0 is valid
        // - ROOT_IDX is zero.
        unsafe { self.nodes.get_unchecked(ROOT_IDX) }
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn setup(&mut self, root: G) {
        self.clear();
        self.nodes.push(Node::new(root, None));
    }

    pub fn best_child_of(&self, idx: usize) -> usize {
        let node = &self.nodes[idx];
        assert!(node.has_children(), "Node has no children");
        let max = node.children().max_by(|&a, &b| {
            let a_rate = self.nodes[a].visits();
            let b_rate = self.nodes[b].visits();
            a_rate.cmp(&b_rate)
        });

        // SAFETY:
        // - node.has_children() is true
        // - as there are children, max is not None
        unsafe { max.unwrap_unchecked() }
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
