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
mod game;
mod gamerunner;
mod mcts;
mod searchtree;
mod tictactoe;
mod treenode;
mod uct;

fn main() {
    println!("iridium-oxide operating at full capacity!");

    let mcts = MonteCarloTreeSearcher::new(Behaviour {
        debug: false,
        readout: true,
        limit: Limit::Time(Duration::from_millis(100)),
    });

    GameRunner::new(Connect4::new(), Player::Computer(mcts), Player::Human).run();
}
