use std::marker::PhantomData;

use crate::{activation::Activation, Matrix, Vector};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Layer<T: Activation, const M: usize, const N: usize> {
    weights: Matrix<N, M>,
    bias: Vector<N>,
    phantom: PhantomData<T>,
}

impl<T: Activation, const M: usize, const N: usize> std::ops::AddAssign<Layer<T, M, N>> for Layer<T, M, N> {
    fn add_assign(&mut self, rhs: Layer<T, M, N>) {
        self.weights += rhs.weights;
        self.bias += rhs.bias;
    }
}

impl<T: Activation, const M: usize, const N: usize> Layer<T, M, N> {
    pub const fn from_raw(weights: Matrix<N, M>, bias: Vector<N>) -> Self {
        Self { weights, bias, phantom: PhantomData }
    }

    pub fn out(&self, inp: Vector<M>) -> Vector<N> {
        (self.weights * inp + self.bias).activate::<T>()
    }

    pub fn transpose_mul(&self, out: Vector<N>) -> Vector<M> {
        self.weights.transpose_mul(out)
    }

    pub fn backprop(
        &self,
        grad: &mut Self,
        mut cumulated: Vector<N>,
        inp: Vector<M>,
        out: Vector<N>,
    ) -> Vector<M> {
        cumulated = cumulated * out.derivative::<T>();

        for (i, row) in grad.weights.iter_mut().enumerate() {
            *row += cumulated[i] * inp;
        }

        grad.bias += cumulated;
        self.transpose_mul(cumulated)
    }

    pub fn adam(
        &mut self,
        grad: &Self,
        momentum: &mut Self,
        velocity: &mut Self,
        adj: f32,
        lr: f32,
    ) {
        self.weights.adam(&grad.weights, &mut momentum.weights, &mut velocity.weights, adj, lr);

        self.bias.adam(grad.bias, &mut momentum.bias, &mut velocity.bias, adj, lr);
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SparseLayer<T: Activation, const M: usize, const N: usize> {
    weights: Matrix<M, N>,
    bias: Vector<N>,
    phantom: PhantomData<T>,
}

impl<T: Activation, const M: usize, const N: usize> std::ops::AddAssign<SparseLayer<T, M, N>> for SparseLayer<T, M, N> {
    fn add_assign(&mut self, rhs: SparseLayer<T, M, N>) {
        self.weights += rhs.weights;
        self.bias += rhs.bias;
    }
}

impl<T: Activation, const M: usize, const N: usize> SparseLayer<T, M, N> {
    pub const fn from_raw(weights: Matrix<M, N>, bias: Vector<N>) -> Self {
        Self { weights, bias, phantom: PhantomData }
    }

    pub fn out(&self, feats: &[usize]) -> Vector<N> {
        let mut res = self.bias;

        for &feat in feats {
            res += self.weights[feat];
        }

        res.activate::<T>()
    }

    pub fn backprop(
        &self,
        grad: &mut Self,
        mut cumulated: Vector<N>,
        feats: &[usize],
        ft: Vector<N>,
    ) {
        cumulated = cumulated * ft.derivative::<T>();

        for &feat in feats.iter() {
            grad.weights[feat] += cumulated;
        }

        grad.bias += cumulated;
    }

    pub fn adam(
        &mut self,
        grad: &Self,
        momentum: &mut Self,
        velocity: &mut Self,
        adj: f32,
        lr: f32,
    ) {
        self.weights.adam(&grad.weights, &mut momentum.weights, &mut velocity.weights, adj, lr);

        self.bias.adam(grad.bias, &mut momentum.bias, &mut velocity.bias, adj, lr);
    }
}
