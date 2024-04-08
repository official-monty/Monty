use super::{moves::Move, Board};

use goober::{activation, layer, FeedForwardNetwork, Matrix, SparseVector, Vector};

pub static POLICY: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/chess-policy004.bin")) };

#[repr(C)]
#[derive(Clone, Copy, FeedForwardNetwork)]
pub struct SubNet {
    ft: layer::SparseConnected<activation::ReLU, 768, 8>,
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
    pub hce: layer::DenseConnected<activation::Identity, 5, 1>,
}

impl PolicyNetwork {
    pub const fn zeroed() -> Self {
        Self { subnets: [SubNet::zeroed(); 128], hce: layer::DenseConnected::zeroed() }
    }

    pub fn get(&self, pos: &Board, mov: &Move, feats: &SparseVector) -> f32 {
        let flip = pos.flip_val();
        let from_subnet = &self.subnets[usize::from(mov.from() ^ flip)];
        let from_vec = from_subnet.out(feats);

        let to_subnet = &self.subnets[64 + usize::from(mov.to() ^ flip)];
        let to_vec = to_subnet.out(feats);

        let hce = self.hce.out(&Self::get_hce_feats(pos, mov))[0];

        from_vec.dot(&to_vec) + hce
    }

    pub fn get_hce_feats(pos: &Board, mov: &Move) -> Vector<5> {
        let mut feats = Vector::zeroed();

        if mov.is_promo() {
            feats[mov.promo_pc() - 3] = 1.0;
        }

        feats[4] = f32::from(pos.see(mov, -108));

        feats
    }
}
