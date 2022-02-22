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

pub trait Vectorisable: Game {
    fn vectorise_state(&self) -> Vec<bool>;
    fn index_move(&self, m: Self::Move) -> usize;
    fn action_space() -> usize;
    fn state_vector_dimensions() -> Vec<usize>;

    fn policy_vector(&self, policy: &[u32]) -> Vec<f64> {
        let mut out = vec![0.0; Self::action_space()];
        let mut buf = Self::Buffer::default();
        self.generate_moves(&mut buf);
        assert_eq!(policy.len(), buf.len());
        let rollout_sum = f64::from(policy.iter().sum::<u32>());
        for (i, &m) in buf.iter().enumerate() {
            let index = self.index_move(m);
            out[index] = f64::from(policy[i]) / rollout_sum;
        }
        out
    }
}
