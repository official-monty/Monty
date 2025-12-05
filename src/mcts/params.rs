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
    root_pst_adjustment: f32 = 0.332, 0.01, 1.0, 0.034, 0.002;
    depth_pst_adjustment: f32 = 1.78, 0.1, 10.0, 0.18, 0.002;
    winning_pst_threshold: f32 = 0.602, 0.0, 1.0, 0.0603, 0.002;
    winning_pst_max: f32 = 1.510, 0.1, 10.0, 0.1615, 0.002;
    base_pst_adjustment: f32 = 0.091, 0.01, 1.0, 0.01, 0.002;
    root_cpuct: f32 = if cfg!(feature = "datagen") { 1.0 } else { 0.442 }, 0.1, 5.0, 0.0422, 0.002;
    cpuct:      f32 = if cfg!(feature = "datagen") { 0.157 } else { 0.257 }, 0.1, 5.0, 0.0269, 0.002;
    cpuct_var_weight: f32 = 0.77, 0.0, 2.0, 0.0808, 0.002;
    cpuct_var_scale: f32 = 0.309, 0.0, 2.0, 0.0278, 0.002;
    cpuct_var_warmup: f32 = 0.5, 0.0, 1.0, 0.05, 0.002;
    cpuct_visits_scale: f32 = 34.65, 1.0, 512.0, 3.691, 0.002;
    expl_tau: f32 = 0.689, 0.1, 1.0, 0.0676, 0.002;
    gini_base: f32 = 0.405, 0.2, 2.0, 0.0463, 0.002;
    gini_ln_multiplier: f32 = 1.421, 0.4, 3.0, 0.1567, 0.002;
    gini_min: f32 = 2.26, 0.5, 4.0, 0.226, 0.002;
    knight_value: i32 = 397, 250, 750, 43, 0.002;
    bishop_value: i32 = 411, 250, 750, 41, 0.002;
    rook_value: i32 = 732, 400, 1000, 76, 0.002;
    queen_value: i32 = 1570, 900, 1600, 151, 0.002;
    material_offset: i32 = 632, 400, 1200, 55, 0.002;
    material_div1: i32 = 32, 16, 64, 4, 0.002;
    material_div2: i32 = 1141, 512, 1536, 122, 0.002;
    tm_opt_value1: f64 = 0.637, 0.1, 1.2, 0.064, 0.002;
    tm_opt_value2: f64 = 0.394, 0.1, 1.0, 0.0434, 0.002;
    tm_opt_value3: f64 = 0.802, 0.1, 1.2, 0.066, 0.002;
    tm_optscale_value1: f64 = 1.584, 0.1, 2.0, 0.1645, 0.002;
    tm_optscale_value2: f64 = 2.502, 0.1, 5.0, 0.2476, 0.002;
    tm_optscale_value3: f64 = 0.481, 0.1, 1.0, 0.0483, 0.002;
    tm_optscale_value4: f64 = 0.249, 0.1, 1.0, 0.026, 0.002;
    tm_max_value1: f64 = 2.609, 1.0, 10.0, 0.2877, 0.002;
    tm_max_value2: f64 = 2.553, 1.0, 10.0, 0.285, 0.002;
    tm_max_value3: f64 = 2.560, 1.0, 10.0, 0.2717, 0.002;
    tm_maxscale_value1: f64 = 12.069, 1.0, 24.0, 1.3275, 0.002;
    tm_maxscale_value2: f64 = 5.481, 1.0, 12.0, 0.5141, 0.002;
    tm_bonus_ply: f64 = 11.774, 1.0, 30.0, 1.1475, 0.002;
    tm_bonus_value1: f64 = 0.398, 0.1, 2.0, 0.0452, 0.002;
    tm_max_time: f64 = 0.988, 0.400, 0.990, 0.0881, 0.002;
    tm_mtg: i32 = 21, 10, 60, 3, 0.002;
    tm_falling_eval1: f32 = 0.058, 0.0, 0.2, 0.0054, 0.002;
    tm_falling_eval2: f32 = 0.744, 0.1, 1.0, 0.0724, 0.002;
    tm_falling_eval3: f32 = 1.774, 0.1, 3.0, 0.1633, 0.002;
    tm_bmi1: f32 = 0.263, 0.1, 1.0, 0.0257, 0.002;
    tm_bmi2: f32 = 0.821, 0.1, 2.0, 0.0835, 0.002;
    tm_bmi3: f32 = 3.733, 0.1, 6.4, 0.3272, 0.002;
    tm_bmv1: f32 = 3.581, 0.1, 5.0, 0.362, 0.002;
    tm_bmv2: f32 = 0.381, 0.1, 1.0, 0.0361, 0.002;
    tm_bmv3: f32 = 0.505, 0.1, 1.0, 0.0479, 0.002;
    tm_bmv4: f32 = 2.720, 0.1, 8.0, 0.2561, 0.002;
    tm_bmv5: f32 = 0.632, 0.1, 1.0, 0.0634, 0.002;
    tm_bmv6: f32 = 1.468, 0.1, 3.0, 0.1894, 0.002;
    butterfly_reduction_factor: i32 = 7089, 1, 65536, 819, 0.002;
    butterfly_policy_divisor: i32 = 16676, 1, 131072, 1638, 0.002;
    policy_top_p: f32 = 0.701, 0.1, 1.0, 0.05, 0.002;
    min_policy_actions: i32 = 7, 1, 32, 1, 0.002;
    visit_threshold_power: i32 = 4, 0, 8, 1, 0.002;
    virtual_loss_weight: f64 = 2.37, 1.0, 5.0, 0.25, 0.002;
    contempt: i32 = 0, -1000, 1000, 10, 0.0; //Do not tune this value!
}
