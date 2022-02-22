#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::time::Duration;

use crate::{
    connectfour::{Connect4, C4Move},
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, MonteCarloTreeSearcher}, game::{Vectorisable, Game},
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

    let mcts1 = MonteCarloTreeSearcher::new(Behaviour {
        debug: false,
        readout: false,
        limit: Limit::Time(Duration::from_millis(1000)),
    });

    GameRunner::<Connect4>::new(Computer(mcts1), Human).run();
}
