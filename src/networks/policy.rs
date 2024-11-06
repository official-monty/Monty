use crate::chess::{Attacks, Board, Move};

use super::{accumulator::Accumulator, layer::Layer};

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const PolicyFileDefaultName: &str = "nn-7b30080083d5.network";


#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    l1: Layer<f32, { 768 * 4 }, 128>,
    l2: Layer<f32, 128, { 1880 * 2 }>,
}

impl PolicyNetwork {
    pub fn hl(&self, pos: &Board) -> Accumulator<f32, 128> {
        let mut res = self.l1.biases;

        pos.map_policy_features(|feat| res.add(&self.l1.weights[feat]));

        for elem in &mut res.0 {
            *elem = elem.clamp(0.0, 1.0).powi(2);
        }

        res
    }

    pub fn get(&self, pos: &Board, mov: &Move, hl: &Accumulator<f32, 128>) -> f32 {
        let idx = map_move_to_index(pos, *mov);      

        let mut res = self.l2.biases.0[idx];

        for (i, row) in self.l2.weights.iter().enumerate() {
            res += row.0[idx] * hl.0[i];
        }

        res
    }
}

const PROMOS: usize = 4 * 22;

fn map_move_to_index(pos: &Board, mov: Move) -> usize {
    let good_see = (OFFSETS[64] + PROMOS) * usize::from(pos.see(&mov, -108));

    let idx = if mov.is_promo() {
        let ffile = mov.src() % 8;
        let tfile = mov.to() % 8;
        let promo_id = 2 * ffile + tfile;

        OFFSETS[64] + 22 * (mov.promo_pc() - 3) + usize::from(promo_id)
    } else {
        let flip = if pos.stm() == 1 { 56 } else { 0 };
        let from = usize::from(mov.src() ^ flip);
        let dest = usize::from(mov.to() ^ flip);
    
        let below = Attacks::ALL_DESTINATIONS[from] & ((1 << dest) - 1);
    
        OFFSETS[from] + below.count_ones() as usize
    };

    good_see + idx
}

const OFFSETS: [usize; 65] = {
    let mut offsets = [0; 65];

    let mut curr = 0;
    let mut sq = 0;

    while sq < 64 {
        offsets[sq] = curr;
        curr += Attacks::ALL_DESTINATIONS[sq].count_ones() as usize;
        sq += 1;
    }

    offsets[64] = curr;

    offsets
};
