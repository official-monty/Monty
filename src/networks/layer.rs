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
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TransposedLayer<T: Copy, const M: usize, const N: usize> {
    pub weights: [Accumulator<T, M>; N],
    pub biases: Accumulator<T, N>,
}
