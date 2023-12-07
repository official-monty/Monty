use crate::Vector;

#[derive(Clone, Copy)]
pub struct Matrix<const M: usize, const N: usize> {
    inner: [Vector<N>; M],
}

impl<const M: usize, const N: usize> std::ops::Mul<Vector<N>> for Matrix<M, N> {
    type Output = Vector<M>;
    fn mul(self, rhs: Vector<N>) -> Self::Output {
        Vector::<M>::from_fn(|i| self.inner[i].dot(&rhs))
    }
}

impl<const M: usize, const N: usize> Matrix<M, N> {
    pub const fn from_raw(inner: [Vector<N>; M]) -> Self {
        Self { inner }
    }
}
