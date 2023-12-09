use crate::{Flag, Move, Position};

use goober::{Vector, Matrix, activation::ReLU, layer::SparseLayer, FeedForwardNetwork, SparseVector};

pub static POLICY_NETWORK: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/policy.bin")) };

#[repr(C)]
#[derive(Clone, Copy, FeedForwardNetwork)]
pub struct SubNet {
    ft: SparseLayer<ReLU, 768, 16>,
}

impl SubNet {
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

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    pub weights: [SubNet; 128],
    pub hce: [f32; 4],
}

impl std::ops::AddAssign<&PolicyNetwork> for PolicyNetwork {
    fn add_assign(&mut self, rhs: &PolicyNetwork) {
        for (i, j) in self.weights.iter_mut().zip(rhs.weights.iter()) {
            *i += j;
        }

        for (i, j) in self.hce.iter_mut().zip(rhs.hce.iter()) {
            *i += *j;
        }
    }
}

impl PolicyNetwork {
    pub fn boxed_and_zeroed() -> Box<Self> {
        unsafe {
            let layout = std::alloc::Layout::new::<Self>();
            let ptr = std::alloc::alloc_zeroed(layout);
            if ptr.is_null() {
                std::alloc::handle_alloc_error(layout);
            }
            Box::from_raw(ptr.cast())
        }
    }

    pub fn write_to_bin(&self, path: &str) {
        use std::io::Write;
        const SIZEOF: usize = std::mem::size_of::<PolicyNetwork>();

        let mut file = std::fs::File::create(path).unwrap();

        unsafe {
            let ptr: *const Self = self;
            let slice_ptr: *const u8 = std::mem::transmute(ptr);
            let slice = std::slice::from_raw_parts(slice_ptr, SIZEOF);
            file.write_all(slice).unwrap();
        }
    }

    fn get_neuron(&self, mov: &Move, feats: &SparseVector, flip: u8) -> f32 {
        let from_subnet = &self.weights[usize::from(mov.from() ^ flip)];
        let from_vec = from_subnet.out(feats);

        let to_subnet = &self.weights[64 + usize::from(mov.to() ^ flip)];
        let to_vec = to_subnet.out(feats);

        from_vec.dot(&to_vec)
    }

    pub fn hce(&self, mov: &Move, pos: &Position) -> f32 {
        let mut score = 0.0;

        if pos.see(mov, -108) {
            score += self.hce[0];
        }

        if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
            score += self.hce[1];
        }

        if mov.is_capture() {
            score += self.hce[2];

            let diff = pos.get_pc(1 << mov.to()) as i32 - i32::from(mov.moved());
            score += self.hce[3] * diff as f32;
        }

        score
    }

    pub fn get(mov: &Move, pos: &Position, policy: &PolicyNetwork, feats: &SparseVector) -> f32 {
        let sq_policy = policy.get_neuron(mov, feats, pos.flip_val());

        let hce_policy = policy.hce(mov, pos);

        sq_policy + hce_policy
    }
}
