pub const WIN_SCORE: f32 = 1.0;
pub const DRAW_SCORE: f32 = WIN_SCORE / 2.0;

pub const TREE_PRINT_DEPTH: usize = 2;
pub const MAX_NODEPOOL_MEM: usize = 2 * 1024 * 1024 * 1024; // 2GB
pub const ROOT_IDX: usize = 0;

pub const INF: i32 = i32::max_value();
pub const N_INF: i32 = -INF;

pub const DEFAULT_EXP_FACTOR: f64 = std::f64::consts::SQRT_2 * WIN_SCORE as f64;
pub const NODE_UNVISITED_VALUE: f64 = f64::MAX;
