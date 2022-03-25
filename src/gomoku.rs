#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    ops::Index,
};

use rand::Rng;

use crate::game::{Game, MoveBuffer};

// TODO: make a more compact representation of the board

const WIN_LINE_LENGTH: usize = 5;
const EMPTY: i8 = 0;
const X: i8 = 1;
const O: i8 = -1;
const ROWS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gomoku<const N: usize> {
    board: [[i8; N]; N],
    moves: usize,
    last_move: Move<N>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move<const N: usize> {
    loc: usize,
}

impl<const N: usize> Move<N> {
    const fn new(loc: usize) -> Self {
        Self { loc }
    }

    const fn row(self) -> usize {
        self.loc / N
    }

    const fn col(self) -> usize {
        self.loc % N
    }
}

impl<const N: usize> Display for Move<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", ROWS[self.row()] as char, self.col() + 1)
    }
}

impl<const N: usize> PartialOrd for Move<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.loc.cmp(&other.loc))
    }
}

impl<const N: usize> Ord for Move<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.loc.cmp(&other.loc)
    }
}

impl<const N: usize> Gomoku<N> {
    const N_I: isize = N as isize;

    const fn new() -> Self {
        Self {
            board: [[0; N]; N],
            moves: 0,
            last_move: Move::new(0),
        }
    }

    fn row_along<const D_X: isize, const D_Y: isize>(&self, row: usize, col: usize) -> bool {
        let mut count = 1;
        let last_piece = -self.turn();

        if !(D_X < 0 && row == 0
            || D_Y < 0 && col == 0
            || D_X > 0 && row == N - 1
            || D_Y > 0 && col == N - 1)
        {
            let mut row_u = row as isize + D_X;
            let mut col_u = col as isize + D_Y;
            loop {
                // count pieces in a direction until we hit a piece of the opposite color or an empty space
                if self.board[row_u as usize][col_u as usize] != last_piece {
                    break;
                }
                count += 1;
                if count == WIN_LINE_LENGTH {
                    return true;
                }
                if D_X < 0 && row_u == 0
                    || D_Y < 0 && col_u == 0
                    || D_X > 0 && row_u == Self::N_I - 1
                    || D_Y > 0 && col_u == Self::N_I - 1
                {
                    break;
                }
                row_u += D_X;
                col_u += D_Y;
            }
        }
        if !(D_X > 0 && row == 0
            || D_Y > 0 && col == 0
            || D_X < 0 && row == N - 1
            || D_Y < 0 && col == N - 1)
        {
            let mut row_d = row as isize - D_X;
            let mut col_d = col as isize - D_Y;
            loop {
                // count pieces in -direction until we hit a piece of the opposite color or an empty space
                if self.board[row_d as usize][col_d as usize] != last_piece {
                    break;
                }
                count += 1;
                if count == WIN_LINE_LENGTH {
                    return true;
                }
                if D_X > 0 && row_d == 0
                    || D_Y > 0 && col_d == 0
                    || D_X < 0 && row_d == Self::N_I - 1
                    || D_Y < 0 && col_d == Self::N_I - 1
                {
                    break;
                }
                row_d -= D_X;
                col_d -= D_Y;
            }
        }

        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Buffer<const N: usize, const SIZE: usize> {
    moves: [Move<N>; SIZE],
    len: usize,
}

impl<const N: usize, const SIZE: usize> Buffer<N, SIZE> {
    const fn new() -> Self {
        Self {
            moves: [Move::new(0); SIZE],
            len: 0,
        }
    }

    unsafe fn push_unchecked(&mut self, m: Move<N>) {
        *self.moves.get_unchecked_mut(self.len) = m;
        self.len += 1;
    }
}

impl<const N: usize, const SIZE: usize> Display for Buffer<N, SIZE> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[")?;
        for m in &self.moves[..self.len - 1] {
            write!(f, "{}, ", m)?;
        }
        write!(f, "{}]", self.moves[self.len - 1])
    }
}

impl<const N: usize, const SIZE: usize> Index<usize> for Buffer<N, SIZE> {
    type Output = Move<N>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

impl<const N: usize, const SIZE: usize> Default for Buffer<N, SIZE> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, const SIZE: usize> MoveBuffer<Move<N>> for Buffer<N, SIZE> {
    fn iter(&self) -> std::slice::Iter<Move<N>> {
        self.moves.iter()
    }

    fn len(&self) -> usize {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn push(&mut self, m: Move<N>) {
        self.moves[self.len] = m;
        self.len += 1;
    }

    fn capacity(&self) -> usize {
        SIZE
    }
}

impl<const N: usize> Default for Gomoku<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Display for Gomoku<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        const RED: &str = "\u{001b}[31m";
        const YELLOW: &str = "\u{001b}[33m";
        const RESET: &str = "\u{001b}[0m";
        write!(f, "  ")?;
        for c in 0..N {
            write!(f, "{} ", c + 1)?;
        }
        writeln!(f)?;
        for (i, row) in self.board.iter().enumerate() {
            write!(f, "{} ", ROWS[i] as char)?;
            for &cell in row.iter() {
                write!(
                    f,
                    "{}",
                    match cell {
                        EMPTY => ". ".to_string(),
                        X => format!("{RED}X{RESET} "),
                        O => format!("{YELLOW}O{RESET} "),
                        _ => unreachable!(),
                    }
                )?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl<const N: usize> Game for Gomoku<N> {
    type Move = Move<N>;

    type Buffer = Buffer<N, { 15 * 15 }>;

    fn turn(&self) -> i8 {
        if self.moves % 2 == 0 {
            1
        } else {
            -1
        }
    }

    fn is_terminal(&self) -> bool {
        self.moves >= N * N || self.evaluate() != 0
    }

    fn push(&mut self, m: Self::Move) {
        self.board[m.row()][m.col()] = self.turn();
        self.moves += 1;
        self.last_move = m;
    }

    fn pop(&mut self, m: Self::Move) {
        self.board[m.row()][m.col()] = EMPTY;
        self.moves -= 1;
    }

    fn evaluate(&self) -> i8 {
        let last_played = self.last_move;
        let row = last_played.row();
        let col = last_played.col();

        if self.row_along::<0, 1>(row, col)
            || self.row_along::<1, 0>(row, col)
            || self.row_along::<1, 1>(row, col)
            || self.row_along::<1, -1>(row, col)
        {
            -self.turn()
        } else {
            0
        }
    }

    fn generate_moves(&self, moves: &mut Self::Buffer) {
        assert!(moves.capacity() >= N * N);
        let mut i = 0;
        for row in self.board {
            for &cell in row.iter() {
                if cell == EMPTY {
                    // SAFETY: `moves` is guaranteed to have enough capacity
                    // (we do at most N * N passed through this inner loop
                    // and we know that `moves` has at least this many slots)
                    unsafe {
                        moves.push_unchecked(Move::new(i));
                    }
                }
                i += 1;
            }
        }
    }

    fn push_random(&mut self) {
        let mut moves = Buffer::new();
        self.generate_moves(&mut moves);
        let index = rand::thread_rng().gen_range(0..moves.len());
        self.push(moves[index]);
    }
}
