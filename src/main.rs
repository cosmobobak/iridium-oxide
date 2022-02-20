#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(dead_code)]

use std::time::Duration;

use crate::{
    connectfour::Connect4,
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, MonteCarloTreeSearcher},
};

mod agent;
mod connectfour;
mod constants;
mod elo;
mod game;
mod gamerunner;
mod mcts;
mod searchtree;
mod tictactoe;
mod treenode;
mod uct;

fn main() {
    use Player::{Computer, Human};

    println!("iridium-oxide operating at full capacity!");

    let mcts1 = MonteCarloTreeSearcher::new(Behaviour {
        debug: false,
        readout: true,
        limit: Limit::Time(Duration::from_millis(1000)),
    });

    GameRunner::<Connect4>::new(Human, Computer(mcts1)).run();
}
