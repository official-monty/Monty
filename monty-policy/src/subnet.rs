use crate::{Vector, Matrix, ReLU, SparseLayer};

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SubNet {
    ft: SparseLayer<ReLU, 768, 16>,
}

impl std::ops::AddAssign<&SubNet> for SubNet {
    fn add_assign(&mut self, rhs: &SubNet) {
        self.ft += rhs.ft;
    }
}

impl SubNet {
    pub fn out(&self, feats: &[usize]) -> Vector<16> {
        self.ft.out(feats)
    }

    pub fn out_with_layers(&self, feats: &[usize]) -> Vector<16> {
        self.ft.out(feats)
    }

    pub fn backprop(
        &self,
        feats: &[usize],
        factor: f32,
        grad: &mut Self,
        other: Vector<16>,
        ft: Vector<16>,
    ) {
        let cumulated = factor * other;
        self.ft.backprop(&mut grad.ft, cumulated, feats, ft);
    }

    pub fn adam(
        &mut self,
        grad: &Self,
        momentum: &mut Self,
        velocity: &mut Self,
        adj: f32,
        lr: f32,
    ) {
        self.ft.adam(&grad.ft, &mut momentum.ft, &mut velocity.ft, adj, lr);
    }

    pub const fn zeroed() -> Self {
        Self {
            ft: SparseLayer::from_raw(Matrix::zeroed(), Vector::zeroed()),
        }
    }

    pub fn from_fn<F: FnMut() -> f32>(mut f: F) -> Self {
        let mut v = [Vector::zeroed(); 768];
        for r in v.iter_mut() {
            *r = Vector::from_fn(|_| f());
        }
        let m = Matrix::from_raw(v);

        Self {
            ft: SparseLayer::from_raw(m, Vector::from_fn(|_| f())),
        }
    }
}
