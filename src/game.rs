use std::{
    fmt::{Debug, Display},
    ops::Index,
};

use crate::treenode::Node;

pub trait MoveBuffer<Move>: Debug + Default + Index<usize, Output = Move> + Display {
    fn iter(&self) -> std::slice::Iter<Move>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn push(&mut self, m: Move);
    fn capacity(&self) -> usize;
}

pub trait Game: Clone + Eq + Debug + Display + Default + Send + Sync {
    type Move: Copy + Eq + Ord + Debug + Display + Default + Send + Sync;
    type Buffer: MoveBuffer<Self::Move>;

    fn turn(&self) -> i8;
    fn generate_moves(&self, moves: &mut Self::Buffer);
    fn is_terminal(&self) -> bool;
    fn evaluate(&self) -> i8;
    fn push(&mut self, m: Self::Move);
    fn push_random(&mut self, rng: &mut fastrand::Rng);

    fn outcome(&self) -> Option<&str> {
        if self.is_terminal() {
            match self.evaluate() {
                1 => Some("1-0"),
                -1 => Some("0-1"),
                0 => Some("1/2-1/2"),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }

    fn sort_moves(&mut self, _moves: &mut Self::Buffer) {
        // intentionally does nothing.
    }

    fn generate_proximates(&self, _moves: &mut Self::Buffer) {
        // intentionally does nothing.
    }

    fn policy(&self, _node: &Node<Self>) -> f64 {
        // default policy is uniform random.
        1.0
    }
}
