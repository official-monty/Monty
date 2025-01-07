use super::{accumulator::Accumulator, activation::Activation};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Layer<T: Copy, const M: usize, const N: usize> {
    pub weights: [Accumulator<T, N>; M],
    pub biases: Accumulator<T, N>,
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

    pub fn quantise_into_i8(&self, dest: &mut Layer<i8, M, N>, qa: i16, warn_limit: f32) {
        for (acc_i, acc_j) in dest.weights.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise_i8(qa, warn_limit);
        }

        dest.biases = self.biases.quantise_i8(qa, warn_limit);
    }

    pub fn quantise_transpose_into_i8(
        &self,
        dest: &mut TransposedLayer<i8, M, N>,
        qa: i16,
        warn_limit: f32,
    ) {
        let mut untrans = vec![Accumulator([0; N]); M];

        for (acc_i, acc_j) in untrans.iter_mut().zip(self.weights.iter()) {
            *acc_i = acc_j.quantise_i8(qa, warn_limit);
        }

        for i in 0..N {
            for (j, row) in untrans.iter().enumerate() {
                dest.weights[i].0[j] = row.0[i];
            }
        }

        dest.biases = self.biases.quantise_i8(qa, warn_limit);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TransposedLayer<T: Copy, const M: usize, const N: usize> {
    pub weights: [Accumulator<T, M>; N],
    pub biases: Accumulator<T, N>,
}
