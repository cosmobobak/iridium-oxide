#![allow(clippy::cast_precision_loss)]

use std::{fmt::Display, ops::Range};

use crate::game::Game;

#[derive(Debug, Clone, PartialEq)]
pub struct Node<G: Game> {
    first_child: u32,    // 4 bytes.
    n_children: u16,     // 2 bytes.
    parent: Option<u32>, // 5 bytes.

    value: f32,      // 4 bytes.
    visits: u32,     // 4 bytes.
    perspective: i8, // 1 byte.

    inbound_edge: G::Move, // ??? bytes.
}

impl<G: Game> Node<G> {
    pub fn new(turn: i8, parent: Option<usize>, inbound_edge: G::Move) -> Self {
        // perspective is set to -turn because what matters is
        // whether a player wants to "enter" a node, and so if
        // the turn is 1, then the perspective is -1, because
        // this node has just been "chosen" by player -1.
        let perspective = -turn;
        Self {
            first_child: 0,
            n_children: 0,
            parent: parent.map(|p| p.try_into().unwrap()),
            value: 0.0,
            visits: 0,
            perspective,
            inbound_edge,
        }
    }

    pub const fn children(&self) -> Range<usize> {
        self.first_child as usize..self.first_child as usize + self.n_children as usize
    }

    pub fn parent(&self) -> Option<usize> {
        self.parent.map(|p| p as usize)
    }

    pub const fn to_move(&self) -> i8 {
        -self.perspective
    }

    pub const fn wins(&self) -> f32 {
        self.value
    }

    pub const fn visits(&self) -> u32 {
        self.visits
    }

    pub const fn inbound_edge(&self) -> G::Move {
        self.inbound_edge
    }

    pub fn win_rate(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            f64::from(self.value) / f64::from(self.visits)
        }
    }

    #[inline]
    pub fn update(&mut self, q: f32) {
        self.visits += 1;
        assert!(self.perspective == 1 || self.perspective == -1);
        // scale the range of q from [-1, 1] to [0, WIN_SCORE]
        let perspective_q = q * f32::from(self.perspective);
        // the whole negative-positive thing really sucks
        assert!((-1.0..=1.0).contains(&q), "q holds invalid value: {q}");
        let value = (perspective_q + 1.0) / 2.0;
        assert!(
            (0.0..=1.0).contains(&value),
            "computed value holds invalid value: expected in range [0, 1], got {value}"
        );
        self.value += value;
    }

    pub fn set_win_score(&mut self, score: f32) {
        self.value = score;
    }

    pub fn add_children(&mut self, start: usize, count: usize) {
        self.first_child = start.try_into().unwrap();
        self.n_children = count
            .try_into()
            .unwrap_or_else(|_| panic!("cannot handle more than {} children at once.", u16::MAX));
    }

    pub const fn has_children(&self) -> bool {
        self.n_children > 0
    }

    pub fn random_child(&self, rng: &mut fastrand::Rng) -> usize {
        rng.usize(self.children())
    }
}

impl<G: Game> Display for Node<G> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node {{ children: {:?}, parent: {:?}, wins: {}, visits: {}, to_move: {}, win_rate: {} }}", self.children(), self.parent, self.value, self.visits, self.to_move(), self.win_rate())
    }
}
