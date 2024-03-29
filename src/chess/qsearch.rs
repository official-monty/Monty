use super::{board::Board, frc::Castling, moves::Move, value::Accumulator};

fn mvv_lva(mov: &Move, pos: &Board) -> i32 {
    8 * pos.get_pc(1 << mov.to()) as i32 - mov.moved() as i32
}

pub fn quiesce(
    pos: &Board,
    castling: &Castling,
    acc: &[Accumulator; 2],
    mut alpha: i32,
    beta: i32,
) -> i32 {
    let mut eval = pos.eval_from_acc(acc);

    // stand-pat
    if eval >= beta {
        return eval;
    }

    alpha = alpha.max(eval);

    let mut caps = pos.gen::<false>(castling);
    caps.sort_by_cached_key(|cap| mvv_lva(cap, pos));

    for &mov in caps.iter() {
        // static exchange eval pruning
        if !pos.see(&mov, 1) {
            continue;
        }

        let mut new = *pos;
        let mut new_acc = *acc;
        new.make(mov, Some(&mut new_acc), castling);

        let score = -quiesce(&new, castling, &new_acc, -beta, -alpha);

        eval = eval.max(score);
        alpha = alpha.max(eval);

        if eval >= beta {
            break;
        }
    }

    eval
}
