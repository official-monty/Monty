use super::moves::Move;

use goober::{activation, layer, FeedForwardNetwork, Matrix, SparseVector, Vector};

#[repr(C)]
#[derive(Clone, Copy, FeedForwardNetwork)]
pub struct SubNet {
    ft: layer::SparseConnected<activation::ReLU, 768, 16>,
    l2: layer::DenseConnected<activation::Identity, 16, 16>,
}

impl SubNet {
    pub const fn zeroed() -> Self {
        Self {
            ft: layer::SparseConnected::zeroed(),
            l2: layer::DenseConnected::zeroed(),
        }
    }

    pub fn from_fn<F: FnMut() -> f32>(mut f: F) -> Self {
        let matrix = Matrix::from_fn(|_, _| f());
        let vector = Vector::from_fn(|_| f());

        let matrix2 = Matrix::from_fn(|_, _| f());
        let vector2 = Vector::from_fn(|_| f());

        Self {
            ft: layer::SparseConnected::from_raw(matrix, vector),
            l2: layer::DenseConnected::from_raw(matrix2, vector2),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    pub subnets: [SubNet; 128],
    pub hce: [f32; 4],
}

impl PolicyNetwork {
    pub fn get(&self, mov: &Move, feats: &SparseVector, flip: u16) -> f32 {
        let from_subnet = &self.subnets[usize::from(mov.from() ^ flip)];
        let from_vec = from_subnet.out(feats);

        let to_subnet = &self.subnets[64 + usize::from(mov.to() ^ flip)];
        let to_vec = to_subnet.out(feats);

        from_vec.dot(&to_vec)
    }
}
