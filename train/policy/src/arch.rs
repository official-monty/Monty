use datagen::{PolicyData, Rand};
use goober::{activation, layer, FeedForwardNetwork, Matrix, OutputLayer, SparseVector, Vector};
use monty::{Board, Move};

use std::io::Write;

#[repr(C)]
#[derive(Clone, Copy, FeedForwardNetwork)]
pub struct SubNet {
    ft: layer::SparseConnected<activation::ReLU, 768, 64>,
    l2: layer::DenseConnected<activation::ReLU, 64, 64>,
}

impl SubNet {
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
    pub subnets: [[SubNet; 2]; 128],
    pub hce: layer::DenseConnected<activation::Identity, 4, 1>,
}

impl PolicyNetwork {
    pub fn get_hce_feats(_: &Board, mov: &Move) -> Vector<4> {
        let mut feats = Vector::zeroed();

        if mov.is_promo() {
            feats[mov.promo_pc() - 3] = 1.0;
        }

        feats
    }

    pub fn update(
        policy: &mut Self,
        grad: &Self,
        adj: f32,
        lr: f32,
        momentum: &mut Self,
        velocity: &mut Self,
    ) {
        for (i, subnet_pair) in policy.subnets.iter_mut().enumerate() {
            for (j, subnet) in subnet_pair.iter_mut().enumerate() {
                subnet.adam(
                    &grad.subnets[i][j],
                    &mut momentum.subnets[i][j],
                    &mut velocity.subnets[i][j],
                    adj,
                    lr,
                );
            }
        }

        policy
            .hce
            .adam(&grad.hce, &mut momentum.hce, &mut velocity.hce, adj, lr);
    }

    pub fn update_single_grad(pos: &PolicyData, policy: &Self, grad: &mut Self, error: &mut f32) {
        let board = Board::from(pos.pos);

        let mut feats = SparseVector::with_capacity(32);
        board.map_policy_features(|feat| feats.push(feat));

        let mut policies = Vec::with_capacity(pos.num);
        let mut total = 0.0;
        let mut total_visits = 0;
        let mut max = -1000.0;

        let flip = board.flip_val();
        let threats = board.threats();

        for &(mov, visits) in &pos.moves[..pos.num] {
            let mov = <Move as From<u16>>::from(mov);

            let from = usize::from(mov.src() ^ flip);
            let to = usize::from(mov.to() ^ flip);
            let from_threat = usize::from(threats & (1 << mov.src()) > 0);
            let good_see = usize::from(board.see(&mov, -108));

            let from_out = policy.subnets[from][from_threat].out_with_layers(&feats);
            let to_out = policy.subnets[to][good_see].out_with_layers(&feats);
            let hce_feats = PolicyNetwork::get_hce_feats(&board, &mov);
            let hce_out = policy.hce.out_with_layers(&hce_feats);
            let score =
                from_out.output_layer().dot(&to_out.output_layer()) + hce_out.output_layer()[0];

            if score > max {
                max = score;
            }

            total_visits += visits;
            policies.push((from_out, to_out, hce_out, mov, visits, score, good_see));
        }

        for (_, _, _, _, _, score, _) in policies.iter_mut() {
            *score = (*score - max).exp();
            total += *score;
        }

        for (from_out, to_out, hce_out, mov, visits, score, good_see) in policies {
            let from = usize::from(mov.src() ^ flip);
            let to = usize::from(mov.to() ^ flip);
            let from_threat = usize::from(threats & (1 << mov.src()) > 0);
            let hce_feats = PolicyNetwork::get_hce_feats(&board, &mov);

            let ratio = score / total;

            let expected = visits as f32 / total_visits as f32;
            let err = ratio - expected;

            *error -= expected * ratio.ln();

            let factor = err;

            policy.subnets[from][from_threat].backprop(
                &feats,
                &mut grad.subnets[from][from_threat],
                factor * to_out.output_layer(),
                &from_out,
            );

            policy.subnets[to][good_see].backprop(
                &feats,
                &mut grad.subnets[to][good_see],
                factor * from_out.output_layer(),
                &to_out,
            );

            policy.hce.backprop(
                &hce_feats,
                &mut grad.hce,
                Vector::from_raw([factor]),
                &hce_out,
            );
        }
    }

    pub fn rand_init() -> Box<Self> {
        let mut policy = Self::boxed_and_zeroed();

        let mut rng = Rand::with_seed();
        for subnet_pair in policy.subnets.iter_mut() {
            for subnet in subnet_pair.iter_mut() {
                *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
            }
        }

        policy
    }

    pub fn add_without_explicit_lifetime(&mut self, rhs: &Self) {
        for (ipair, jpair) in self.subnets.iter_mut().zip(rhs.subnets.iter()) {
            for (i, j) in ipair.iter_mut().zip(jpair.iter()) {
                *i += j;
            }
        }

        self.hce += &rhs.hce;
    }

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
        let size_of = std::mem::size_of::<Self>();

        let mut file = std::fs::File::create(path).unwrap();

        unsafe {
            let ptr: *const Self = self;
            let slice_ptr: *const u8 = std::mem::transmute(ptr);
            let slice = std::slice::from_raw_parts(slice_ptr, size_of);
            file.write_all(slice).unwrap();
        }
    }
}
