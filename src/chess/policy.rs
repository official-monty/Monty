use super::{board::Board, consts::Flag, moves::Move};

use goober::{activation, layer, FeedForwardNetwork, Matrix, SparseVector, Vector};

pub static POLICY_NETWORK: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/chess-policy.bin")) };

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
    pub weights: [SubNet; 128],
    pub hce: [f32; 4],
}

impl PolicyNetwork {
    fn get_neuron(&self, mov: &Move, feats: &SparseVector, flip: u16) -> f32 {
        let from_subnet = &self.weights[usize::from(mov.from() ^ flip)];
        let from_vec = from_subnet.out(feats);

        let to_subnet = &self.weights[64 + usize::from(mov.to() ^ flip)];
        let to_vec = to_subnet.out(feats);

        from_vec.dot(&to_vec)
    }

    pub fn hce(&self, mov: &Move, pos: &Board) -> f32 {
        let mut score = 0.0;

        if pos.see(mov, -108) {
            score += self.hce[0];
        }

        if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
            score += self.hce[1];
        }

        if mov.is_capture() {
            score += self.hce[2];

            let diff = pos.get_pc(1 << mov.to()) as i32 - pos.get_pc(1 << mov.from()) as i32;
            score += self.hce[3] * diff as f32;
        }

        score
    }

    pub fn get(mov: &Move, pos: &Board, policy: &PolicyNetwork, feats: &SparseVector) -> f32 {
        let sq_policy = policy.get_neuron(mov, feats, pos.flip_val());

        let hce_policy = policy.hce(mov, pos);

        sq_policy + hce_policy
    }
}
