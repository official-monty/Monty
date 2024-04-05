#[derive(Clone)]
struct Param {
    val: f32,
    min: f32,
    max: f32,
}

impl Param {
    fn new(val: f32, min: f32, max: f32) -> Self {
        Self { val, min, max }
    }

    fn set(&mut self, val: f32) {
        self.val = val.clamp(self.min, self.max);
    }

    fn info(&self, name: &str) {
        println!(
            "option name {} type spin default {:.0} min {:.0} max {:.0}",
            name,
            self.val * 100.0,
            self.min * 100.0,
            self.max * 100.0,
        );
    }
}

macro_rules! make_mcts_params {
    ($($name:ident: $val:expr, $min:expr, $max:expr,)*) => {
        #[derive(Clone)]
        pub struct MctsParams {
            $($name: Param,)*
        }

        impl Default for MctsParams {
            fn default() -> Self {
                Self {
                    $($name: Param::new($val, $min, $max),)*
                }
            }
        }

        impl MctsParams {
        $(
            pub fn $name(&self) -> f32 {
                self.$name.val
            }
        )*

            pub fn info(self) {
                $(self.$name.info(stringify!($name));)*
            }

            pub fn set(&mut self, name: &str, val: f32) {
                match name {
                    $(stringify!($name) => self.$name.set(val),)*
                    _ => println!("unknown option!"),
                }
            }
        }
    };
}

make_mcts_params! {
    root_pst: 1.0, 1.0, 2.5,
    root_cpuct: 1.41, 0.1, 5.0,
    cpuct: 1.41, 0.1, 5.0,
    mate_bonus: 1.0, 0.0, 10.0,
}
