#[derive(Clone)]
pub struct MctsParams {
    root_pst: Param,
    root_cpuct: Param,
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

impl Default for MctsParams {
    fn default() -> Self {
        Self {
            root_pst: Param::new(1.0, 1.0, 2.5),
            root_cpuct: Param::new(1.41, 0.1, 5.0),
            cpuct: Param::new(1.41, 0.1, 5.0),
            mate_bonus: Param::new(1.0, 0.0, 10.0),
        }
    }
}

impl MctsParams {
    pub fn root_pst(&self) -> f32 {
        self.root_pst.val
    }

    pub fn root_cpuct(&self) -> f32 {
        self.root_cpuct.val
    }

    pub fn cpuct(&self) -> f32 {
        self.cpuct.val
    }

    pub fn mate_bonus(&self) -> f32 {
        self.mate_bonus.val
    }

    pub fn info(self) {
        self.root_pst.info("root_pst");
        self.root_cpuct.info("root_cpuct");
        self.cpuct.info("cpuct");
        self.mate_bonus.info("mate_bonus");
    }

    pub fn set(&mut self, name: &str, val: f32) {
        match name {
            "root_pst" => self.root_pst.set(val),
            "root_cpuct" => self.root_cpuct.set(val),
            "cpuct" => self.cpuct.set(val),
            "mate_bonus" => self.mate_bonus.set(val),
            _ => println!("unknown option!"),
        }
    }
}
