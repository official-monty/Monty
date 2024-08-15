use std::ops::{AddAssign, Mul};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct Accumulator<T: Copy, const N: usize>(pub [T; N]);

impl<T: AddAssign<T> + Copy + Mul<T, Output = T>, const N: usize> Accumulator<T, N> {
    pub fn add(&mut self, other: &Self) {
        for (i, &j) in self.0.iter_mut().zip(other.0.iter()) {
            *i += j;
        }
    }

    pub fn madd(&mut self, mul: T, other: &Self) {
        for (i, &j) in self.0.iter_mut().zip(other.0.iter()) {
            *i += mul * j;
        }
    }
}

impl<const N: usize> Accumulator<f32, N> {
    pub fn quantise(&self, qa: i16) -> Accumulator<i16, N> {
        let mut res = Accumulator([0; N]);

        for (i, j) in res.0.iter_mut().zip(self.0.iter()) {
            *i = (*j * f32::from(qa)) as i16;
        }

        res
    }
}
