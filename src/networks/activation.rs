pub trait Activation {
    fn activate(x: f32) -> f32;
}

pub struct SCReLU;
impl Activation for SCReLU {
    #[inline]
    fn activate(x: f32) -> f32 {
        x.clamp(0.0, 1.0).powi(2)
    }
}
