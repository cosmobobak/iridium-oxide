use std::{
    fmt::{Debug, Display},
    ops::Index,
};

pub trait MoveBuffer<Move>: Debug + Default + Index<usize, Output = Move> + Display {
    fn iter(&self) -> std::slice::Iter<Move>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool;
    fn push(&mut self, m: Move);
}

pub trait Game: Copy + Eq + Debug + Display + Default {
    type Move: Copy + Eq + Ord + Debug + Display;
    type Buffer: MoveBuffer<Self::Move>;

    fn turn(&self) -> i8;
    fn generate_moves(&self, moves: &mut Self::Buffer);
    fn is_terminal(&self) -> bool;
    fn evaluate(&self) -> i8;
    fn push(&mut self, m: Self::Move);
    fn pop(&mut self, m: Self::Move);
    fn push_random(&mut self);

    fn outcome(&self) -> Option<String> {
        if self.is_terminal() {
            match self.evaluate() {
                1 => Some("1-0".to_string()),
                -1 => Some("0-1".to_string()),
                0 => Some("1/2-1/2".to_string()),
                _ => unreachable!(),
            }
        } else {
            None
        }
    }
}
