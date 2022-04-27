#![allow(clippy::cast_precision_loss)]

use std::{fmt::Display, ops::Range};

use rand::Rng;

use crate::{constants::WIN_SCORE, game::Game};

#[derive(Debug, Clone, PartialEq)]
pub struct Node<G: Game> {
    board: G,
    first_child: usize,
    n_children: usize,
    parent: Option<usize>,

    value: f32,
    visits: u32,
    perspective: i8, // 1 for max, -1 for min

    terminal: bool,

    inbound_edge: G::Move,
}

impl<G: Game> Node<G> {
    pub fn new(board: G, parent: Option<usize>, inbound_edge: G::Move) -> Self {
        // perspective is set to -turn because what matters is
        // whether a player wants to "enter" a node, and so if
        // the turn is 1, then the perspective is -1, because
        // this node has just been "chosen" by player -1.
        let perspective = -board.turn();
        let terminal = board.is_terminal();
        Self {
            board,
            first_child: 0,
            n_children: 0,
            parent,
            value: 0.0,
            visits: 0,
            perspective,
            terminal,
            inbound_edge,
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

    pub fn to_move(&self) -> i8 {
        -self.perspective
    }

    pub fn wins(&self) -> f32 {
        self.value
    }

    pub fn visits(&self) -> u32 {
        self.visits
    }

    pub fn terminal(&self) -> bool {
        self.terminal
    }

    pub fn inbound_edge(&self) -> G::Move {
        self.inbound_edge
    }

    pub fn win_rate(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            f64::from(self.value) / f64::from(self.visits) / f64::from(WIN_SCORE)
        }
    }

    #[inline]
    pub fn update(&mut self, q: f32) {
        self.visits += 1;
        assert!(self.perspective == 1 || self.perspective == -1);
        // scale the range of q from [-1, 1] to [0, WIN_SCORE]
        let perspective_q = q * f32::from(self.perspective);
        // the whole negative-positive thing really sucks
        assert!((-1.0..=1.0).contains(&q), "q holds invalid value: {}", q);
        let value = (perspective_q + 1.0) / 2.0 * WIN_SCORE;
        assert!((0.0..=WIN_SCORE).contains(&value), "computed value holds invalid value: expected in range [0, {}], got {}", WIN_SCORE, value);
        self.value += value;
    }

    pub fn set_win_score(&mut self, score: f32) {
        self.value = score;
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
        write!(f, "Node {{ board: {:?}, children: {:?}, parent: {:?}, wins: {}, visits: {}, to_move: {}, win_rate: {} }}", self.board, self.children(), self.parent, self.value, self.visits, self.board.turn(), self.win_rate())
    }
}
