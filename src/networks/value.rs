use crate::chess::Board;

use super::{
    activation::SCReLU,
    layer::{Layer, TransposedLayer},
    threats, Accumulator,
};

// DO NOT MOVE
#[allow(non_upper_case_globals, dead_code)]
pub const ValueFileDefaultName: &str = "nn-5601bb8c241d.network";
#[allow(non_upper_case_globals, dead_code)]
pub const CompressedValueName: &str = "nn-f004da0ebf25.network";
#[allow(non_upper_case_globals, dead_code)]
pub const DatagenValueFileName: &str = "nn-5601bb8c241d.network";

const QA: i16 = 512;
const QB: i16 = 1024;
const FACTOR: i16 = 32;

const L1: usize = 3072;

#[repr(C)]
pub struct ValueNetwork {
    pst: [Accumulator<f32, 3>; threats::TOTAL],
    l1: Layer<i16, { threats::TOTAL }, L1>,
    l2: TransposedLayer<i16, { L1 / 2 }, 16>,
    l3: Layer<f32, 16, 128>,
    l4: Layer<f32, 128, 3>,
}

impl ValueNetwork {
    pub fn eval(&self, board: &Board) -> (f32, f32, f32) {
        let mut pst = Accumulator([0.0; 3]);

        let mut count = 0;
        let mut feats = [0; 160];
        threats::map_features(board, |feat| {
            feats[count] = feat;
            pst.add(&self.pst[feat]);
            count += 1;
        });

        let mut l2 = self.l1.biases;

        l2.add_multi(&feats[..count], &self.l1.weights);

        let mut act = [0; L1 / 2];

        for (a, (&i, &j)) in act
            .iter_mut()
            .zip(l2.0.iter().take(L1 / 2).zip(l2.0.iter().skip(L1 / 2)))
        {
            let i = i32::from(i).clamp(0, i32::from(QA));
            let j = i32::from(j).clamp(0, i32::from(QA));
            *a = ((i * j) / i32::from(QA / FACTOR)) as i16;
        }

        let mut fwd = [0; 16];

        for (f, row) in fwd.iter_mut().zip(self.l2.weights.iter()) {
            for (&a, &w) in act.iter().zip(row.0.iter()) {
                *f += i32::from(a) * i32::from(w);
            }
        }

        let mut l3 = Accumulator([0.0; 16]);

        for (r, (&f, &b)) in l3.0.iter_mut().zip(fwd.iter().zip(self.l2.biases.0.iter())) {
            *r = (f as f32 / f32::from(QA * FACTOR) + f32::from(b)) / f32::from(QB);
        }

        let l4 = self.l3.forward::<SCReLU>(&l3);
        let mut out = self.l4.forward::<SCReLU>(&l4);
        out.add(&pst);

        let mut win = out.0[2];
        let mut draw = out.0[1];
        let mut loss = out.0[0];

        let max = win.max(draw).max(loss);

        win = (win - max).exp();
        draw = (draw - max).exp();
        loss = (loss - max).exp();

        let sum = win + draw + loss;

        (win / sum, draw / sum, loss / sum)
    }
}
