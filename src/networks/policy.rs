mod inputs;
mod outputs;
mod see;

use montyformat::chess::{Move, Position};

use super::common::{Accumulator, Layer, TransposedLayer};

// DO NOT MOVE
#[allow(non_upper_case_globals, dead_code)]
pub const PolicyFileDefaultName: &str = "nn-06e27b5ef6e7.network";
#[allow(non_upper_case_globals, dead_code)]
pub const CompressedPolicyName: &str = "nn-bef5cb915ecf.network";
#[allow(non_upper_case_globals, dead_code)]
pub const DatagenPolicyFileName: &str = "nn-06e27b5ef6e7.network";

const QA: i16 = 128;
const QB: i16 = 128;
const FACTOR: i16 = 32;

#[cfg(not(feature = "datagen"))]
pub const L1: usize = 16384;

#[cfg(feature = "datagen")]
pub const L1: usize = 16384;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    l1: Layer<i8, { 768 * 4 }, L1>,
    l2: TransposedLayer<i8, { L1 / 2 }, { outputs::NUM_MOVES_INDICES }>,
}

impl PolicyNetwork {
    pub fn hl(&self, pos: &Position) -> Accumulator<i16, { L1 / 2 }> {
        let mut l1 = Accumulator([0; L1]);

        for (r, &b) in l1.0.iter_mut().zip(self.l1.biases.0.iter()) {
            *r = i16::from(b);
        }

        let mut feats = [0usize; 256];
        let mut count = 0;
        inputs::map_features(pos, |feat| {
            feats[count] = feat;
            count += 1;
        });

        l1.add_multi_i8(&feats[..count], &self.l1.weights);

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

    pub fn get(&self, pos: &Position, mov: &Move, hl: &Accumulator<i16, { L1 / 2 }>) -> f32 {
        let idx = outputs::map_move_to_index(pos, *mov);
        let weights = &self.l2.weights[idx];

        let mut res = 0;

        for (&w, &v) in weights.0.iter().zip(hl.0.iter()) {
            res += i32::from(w) * i32::from(v);
        }

        (res as f32 / f32::from(QA * FACTOR) + f32::from(self.l2.biases.0[idx])) / f32::from(QB)
    }
}
