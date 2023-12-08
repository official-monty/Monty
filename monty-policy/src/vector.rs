use crate::activation::Activation;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Vector<const N: usize> {
    inner: [f32; N],
}

impl<const N: usize> std::ops::Index<usize> for Vector<N> {
    type Output = f32;
    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

impl<const N: usize> std::ops::Add<Vector<N>> for Vector<N> {
    type Output = Vector<N>;
    fn add(mut self, rhs: Vector<N>) -> Self::Output {
        for (i, j) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *i += *j;
        }

        self
    }
}

impl<const N: usize> std::ops::Add<f32> for Vector<N> {
    type Output = Vector<N>;
    fn add(mut self, rhs: f32) -> Self::Output {
        for i in self.inner.iter_mut() {
            *i += rhs;
        }

        self
    }
}

impl<const N: usize> std::ops::AddAssign<Vector<N>> for Vector<N> {
    fn add_assign(&mut self, rhs: Vector<N>) {
        for (i, j) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *i += *j;
        }
    }
}

impl<const N: usize> std::ops::Div<Vector<N>> for Vector<N> {
    type Output = Vector<N>;
    fn div(mut self, rhs: Vector<N>) -> Self::Output {
        for (i, j) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *i /= *j;
        }

        self
    }
}

impl<const N: usize> std::ops::Mul<Vector<N>> for Vector<N> {
    type Output = Vector<N>;
    fn mul(mut self, rhs: Vector<N>) -> Self::Output {
        for (i, j) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *i *= *j;
        }

        self
    }
}

impl<const N: usize> std::ops::Mul<Vector<N>> for f32 {
    type Output = Vector<N>;
    fn mul(self, mut rhs: Vector<N>) -> Self::Output {
        for i in rhs.inner.iter_mut() {
            *i *= self;
        }

        rhs
    }
}

impl<const N: usize> std::ops::SubAssign<Vector<N>> for Vector<N> {
    fn sub_assign(&mut self, rhs: Vector<N>) {
        for (i, j) in self.inner.iter_mut().zip(rhs.inner.iter()) {
            *i -= *j;
        }
    }
}

impl<const N: usize> Vector<N> {
    pub fn from_fn<F: FnMut(usize) -> f32>(mut f: F) -> Self {
        let mut res = Self::zeroed();

        for i in 0..N {
            res.inner[i] = f(i);
        }

        res
    }

    pub fn dot(&self, other: &Vector<N>) -> f32 {
        let mut score = 0.0;
        for (&i, &j) in self.inner.iter().zip(other.inner.iter()) {
            score += i * j;
        }

        score
    }

    pub fn out<T: Activation>(&self, other: &Vector<N>) -> f32 {
        let mut score = 0.0;
        for (i, j) in self.inner.iter().zip(other.inner.iter()) {
            score += T::activate(*i) * T::activate(*j);
        }

        score
    }

    pub fn sqrt(mut self) -> Self {
        for i in self.inner.iter_mut() {
            *i = i.sqrt();
        }

        self
    }

    pub const fn from_raw(inner: [f32; N]) -> Self {
        Self { inner }
    }

    pub const fn zeroed() -> Self {
        Self::from_raw([0.0; N])
    }

    pub fn activate<T: Activation>(mut self) -> Self {
        for i in self.inner.iter_mut() {
            *i = T::activate(*i);
        }

        self
    }

    pub fn derivative<T: Activation>(mut self) -> Self {
        for i in self.inner.iter_mut() {
            *i = T::derivative(*i);
        }

        self
    }

    pub fn adam(&mut self, mut g: Self, m: &mut Self, v: &mut Self, adj: f32, lr: f32) {
        const B1: f32 = 0.9;
        const B2: f32 = 0.999;

        g = adj * g;
        *m = B1 * *m + (1. - B1) * g;
        *v = B2 * *v + (1. - B2) * g * g;
        *self -= lr * *m / (v.sqrt() + 0.000_000_01);
    }
}
