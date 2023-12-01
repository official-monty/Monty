#[derive(Clone)]
pub struct TunableParams {
    cpuct: Param,
    mate_bonus: Param,
}

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

    fn uci(&self, name: &str) {
        println!(
            "option name {} type spin default {:.0} min {:.0} max {:.0}",
            name,
            self.val * 100.0,
            self.min * 100.0,
            self.max * 100.0,
        );
    }
}

impl Default for TunableParams {
    fn default() -> Self {
        Self {
            cpuct: Param::new(1.41, 0.1, 5.0),
            mate_bonus: Param::new(1.0, 0.0, 10.0),
        }
    }
}

impl TunableParams {
    pub fn cpuct(&self) -> f32 {
        self.cpuct.val
    }

    pub fn mate_bonus(&self) -> f32 {
        self.mate_bonus.val
    }

    pub fn uci_info() {
        let def = Self::default();

        def.cpuct.uci("cpuct");
        def.mate_bonus.uci("mate_bonus");
    }

    pub fn set(&mut self, name: &str, val: f32) {
        match name {
            "cpuct" => self.cpuct.set(val),
            "mate_bonus" => self.mate_bonus.set(val),
            _ => panic!("unknown option!"),
        }
    }
}
