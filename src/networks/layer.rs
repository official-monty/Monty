use crate::Board;

use super::{accumulator::Accumulator, activation::Activation};

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

    pub fn forward_from_i16<T: Activation, const QA: i16>(
        &self,
        inputs: &Accumulator<i16, M>,
    ) -> Accumulator<f32, N> {
        let mut act = [0; M];

        for (a, &i) in act.iter_mut().zip(inputs.0.iter()) {
            *a = (i32::from(i).clamp(0, i32::from(QA)).pow(2) / i32::from(QA)) as i16;
        }

        let mut fwd = [0; N];

        for i in 0..N {
            for j in 0..M {
                fwd[i] += i32::from(act[j]) * i32::from(self.weights[j].0[i]);
            }
        }

        let mut res = [0.0; N];

        for (r, (&f, &b)) in res.iter_mut().zip(fwd.iter().zip(self.biases.0.iter())) {
            *r = (f as f32 / f32::from(QA) + f32::from(b)) / f32::from(QA);
        }

        Accumulator(res)
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

    pub fn forward_from_i16<T: Activation, const QA: i16>(
        &self,
        inputs: &Accumulator<i16, M>,
    ) -> Accumulator<f32, N> {
        let mut fwd = self.biases;

        for (i, d) in inputs.0.iter().zip(self.weights.iter()) {
            let act = T::activate(f32::from(*i) / f32::from(QA));
            fwd.madd(act, d);
        }

        fwd
    }

    pub fn quantise_into_i16(&self, dest: &mut Layer<i16, M, N>, qa: i16) {
        for (acc_i, acc_j) in dest.weights.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise_i16(qa);
        }

        dest.biases = self.biases.quantise_i16(qa);
    }

    pub fn quantise_i16(&self, qa: i16) -> Layer<i16, M, N> {
        let mut res = Layer {
            weights: [Accumulator([0; N]); M],
            biases: Accumulator([0; N]),
        };

        self.quantise_into_i16(&mut res, qa);

        res
    }
}
