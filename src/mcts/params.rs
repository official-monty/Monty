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
        let actual = val as f32 / 1000.0;
        self.val = actual.clamp(self.min, self.max);
    }

    fn info(&self, name: &str) {
        println!(
            "option name {} type spin default {:.0} min {:.0} max {:.0}",
            name,
            self.val * 1000.0,
            self.min * 1000.0,
            self.max * 1000.0,
        );
    }

    fn list(&self, name: &str, step: f32, r: f32) {
        println!(
            "{}, {}, {}, {}, {}, {}",
            name,
            self.val * 1000.0,
            self.min * 1000.0,
            self.max * 1000.0,
            step * 1000.0,
            r,
        );
    }
}

impl Param<f64> {
    fn set(&mut self, val: i32) {
        let actual = val as f64 / 1000.0;
        self.val = actual.clamp(self.min, self.max);
    }

    fn info(&self, name: &str) {
        println!(
            "option name {} type spin default {:.0} min {:.0} max {:.0}",
            name,
            self.val * 1000.0,
            self.min * 1000.0,
            self.max * 1000.0,
        );
    }

    fn list(&self, name: &str, step: f64, r: f64) {
        println!(
            "{}, {}, {}, {}, {}, {}",
            name,
            self.val * 1000.0,
            self.min * 1000.0,
            self.max * 1000.0,
            step * 1000.0,
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
    root_pst: f32 = 3.64, 1.0, 10.0, 0.4, 0.002;
    root_cpuct: f32 = 0.624, 0.1, 5.0, 0.065, 0.002;
    cpuct: f32 = 0.314, 0.1, 5.0, 0.065, 0.002;
    cpuct_var_weight: f32 = 0.851, 0.0, 2.0, 0.085, 0.002;
    cpuct_var_scale: f32 = 0.257, 0.0, 2.0, 0.02, 0.002;
    cpuct_visits_scale: f32 = 37.3, 1.0, 512.0, 3.2, 0.002;
    expl_tau: f32 = 0.623, 0.1, 1.0, 0.05, 0.002;
    knight_value: i32 = 450, 250, 750, 25, 0.002;
    bishop_value: i32 = 450, 250, 750, 25, 0.002;
    rook_value: i32 = 650, 400, 1000, 30, 0.002;
    queen_value: i32 = 1250, 900, 1600, 35, 0.002;
    material_offset: i32 = 700, 400, 1200, 40, 0.002;
    material_div1: i32 = 32, 16, 64, 3, 0.002;
    material_div2: i32 = 1024, 512, 1536, 64, 0.002;
    tm_opt_value1: f64 = 0.48, 0.1, 1.0, 0.05, 0.002;
    tm_opt_value2: f64 = 0.32, 0.1, 1.0, 0.03, 0.002;
    tm_opt_value3: f64 = 0.60, 0.1, 1.0, 0.06, 0.002;
    tm_optscale_value1: f64 = 1.25, 0.1, 2.0, 0.1, 0.002;
    tm_optscale_value2: f64 = 2.5, 0.1, 5.0, 0.15, 0.002;
    tm_optscale_value3: f64 = 0.50, 0.1, 1.0, 0.05, 0.002;
    tm_optscale_value4: f64 = 0.25, 0.1, 1.0, 0.02, 0.002;
    tm_max_value1: f64 = 3.39, 1.0, 10.0, 0.4, 0.002;
    tm_max_value2: f64 = 3.01, 1.0, 10.0, 0.4, 0.002;
    tm_max_value3: f64 = 2.93, 1.0, 10.0, 0.4, 0.002;
    tm_maxscale_value1: f64 = 12.0, 1.0, 24.0, 1.2, 0.002;
    tm_maxscale_value2: f64 = 6.64, 1.0, 12.0, 0.6, 0.002;
    tm_bonus_ply: f64 = 11.0, 1.0, 30.0, 1.5, 0.002;
    tm_bonus_value1: f64 = 0.5, 0.1, 2.0, 0.05, 0.002;
    tm_max_time: f64 = 0.850, 0.400, 0.990, 0.085, 0.002;
}
