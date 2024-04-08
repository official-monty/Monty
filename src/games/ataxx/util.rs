pub struct Side;
impl Side {
    pub const RED: usize = 0;
    pub const BLU: usize = 1;
}

pub struct Bitboard;
impl Bitboard {
    pub const ALL: u64 = 0x1_ffff_ffff_ffff;
    pub const NOTR: u64 = 0xfdfb_f7ef_dfbf;
    pub const NOTL: u64 = 0x1_fbf7_efdf_bf7e;

    pub const fn expand(bb: u64) -> u64 {
        let right = (bb & Self::NOTR) << 1;
        let left = (bb & Self::NOTL) >> 1;

        let bb2 = bb | right | left;

        let up = (bb2 << 7) & Self::ALL;
        let down = bb2 >> 7;

        right | left | up | down
    }

    pub const fn not(bb: u64) -> u64 {
        !bb & Self::ALL
    }

    pub fn singles(sq: usize) -> u64 {
        SINGLES[sq]
    }

    pub fn doubles(sq: usize) -> u64 {
        DOUBLES[sq]
    }
}

static SINGLES: [u64; 49] = {
    let mut res = [0; 49];
    let mut sq = 0;

    while sq < 49 {
        res[sq] = Bitboard::expand(1 << sq);
        sq += 1;
    }

    res
};

static DOUBLES: [u64; 49] = {
    let mut res = [0; 49];
    let mut sq = 0;

    while sq < 49 {
        let bb = 1 << sq;

        let singles = Bitboard::expand(bb);
        res[sq] = Bitboard::expand(singles) & Bitboard::not(singles);

        sq += 1;
    }

    res
};
