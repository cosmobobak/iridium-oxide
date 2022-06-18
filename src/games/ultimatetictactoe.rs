
use std::{fmt::{Display, Formatter, self}, ops::Index};

use crate::game::{Game, MoveBuffer};

const BOARD_WIDTH: usize = 9;
const BOARD_HEIGHT: usize = 9;
const SUB_BOARD_WIDTH: usize = 3;
const SUB_BOARD_HEIGHT: usize = 3;

const NO_PIECE: u8 = 0;
const X: u8 = 1;
const O: u8 = 2;
const NO_FORCED_BOX: u8 = 10;
const NO_MOVE: Move = Move(81);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Undo {
    forced_box: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Move(u8);

impl Display for Move {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        #![allow(clippy::cast_possible_truncation)]
        if self == &NO_MOVE { return write!(f, "NO_MOVE"); }
        let row = self.0 / BOARD_WIDTH as u8;
        let col = self.0 % BOARD_WIDTH as u8;
        write!(f, "{}{}", (b'A' + row as u8) as char, col + 1)
    }
}

impl Default for Move {
    fn default() -> Self {
        NO_MOVE
    }
}

/// Representation of a single game state in Ultimate Tic-Tac-Toe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct UltimateTicTacToe {
    pieces: [u8; BOARD_WIDTH * BOARD_HEIGHT],
    moves: u8,
    forced_box: u8,
    history: Vec<Undo>,
}

impl UltimateTicTacToe {
    pub const fn new() -> Self {
        Self { 
            pieces: [NO_PIECE; BOARD_WIDTH * BOARD_HEIGHT],
            moves: 0,
            forced_box: NO_FORCED_BOX,
            history: Vec::new(),
        }
    }
}

impl Default for UltimateTicTacToe {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for UltimateTicTacToe {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for y in 0..BOARD_HEIGHT {
            for x in 0..BOARD_WIDTH {
                let piece = self.pieces[y * BOARD_WIDTH + x];
                let piece_char = match piece {
                    NO_PIECE => '.',
                    X => 'X',
                    O => 'O',
                    _ => unreachable!(),
                };
                write!(f, "{} ", piece_char)?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Buffer {
    moves: [Move; 81],
    n_moves: usize,
}

impl Display for Buffer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[")?;
        for m in &self.moves[..self.moves.len() - 1] {
            write!(f, "{}, ", m)?;
        }
        write!(f, "{}]", self.moves[self.moves.len() - 1])
    }
}

impl Index<usize> for Buffer {
    type Output = Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[..self.n_moves][index]
    }
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            moves: [NO_MOVE; 81],
            n_moves: 0,
        }
    }
}

impl MoveBuffer<Move> for Buffer {
    fn iter(&self) -> std::slice::Iter<Move> {
        self.moves[..self.n_moves].iter()
    }

    fn len(&self) -> usize {
        self.n_moves
    }

    fn is_empty(&self) -> bool {
        self.n_moves == 0
    }

    fn push(&mut self, m: Move) {
        self.moves[self.n_moves] = m;
        self.n_moves += 1;
    }

    fn capacity(&self) -> usize {
        self.moves.len()
    }
}

impl Game for UltimateTicTacToe {
    type Move = Move;

    type Buffer = Buffer;

    fn turn(&self) -> i8 {
        todo!()
    }

    fn generate_moves(&self, moves: &mut Self::Buffer) {
        todo!()
    }

    fn is_terminal(&self) -> bool {
        todo!()
    }

    fn evaluate(&self) -> i8 {
        todo!()
    }

    fn push(&mut self, m: Self::Move) {
        todo!()
    }

    fn pop(&mut self, m: Self::Move) {
        todo!()
    }

    fn push_random(&mut self) {
        todo!()
    }
}