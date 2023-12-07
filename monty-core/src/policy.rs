use crate::{FeatureList, Flag, Move, Position};

use monty_policy::{ReLU, SubNet, Vector};

pub type PolicyVal = Vector<{ NetworkDims::NEURONS }>;

pub static POLICY_NETWORK: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/policy.bin")) };

pub struct NetworkDims;
impl NetworkDims {
    pub const INDICES: usize = 2 * 64;
    pub const FEATURES: usize = 769;
    pub const NEURONS: usize = 16;
    pub const HCE: usize = 4;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    pub weights:
        [SubNet<ReLU, { NetworkDims::NEURONS }, { NetworkDims::FEATURES }>; NetworkDims::INDICES],
    pub hce: [f32; NetworkDims::HCE],
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

    fn get_neuron(&self, mov: &Move, feats: &FeatureList, flip: u8) -> f32 {
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

    pub fn get(mov: &Move, pos: &Position, policy: &PolicyNetwork, feats: &FeatureList) -> f32 {
        let sq_policy = policy.get_neuron(mov, feats, pos.flip_val());

        let hce_policy = policy.hce(mov, pos);

        sq_policy + hce_policy
    }
}
