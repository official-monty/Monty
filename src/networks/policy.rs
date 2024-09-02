use crate::{
    boxed_and_zeroed,
    chess::{Board, Move},
};

use super::{accumulator::Accumulator, activation::ReLU, layer::Layer};

const QA: i16 = 512;

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const PolicyFileDefaultName: &str = "nn-1b01b6e89ea1.network";

#[repr(C)]
#[derive(Clone, Copy)]
struct SubNet {
    ft: Layer<i16, 768, 32>,
    l2: Layer<f32, 32, 32>,
}

impl SubNet {
    fn out(&self, feats: &[usize]) -> Accumulator<f32, 32> {
        let l2 = self.ft.forward_from_slice(feats);
        self.l2.forward_from_i16::<ReLU, QA>(&l2)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PolicyNetwork {
    subnets: [[SubNet; 2]; 128],
    hce: Layer<f32, 4, 1>,
}

impl PolicyNetwork {
    pub fn get(&self, pos: &Board, mov: &Move, feats: &[usize], threats: u64) -> f32 {
        let flip = pos.flip_val();

        let from_threat = usize::from(threats & (1 << mov.src()) > 0);
        let from_subnet = &self.subnets[usize::from(mov.src() ^ flip)][from_threat];
        let from_vec = from_subnet.out(feats);

        let good_see = usize::from(pos.see(mov, -108));
        let to_subnet = &self.subnets[64 + usize::from(mov.to() ^ flip)][good_see];
        let to_vec = to_subnet.out(feats);

        let hce = self.hce.forward::<ReLU>(&Self::get_hce_feats(pos, mov)).0[0];

        from_vec.dot::<ReLU>(&to_vec) + hce
    }

    pub fn get_hce_feats(_: &Board, mov: &Move) -> Accumulator<f32, 4> {
        let mut feats = [0.0; 4];

        if mov.is_promo() {
            feats[mov.promo_pc() - 3] = 1.0;
        }

        Accumulator(feats)
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct UnquantisedSubNet {
    ft: Layer<f32, 768, 32>,
    l2: Layer<f32, 32, 32>,
}

impl UnquantisedSubNet {
    fn quantise(&self, qa: i16) -> SubNet {
        SubNet {
            ft: self.ft.quantise_i16(qa, 1.98),
            l2: self.l2,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct UnquantisedPolicyNetwork {
    subnets: [[UnquantisedSubNet; 2]; 128],
    hce: Layer<f32, 4, 1>,
}

impl UnquantisedPolicyNetwork {
    pub fn quantise(&self) -> Box<PolicyNetwork> {
        let mut quant: Box<PolicyNetwork> = unsafe { boxed_and_zeroed() };

        for (qpair, unqpair) in quant.subnets.iter_mut().zip(self.subnets.iter()) {
            for (qsubnet, unqsubnet) in qpair.iter_mut().zip(unqpair.iter()) {
                *qsubnet = unqsubnet.quantise(QA);
            }
        }

        quant.hce = self.hce;

        quant
    }
}
