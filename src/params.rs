#[derive(Clone)]
pub struct TunableParams {
    cpuct: Param,
    fpu: Param,
    cap: Param,
    pawn_threat: Param,
}

#[derive(Clone)]
struct Param {
    val: f64,
    min: f64,
    max: f64,
}

impl Param {
    fn new(val: f64, min: f64, max: f64) -> Self {
        Self { val, min, max }
    }

    fn set(&mut self, val: f64) {
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
            fpu: Param::new(0.5, 0.0, 1.0),
            cap: Param::new(2.0, 0.0, 5.0),
            pawn_threat: Param::new(1.0, 0.0, 5.0),
        }
    }
}

impl TunableParams {
    pub fn cpuct(&self) -> f64 {
        self.cpuct.val
    }

    pub fn fpu(&self) -> f64 {
        self.fpu.val
    }

    pub fn cap(&self) -> f64 {
        self.cap.val
    }

    pub fn pawn_threat(&self) -> f64 {
        self.pawn_threat.val
    }

    pub fn uci_info() {
        let def = Self::default();

        def.cpuct.uci("cpuct");
        def.fpu.uci("fpu");
        def.cap.uci("cap");
    }

    pub fn set(&mut self, name: &str, val: f64) {
        match name {
            "cpuct" => self.cpuct.set(val),
            "fpu" => self.fpu.set(val),
            "cap" => self.cap.set(val),
            _ => panic!("unknown option!")
        }
    }
}