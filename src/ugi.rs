use std::{sync::{atomic::{AtomicBool, self}, mpsc, Mutex}, io::Write};

use crate::mcts::{SearchInfo, MCTS};

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

pub fn main() {
    let stdin = Mutex::new(stdin_reader());
    let mut search_info = SearchInfo::new(&stdin);
    
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
                #[cfg(feature = "tuning")]
                print_uci_response(true);
                #[cfg(not(feature = "tuning"))]
                print_uci_response(false);
                PRETTY_PRINT.store(false, Ordering::SeqCst);
                Ok(())
            }
            "ucifull" => {
                print_uci_response(true);
                PRETTY_PRINT.store(false, Ordering::SeqCst);
                Ok(())
            }
            arg @ ("ucidump" | "ucidumpfull") => {
                // dump the values of the current UCI options
                println!("Hash: {}", tt.size() / MEGABYTE);
                println!("Threads: {}", thread_data.len());
                println!("PrettyPrint: {}", PRETTY_PRINT.load(Ordering::SeqCst));
                println!("UseNNUE: {}", USE_NNUE.load(Ordering::SeqCst));
                println!("SyzygyPath: {}", SYZYGY_PATH.lock().expect("failed to lock syzygy path"));
                println!("SyzygyProbeLimit: {}", SYZYGY_PROBE_LIMIT.load(Ordering::SeqCst));
                println!("SyzygyProbeDepth: {}", SYZYGY_PROBE_DEPTH.load(Ordering::SeqCst));
                // println!("MultiPV: {}", MULTI_PV.load(Ordering::SeqCst));
                if arg == "ucidumpfull" {
                    for (id, default) in SearchParams::default().ids_with_values() {
                        println!("{id}: {default}");
                    }
                }
                Ok(())
            }
            "isready" => {
                println!("readyok");
                Ok(())
            }
            "quit" => {
                info.quit = true;
                break;
            }
            "ucinewgame" => do_newgame(&mut pos, &tt, &mut thread_data),
            "eval" => {
                let eval = if pos.in_check::<{ Board::US }>() {
                    0
                } else {
                    pos.evaluate::<true>(
                        thread_data.first_mut().expect("the thread headers are empty."),
                        0,
                    )
                };
                println!("{eval}");
                Ok(())
            }
            "show" => {
                println!("{pos}");
                Ok(())
            }
            input if input.starts_with("setoption") => {
                let pre_config = SetOptions {
                    search_config: get_search_params().clone(),
                    hash_mb: None,
                    threads: None,
                };
                let res = parse_setoption(input, &mut info, pre_config);
                match res {
                    Ok(conf) => {
                        unsafe {
                            set_search_params(conf.search_config);
                        }
                        if let Some(hash_mb) = conf.hash_mb {
                            let new_size = hash_mb * MEGABYTE;
                            tt.resize(new_size);
                        }
                        if let Some(threads) = conf.threads {
                            thread_data = (0..threads).map(ThreadData::new).collect();
                            for t in &mut thread_data {
                                t.nnue.refresh_acc(&pos);
                                t.alloc_tables();
                            }
                        }
                        Ok(())
                    }
                    Err(err) => Err(err),
                }
            }
            input if input.starts_with("position") => {
                let res = parse_position(input, &mut pos);
                if res.is_ok() {
                    for t in &mut thread_data {
                        t.nnue.refresh_acc(&pos);
                    }
                }
                res
            }
            input if input.starts_with("go perft") || input.starts_with("perft") => {
                let tail = input.trim_start_matches("go perft ").trim_start_matches("perft ");
                match tail.split_whitespace().next() {
                    Some("divide" | "split") => {
                        let depth = tail.trim_start_matches("divide ").trim_start_matches("split ");
                        depth
                            .parse::<usize>()
                            .map_err(|_| UciError::InvalidFormat(format!("cannot parse \"{depth}\" as usize")))
                            .map(|depth| divide_perft(depth, &mut pos))
                    }
                    Some(depth) => {
                        depth
                            .parse::<usize>()
                            .map_err(|_| UciError::InvalidFormat(format!("cannot parse \"{depth}\" as usize")))
                            .map(|depth| block_perft(depth, &mut pos))
                    }
                    None => Err(UciError::InvalidFormat(
                        "expected a depth after 'go perft'".to_string(),
                    )),
                }
            }
            input if input.starts_with("go") => {
                let res = parse_go(input, &mut info, &mut pos, get_search_params());
                if res.is_ok() {
                    tt.increase_age();
                    if USE_NNUE.load(Ordering::SeqCst) {
                        pos.search_position::<true>(&mut info, &mut thread_data, tt.view());
                    } else {
                        pos.search_position::<false>(&mut info, &mut thread_data, tt.view());
                    }
                }
                res
            }
            benchcmd @ ("bench" | "benchfull") => 'bench: {
                info.print_to_stdout = false;
                let mut node_sum = 0u64;
                for fen in BENCH_POSITIONS {
                    let res = do_newgame(&mut pos, &tt, &mut thread_data);
                    if let Err(e) = res {
                        info.print_to_stdout = true;
                        break 'bench Err(e);
                    }
                    let res = parse_position(&format!("position fen {fen}\n"), &mut pos);
                    if let Err(e) = res {
                        info.print_to_stdout = true;
                        break 'bench Err(e);
                    }
                    for t in &mut thread_data {
                        t.nnue.refresh_acc(&pos);
                    }
                    let res = parse_go("go depth 12\n", &mut info, &mut pos, get_search_params());
                    if let Err(e) = res {
                        info.print_to_stdout = true;
                        break 'bench Err(e);
                    }
                    tt.increase_age();
                    if USE_NNUE.load(Ordering::SeqCst) {
                        pos.search_position::<true>(&mut info, &mut thread_data, tt.view());
                    } else {
                        pos.search_position::<false>(&mut info, &mut thread_data, tt.view());
                    }
                    node_sum += info.nodes;
                    if benchcmd == "benchfull" {
                        println!("{fen} has {} nodes", info.nodes);
                    }
                }
                println!("{node_sum} nodes");
                info.print_to_stdout = true;
                Ok(())
            }
            _ => Err(UciError::UnknownCommand(input.to_string())),
        };

        if let Err(e) = res {
            println!("info string {e}");
        }

        if info.quit {
            // quit can be set true in parse_go
            break;
        }
    }
    KEEP_RUNNING.store(false, atomic::Ordering::SeqCst);
}