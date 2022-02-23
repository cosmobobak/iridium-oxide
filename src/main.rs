#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::time::Duration;

use crate::{
    connectfour::Connect4,
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, MCTS},
};

mod agent;
mod connectfour;
mod constants;
mod elo;
mod game;
mod gamerunner;
mod iterbits;
mod mcts;
mod searchtree;
mod tictactoe;
mod treenode;
mod uct;

#[allow(unused_imports)]
use Player::{Computer, Human};

fn main() {
    println!("iridium-oxide operating at full capacity!");

    let engine = MCTS::new(Behaviour {
        debug: false,
        readout: false,
        limit: Limit::Time(Duration::from_millis(1000)),
    });

    GameRunner::<Connect4>::new(Computer(engine), Human).run();
}
