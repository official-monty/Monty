use crate::Board;

use super::{accumulator::Accumulator, activation::Activation, QA};

#[derive(Clone, Copy)]
pub struct Layer<T: Copy, const M: usize, const N: usize> {
    weights: [Accumulator<T, N>; M],
    biases: Accumulator<T, N>,
}

impl<const M: usize, const N: usize> Layer<i16, M, N> {
    pub fn forward(&self, board: &Board) -> Accumulator<i16, N> {
        let mut out = self.biases;

        board.map_value_features(|feat| out.add(&self.weights[feat]));

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

    pub fn forward_from_i16<T: Activation>(
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

    pub fn quantise_into(&self, dest: &mut Layer<i16, M, N>, qa: i16) {
        for (acc_i, acc_j) in dest.weights.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise(qa);
        }

        dest.biases = self.biases.quantise(qa);
    }

    pub fn quantise(&self, qa: i16) -> Layer<i16, M, N> {
        let mut res = Layer {
            weights: [Accumulator([0; N]); M],
            biases: Accumulator([0; N]),
        };

        self.quantise_into(&mut res, qa);

        res
    }
}
