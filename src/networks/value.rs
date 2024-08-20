use crate::{boxed_and_zeroed, Board};

use super::{activation::SCReLU, layer::Layer};

// DO NOT MOVE
#[allow(non_upper_case_globals)]
pub const ValueFileDefaultName: &str = "quantised.network";

const QA: i16 = 64;
const SCALE: i32 = 400;

#[repr(C)]
pub struct ValueNetwork {
    l1: Layer<i8, { 768 * 4 }, 2048>,
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
        let l3 = self.l2.forward_from_i16::<SCReLU, QA>(&l2);
        let l4 = self.l3.forward::<SCReLU>(&l3);
        let l5 = self.l4.forward::<SCReLU>(&l4);
        let l6 = self.l5.forward::<SCReLU>(&l5);
        let l7 = self.l6.forward::<SCReLU>(&l6);
        let l8 = self.l7.forward::<SCReLU>(&l7);
        let l9 = self.l8.forward::<SCReLU>(&l8);
        let l10 = self.l9.forward::<SCReLU>(&l9);
        let l11 = self.l10.forward::<SCReLU>(&l10);
        let out = self.l11.forward::<SCReLU>(&l11);

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

        self.l1.quantise_into_i8(&mut quantised.l1, QA as i8);

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
