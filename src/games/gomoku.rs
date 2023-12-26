#![allow(clippy::cast_possible_wrap, clippy::cast_sign_loss)]

use std::{
    cmp::Ordering,
    fmt::{self, Display, Formatter},
    ops::Index,
};

use crate::{
    game::{Game, MoveBuffer},
    mcts::MCTSExt, datageneration::VectoriseState,
};

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

type MoveInnerRepr = u16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move<const N: usize> {
    loc: MoveInnerRepr,
}

impl<const N: usize> Move<N> {
    const fn new(loc: usize) -> Self {
        #[allow(clippy::cast_possible_truncation)]
        Self {
            loc: loc as MoveInnerRepr,
        }
    }

    const fn row(self) -> usize {
        self.loc as usize / N
    }

    const fn col(self) -> usize {
        self.loc as usize % N
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

    const fn in_bounds(row: isize, col: isize) -> bool {
        row >= 0 && row < Self::N_I && col >= 0 && col < Self::N_I
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer<const N: usize> {
    moves: Vec<Move<N>>,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self {
            moves: Vec::with_capacity(N * N),
        }
    }
}

impl<const N: usize> Display for Buffer<N> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[")?;
        for m in &self.moves[..self.moves.len() - 1] {
            write!(f, "{m}, ")?;
        }
        write!(f, "{}]", self.moves[self.moves.len() - 1])
    }
}

impl<const N: usize> Index<usize> for Buffer<N> {
    type Output = Move<N>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

impl<const N: usize> Default for Buffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> MoveBuffer<Move<N>> for Buffer<N> {
    fn iter(&self) -> std::slice::Iter<Move<N>> {
        self.moves.iter()
    }

    fn len(&self) -> usize {
        self.moves.len()
    }

    fn is_empty(&self) -> bool {
        self.moves.is_empty()
    }

    fn push(&mut self, m: Move<N>) {
        self.moves.push(m);
    }

    fn capacity(&self) -> usize {
        self.moves.capacity()
    }
}

impl<const N: usize> Default for Gomoku<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Default for Move<N> {
    fn default() -> Self {
        Self::new(N * N)
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
            for &cell in row {
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

    type Buffer = Buffer<N>;

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
        for row in &self.board {
            for &cell in row {
                if cell == EMPTY {
                    moves.push(Move::new(i));
                }
                i += 1;
            }
        }
    }

    fn push_random(&mut self, rng: &mut fastrand::Rng) {
        #![allow(clippy::cast_precision_loss)]
        let filled_factor = self.moves as f64 / (N * N) as f64;
        // if the board is mostly full, generate moves and then select.
        // otherwise, just guess moves until we find an empty square.
        if filled_factor > 0.95 {
            let mut moves = Buffer::new();
            self.generate_moves(&mut moves);
            let index = rng.usize(..moves.len());
            return self.push(moves[index]);
        }
        // we expect this loop to run only a few times
        // (at most 95% of the board is full, so we expect to find an empty square in 20 tries)
        let index = loop {
            let index = rng.usize(..N * N);
            if self.board[index / N][index % N] == EMPTY {
                break index;
            }
        };
        self.push(Move::new(index));
    }

    fn policy(&self, node: &crate::treenode::Node<Self>) -> f32 {
        #![allow(clippy::cast_possible_truncation)]
        let move_that_lead_to_it = node.inbound_edge();
        let target_sq = move_that_lead_to_it.loc;
        // if the target square is adjacent to an existing piece, it's a good move
        let row = target_sq / N as u16;
        let col = target_sq % N as u16;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if (dx != 0 || dy != 0) && Self::in_bounds(row as isize + dx, col as isize + dy) {
                    let index = (row as isize + dx) as usize * N + (col as isize + dy) as usize;
                    if self.board[index / N][index % N] != EMPTY {
                        return 3.0;
                    }
                }
            }
        }
        // otherwise, it's a bad move
        1.0
    }
}

impl<const N: usize> VectoriseState for Gomoku<N> {
    fn csv_header() -> String {
        String::new()
    }

    fn vectorise_state(&self) -> crate::datageneration::StateVector {
        let mut v: Vec<u8> = Vec::with_capacity(N * N * 2);

        for row in &self.board {
            for &cell in row {
                v.push(u8::from(cell == X));
            }
        }
        for row in &self.board {
            for &cell in row {
                v.push(u8::from(cell == O));
            }
        }

        assert_eq!(v.len(), N * N * 2);

        crate::datageneration::StateVector { data: v }
    }

    fn index_move(m: Self::Move) -> usize {
        m.loc as usize
    }

    fn action_space() -> usize {
        N * N
    }

    fn state_vector_dimensions() -> Vec<usize> {
        vec![N, N, 2]
    }
}

impl<const N: usize> MCTSExt for Gomoku<N> {}