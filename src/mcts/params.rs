use rand_distr::Distribution;

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

#[cfg(feature = "datagen")]
impl MctsParams {
    pub fn perturb(&self) -> Self {
        use rand::thread_rng;
        use rand_distr::Normal;

        let mut rng = thread_rng();
        let dist = Normal::<f32>::new(1.0, 0.07).unwrap();

        let mut cln = self.clone();

        let mut sample = || dist.sample(&mut rng).max(0.5);

        cln.root_pst.val *= sample();
        cln.root_cpuct.val *= sample();
        cln.cpuct.val *= sample();
        cln.cpuct_var_weight.val *= sample();
        cln.cpuct_var_scale.val *= sample();
        cln.cpuct_visits_scale.val *= sample();

        cln
    }
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
}
