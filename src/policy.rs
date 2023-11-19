use crate::{consts::Flag, moves::Move, params::TunableParams, position::Position};

pub fn get_policy(
    mov: &Move,
    pos: &Position,
    pawn_threats: u64,
    params: &TunableParams
) -> f64 {
    let mut score = 0.0;

    if pos.see(mov, -108) {
        score += params.good_see()
    }

    if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
        score += params.promo();
    }

    if mov.is_capture() {
        score += params.cap();

        let diff = pos.get_pc(1 << mov.to()) as i32 - i32::from(mov.moved());
        score += params.mvv_lva() * f64::from(diff);
    }

    if pawn_threats & (1 << mov.to()) == 0 {
        score += params.pawn_threat();
    }

    score
}
