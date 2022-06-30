#![allow(clippy::cast_precision_loss)]

use std::{
    fmt::{Display, self, Write},
    ops::{Index, IndexMut},
};

use crate::{
    constants::{MAX_NODEPOOL_MEM, ROOT_IDX, TREE_PRINT_DEPTH},
    game::{Game, MoveBuffer},
    treenode::Node,
};

/// The structure of a `SearchTree` is as follows:
/// │            None
/// │              ▲
/// │              │
/// │              │
/// │              │    ┌─────────────────┬──────────────────┐
/// │              │   ▼                 │                  │
/// │         ┌────┼─────────────┬────────┼─────────┬────────┼─────────┬──────────────────┬────
/// │         │    │             │        │         │        │         │                  │
/// │         │  Option<usize>   │  Option<usize>   │  Option<usize>   │  Option<usize>   │
/// │         │      Parent      │      Parent      │      Parent      │      Parent      │
/// │         │                  │                  │                  │                  │
/// │  nodes: ├──────────────────┼──────────────────┼──────────────────┼──────────────────┤ ... array continues this way ==>
/// │         │                  │                  │                  │                  │
/// │         │     Children     │     Children     │     Children     │     Children     │
/// │         │  [usize, usize)  │  [usize, usize)  │  [usize, usize)  │  [usize, usize)  │
/// │         │     │      │     │                  │                  │                  │
/// │         └─────┼──────┼─────┴──────────────────┴──────────────────┴──────────────────┴────
/// │               │      │     ▲       ▲                           ▲        ▲
/// │ Left-hand side│      │     │        │                            │         │
/// │ points to the │      │     └────────┼──────────────┬─────────────┘         │
/// │ first child.  │      │              │   [The range left..right]            │
/// │               └──────┼──────────────┘                                      │
/// │                      │                                                     │
/// │                      └─────────────────────────────────────────────────────┘
/// │                         Right-hand side points to one after the last child.
/// │

#[derive(Clone)]
pub struct SearchTree<G: Game> {
    pub root: Option<G>,
    pub nodes: Vec<Node<G>>,
    capacity: usize,
    rollouts: u32,
}

impl<G: Game> SearchTree<G> {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            root: None,
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

    pub const fn rollouts(&self) -> u32 {
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

    pub fn show_root_distribution(&self, root: &G) -> Result<String, fmt::Error> {
        let mut buf = String::new();
        let counts = self.root_distribution();
        if counts.is_empty() {
            return Ok("No moves yet searched.".to_string())
        }
        let mut buffer = G::Buffer::default();
        root.generate_moves(&mut buffer);
        assert_eq!(buffer.len(), counts.len());
        write!(buf, "[")?;
        for (&m, &count) in buffer.iter().zip(counts.iter()) {
            write!(buf, "{}: {:.0}% ", m, count * 100.0)?;
        }
        write!(buf, "]")?;
        Ok(buf)
    }

    pub fn clear(&mut self) {
        self.nodes.clear();
    }

    pub fn setup(&mut self, root: G) {
        self.clear();
        self.nodes.push(Node::new(root.turn(), None, G::Move::default()));
        self.root = Some(root);
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

    pub fn expand(&mut self, idx: usize, movegen_board: &G) {
        let start = self.nodes.len();
        let node = self.nodes.get_mut(idx).expect("Node does not exist");
        assert!(!node.has_children(), "Node already has children");

        let mut move_buffer = G::Buffer::default();
        movegen_board.generate_moves(&mut move_buffer);
        for m in move_buffer.iter() {
            if self.nodes.len() == self.capacity {
                println!("{}", self);
                panic!("SearchTree full, aborting...");
            }
            self.nodes.push(Node::new(-movegen_board.turn(), Some(idx), *m));
        }
        // SAFETY: we have already accessed this location in the vector
        // and we do not reduce the size of the vector between the accesses.
        // The only reason that we are re-accessing at all is to satisfy borrowchk,
        // as we know that the Vec will not realloc, so holding references is safe.
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

    pub unsafe fn get_unchecked(&self, idx: usize) -> &Node<G> {
        self.nodes.get_unchecked(idx)
    }

    pub unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut Node<G> {
        self.nodes.get_unchecked_mut(idx)
    }

    pub fn pv_string(&self) -> String {
        let mut buf = String::new();
        let mut idx = ROOT_IDX;
        while let Some(node) = self.nodes.get(idx) {
            if !node.has_children() {
                break;
            }
            idx = self.best_child_by_visits(idx);
            write!(buf, "{} ", self.nodes.get(idx).unwrap().inbound_edge()).unwrap();
        }
        buf
    }

    pub fn pv_depth(&self) -> usize {
        let mut depth = 0;
        let mut idx = ROOT_IDX;
        while let Some(node) = self.nodes.get(idx) {
            if !node.has_children() {
                break;
            }
            idx = self.best_child_by_visits(idx);
            depth += 1;
        }
        depth
    }

    fn average_depth_of(&self, node_idx: usize) -> f64 {
        let node = self.nodes.get(node_idx).expect("Node does not exist");
        if node.has_children() {
            node.children().map(|i| self.average_depth_of(i) + 1.0).sum::<f64>() / node.children().len() as f64
        } else {
            0.0
        }
    }

    pub fn average_depth(&self) -> f64 {
        self.average_depth_of(ROOT_IDX)
    }

    pub fn eval(&self) -> f64 {
        let root = self.get(ROOT_IDX).expect("Root node does not exist");
        let q = root.wins();
        let n = root.visits();
        assert_eq!(n, self.rollouts);
        // scale [0, 1] to [-1, 1]
        let zero_to_one = f64::from(q) / f64::from(n);
        zero_to_one.mul_add(2.0, -1.0) * f64::from(-self.nodes.first().unwrap().to_move())
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
