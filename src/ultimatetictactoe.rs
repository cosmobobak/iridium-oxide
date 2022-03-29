
use crate::game::{Game, MoveBuffer};
use bitvec::prelude::*;

/// Representation of a single game state in Ultimate Tic-Tac-Toe.
/// Uses a compact representation of the board, with a bitvec 
/// storing the X and O halves of the 9x9 board.
struct UltimateTicTacToe {
    board: BitVec<u64>,
    moves: u8,
    forced_box: u8,
}