use crate::Board;

use super::{accumulator::Accumulator, activation::Activation};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Layer<T: Copy, const M: usize, const N: usize> {
    weights: [Accumulator<T, N>; M],
    biases: Accumulator<T, N>,
}

impl<const M: usize, const N: usize> Layer<i16, M, N> {
    pub fn forward(&self, board: &Board) -> Accumulator<i16, N> {
        let mut count = 0;
        let mut feats = [0; 32];
        board.map_value_features(|feat| {
            feats[count] = feat;
            count += 1;
        });

        let mut out = self.biases;

        out.add_multi(&feats[..count], &self.weights);

        out
    }

    pub fn forward_from_slice(&self, feats: &[usize]) -> Accumulator<i16, N> {
        let mut out = self.biases;

        for &feat in feats {
            out.add(&self.weights[feat])
        }

        out
    }
}

impl<const M: usize, const N: usize> Layer<f32, M, N> {
    pub fn forward<T: Activation>(&self, inputs: &Accumulator<f32, M>) -> Accumulator<f32, N> {
        let mut fwd = self.biases;

        for (i, d) in inputs.0.iter().zip(self.weights.iter()) {
            let act = T::activate(*i);
            fwd.madd(act, d);
        }

        fwd
    }

    pub fn quantise_into_i16(&self, dest: &mut Layer<i16, M, N>, qa: i16, warn_limit: f32) {
        for (acc_i, acc_j) in dest.weights.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise_i16(qa, warn_limit);
        }

        dest.biases = self.biases.quantise_i16(qa, warn_limit);
    }

    pub fn quantise_i16(&self, qa: i16, warn_limit: f32) -> Layer<i16, M, N> {
        let mut res = Layer {
            weights: [Accumulator([0; N]); M],
            biases: Accumulator([0; N]),
        };

        self.quantise_into_i16(&mut res, qa, warn_limit);

        res
    }

    pub fn quantise_transpose_into_i16(
        &self,
        dest: &mut TransposedLayer<i16, M, N>,
        qa: i16,
        warn_limit: f32,
    ) {
        let mut untrans = [Accumulator([0; N]); M];

        for (acc_i, acc_j) in untrans.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise_i16(qa, warn_limit);
        }

        for i in 0..N {
            for (j, row) in untrans.iter().enumerate() {
                dest.weights[i].0[j] = row.0[i];
            }
        }

        dest.biases = self.biases.quantise_i16(qa, warn_limit);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TransposedLayer<T: Copy, const M: usize, const N: usize> {
    weights: [Accumulator<T, M>; N],
    biases: Accumulator<T, N>,
}

impl<const M: usize, const N: usize> TransposedLayer<i16, M, N> {
    pub fn forward_from_i16<T: Activation, const QA: i16, const QB: i16, const FACTOR: i16>(
        &self,
        inputs: &Accumulator<i16, M>,
    ) -> Accumulator<f32, N> {
        let mut act = [0; M];

        for (a, &i) in act.iter_mut().zip(inputs.0.iter()) {
            *a = (i32::from(i).clamp(0, i32::from(QA)).pow(2) / i32::from(QA / FACTOR)) as i16;
        }

        let mut fwd = [0; N];

        for (f, row) in fwd.iter_mut().zip(self.weights.iter()) {
            for (&a, &w) in act.iter().zip(row.0.iter()) {
                *f += i32::from(a) * i32::from(w);
            }
        }

        let mut res = [0.0; N];

        for (r, (&f, &b)) in res.iter_mut().zip(fwd.iter().zip(self.biases.0.iter())) {
            *r = (f as f32 / f32::from(QA * FACTOR) + f32::from(b)) / f32::from(QB);
        }

        Accumulator(res)
    }
}
