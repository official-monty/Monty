use crate::{
    boxed_and_zeroed,
    chess::{Attacks, Board, Move},
};

use super::{
    accumulator::Accumulator,
    layer::{Layer, TransposedLayer},
};

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const PolicyFileDefaultName: &str = "nn-9f4927501e11.network";

const QA: i16 = 128;
const QB: i16 = 128;
const FACTOR: i16 = 32;

pub const L1: usize = 4096;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    l1: Layer<i8, { 768 * 4 }, L1>,
    l2: TransposedLayer<i8, { L1 / 2 }, { 1880 * 2 }>,
}

impl PolicyNetwork {
    pub fn hl(&self, pos: &Board) -> Accumulator<i16, { L1 / 2 }> {
        let mut l1 = Accumulator([0; L1]);

        for (r, &b) in l1.0.iter_mut().zip(self.l1.biases.0.iter()) {
            *r = i16::from(b);
        }

        pos.map_features(|feat| {
            for (r, &w) in l1.0.iter_mut().zip(self.l1.weights[feat].0.iter()) {
                *r += i16::from(w);
            }
        });

        let mut res = Accumulator([0; L1 / 2]);

        for (elem, (&i, &j)) in res
            .0
            .iter_mut()
            .zip(l1.0.iter().take(L1 / 2).zip(l1.0.iter().skip(L1 / 2)))
        {
            let i = i32::from(i).clamp(0, i32::from(QA));
            let j = i32::from(j).clamp(0, i32::from(QA));
            *elem = ((i * j) / i32::from(QA / FACTOR)) as i16;
        }

        res
    }

    pub fn get(&self, pos: &Board, mov: &Move, hl: &Accumulator<i16, { L1 / 2 }>) -> f32 {
        let idx = map_move_to_index(pos, *mov);
        let weights = &self.l2.weights[idx];

        let mut res = 0;

        for (&w, &v) in weights.0.iter().zip(hl.0.iter()) {
            res += i32::from(w) * i32::from(v);
        }

        (res as f32 / f32::from(QA * FACTOR) + f32::from(self.l2.biases.0[idx])) / f32::from(QB)
    }
}

const PROMOS: usize = 4 * 22;

fn map_move_to_index(pos: &Board, mov: Move) -> usize {
    let hm = if pos.king_index() % 8 > 3 { 7 } else { 0 };
    let good_see = (OFFSETS[64] + PROMOS) * usize::from(pos.see(&mov, -108));

    let idx = if mov.is_promo() {
        let ffile = (mov.src() ^ hm) % 8;
        let tfile = (mov.to() ^ hm) % 8;
        let promo_id = 2 * ffile + tfile;

        OFFSETS[64] + 22 * (mov.promo_pc() - 3) + usize::from(promo_id)
    } else {
        let flip = if pos.stm() == 1 { 56 } else { 0 };
        let from = usize::from(mov.src() ^ flip ^ hm);
        let dest = usize::from(mov.to() ^ flip ^ hm);

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

#[repr(C)]
pub struct UnquantisedPolicyNetwork {
    l1: Layer<f32, { 768 * 4 }, L1>,
    l2: Layer<f32, { L1 / 2 }, { 1880 * 2 }>,
}

impl UnquantisedPolicyNetwork {
    pub fn quantise(&self) -> Box<PolicyNetwork> {
        let mut quantised: Box<PolicyNetwork> = unsafe { boxed_and_zeroed() };

        self.l1.quantise_into_i8(&mut quantised.l1, QA, 0.99);
        self.l2
            .quantise_transpose_into_i8(&mut quantised.l2, QB, 0.99);

        quantised
    }
}
