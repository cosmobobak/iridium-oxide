pub const WIN_SCORE: f32 = 10.0;

pub const TREE_PRINT_DEPTH: usize = 2;
pub const MAX_NODEPOOL_MEM: usize = 4 * 1024 * 1024 * 1024; // 4GB
pub const ROOT_IDX: usize = 0;

pub const INF: i32 = i32::max_value();
pub const N_INF: i32 = -INF;

pub const EXPLORATION_FACTOR: f64 = 5.0 * WIN_SCORE as f64;
pub const NODE_UNVISITED_VALUE: f64 = f64::MAX;
