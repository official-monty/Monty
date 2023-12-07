use std::marker::PhantomData;

use crate::{activation::Activation, Vector};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SubNet<T: Activation, const N: usize, const FEATS: usize> {
    ft: [Vector<N>; FEATS],
    phantom: PhantomData<T>,
}

impl<T: Activation, const N: usize, const FEATS: usize> std::ops::AddAssign<&SubNet<T, N, FEATS>>
    for SubNet<T, N, FEATS>
{
    fn add_assign(&mut self, rhs: &SubNet<T, N, FEATS>) {
        for (u, v) in self.ft.iter_mut().zip(rhs.ft.iter()) {
            *u += *v;
        }
    }
}

impl<T: Activation, const N: usize, const FEATS: usize> SubNet<T, N, FEATS> {
    pub fn from_fn<F: FnMut(usize) -> Vector<N>>(mut f: F) -> Self {
        let mut res = Self {
            ft: [Vector::zeroed(); FEATS],
            phantom: PhantomData,
        };

        for (i, v) in res.ft.iter_mut().enumerate() {
            *v = f(i);
        }

        res
    }

    pub fn out(&self, feats: &[usize]) -> Vector<N> {
        self.ft(feats).activate::<T>()
    }

    pub fn ft(&self, feats: &[usize]) -> Vector<N> {
        let mut res = Vector::<N>::zeroed();

        for &feat in feats {
            res += self.ft[feat];
        }

        res
    }

    pub fn backprop(
        &self,
        feats: &[usize],
        factor: f32,
        grad: &mut Self,
        other: Vector<N>,
        ft: Vector<N>,
    ) {
        let adj = factor * other * ft.derivative::<T>();
        for &feat in feats.iter() {
            grad.ft[feat] += adj;
        }
    }

    pub fn adam(
        &mut self,
        grad: &Self,
        momentum: &mut Self,
        velocity: &mut Self,
        adj: f32,
        lr: f32,
    ) {
        const B1: f32 = 0.9;
        const B2: f32 = 0.999;

        for i in 0..FEATS {
            let g = adj * grad.ft[i];
            let m = &mut momentum.ft[i];
            let v = &mut velocity.ft[i];
            let p = &mut self.ft[i];

            *m = B1 * *m + (1. - B1) * g;
            *v = B2 * *v + (1. - B2) * g * g;
            *p -= lr * *m / (v.sqrt() + 0.000_000_01);
        }
    }
}
