#![allow(clippy::unusual_byte_groupings)]

use std::{
    fmt::{Debug, Display},
    ops::Index,
};

use rand::Rng;

use crate::{
    datageneration::{StateVector, VectoriseState},
    game::{Game, MoveBuffer},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TicTacToe {
    board: [u16; 2],
    moves: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TicTacToeMove(usize);

impl TicTacToeMove {
    pub const fn new(idx: usize) -> Self {
        Self(idx)
    }
}

impl Default for TicTacToeMove {
    fn default() -> Self {
        Self(9)
    }
}

impl Debug for TicTacToeMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "idx: {}", self.0)
    }
}

impl Display for TicTacToeMove {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0 + 1)
    }
}

#[derive(Debug, Clone)]
pub struct TTTMoveBuf {
    data: [TicTacToeMove; 9],
    n_moves: usize,
}

impl Default for TTTMoveBuf {
    fn default() -> Self {
        Self {
            data: [TicTacToeMove(0); 9],
            n_moves: 0,
        }
    }
}

impl Index<usize> for TTTMoveBuf {
    type Output = TicTacToeMove;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl Display for TTTMoveBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for m in &self.data[..self.n_moves - 1] {
            write!(f, "{}, ", m)?;
        }
        write!(f, "{}]", self.data[self.n_moves - 1])
    }
}

impl MoveBuffer<TicTacToeMove> for TTTMoveBuf {
    #[inline]
    fn iter(&self) -> std::slice::Iter<TicTacToeMove> {
        self.data.iter()
    }

    #[inline]
    fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    fn push(&mut self, m: TicTacToeMove) {
        self.data[self.n_moves] = m;
        self.n_moves += 1;
    }

    fn capacity(&self) -> usize {
        self.data.len()
    }
}

impl TicTacToe {
    pub const fn new() -> Self {
        Self {
            board: [0; 2],
            moves: 0,
        }
    }

    #[inline]
    const fn probe_spot(&self, spot: usize) -> bool {
        // returns true if the chosen location is occupied by
        // the side to move
        self.board[(self.moves + 1) & 1] & (1 << spot) != 0
    }

    #[inline]
    const fn pos_filled(&self, i: usize) -> bool {
        (self.board[0] | self.board[1]) & (1 << i) != 0
    }

    fn player_at(&self, i: usize) -> bool {
        assert!(self.pos_filled(i));
        self.board[0] & (1 << i) != 0
    }

    fn char_at(&self, x: usize, y: usize) -> char {
        if self.pos_filled(y * 3 + x) {
            if self.player_at(y * 3 + x) {
                'X'
            } else {
                'O'
            }
        } else {
            '.'
        }
    }
}

impl Game for TicTacToe {
    type Move = TicTacToeMove;
    type Buffer = TTTMoveBuf;

    #[inline]
    fn turn(&self) -> i8 {
        if self.moves & 1 == 0 {
            1
        } else {
            -1
        }
    }

    fn evaluate(&self) -> i8 {
        // check first diagonal
        if self.probe_spot(0) && self.probe_spot(4) && self.probe_spot(8) {
            return -self.turn();
        }

        // check second diagonal
        if self.probe_spot(2) && self.probe_spot(4) && self.probe_spot(6) {
            return -self.turn();
        }

        // check rows
        for i in 0..3 {
            if self.probe_spot(i * 3) && self.probe_spot(i * 3 + 1) && self.probe_spot(i * 3 + 2) {
                return -self.turn();
            }
        }
        // check columns
        for i in 0..3 {
            if self.probe_spot(i) && self.probe_spot(i + 3) && self.probe_spot(i + 6) {
                return -self.turn();
            }
        }

        0
    }

    fn is_terminal(&self) -> bool {
        self.moves == 9 || self.evaluate() != 0
    }

    fn generate_moves(&self, buffer: &mut Self::Buffer) {
        let bb = self.board[0] | self.board[1];
        let mut bb = !bb & 0b111_111_111;
        while bb != 0 {
            buffer.push(TicTacToeMove::new(bb.trailing_zeros() as usize));
            bb &= bb - 1; // clear the least significant bit set
        }
    }

    fn push(&mut self, m: Self::Move) {
        self.board[self.moves & 1] |= 1 << m.0;
        self.moves += 1;
    }

    fn pop(&mut self, m: Self::Move) {
        self.moves -= 1;
        self.board[self.moves & 1] ^= 1 << m.0;
    }

    fn push_random(&mut self) {
        let bb = self.board[0] | self.board[1];
        let mut bb = !bb & 0b111_111_111;
        let possible_moves = bb.count_ones() as usize;
        let choice = rand::thread_rng().gen_range(0..possible_moves);
        for _ in 0..choice {
            bb &= bb - 1; // clear the least significant bit set
        }
        self.push(TicTacToeMove::new(bb.trailing_zeros() as usize));
    }
}

impl VectoriseState for TicTacToe {
    fn action_space() -> usize {
        9
    }

    fn vectorise_state(&self) -> StateVector {
        let mut v: Vec<u8> = Vec::with_capacity(3 * 3 * 2);

        for shift in 0..7 {
            v.push(((self.board[1] >> shift) & 1) as u8);
            v.push(((self.board[0] >> shift) & 1) as u8);
        }

        assert_eq!(v.len(), 3 * 3 * 2);
        StateVector { data: v }
    }

    fn index_move(m: Self::Move) -> usize {
        m.0
    }

    fn state_vector_dimensions() -> Vec<usize> {
        vec![3, 3, 2]
    }

    fn csv_header() -> String {
        "outcome,moves,board,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,policy,-,-,-,-,-,-,-,-".to_string()
    }
}

impl Display for TicTacToe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const RED: &str = "\u{001b}[31m";
        const YELLOW: &str = "\u{001b}[33m";
        const RESET: &str = "\u{001b}[0m";
        for y in 0..3 {
            for x in 0..3 {
                match self.char_at(x, y) {
                    '.' => write!(f, ". ")?,
                    'X' => write!(f, "{RED}X{RESET} ")?,
                    'O' => write!(f, "{YELLOW}O{RESET} ")?,
                    _ => unreachable!(),
                }
            }
            writeln!(f)?;
        }

        Ok(())
    }
}

impl Default for TicTacToe {
    fn default() -> Self {
        Self::new()
    }
}

// #[cfg(test)]
// mod tests {
//     use crate::perft::perft;

//     use super::TicTacToe;

//     #[test]
//     fn depth1() {
//         let mut board = TicTacToe::new();
//         assert_eq!(perft(&mut board, 1), 9);
//     }

//     #[test]
//     fn depth2() {
//         let mut board = TicTacToe::new();
//         assert_eq!(perft(&mut board, 2), 72);
//     }

//     #[test]
//     fn fullperft() {
//         let mut board = TicTacToe::new();
//         assert_eq!(perft(&mut board, 10), 255168);
//     }
// }
