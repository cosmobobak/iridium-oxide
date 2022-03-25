use std::{fmt::Display, ops::Range};

use rand::Rng;

use crate::{
    constants::{DRAW_SCORE, WIN_SCORE},
    game::Game,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node<G: Game> {
    board: G,
    first_child: usize,
    n_children: usize,
    parent: Option<usize>,

    wins: i32,
    visits: u32,
    perspective: i8, // 1 for max, -1 for min
}

impl<G: Game> Node<G> {
    pub fn new(board: G, parent: Option<usize>) -> Self {
        // perspective is set to -turn because what matters is
        // whether a player wants to "enter" a node, and so if
        // the turn is 1, then the perspective is -1, because
        // this node has just been "chosen" by player -1.
        let perspective = -board.turn();
        Self {
            board,
            first_child: 0,
            n_children: 0,
            parent,
            wins: 0,
            visits: 0,
            perspective,
        }
    }

    pub fn state(&self) -> &G {
        &self.board
    }

    pub fn children(&self) -> Range<usize> {
        self.first_child..self.first_child + self.n_children
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent
    }

    // pub fn orphanise(&mut self) {
    //     // this method is for a future implementation of tree pruning and compaction.
    //     self.parent = None;
    // }

    pub fn to_move(&self) -> i8 {
        self.board.turn()
    }

    pub fn wins(&self) -> i32 {
        self.wins
    }

    pub fn visits(&self) -> u32 {
        self.visits
    }

    pub fn win_rate(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            f64::from(self.wins) / f64::from(self.visits) / f64::from(WIN_SCORE)
        }
    }

    #[inline]
    pub fn update(&mut self, result: i8) {
        self.visits += 1;
        // the whole negative-positive thing really sucks
        assert!(
            result == 1 || result == -1 || result == 0,
            "result holds invalid value: {}",
            result
        );
        assert!(self.perspective == 1 || self.perspective == -1);
        if result == self.perspective {
            // rollout was a win for us
            self.wins += WIN_SCORE;
        } else if result == 0 {
            // rollout was a draw
            self.wins += DRAW_SCORE;
        } else {
            // rollout was a loss for us
            // no-op, because LOSS_SCORE = 0.
        }
    }

    pub fn set_win_score(&mut self, score: i32) {
        self.wins = score;
    }

    pub fn add_children(&mut self, start: usize, count: usize) {
        self.first_child = start;
        self.n_children = count;
    }

    pub fn has_children(&self) -> bool {
        self.n_children > 0
    }

    pub fn random_child(&self) -> usize {
        rand::thread_rng().gen_range(self.children())
    }
}

impl<G: Game> Display for Node<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node {{ board: {:?}, children: {:?}, parent: {:?}, wins: {}, visits: {}, to_move: {}, win_rate: {} }}", self.board, self.children(), self.parent, self.wins, self.visits, self.board.turn(), self.win_rate())
    }
}
