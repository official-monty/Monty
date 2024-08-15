use crate::{boxed_and_zeroed, Board};

use super::{layer::Layer, QA};

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const ValueFileDefaultName: &str = "nn-c3e7b78c4f09.network";

const SCALE: i32 = 400;

#[repr(C)]
pub struct ValueNetwork {
    l1: Layer<i16, { 768 * 4 }, 2048>,
    l2: Layer<f32, 2048, 16>,
    l3: Layer<f32, 16, 16>,
    l4: Layer<f32, 16, 16>,
    l5: Layer<f32, 16, 16>,
    l6: Layer<f32, 16, 16>,
    l7: Layer<f32, 16, 16>,
    l8: Layer<f32, 16, 16>,
    l9: Layer<f32, 16, 16>,
    l10: Layer<f32, 16, 16>,
    l11: Layer<f32, 16, 1>,
}

impl ValueNetwork {
    pub fn eval(&self, board: &Board) -> i32 {
        let l2 = self.l1.forward(board);
        let l3 = self.l2.forward_from_i16(&l2);
        let l4 = self.l3.forward(&l3);
        let l5 = self.l4.forward(&l4);
        let l6 = self.l5.forward(&l5);
        let l7 = self.l6.forward(&l6);
        let l8 = self.l7.forward(&l7);
        let l9 = self.l8.forward(&l8);
        let l10 = self.l9.forward(&l9);
        let l11 = self.l10.forward(&l10);
        let out = self.l11.forward(&l11);

        (out.0[0] * SCALE as f32) as i32
    }
}

#[repr(C)]
pub struct UnquantisedValueNetwork {
    l1: Layer<f32, { 768 * 4 }, 2048>,
    l2: Layer<f32, 2048, 16>,
    l3: Layer<f32, 16, 16>,
    l4: Layer<f32, 16, 16>,
    l5: Layer<f32, 16, 16>,
    l6: Layer<f32, 16, 16>,
    l7: Layer<f32, 16, 16>,
    l8: Layer<f32, 16, 16>,
    l9: Layer<f32, 16, 16>,
    l10: Layer<f32, 16, 16>,
    l11: Layer<f32, 16, 1>,
}

impl UnquantisedValueNetwork {
    pub fn quantise(&self) -> Box<ValueNetwork> {
        let mut quantised: Box<ValueNetwork> = unsafe { boxed_and_zeroed() };

        self.l1.quantise_into(&mut quantised.l1, QA);

        quantised.l2 = self.l2;
        quantised.l3 = self.l3;
        quantised.l4 = self.l4;
        quantised.l5 = self.l5;
        quantised.l6 = self.l6;
        quantised.l7 = self.l7;
        quantised.l8 = self.l8;
        quantised.l9 = self.l9;
        quantised.l10 = self.l10;
        quantised.l11 = self.l11;

        quantised
    }
}
