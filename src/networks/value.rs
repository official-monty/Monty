use crate::{boxed_and_zeroed, Board};

use super::{
    activation::SCReLU,
    layer::{Layer, TransposedLayer},
};

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const ValueFileDefaultName: &str = "nn-4587c7cdf848.network";

const QA: i16 = 512;
const QB: i16 = 1024;
const SCALE: i32 = 400;

const FACTOR: i16 = 32;

#[repr(C)]
pub struct ValueNetwork {
    l1: Layer<i16, { 768 * 4 }, 4096>,
    l2: TransposedLayer<i16, 4096, 16>,
    l3: Layer<f32, 16, 128>,
    l4: Layer<f32, 128, 1>,
}

impl ValueNetwork {
    pub fn eval(&self, board: &Board) -> i32 {
        let l2 = self.l1.forward(board);
        let l3 = self.l2.forward_from_i16::<SCReLU, QA, QB, FACTOR>(&l2);
        let l4 = self.l3.forward::<SCReLU>(&l3);
        let out = self.l4.forward::<SCReLU>(&l4);

        (out.0[0] * SCALE as f32) as i32
    }
}

#[repr(C)]
pub struct UnquantisedValueNetwork {
    l1: Layer<f32, { 768 * 4 }, 4096>,
    l2: Layer<f32, 4096, 16>,
    l3: Layer<f32, 16, 128>,
    l4: Layer<f32, 128, 1>,
}

impl UnquantisedValueNetwork {
    pub fn quantise(&self) -> Box<ValueNetwork> {
        let mut quantised: Box<ValueNetwork> = unsafe { boxed_and_zeroed() };

        self.l1.quantise_into_i16(&mut quantised.l1, QA, 0.99);
        self.l2
            .quantise_transpose_into_i16(&mut quantised.l2, QB, 0.99);

        quantised.l3 = self.l3;
        quantised.l4 = self.l4;

        quantised
    }
}
