use std::{
    fmt::{Debug, Display},
    ops::Index,
};

use rand::Rng;

use crate::{game::{Game, MoveBuffer}, datageneration::{StateVector, Vectorisable}};

type Bitrow = u8;

const ROWS: usize = 6;
const COLS: usize = 7;
const BITROW_MASK: Bitrow = 0b0111_1111;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct C4Move(pub usize);

impl Debug for C4Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "idx: {}", self.0)
    }
}

impl Display for C4Move {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0 + 1)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Connect4 {
    board: [[Bitrow; ROWS]; 2],
    moves: u8,
}

impl Connect4 {
    pub const fn new() -> Self {
        Self {
            board: [[0; ROWS]; 2],
            moves: 0,
        }
    }

    const fn filled(&self, row: usize, col: usize) -> bool {
        self.board[0][row] & (1 << col) != 0 || self.board[1][row] & (1 << col) != 0
    }

    const fn player_at(&self, row: usize, col: usize) -> i8 {
        if self.board[0][row] & (1 << col) != 0 {
            1
        } else if self.board[1][row] & (1 << col) != 0 {
            -1
        } else {
            0
        }
    }

    #[inline]
    const fn probe(&self, row: usize, col: usize) -> bool {
        self.board[((self.moves & 1) ^ 1) as usize][row] & (1 << col) != 0
    }

    fn horizontal_eval(&self) -> i8 {
        for row in 0..ROWS {
            for bitshift in 0..COLS {
                if ((self.board[((self.moves + 1) & 1) as usize][row] >> bitshift) & 0b1111)
                    == 0b1111
                {
                    return -self.turn();
                }
            }
        }
        0
    }

    fn vertical_eval(&self) -> i8 {
        for row in 0..ROWS - 3 {
            for col in 0..COLS {
                if self.probe(row, col)
                    && self.probe(row + 1, col)
                    && self.probe(row + 2, col)
                    && self.probe(row + 3, col)
                {
                    return -self.turn();
                }
            }
        }
        0
    }

    fn diag_up_eval(&self) -> i8 {
        for row in 3..ROWS {
            for col in 0..COLS - 3 {
                if self.probe(row, col)
                    && self.probe(row - 1, col + 1)
                    && self.probe(row - 2, col + 2)
                    && self.probe(row - 3, col + 3)
                {
                    return -self.turn();
                }
            }
        }
        0
    }

    fn diag_down_eval(&self) -> i8 {
        for row in 0..ROWS - 3 {
            for col in 0..COLS - 3 {
                if self.probe(row, col)
                    && self.probe(row + 1, col + 1)
                    && self.probe(row + 2, col + 2)
                    && self.probe(row + 3, col + 3)
                {
                    return -self.turn();
                }
            }
        }
        0
    }
}

impl Default for Connect4 {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for C4Move {
    fn default() -> Self {
        Self(7)
    }
}

impl Display for Connect4 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const RED: &str = "\u{001b}[31m";
        const YELLOW: &str = "\u{001b}[33m";
        const RESET: &str = "\u{001b}[0m";
        for row in 0..ROWS {
            for col in 0..COLS {
                match self.player_at(row, col) {
                    1 => write!(f, "{RED}X{RESET} ")?,
                    -1 => write!(f, "{YELLOW}O{RESET} ")?,
                    _ => write!(f, ". ")?,
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Game for Connect4 {
    type Move = C4Move;
    type Buffer = MoveBuf;

    fn turn(&self) -> i8 {
        if self.moves % 2 == 0 {
            1
        } else {
            -1
        }
    }

    fn generate_moves(&self, moves: &mut Self::Buffer) {
        let bb = self.board[0][0] | self.board[1][0];

        let mut bb = !bb & BITROW_MASK;

        while bb != 0 {
            moves.push(C4Move(bb.trailing_zeros() as usize));
            bb &= bb - 1;
        }
    }

    fn is_terminal(&self) -> bool {
        self.moves == 42
            || self.horizontal_eval() != 0
            || self.vertical_eval() != 0
            || self.diag_up_eval() != 0
            || self.diag_down_eval() != 0
    }

    fn evaluate(&self) -> i8 {
        let h = self.horizontal_eval();
        if h != 0 {
            return h;
        }

        let v = self.vertical_eval();
        if v != 0 {
            return v;
        }

        let du = self.diag_up_eval();
        if du != 0 {
            return du;
        }

        self.diag_down_eval()
    }

    fn push(&mut self, m: Self::Move) {
        assert!(!self.filled(0, m.0));
        let mut row = ROWS;
        while self.filled(row - 1, m.0) {
            row -= 1;
        }

        assert!(row > 0 && row - 1 < ROWS);
        self.board[(self.moves & 1) as usize][row - 1] |= 1 << m.0;

        self.moves += 1;
    }

    fn pop(&mut self, m: Self::Move) {
        assert!(self.filled(ROWS, m.0));

        self.moves -= 1;

        let mut row = 0;
        while !self.filled(row, m.0) {
            row += 1;
        }

        assert!(row < ROWS);
        self.board[(self.moves & 1) as usize][row] &= !(1 << m.0);
    }

    fn push_random(&mut self) {
        let bb = self.board[0][0] | self.board[1][0];
        let bb = !bb & BITROW_MASK;

        let n_moves = bb.count_ones() as usize;

        let choice = rand::thread_rng().gen_range(0..n_moves);

        let mut bb = bb;

        for _ in 0..choice {
            bb &= bb - 1;
        }

        assert!(bb != 0, "you fucked up");

        self.push(C4Move(bb.trailing_zeros() as usize));
    }
}

impl Vectorisable for Connect4 {
    fn vectorise_state(&self) -> StateVector {
        let mut v: Vec<u8> = Vec::with_capacity(ROWS * COLS * 2);

        for (&rowl, &rowr) in self.board[0].iter().zip(self.board[1].iter()) {
            for col_shift in 0..COLS {
                v.push((rowr >> col_shift) & 1);
                v.push((rowl >> col_shift) & 1);
            }
        }

        assert_eq!(v.len(), ROWS * COLS * 2);
        StateVector { data: v }
    }

    fn index_move(m: Self::Move) -> usize {
        m.0
    }

    fn action_space() -> usize {
        COLS
    }

    fn state_vector_dimensions() -> Vec<usize> {
        vec![ROWS, COLS, 2]
    }

    fn csv_header() -> String {
        "outcome,moves,board,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,-,policy,-,-,-,-,-,-".to_string()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveBuf {
    moves: [C4Move; COLS],
    n_moves: usize,
}

impl Default for MoveBuf {
    fn default() -> Self {
        Self {
            moves: [C4Move(0); COLS],
            n_moves: 0,
        }
    }
}

impl MoveBuffer<C4Move> for MoveBuf {
    fn iter(&self) -> std::slice::Iter<C4Move> {
        self.moves[..self.n_moves].iter()
    }

    fn len(&self) -> usize {
        self.n_moves
    }

    fn is_empty(&self) -> bool {
        self.n_moves == 0
    }

    fn push(&mut self, m: C4Move) {
        self.moves[self.n_moves] = m;
        self.n_moves += 1;
    }

    fn capacity(&self) -> usize {
        self.moves.len()
    }
}

impl Index<usize> for MoveBuf {
    type Output = C4Move;

    fn index(&self, index: usize) -> &Self::Output {
        &self.moves[index]
    }
}

impl Display for MoveBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")?;
        for m in &self.moves[..self.n_moves - 1] {
            write!(f, "{}, ", m)?;
        }
        write!(f, "{}]", self.moves[self.n_moves - 1])
    }
}
