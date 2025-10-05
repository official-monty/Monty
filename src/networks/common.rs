use std::ops::{AddAssign, Mul};

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

impl<T: AddAssign<T> + Copy + Mul<T, Output = T> + From<i16>, const N: usize> Accumulator<T, N> {
    pub fn madd_i16(&mut self, mul: T, other: &Accumulator<i16, N>) {
        for (i, &j) in self.0.iter_mut().zip(other.0.iter()) {
            *i += mul * T::from(j);
        }
    }
}

impl<const N: usize> Accumulator<i16, N> {
    pub fn add_multi(&mut self, adds: &[usize], weights: &[Self]) {
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
                    *reg += this_weight.0[offset + j];
                }
            }

            for (j, reg) in regs.iter().enumerate() {
                self.0[offset + j] = *reg;
            }
        }
    }
}

impl<const N: usize> Accumulator<i16, N> {
    pub fn add_multi_i8(&mut self, adds: &[usize], weights: &[Accumulator<i8, N>]) {
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

impl<const N: usize> Accumulator<i16, N> {
    pub fn dot<T: Activation, const QA: i16>(&self, other: &Self) -> f32 {
        let mut res = 0.0;

        for (i, j) in self.0.iter().zip(other.0.iter()) {
            let i = f32::from(*i);
            let j = f32::from(*j);
            res += T::activate(i) * T::activate(j);
        }

        res / f32::from(QA) / f32::from(QA)
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

    pub fn quantise_i16(&self, qa: i16, warn_limit: f32) -> Accumulator<i16, N> {
        let mut res = Accumulator([0; N]);

        for (i, &j) in res.0.iter_mut().zip(self.0.iter()) {
            if j.abs() > warn_limit {
                println!("WARNING: {j} > {warn_limit}")
            }

            let unq = j * f32::from(qa);
            *i = unq as i16;

            assert_eq!(unq.trunc(), f32::from(*i));
        }

        res
    }

    pub fn quantise_i8(&self, qa: i16, warn_limit: f32) -> Accumulator<i8, N> {
        let mut res = Accumulator([0; N]);

        for (i, &j) in res.0.iter_mut().zip(self.0.iter()) {
            if j.abs() > warn_limit {
                println!("WARNING: {j} > {warn_limit}")
            }

            let unq = j * f32::from(qa);
            *i = unq as i8;

            assert_eq!(unq.trunc(), f32::from(*i));
        }

        res
    }
}

pub trait Activation {
    fn activate(x: f32) -> f32;
}

pub struct SCReLU;
impl Activation for SCReLU {
    #[inline]
    fn activate(x: f32) -> f32 {
        x.clamp(0.0, 1.0).powi(2)
    }
}

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
