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
    root_pst_adjustment: f32 = 0.34, 0.01, 1.0, 0.034, 0.002;
    depth_pst_adjustment: f32 = 1.8, 0.1, 10.0, 0.18, 0.002;
    winning_pst_threshold: f32 = 0.603, 0.0, 1.0, 0.0603, 0.002;
    winning_pst_max: f32 = 1.615, 0.1, 10.0, 0.1615, 0.002;
    base_pst_adjustment: f32 = 0.1, 0.01, 1.0, 0.01, 0.002;
    root_cpuct: f32 = if cfg!(feature = "datagen") { 1.0 } else { 0.422 }, 0.1, 5.0, 0.0422, 0.002;
    cpuct:      f32 = if cfg!(feature = "datagen") { 0.157 } else { 0.269 }, 0.1, 5.0, 0.0269, 0.002;
    cpuct_var_weight: f32 = 0.808, 0.0, 2.0, 0.0808, 0.002;
    cpuct_var_scale: f32 = 0.278, 0.0, 2.0, 0.0278, 0.002;
    cpuct_var_warmup: f32 = 0.5, 0.0, 1.0, 0.05, 0.002;
    cpuct_visits_scale: f32 = 36.91, 1.0, 512.0, 3.691, 0.002;
    expl_tau: f32 = 0.676, 0.1, 1.0, 0.0676, 0.002;
    gini_base: f32 = 0.463, 0.2, 2.0, 0.0463, 0.002;
    gini_ln_multiplier: f32 = 1.567, 0.4, 3.0, 0.1567, 0.002;
    gini_min: f32 = 2.26, 0.5, 4.0, 0.226, 0.002;
    knight_value: i32 = 437, 250, 750, 43, 0.002;
    bishop_value: i32 = 409, 250, 750, 41, 0.002;
    rook_value: i32 = 768, 400, 1000, 76, 0.002;
    queen_value: i32 = 1512, 900, 1600, 151, 0.002;
    material_offset: i32 = 559, 400, 1200, 55, 0.002;
    material_div1: i32 = 36, 16, 64, 4, 0.002;
    material_div2: i32 = 1226, 512, 1536, 122, 0.002;
    tm_opt_value1: f64 = 0.64, 0.1, 1.2, 0.064, 0.002;
    tm_opt_value2: f64 = 0.434, 0.1, 1.0, 0.0434, 0.002;
    tm_opt_value3: f64 = 0.66, 0.1, 1.2, 0.066, 0.002;
    tm_optscale_value1: f64 = 1.645, 0.1, 2.0, 0.1645, 0.002;
    tm_optscale_value2: f64 = 2.476, 0.1, 5.0, 0.2476, 0.002;
    tm_optscale_value3: f64 = 0.483, 0.1, 1.0, 0.0483, 0.002;
    tm_optscale_value4: f64 = 0.26, 0.1, 1.0, 0.026, 0.002;
    tm_max_value1: f64 = 2.877, 1.0, 10.0, 0.2877, 0.002;
    tm_max_value2: f64 = 2.85, 1.0, 10.0, 0.285, 0.002;
    tm_max_value3: f64 = 2.717, 1.0, 10.0, 0.2717, 0.002;
    tm_maxscale_value1: f64 = 13.275, 1.0, 24.0, 1.3275, 0.002;
    tm_maxscale_value2: f64 = 5.141, 1.0, 12.0, 0.5141, 0.002;
    tm_bonus_ply: f64 = 11.475, 1.0, 30.0, 1.1475, 0.002;
    tm_bonus_value1: f64 = 0.452, 0.1, 2.0, 0.0452, 0.002;
    tm_max_time: f64 = 0.881, 0.400, 0.990, 0.0881, 0.002;
    tm_mtg: i32 = 28, 10, 60, 3, 0.002;
    tm_falling_eval1: f32 = 0.054, 0.0, 0.2, 0.0054, 0.002;
    tm_falling_eval2: f32 = 0.724, 0.1, 1.0, 0.0724, 0.002;
    tm_falling_eval3: f32 = 1.633, 0.1, 3.0, 0.1633, 0.002;
    tm_bmi1: f32 = 0.257, 0.1, 1.0, 0.0257, 0.002;
    tm_bmi2: f32 = 0.835, 0.1, 2.0, 0.0835, 0.002;
    tm_bmi3: f32 = 3.272, 0.1, 6.4, 0.3272, 0.002;
    tm_bmv1: f32 = 3.620, 0.1, 5.0, 0.362, 0.002;
    tm_bmv2: f32 = 0.361, 0.1, 1.0, 0.0361, 0.002;
    tm_bmv3: f32 = 0.479, 0.1, 1.0, 0.0479, 0.002;
    tm_bmv4: f32 = 2.561, 0.1, 8.0, 0.2561, 0.002;
    tm_bmv5: f32 = 0.634, 0.1, 1.0, 0.0634, 0.002;
    tm_bmv6: f32 = 1.894, 0.1, 3.0, 0.1894, 0.002;
    butterfly_reduction_factor: i32 = 8192, 1, 65536, 819, 0.002;
    butterfly_policy_divisor: i32 = 16384, 1, 131072, 1638, 0.002;
    policy_top_p: f32 = 0.7, 0.1, 1.0, 0.05, 0.002;
    min_policy_actions: i32 = 6, 1, 32, 1, 0.002;
    visit_threshold_power: i32 = 3, 0, 8, 1, 0.002;
    virtual_loss_weight: f64 = 2.5, 1.0, 5.0, 0.25, 0.002;
    contempt: i32 = 0, -1000, 1000, 10, 0.0; //Do not tune this value!
}
