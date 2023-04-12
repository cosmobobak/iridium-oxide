use std::{fmt::Display, ops::Index};

use crate::{game::{Game, MoveBuffer}, mcts::{MCTSExt, self}};

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
                let square = cozy_chess::Square::new(cozy_chess::File::index(file), cozy_chess::Rank::index(rank));
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
pub struct ChessMove(cozy_chess::Move);

impl Default for ChessMove {
    fn default() -> Self {
        Self("a1a1".parse().unwrap())
    }
}

impl Display for ChessMove {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl PartialOrd for ChessMove {
    fn partial_cmp(&self, _other: &Self) -> Option<std::cmp::Ordering> {
        Some(std::cmp::Ordering::Equal)
    }
}

impl Ord for ChessMove {
    fn cmp(&self, _other: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChessMoveBuffer{
    buf: [ChessMove; 218],
    siz: usize,
}

impl Default for ChessMoveBuffer {
    fn default() -> Self {
        Self {
            buf: [ChessMove::default(); 218],
            siz: 0,
        }
    }
}

impl Display for ChessMoveBuffer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let moves = self.buf.iter().take(self.siz).map(ToString::to_string).collect::<Vec<_>>();
        write!(f, "{}", moves.join(" "))
    }
}

impl Index<usize> for ChessMoveBuffer {
    type Output = ChessMove;

    fn index(&self, index: usize) -> &Self::Output {
        &self.buf[..self.siz][index]
    }
}

impl MoveBuffer<ChessMove> for ChessMoveBuffer {
    fn iter(&self) -> std::slice::Iter<ChessMove> {
        self.buf[..self.siz].iter()
    }

    fn len(&self) -> usize {
        self.siz
    }

    fn is_empty(&self) -> bool {
        self.siz == 0
    }

    fn push(&mut self, m: ChessMove) {
        self.buf[self.siz] = m;
        self.siz += 1;
    }

    fn capacity(&self) -> usize {
        self.buf.len()
    }
}

impl Game for Chess {
    type Move = ChessMove;
    type Buffer = ChessMoveBuffer;

    fn turn(&self) -> i8 {
        if self.inner.side_to_move() == cozy_chess::Color::White {
            1
        } else {
            -1
        }
    }

    fn generate_moves(&self, moves: &mut Self::Buffer) {
        self.inner.generate_moves(|m| {
            m.into_iter().map(ChessMove).for_each(|m| moves.push(m));
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