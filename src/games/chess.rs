use std::{fmt::Display, ops::Index};

use crate::{
    game::{Game, MoveBuffer},
    mcts::{self, MCTSExt}, datageneration::{VectoriseState, StateVector},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chess {
    inner: cozy_chess::Board,
}

impl Default for Chess {
    fn default() -> Self {
        Self {
            inner: cozy_chess::Board::startpos(),
        }
    }
}

impl Display for Chess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in [7, 6, 5, 4, 3, 2, 1, 0] {
            write!(f, "{} ", rank + 1)?;
            for file in 0..8 {
                let square = cozy_chess::Square::new(
                    cozy_chess::File::index(file),
                    cozy_chess::Rank::index(rank),
                );
                let Some(piece) = self.inner.piece_on(square) else {
                    write!(f, ". ")?;
                    continue;
                };
                let ch = piece.to_string();
                if self.inner.color_on(square).unwrap() == cozy_chess::Color::White {
                    write!(f, "{} ", ch.to_ascii_uppercase())?;
                } else {
                    write!(f, "{ch} ")?;
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "  a b c d e f g h")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move(cozy_chess::Move);

impl Default for Move {
    fn default() -> Self {
        Self("a1a1".parse().unwrap())
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for Move {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Move {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct _MoveBuffer {
    buf: [Move; 218],
    siz: usize,
}

impl Default for _MoveBuffer {
    fn default() -> Self {
        Self {
            buf: [Move::default(); 218],
            siz: 0,
        }
    }
}

impl Display for _MoveBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let moves = self
            .buf
            .iter()
            .take(self.siz)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        write!(f, "{}", moves.join(" "))
    }
}

impl Index<usize> for _MoveBuffer {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.buf[..self.siz][index]
    }
}

impl MoveBuffer<Move> for _MoveBuffer {
    fn iter(&self) -> std::slice::Iter<Move> {
        self.buf[..self.siz].iter()
    }

    fn len(&self) -> usize {
        self.siz
    }

    fn is_empty(&self) -> bool {
        self.siz == 0
    }

    fn push(&mut self, m: Move) {
        self.buf[self.siz] = m;
        self.siz += 1;
    }

    fn capacity(&self) -> usize {
        self.buf.len()
    }
}

impl Game for Chess {
    type Move = Move;
    type Buffer = _MoveBuffer;

    fn turn(&self) -> i8 {
        if self.inner.side_to_move() == cozy_chess::Color::White {
            1
        } else {
            -1
        }
    }

    fn generate_moves(&self, moves: &mut Self::Buffer) {
        self.inner.generate_moves(|m| {
            m.into_iter().map(Move).for_each(|m| moves.push(m));
            false
        });
    }

    fn is_terminal(&self) -> bool {
        self.inner.status() != cozy_chess::GameStatus::Ongoing
    }

    fn evaluate(&self) -> i8 {
        match self.inner.status() {
            cozy_chess::GameStatus::Drawn | cozy_chess::GameStatus::Ongoing => 0,
            cozy_chess::GameStatus::Won => {
                if self.inner.side_to_move() == cozy_chess::Color::White {
                    -1
                } else {
                    1
                }
            }
        }
    }

    fn push(&mut self, m: Self::Move) {
        self.inner.play(m.0);
    }

    fn push_random(&mut self, rng: &mut fastrand::Rng) {
        let mut moves = Vec::new();
        self.inner.generate_moves(|m| {
            moves.extend(m);
            false
        });
        self.inner.play(moves[rng.usize(..moves.len())]);
    }
}

impl MCTSExt for Chess {
    fn rollout_policy() -> mcts::RolloutPolicy {
        mcts::RolloutPolicy::DecisiveCutoff { moves: 50 }
    }
}

impl Chess {
    pub const fn from_raw_board(board: cozy_chess::Board) -> Self {
        Self { inner: board }
    }
}

impl VectoriseState for Chess {
    fn csv_header() -> String {
        String::new()
    }

    fn vectorise_state(&self) -> crate::datageneration::StateVector {
        StateVector { data: Vec::new() }
    }

    fn index_move(_m: Self::Move) -> usize {
        0
    }

    fn action_space() -> usize {
        1
    }

    fn state_vector_dimensions() -> Vec<usize> {
        vec![1]
    }
}