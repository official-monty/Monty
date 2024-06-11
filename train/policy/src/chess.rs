use datagen::{PolicyData, Rand};
use goober::{FeedForwardNetwork, OutputLayer, SparseVector, Vector};
use monty::{Board, Move, PolicyNetwork, SubNet};

use crate::TrainablePolicy;

impl TrainablePolicy for PolicyNetwork {
    type Data = PolicyData;

    fn update(
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

    fn update_single_grad(pos: &Self::Data, policy: &Self, grad: &mut Self, error: &mut f32) {
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
            let pc = board.get_pc(1 << mov.src()) - 1;

            let from = usize::from(mov.src() ^ flip);
            let to = 64 * pc + usize::from(mov.to() ^ flip);
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
            let pc = board.get_pc(1 << mov.src()) - 1;
            let from = usize::from(mov.src() ^ flip);
            let to = 64 * pc + usize::from(mov.to() ^ flip);
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

    fn rand_init() -> Box<Self> {
        let mut policy = Self::boxed_and_zeroed();

        let mut rng = Rand::with_seed();
        for subnet_pair in policy.subnets.iter_mut() {
            for subnet in subnet_pair.iter_mut() {
                *subnet = SubNet::from_fn(|| rng.rand_f32(0.2));
            }
        }

        policy
    }

    fn add_without_explicit_lifetime(&mut self, rhs: &Self) {
        for (ipair, jpair) in self.subnets.iter_mut().zip(rhs.subnets.iter()) {
            for (i, j) in ipair.iter_mut().zip(jpair.iter()) {
                *i += j;
            }
        }

        self.hce += &rhs.hce;
    }
}
