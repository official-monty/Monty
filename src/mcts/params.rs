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
    root_pst: f32 = 3.102, 1.0, 10.0, 0.4, 0.002;
    depth_2_pst_adjustment: f32 = 0.23, 1.0, 10.0, 0.025, 0.002;
    winning_pst_threshold: f32 = 0.603, 0.0, 1.0, 0.05, 0.002;
    winning_pst_max: f32 = 1.615, 0.1, 10.0, 0.1, 0.002;
    base_pst_adjustment: f32 = -0.05, 0.1, 10.0, 0.005, 0.002;
    root_cpuct: f32 = 0.422, 0.1, 5.0, 0.065, 0.002;
    cpuct: f32 = 0.269, 0.1, 5.0, 0.065, 0.002;
    cpuct_var_weight: f32 = 0.808, 0.0, 2.0, 0.085, 0.002;
    cpuct_var_scale: f32 = 0.278, 0.0, 2.0, 0.02, 0.002;
    cpuct_var_warmup: f32 = 0.5, 0.0, 1.0, 0.01, 0.002;
    cpuct_visits_scale: f32 = 36.91, 1.0, 512.0, 3.2, 0.002;
    expl_tau: f32 = 0.676, 0.1, 1.0, 0.05, 0.002;
    gini_base: f32 = 0.463, 0.2, 2.0, 0.0679, 0.002;
    gini_ln_multiplier: f32 = 1.567, 0.4, 3.0, 0.1634, 0.002;
    gini_min: f32 = 2.26, 0.5, 4.0, 0.21, 0.002;
    knight_value: i32 = 437, 250, 750, 25, 0.002;
    bishop_value: i32 = 409, 250, 750, 25, 0.002;
    rook_value: i32 = 768, 400, 1000, 30, 0.002;
    queen_value: i32 = 1512, 900, 1600, 35, 0.002;
    material_offset: i32 = 559, 400, 1200, 40, 0.002;
    material_div1: i32 = 36, 16, 64, 3, 0.002;
    material_div2: i32 = 1226, 512, 1536, 64, 0.002;
    tm_opt_value1: f64 = 0.64, 0.1, 1.2, 0.072, 0.002;
    tm_opt_value2: f64 = 0.434, 0.1, 1.0, 0.045, 0.002;
    tm_opt_value3: f64 = 0.66, 0.1, 1.2, 0.08, 0.002;
    tm_optscale_value1: f64 = 1.645, 0.1, 2.0, 0.15, 0.002;
    tm_optscale_value2: f64 = 2.476, 0.1, 5.0, 0.3, 0.002;
    tm_optscale_value3: f64 = 0.483, 0.1, 1.0, 0.05, 0.002;
    tm_optscale_value4: f64 = 0.26, 0.1, 1.0, 0.025, 0.002;
    tm_max_value1: f64 = 2.877, 1.0, 10.0, 0.4, 0.002;
    tm_max_value2: f64 = 2.85, 1.0, 10.0, 0.4, 0.002;
    tm_max_value3: f64 = 2.717, 1.0, 10.0, 0.4, 0.002;
    tm_maxscale_value1: f64 = 13.275, 1.0, 24.0, 1.2, 0.002;
    tm_maxscale_value2: f64 = 5.141, 1.0, 12.0, 0.6, 0.002;
    tm_bonus_ply: f64 = 11.475, 1.0, 30.0, 1.5, 0.002;
    tm_bonus_value1: f64 = 0.452, 0.1, 2.0, 0.05, 0.002;
    tm_max_time: f64 = 0.881, 0.400, 0.990, 0.085, 0.002;
    tm_mtg: i32 = 28, 10, 60, 3, 0.002;
    tm_falling_eval1: f32 = 0.054, 0.0, 0.2, 0.007, 0.002;
    tm_falling_eval2: f32 = 0.724, 0.1, 1.0, 0.06, 0.002;
    tm_falling_eval3: f32 = 1.633, 0.1, 3.0, 0.18, 0.002;
    tm_bmi1: f32 = 0.257, 0.1, 1.0, 0.04, 0.002;
    tm_bmi2: f32 = 0.835, 0.1, 2.0, 0.1, 0.002;
    tm_bmi3: f32 = 3.272, 0.1, 6.4, 0.32, 0.002;
    tm_bmv1: f32 = 3.620, 0.1, 5.0, 0.25, 0.002;
    tm_bmv2: f32 = 0.361, 0.1, 1.0, 0.035, 0.002;
    tm_bmv3: f32 = 0.479, 0.1, 1.0, 0.06, 0.002;
    tm_bmv4: f32 = 2.561, 0.1, 8.0, 0.4, 0.002;
    tm_bmv5: f32 = 0.634, 0.1, 1.0, 0.055, 0.002;
    tm_bmv6: f32 = 1.894, 0.1, 3.0, 0.15, 0.002;
}
