use crate::GameState;

use goober::SparseVector;

use super::{
    moves::Move,
    util::{Bitboard, Side},
    STARTPOS,
};

use std::{cmp::Ordering, fmt::Display};

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Board {
    bbs: [u64; 2],
    gaps: u64,
    stm: bool,
    halfm: u8,
    fullm: u16,
}

impl Default for Board {
    fn default() -> Self {
        Self::from_fen(STARTPOS)
    }
}

impl Board {
    pub fn stm(&self) -> usize {
        usize::from(self.stm)
    }

    pub fn occ(&self) -> u64 {
        self.bbs[0] | self.bbs[1] | self.gaps
    }

    pub fn bbs(&self) -> [u64; 3] {
        [self.bbs[0], self.bbs[1], self.gaps]
    }

    pub fn halfm(&self) -> u8 {
        self.halfm
    }

    pub fn boys(&self) -> u64 {
        self.bbs[self.stm()]
    }

    pub fn opps(&self) -> u64 {
        self.bbs[self.stm() ^ 1]
    }

    pub fn fullm(&self) -> u16 {
        self.fullm
    }

    pub fn is_hfm_draw(&self, count: u8) -> bool {
        self.halfm() >= count
    }

    pub fn make(&mut self, mov: Move) {
        if !mov.is_pass() {
            let stm = self.stm();
            let from = mov.from();
            let to = mov.to();

            self.fullm += u16::from(stm == Side::BLU);

            if from != 63 {
                self.bbs[stm] ^= 1 << from;
                self.halfm += 1;
            } else {
                self.halfm = 0;
            }

            self.bbs[stm] ^= 1 << to;

            let singles = Bitboard::singles(to);
            let captures = singles & self.bbs[stm ^ 1];

            self.bbs[0] ^= captures;
            self.bbs[1] ^= captures;
        }

        self.stm = !self.stm;
    }

    pub fn game_over(&self) -> bool {
        let bocc = self.bbs[Side::BLU].count_ones();
        let rocc = self.bbs[Side::RED].count_ones();
        bocc == 0 || rocc == 0 || bocc + rocc == 49 || self.is_hfm_draw(100)
    }

    pub fn material(&self) -> i32 {
        let socc = self.boys().count_ones();
        let nocc = self.opps().count_ones();

        socc as i32 - nocc as i32
    }

    pub fn game_state(&self) -> GameState {
        let socc = self.boys().count_ones();
        let nocc = self.opps().count_ones();

        if socc + nocc == 49 {
            match socc.cmp(&nocc) {
                Ordering::Greater => GameState::Won(0),
                Ordering::Less => GameState::Lost(0),
                Ordering::Equal => GameState::Draw,
            }
        } else if socc == 0 {
            GameState::Lost(0)
        } else if nocc == 0 {
            GameState::Won(0)
        } else if self.is_hfm_draw(100) {
            GameState::Draw
        } else {
            GameState::Ongoing
        }
    }

    #[cfg(feature = "datagen")]
    pub fn bbs(&self) -> [u64; 3] {
        [self.bbs[0], self.bbs[1], self.gaps]
    }

    pub fn map_legal_moves<F: FnMut(Move)>(&self, mut f: F) {
        if self.game_over() {
            return;
        }

        let occ = self.occ();
        let nocc = Bitboard::not(occ);
        let mut boys = self.boys();
        let mut singles = Bitboard::expand(boys) & nocc;

        let mut num = singles.count_ones();

        while singles > 0 {
            let sq = singles.trailing_zeros();
            singles &= singles - 1;

            f(Move::new_single(sq as u8));
        }

        while boys > 0 {
            let from = boys.trailing_zeros();
            boys &= boys - 1;

            let mut doubles = Bitboard::doubles(from as usize) & nocc;

            while doubles > 0 {
                let to = doubles.trailing_zeros();
                doubles &= doubles - 1;

                num += 1;
                f(Move::new_double(from as u8, to as u8));
            }
        }

        if num == 0 {
            f(Move::new_pass());
        }
    }

    pub fn value_features_map<F: FnMut(usize)>(&self, mut f: F) {
        const PER_TUPLE: usize = 3usize.pow(4);
        const POWERS: [usize; 4] = [1, 3, 9, 27];
        const MASK: u64 = 0b0001_1000_0011;

        let boys = self.boys();
        let opps = self.opps();

        for i in 0..6 {
            for j in 0..6 {
                let tuple = 6 * i + j;
                let mut feat = PER_TUPLE * tuple;

                let offset = 7 * i + j;
                let mut b = (boys >> offset) & MASK;
                let mut o = (opps >> offset) & MASK;

                while b > 0 {
                    let mut sq = b.trailing_zeros() as usize;
                    if sq > 6 {
                        sq -= 5;
                    }

                    feat += POWERS[sq];

                    b &= b - 1;
                }

                while o > 0 {
                    let mut sq = o.trailing_zeros() as usize;
                    if sq > 6 {
                        sq -= 5;
                    }

                    feat += 2 * POWERS[sq];

                    o &= o - 1;
                }

                f(feat);
            }
        }
    }

    pub fn get_features(&self) -> SparseVector {
        let mut feats = SparseVector::with_capacity(36);

        self.value_features_map(|feat| feats.push(feat));

        feats
    }

    #[cfg(not(feature = "datagen"))]
    pub fn movegen_bulk(&self, pass: bool) -> u64 {
        let mut moves = u64::from(pass);

        let occ = self.occ();
        let nocc = Bitboard::not(occ);
        let mut boys = self.boys();

        let singles = Bitboard::expand(boys) & nocc;
        moves += u64::from(singles.count_ones());

        while boys > 0 {
            let from = boys.trailing_zeros();
            boys &= boys - 1;

            let doubles = Bitboard::doubles(from as usize) & nocc;
            moves += u64::from(doubles.count_ones());
        }

        moves
    }

    pub fn as_fen(&self) -> String {
        let mut fen = String::new();

        let occ = self.occ();

        let mut empty = 0;

        for rank in (0..7).rev() {
            for file in 0..7 {
                let sq = 7 * rank + file;
                let bit = 1 << sq;

                if occ & bit > 0 {
                    if empty > 0 {
                        fen += format!("{empty}").as_str();
                        empty = 0;
                    }

                    fen += if bit & self.bbs[Side::RED] > 0 {
                        "x"
                    } else if bit & self.bbs[Side::BLU] > 0 {
                        "o"
                    } else {
                        "-"
                    };
                } else {
                    empty += 1;
                }
            }

            if empty > 0 {
                fen += format!("{empty}").as_str();
                empty = 0;
            }

            if rank > 0 {
                fen += "/";
            }
        }

        fen += [" x", " o"][usize::from(self.stm)];
        fen += format!(" {}", self.halfm).as_str();
        fen += format!(" {}", self.fullm).as_str();

        fen
    }

    pub fn from_fen(fen: &str) -> Self {
        let split: Vec<_> = fen.split_whitespace().collect();

        let rows = split[0].split('/').collect::<Vec<_>>();
        let stm = split[1] == "o";
        let halfm = split.get(2).map(|x| x.parse().unwrap_or(0)).unwrap_or(0);
        let fullm = split.get(3).map(|x| x.parse().unwrap_or(1)).unwrap_or(1);

        let mut bbs = [0; 2];
        let mut gaps = 0;
        let mut sq = 0;

        for row in rows.iter().rev() {
            for mut ch in row.chars() {
                ch = ch.to_ascii_lowercase();
                if ('1'..='7').contains(&ch) {
                    sq += ch.to_string().parse().unwrap_or(0);
                } else if let Some(pc) = "xo".chars().position(|el| el == ch) {
                    bbs[pc] |= 1 << sq;
                    sq += 1;
                } else if ch == '-' {
                    gaps |= 1 << sq;
                    sq += 1;
                }
            }
        }

        Self {
            bbs,
            gaps,
            stm,
            halfm,
            fullm,
        }
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for rank in (0..7).rev() {
            for file in 0..7 {
                let sq = 7 * rank + file;
                let bit = 1 << sq;

                let add = if bit & self.bbs[Side::RED] > 0 {
                    " x"
                } else if bit & self.bbs[Side::BLU] > 0 {
                    " o"
                } else if bit & self.gaps > 0 {
                    " -"
                } else {
                    " ."
                };

                write!(f, "{add}")?;
            }

            writeln!(f)?;
        }

        Ok(())
    }
}
