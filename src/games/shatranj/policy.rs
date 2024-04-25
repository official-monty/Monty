use super::{moves::Move, Board};

use goober::{activation, layer, FeedForwardNetwork, Matrix, SparseVector, Vector};

#[repr(C)]
#[derive(Clone, Copy, FeedForwardNetwork)]
pub struct SubNet {
    ft: layer::SparseConnected<activation::ReLU, 768, 4>,
}

impl SubNet {
    pub const fn zeroed() -> Self {
        Self {
            ft: layer::SparseConnected::zeroed(),
        }
    }

    pub fn from_fn<F: FnMut() -> f32>(mut f: F) -> Self {
        let matrix = Matrix::from_fn(|_, _| f());
        let vector = Vector::from_fn(|_| f());

        Self {
            ft: layer::SparseConnected::from_raw(matrix, vector),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    pub subnets: [SubNet; 128],
}

impl PolicyNetwork {
    pub const fn zeroed() -> Self {
        Self {
            subnets: [SubNet::zeroed(); 128],
        }
    }

    pub fn get(&self, pos: &Board, mov: &Move, feats: &SparseVector) -> f32 {
        let flip = pos.flip_val() as u8;

        let from_subnet = &self.subnets[usize::from(mov.from() ^ flip)];
        let from_vec = from_subnet.out(feats);

        let to_subnet = &self.subnets[64 + usize::from(mov.to() ^ flip)];
        let to_vec = to_subnet.out(feats);

        from_vec.dot(&to_vec)
    }
}
