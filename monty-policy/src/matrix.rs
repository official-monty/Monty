use crate::Vector;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Matrix<const M: usize, const N: usize> {
    inner: [Vector<N>; M],
}

impl<const M: usize, const N: usize> std::ops::AddAssign<Matrix<M, N>> for Matrix<M, N> {
    fn add_assign(&mut self, rhs: Matrix<M, N>) {
        for (u, v) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *u += *v;
        }
    }
}

impl<const M: usize, const N: usize> std::ops::Deref for Matrix<M, N> {
    type Target = [Vector<N>; M];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<const M: usize, const N: usize> std::ops::DerefMut for Matrix<M, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<const M: usize, const N: usize> std::ops::Mul<Vector<N>> for Matrix<M, N> {
    type Output = Vector<M>;
    fn mul(self, rhs: Vector<N>) -> Self::Output {
        Vector::<M>::from_fn(|i| self.inner[i].dot(&rhs))
    }
}

impl<const M: usize, const N: usize> Matrix<M, N> {
    pub const fn zeroed() -> Self {
        Self::from_raw([Vector::zeroed(); M])
    }

    pub const fn from_raw(inner: [Vector<N>; M]) -> Self {
        Self { inner }
    }

    pub fn transpose_mul(&self, out: Vector<M>) -> Vector<N> {
        Vector::from_fn(|i| {
            let mut v = 0.0;
            for j in 0..M {
                v += self.inner[j][i] * out[j];
            }
            v
        })
    }

    pub fn adam(&mut self, g: &Self, m: &mut Self, v: &mut Self, adj: f32, lr: f32) {
        for i in 0..M {
            self.inner[i].adam(
                g.inner[i],
                &mut m.inner[i],
                &mut v.inner[i],
                adj,
                lr,
            );
        }
    }
}
