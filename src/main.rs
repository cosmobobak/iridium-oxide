#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{io::Write, time::Instant};

use crate::{
    gamerunner::{GameRunner, Player},
    mcts::{Behaviour, Limit, RolloutPolicy, MCTS}, games::{connectfour::Connect4, tictactoe::TicTacToe, gomoku::Gomoku},
};

mod games;
mod agent;
mod constants;
mod elo;
mod game;
mod gamerunner;
mod iterbits;
mod mcts;
mod searchtree;
mod treenode;
mod ucb;
mod datageneration;

use Player::{Computer, Human};
use datageneration::VectoriseState;
use game::Game;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("play") => {
            let game = args[2].as_str();
            let player = args[3].parse::<usize>().unwrap();
            assert!(player == 1 || player == 2, "player must be 1 (you play first) or 2 (i play first)");
            match game {
                "connect4" => play::<Connect4>(&Behaviour::default(), player),
                "tictactoe" => play::<TicTacToe>(&Behaviour::default(), player),
                "gomoku9" => play::<Gomoku<9>>(&Behaviour::default(), player),
                "gomoku13" => play::<Gomoku<13>>(&Behaviour::default(), player),
                "gomoku19" => play::<Gomoku<19>>(&Behaviour::default(), player),
                "reversi" |
                "uttt" => todo!(),
                unknown => {
                    if unknown != "help" { eprintln!("Unknown game: {unknown}"); }
                    eprintln!("Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt");
                }
            }
        }
        Some("generate") => {
            let game = args[2].as_str();
            let games = args[3].parse().unwrap();
            let fname = args[4].as_str();
            println!("{games} games will be played");
            let start = Instant::now();
            match game {
                "connect4" => generate_data::<Connect4>(games, fname),
                "tictactoe" => generate_data::<TicTacToe>(games, fname),
                "gomoku9" |
                "gomoku13" |
                "gomoku19" => unimplemented!(),
                "reversi" |
                "uttt" => todo!(),
                unknown => {
                    if unknown != "help" { eprintln!("Unknown game: {unknown}"); }
                    eprintln!("Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt");
                    return;
                }
            }
            let secs = start.elapsed().as_secs_f64();
            println!("Generating data took {secs:.2} seconds");
        }
        None => {
            println!("Available commands:");
            println!("1. Play against a the computer ({} play <game> <1|2>)", args[0]);
            println!("2. Generate data for a game ({} generate <game> <count> <fname>)", args[0]);
        }
        Some(unknown) => {
            if unknown != "help" { eprintln!("Unknown command: {unknown}"); }
            println!("Available commands:");
            println!("1. Play against a the computer ({} play <game> <1|2>)", args[0]);
            println!("2. Generate data for a game ({} generate <game> <count> <fname>)", args[0]);
        }
    }
}

fn play<G: Game>(config: &Behaviour, player: usize) {
    println!("iridium-oxide operating at full capacity!");
    match player {
        2 => GameRunner::<G>::new(Human, Computer(MCTS::new(config))).run(),
        1 => GameRunner::<G>::new(Computer(MCTS::new(config)), Human).run(),
        _ => panic!("fastplay: player must be 1 or 2"),
    }
}

fn generate_data<G: VectoriseState>(games: u32, fname: &str) {
    let limit = Limit::Rollouts(500_000);
    let config = Behaviour {
        debug: false,
        readout: false,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
        exp_factor: 5.0,
        training: true,
    };

    let episode_data = (0..games).map(|it| { 
        // print progress bar   
        print!("{:.2}%     \r", f64::from(it) * 100.0 / f64::from(games));
        std::io::stdout().flush().unwrap();
        GameRunner::<G>::play_training_game(&config)
    }).reduce(|a, b| a + b).expect("failed to generate training data");
    println!("100%     ");
    episode_data.save::<G>(&format!("datasets/{fname}.csv")).expect("failed to write file");
    episode_data.summary();
}
