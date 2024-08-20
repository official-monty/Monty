use std::ops::{AddAssign, Mul};

use super::activation::Activation;

#[repr(C)]
#[derive(Clone, Copy)]
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

impl<const N: usize> Accumulator<i16, N> {
    pub fn add_multi(&mut self, adds: &[usize], weights: &[Accumulator<i8, N>]) {
        const REGS: usize = 8;
        const PER: usize = REGS * 16;

        let mut regs = [0i16; PER];

        for i in 0..N / PER {
            let offset = PER * i;

            for (j, reg) in regs.iter_mut().enumerate() {
                *reg = self.0[offset + j];
            }

            for &add in adds {
                let this_weight = &weights[add];

                for (j, reg) in regs.iter_mut().enumerate() {
                    *reg += i16::from(this_weight.0[offset + j]);
                }
            }

            for (j, reg) in regs.iter().enumerate() {
                self.0[offset + j] = *reg;
            }
        }
    }
}

impl<const N: usize> Accumulator<f32, N> {
    pub fn dot<T: Activation>(&self, other: &Self) -> f32 {
        let mut res = 0.0;

        for (i, j) in self.0.iter().zip(other.0.iter()) {
            res += T::activate(*i) * T::activate(*j);
        }

        res
    }

    pub fn quantise_i16(&self, qa: i16) -> Accumulator<i16, N> {
        let mut res = Accumulator([0; N]);

        for (i, &j) in res.0.iter_mut().zip(self.0.iter()) {
            if j > 1.98 {
                println!("{j}")
            }

            *i = (j * f32::from(qa)) as i16;
        }

        res
    }

    pub fn quantise_i8(&self, qa: i8) -> Accumulator<i8, N> {
        let mut res = Accumulator([0; N]);

        for (i, &j) in res.0.iter_mut().zip(self.0.iter()) {
            if j > 1.98 {
                println!("{j}")
            }

            *i = (j * f32::from(qa)) as i8;
        }

        res
    }
}
