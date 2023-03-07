use std::f64::consts::{LN_10, PI};

pub struct Difference {
    pub difference: f64,
    pub error: f64,
}

pub fn difference(wins: i32, losses: i32, draws: i32) -> Difference {
    let wins = f64::from(wins);
    let losses = f64::from(losses);
    let draws = f64::from(draws);
    let score = wins + draws / 2.0;
    let total = wins + draws + losses;
    let percentage = score / total;
    let difference = -400.0 * (1.0 / percentage - 1.0).ln() / LN_10;

    let error = error_margin(wins, draws, losses);
    Difference { difference, error }
}

fn error_margin(wins: f64, draws: f64, losses: f64) -> f64 {
    let total = wins + draws + losses;
    let win_p = wins / total;
    let draw_p = draws / total;
    let loss_p = losses / total;
    let percentage = draws.mul_add(0.5, wins) / total;
    let wins_dev = win_p * f64::powf(1.0 - percentage, 2.0);
    let draws_dev = draw_p * f64::powf(0.5 - percentage, 2.0);
    let losses_dev = loss_p * f64::powf(0.0 - percentage, 2.0);
    let std_deviation = f64::sqrt(wins_dev + draws_dev + losses_dev) / f64::sqrt(total);

    let confidence_p = 0.95;
    let min_confidence_p = (1.0 - confidence_p) / 2.0;
    let max_confidence_p = 1.0 - min_confidence_p;
    let dev_min = phi_inv(min_confidence_p).mul_add(std_deviation, percentage);
    let dev_max = phi_inv(max_confidence_p).mul_add(std_deviation, percentage);

    let difference = elo_diff_from_percent(dev_max) - elo_diff_from_percent(dev_min);

    difference / 2.0
}

fn elo_diff_from_percent(percentage: f64) -> f64 {
    -400.0 * (1.0 / percentage - 1.0).ln() / LN_10
}

fn phi_inv(p: f64) -> f64 {
    f64::sqrt(2.0) * inverse_error(2.0f64.mul_add(p, -1.0))
}

fn inverse_error(x: f64) -> f64 {
    let pi = PI;
    let a = 8.0 * (pi - 3.0) / (3.0 * pi * (4.0 - pi));
    let y = x.mul_add(-x, 1.0).ln();
    let z = 2.0 / (pi * a) + y / 2.0;

    let ret = f64::sqrt(f64::sqrt(z.mul_add(z, -y / a)) - z);

    if x < 0.0 {
        return -ret;
    }

    ret
}
