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
    root_pst_adjustment: f32 = 0.34054, 0.01, 1.0, 0.034, 0.002;
    depth_pst_adjustment: f32 = 1.788, 0.1, 10.0, 0.18, 0.002;
    winning_pst_threshold: f32 = 0.600, 0.0, 1.0, 0.05, 0.002;
    winning_pst_max: f32 = 1.622, 0.1, 10.0, 0.1, 0.002;
    base_pst_adjustment: f32 = 0.1055, 0.01, 1.0, 0.01, 0.002;
    root_cpuct: f32 = if cfg!(feature = "datagen") { 1.0 } else { 0.42090 }, 0.1, 5.0, 0.065, 0.002;
    cpuct:      f32 = if cfg!(feature = "datagen") { 0.157 } else { 0.26915 }, 0.1, 5.0, 0.065, 0.002;
    cpuct_var_weight: f32 = 0.8109, 0.0, 2.0, 0.085, 0.002;
    cpuct_var_scale: f32 = 0.28056, 0.0, 2.0, 0.02, 0.002;
    cpuct_var_warmup: f32 = 0.50087, 0.0, 1.0, 0.01, 0.002;
    cpuct_visits_scale: f32 = 36.931, 1.0, 512.0, 3.2, 0.002;
    expl_tau: f32 = 0.670, 0.1, 1.0, 0.05, 0.002;
    gini_base: f32 = 0.4662, 0.2, 2.0, 0.0679, 0.002;
    gini_ln_multiplier: f32 = 1.568, 0.4, 3.0, 0.1634, 0.002;
    gini_min: f32 = 2.265, 0.5, 4.0, 0.21, 0.002;
    sharpness_scale: f32 = 2.459, 0.0, 5.0, 0.1, 0.002;
    sharpness_quadratic: f32 = 0.8724, -5.0, 5.0, 0.1, 0.002;
    tm_opt_value1: f64 = 0.639, 0.1, 1.2, 0.072, 0.002;
    tm_opt_value2: f64 = 0.43186, 0.1, 1.0, 0.045, 0.002;
    tm_opt_value3: f64 = 0.66352, 0.1, 1.2, 0.08, 0.002;
    tm_optscale_value1: f64 = 1.640, 0.1, 2.0, 0.15, 0.002;
    tm_optscale_value2: f64 = 2.4785, 0.1, 5.0, 0.3, 0.002;
    tm_optscale_value3: f64 = 0.4843, 0.1, 1.0, 0.05, 0.002;
    tm_optscale_value4: f64 = 0.2602, 0.1, 1.0, 0.025, 0.002;
    tm_max_value1: f64 = 2.8707, 1.0, 10.0, 0.4, 0.002;
    tm_max_value2: f64 = 2.8376, 1.0, 10.0, 0.4, 0.002;
    tm_max_value3: f64 = 2.7119, 1.0, 10.0, 0.4, 0.002;
    tm_maxscale_value1: f64 = 13.282, 1.0, 24.0, 1.2, 0.002;
    tm_maxscale_value2: f64 = 5.1389, 1.0, 12.0, 0.6, 0.002;
    tm_bonus_ply: f64 = 11.453, 1.0, 30.0, 1.5, 0.002;
    tm_bonus_value1: f64 = 0.4536, 0.1, 2.0, 0.05, 0.002;
    tm_max_time: f64 = 0.876, 0.400, 0.990, 0.085, 0.002;
    tm_mtg: i32 = 28, 10, 60, 3, 0.002;
    tm_falling_eval1: f32 = 0.05428, 0.0, 0.2, 0.007, 0.002;
    tm_falling_eval2: f32 = 0.7255, 0.1, 1.0, 0.06, 0.002;
    tm_falling_eval3: f32 = 1.6281, 0.1, 3.0, 0.18, 0.002;
    tm_bmi1: f32 = 0.2567, 0.1, 1.0, 0.04, 0.002;
    tm_bmi2: f32 = 0.8305, 0.1, 2.0, 0.1, 0.002;
    tm_bmi3: f32 = 3.2734, 0.1, 6.4, 0.32, 0.002;
    tm_bmv1: f32 = 3.6254, 0.1, 5.0, 0.25, 0.002;
    tm_bmv2: f32 = 0.3613, 0.1, 1.0, 0.035, 0.002;
    tm_bmv3: f32 = 0.4755, 0.1, 1.0, 0.06, 0.002;
    tm_bmv4: f32 = 2.5457, 0.1, 8.0, 0.4, 0.002;
    tm_bmv5: f32 = 0.6282, 0.1, 1.0, 0.055, 0.002;
    tm_bmv6: f32 = 1.9047, 0.1, 3.0, 0.15, 0.002;
    butterfly_reduction_factor: i32 = 8140, 1, 65536, 819, 0.002;
    butterfly_policy_divisor: i32 = 16405, 1, 131072, 1638, 0.002;
    policy_top_p: f32 = 0.703, 0.1, 1.0, 0.05, 0.002;
    min_policy_actions: i32 = 6, 1, 32, 1, 0.002;
    visit_threshold_power: i32 = 3, 0, 8, 1, 0.002;
    virtual_loss_weight: f64 = 2.501, 1.0, 5.0, 0.25, 0.002;
    contempt: i32 = 0, -1000, 1000, 10, 0.0; //Do not tune this value!
}
