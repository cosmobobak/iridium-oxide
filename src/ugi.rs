use std::{sync::{atomic::{AtomicBool, self}, mpsc, Mutex}, io::Write, str::FromStr};

use cozy_chess::Board;

use crate::{mcts::{SearchInfo, MCTS, Behaviour}, NAME, VERSION, games::chess::Chess};

static KEEP_RUNNING: AtomicBool = AtomicBool::new(true);

fn stdin_reader() -> mpsc::Receiver<String> {
    let (sender, reciever) = mpsc::channel();
    std::thread::Builder::new()
        .name("stdin-reader".into())
        .spawn(|| stdin_reader_worker(sender))
        .expect("Couldn't start stdin reader worker thread");
    reciever
}

fn stdin_reader_worker(sender: mpsc::Sender<String>) {
    let mut linebuf = String::with_capacity(128);
    while std::io::stdin().read_line(&mut linebuf).is_ok() {
        let cmd = linebuf.trim();
        if cmd.is_empty() {
            linebuf.clear();
            continue;
        }
        if sender.send(cmd.to_owned()).is_err() {
            break;
        }
        if !KEEP_RUNNING.load(atomic::Ordering::SeqCst) {
            break;
        }
        linebuf.clear();
    }
    std::mem::drop(sender);
}

fn print_uci_response() {
    println!("id name {NAME} {VERSION}");
    println!("id author Cosmo");
    println!("uciok");
}

pub fn main() {
    let stdin = Mutex::new(stdin_reader());
    let mut search_info = SearchInfo::new(&stdin);
    let behaviour = Behaviour::for_game::<Chess>();
    let mut engine = MCTS::<Chess>::new(&behaviour);
    let mut pos = Board::startpos();
    
    loop {
        std::io::stdout().flush().expect("couldn't flush stdout");
        let line = stdin
            .lock()
            .expect("failed to take lock on stdin")
            .recv()
            .expect("couldn't receive from stdin");
        let input = line.trim();

        let res = match input {
            "\n" => continue,
            "uci" => {
                print_uci_response();
                Ok(())
            }
            "isready" => {
                println!("readyok");
                Ok(())
            }
            "quit" => {
                search_info.quit = true;
                break;
            }
            "ucinewgame" => Ok(()),
            input if input.starts_with("position") => {
                let mut words = input.split_whitespace();
                match words.nth(1) {
                    Some("startpos") => {
                        pos = Board::startpos();
                        Ok(())
                    },
                    Some("fen") => {
                        let fen = words.next().unwrap();
                        pos = Board::from_fen(fen, false).unwrap();
                        if words.next() == Some("moves") {
                            for m in words {
                                let mv = cozy_chess::Move::from_str(m).unwrap();
                                pos.play(mv);
                            }
                        }
                        Ok(())
                    },
                    _ => Err(Box::new("expected 'startpos' or 'fen' after 'position'")),
                }
            }
            input if input.starts_with("go") => {
                let position = Chess::from_raw_board(pos.clone());
                let search_results = engine.search(&position);
                eprintln!("info string {search_results:?}");
                println!("bestmove X");
                Ok(())
            }
            _ => Err(Box::new("unknown command")),
        };

        if let Err(e) = res {
            println!("info string {e}");
        }

        if search_info.quit {
            // quit can be set true in parse_go
            break;
        }
    }
    KEEP_RUNNING.store(false, atomic::Ordering::SeqCst);
}