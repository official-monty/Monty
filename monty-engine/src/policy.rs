use monty_core::{Flag, Move, Position, FeatureList};

pub static POLICY_NETWORK: PolicyNetwork =
    unsafe { std::mem::transmute(*include_bytes!("../../resources/policy.bin")) };

pub struct NetworkDims;

impl NetworkDims {
    pub const INDICES: usize = 6 * 64;
    pub const FEATURES: usize = 769;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    pub weights: [[f32; NetworkDims::FEATURES]; NetworkDims::INDICES],
}

impl std::ops::AddAssign<&PolicyNetwork> for PolicyNetwork {
    fn add_assign(&mut self, rhs: &PolicyNetwork) {
        for (i, j) in self.weights.iter_mut().zip(rhs.weights.iter()) {
            for (a, b) in i.iter_mut().zip(j.iter()) {
                *a += *b;
            }
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

    fn get_neuron(&self, idx: usize, feats: &FeatureList) -> f32 {
        let wref = &self.weights[idx];
        let mut score = 0.0;

        for &feat in feats.iter() {
            score += wref[feat];
        }

        score
    }

    pub fn hce(mov: &Move, pos: &Position) -> f32 {
        let mut score = 0.0;

        if pos.see(mov, -108) {
            score += 2.0;
        }

        if [Flag::QPR, Flag::QPC].contains(&mov.flag()) {
            score += 2.0;
        }

        if mov.is_capture() {
            score += 2.0;

            let diff = pos.get_pc(1 << mov.to()) as i32 - i32::from(mov.moved());
            score += 0.2 * diff as f32;
        }

        score
    }

    pub fn get(mov: &Move, pos: &Position, policy: &PolicyNetwork, feats: &FeatureList) -> f32 {
        let idx = mov.index(pos.flip_val());
        let sq_policy = policy.get_neuron(idx, feats);

        let hce_policy = PolicyNetwork::hce(mov, pos);

        sq_policy + hce_policy
    }
}
