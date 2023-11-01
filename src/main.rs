#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use std::{io::Write, time::Instant};

use crate::{
    gamerunner::{GameRunner, Player},
    games::{connectfour::Connect4, gomoku::Gomoku, tictactoe::TicTacToe},
    mcts::{Behaviour, Limit, RolloutPolicy, MCTS},
};

mod agent;
mod constants;
mod datageneration;
mod elo;
mod game;
mod gamerunner;
mod games;
mod iterbits;
mod mcts;
mod searchtree;
mod treenode;
mod ucb;
mod ugi;

use datageneration::VectoriseState;
use game::Game;
use games::chess::Chess;
use mcts::MCTSExt;
use Player::{Computer, Human};

/// The name of the engine.
pub static NAME: &str = "Iridium";
/// The version of the engine.
pub static VERSION: &str = env!("CARGO_PKG_VERSION");

#[allow(clippy::too_many_lines)]
fn main() {
    let args: Vec<String> = std::env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("play") => {
            let game = args.get(2);
            let player = args.get(3).map(String::as_str);
            match game.map(String::as_str) {
                Some("connect4") => play::<Connect4>(player),
                Some("tictactoe") => play::<TicTacToe>(player),
                Some("gomoku9") => play::<Gomoku<9>>(player),
                Some("gomoku13") => play::<Gomoku<13>>(player),
                Some("gomoku15") => play::<Gomoku<15>>(player),
                Some("gomoku19") => play::<Gomoku<19>>(player),
                Some("chess") => play::<Chess>(player),
                Some("reversi" | "uttt") => todo!(),
                Some(unknown) => {
                    if unknown != "help" {
                        eprintln!("Unknown game: {unknown}");
                    }
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,15,19}}, reversi, uttt, chess"
                    );
                }
                None => {
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,15,19}}, reversi, uttt, chess"
                    );
                }
            }
        }
        Some("generate") => {
            let game = args.get(2);
            let games = args[3].parse().unwrap();
            let fname = args[4].as_str();
            println!("{games} games will be played");
            let start = Instant::now();
            match game.map(String::as_str) {
                Some("connect4") => generate_data::<Connect4>(games, fname),
                Some("tictactoe") => generate_data::<TicTacToe>(games, fname),
                Some("gomoku9" | "gomoku13" | "gomoku15" | "gomoku19") => unimplemented!(),
                Some("reversi" | "uttt") => todo!(),
                Some(unknown) => {
                    if unknown != "help" {
                        eprintln!("Unknown game: {unknown}");
                    }
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt, chess"
                    );
                    return;
                }
                None => {
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,19}}, reversi, uttt, chess"
                    );
                    return;
                }
            }
            let secs = start.elapsed().as_secs_f64();
            println!("Generating data took {secs:.2} seconds");
        }
        Some("match") => {
            // run match between two configurations
            let game = args.get(2).map(String::as_str);
            let rounds = args.get(3).map_or(1, |it| it.parse().unwrap());
            let config1 = args.get(4).map(String::as_str);
            let config2 = args.get(5).map(String::as_str);
            match game {
                Some("connect4") => run_test::<Connect4>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("tictactoe") => run_test::<TicTacToe>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("gomoku9") => run_test::<Gomoku<9>>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("gomoku13") => run_test::<Gomoku<13>>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("gomoku15") => run_test::<Gomoku<15>>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("gomoku19") => run_test::<Gomoku<19>>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("chess") => run_test::<Chess>(
                    rounds,
                    config1.expect("no config"),
                    config2.expect("no config"),
                ),
                Some("reversi" | "uttt") => todo!(),
                Some(unknown) => {
                    if unknown != "help" {
                        eprintln!("Unknown game: {unknown}");
                    }
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,15,19}}, reversi, uttt, chess"
                    );
                }
                None => {
                    println!(
                        "Available games: connect4, tictactoe, gomoku{{9,13,15,19}}, reversi, uttt, chess"
                    );
                }
            }
        }
        Some("uci") => ugi::main(),
        None => {
            println!("Available commands:");
            println!("1. Play against a the computer ({NAME} play <game> <1|2>)");
            println!("2. Generate data for a game ({NAME} generate <game> <count> <fname>)");
            println!("3. Run a match between two configurations ({NAME} match <game> <rounds> <config1> <config2>)");
        }
        Some(unknown) => {
            if unknown != "help" {
                eprintln!("Unknown command: {unknown}");
            }
            println!("Available commands:");
            println!("1. Play against a the computer ({NAME} play <game> <1|2>)");
            println!("2. Generate data for a game ({NAME} generate <game> <count> <fname>)");
            println!("3. Run a match between two configurations ({NAME} match <game> <rounds> <config1> <config2>)");
        }
    }
}

fn play<G: Game + MCTSExt>(player: Option<&str>) {
    println!("iridium-oxide operating at full capacity!");
    let config = &Behaviour::for_game::<G>();
    let player = player
        .unwrap_or_else(|| panic!("No side provided."))
        .parse()
        .unwrap();
    match player {
        1 => GameRunner::<G>::new(Human, Computer(MCTS::new(config))).run(),
        2 => GameRunner::<G>::new(Computer(MCTS::new(config)), Human).run(),
        _ => panic!("fastplay: player must be 1 (you play first) or 2 (i play first)"),
    }
}

fn generate_data<G: VectoriseState + MCTSExt>(games: u32, fname: &str) {
    let limit = Limit::Rollouts(500_000);
    let config = Behaviour {
        debug: false,
        readout: false,
        log: false,
        limit,
        root_parallelism_count: 1,
        rollout_policy: RolloutPolicy::Random,
        exp_factor: 5.0,
        training: true,
    };

    let episode_data = (0..games)
        .map(|it| {
            // print progress bar
            print!("{:.2}%     \r", f64::from(it) * 100.0 / f64::from(games));
            std::io::stdout().flush().unwrap();
            GameRunner::<G>::play_training_game(&config)
        })
        .reduce(|a, b| a + b)
        .expect("failed to generate training data");
    println!("100%     ");
    episode_data
        .save::<G>(&format!("datasets/{fname}.csv"))
        .expect("failed to write file");
    episode_data.summary();
}

fn run_test<G: Game + MCTSExt>(rounds: usize, config1: &str, config2: &str) {
    let behaviour_1: Behaviour = config1.parse().unwrap();
    let behaviour_2: Behaviour = config2.parse().unwrap();
    let player_1 = Computer(MCTS::<G>::new(&behaviour_1));
    let player_2 = Computer(MCTS::<G>::new(&behaviour_2));
    let mut runner = GameRunner::<G>::new(player_1, player_2);
    runner.play_match(rounds * 2);
}
