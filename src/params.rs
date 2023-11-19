#[derive(Clone)]
pub struct TunableParams {
    cpuct: Param,
    fpu: Param,
    cap: Param,
    promo: Param,
    mate_bonus: Param,
    scale: Param,
    mvv_lva: Param,
    good_see: Param,
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
            promo: Param::new(2.0, 0.0, 5.0),
            mate_bonus: Param::new(1.0, 0.0, 10.0),
            scale: Param::new(4.0, 1.0, 8.0),
            mvv_lva: Param::new(0.2, 0.0, 5.0),
            good_see: Param::new(2.0, 0.0, 5.0),
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

    pub fn promo(&self) -> f64 {
        self.promo.val
    }

    pub fn mate_bonus(&self) -> f64 {
        self.mate_bonus.val
    }

    pub fn scale(&self) -> f64 {
        self.scale.val
    }

    pub fn mvv_lva(&self) -> f64 {
        self.mvv_lva.val
    }

    pub fn good_see(&self) -> f64 {
        self.good_see.val
    }

    pub fn uci_info() {
        let def = Self::default();

        def.cpuct.uci("cpuct");
        def.fpu.uci("fpu");
        def.cap.uci("cap");
        def.promo.uci("promo");
        def.mate_bonus.uci("mate_bonus");
        def.scale.uci("scale");
        def.mvv_lva.uci("mvv_lva");
        def.good_see.uci("good_see")
    }

    pub fn set(&mut self, name: &str, val: f64) {
        match name {
            "cpuct" => self.cpuct.set(val),
            "fpu" => self.fpu.set(val),
            "cap" => self.cap.set(val),
            "promo" => self.promo.set(val),
            "mate_bonus" => self.mate_bonus.set(val),
            "scale" => self.scale.set(val),
            "mvv_lva" => self.mvv_lva.set(val),
            "good_see" => self.good_see.set(val),
            _ => panic!("unknown option!"),
        }
    }
}
