#[derive(Clone)]
struct Param<T> {
    val: T,
    min: T,
    max: T,
}

impl<T> Param<T> {
    fn new(val: T, min: T, max: T) -> Self {
        Self { val, min, max }
    }
}

impl Param<i32> {
    fn set(&mut self, val: i32) {
        self.val = val.clamp(self.min, self.max);
    }

    fn info(&self, name: &str) {
        println!(
            "option name {} type spin default {:.0} min {:.0} max {:.0}",
            name, self.val, self.min, self.max,
        );
    }

    fn list(&self, name: &str, step: i32, r: f32) {
        println!(
            "{}, {}, {}, {}, {}, {}",
            name, self.val, self.min, self.max, step, r,
        );
    }
}

impl Param<f32> {
    fn set(&mut self, val: i32) {
        let actual = val as f32 / 100.0;
        self.val = actual.clamp(self.min, self.max);
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

    fn list(&self, name: &str, step: f32, r: f32) {
        println!(
            "{}, {}, {}, {}, {}, {}",
            name,
            self.val * 100.0,
            self.min * 100.0,
            self.max * 100.0,
            step * 100.0,
            r,
        );
    }
}

macro_rules! make_mcts_params {
    ($($name:ident: $t:ty = $val:expr, $min:expr, $max:expr, $step:expr, $r:expr;)*) => {
        #[derive(Clone)]
        pub struct MctsParams {
            $($name: Param<$t>,)*
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
            pub fn $name(&self) -> $t {
                self.$name.val
            }
        )*

            pub fn info(self) {
                $(self.$name.info(stringify!($name));)*
            }

            pub fn set(&mut self, name: &str, val: i32) {
                match name {
                    $(stringify!($name) => self.$name.set(val),)*
                    _ => println!("unknown option!"),
                }
            }

            pub fn list_spsa(&self) {
                $(self.$name.list(stringify!($name), $step, $r);)*
            }
        }
    };
}

make_mcts_params! {
    root_pst: f32 = 4.0, 1.0, 10.0, 0.2, 0.002;
    root_cpuct: f32 = 0.65, 0.1, 5.0, 0.1, 0.002;
    cpuct: f32 = 0.65, 0.1, 5.0, 0.1, 0.002;
    cpuct_var_weight: f32 = 0.85, 0.0, 2.0, 0.1, 0.002;
    cpuct_var_scale: f32 = 0.2, 0.0, 2.0, 0.05, 0.002;
    cpuct_visits_scale: i32 = 64, 1, 512, 4, 0.002;
    expl_tau: f32 = 0.5, 0.1, 1.0, 0.05, 0.002;
}
