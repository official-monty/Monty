use std::io::{Error, ErrorKind, Write};

use crate::{
    chess::{Castling, Move, Position},
    interleave::{interleave, FastDeserialise},
    read_into_primitive,
};

const GAME_HEADER_SIZE: usize = 43;

pub struct SearchData {
    pub best_move: Move,
    pub score: f32,
    pub visit_distribution: Option<Vec<(Move, u32)>>,
}

impl SearchData {
    pub fn new<T: Copy + Into<Move>>(
        best_move: T,
        score: f32,
        visit_distribution: Option<Vec<(T, u32)>>,
    ) -> Self {
        let mut visit_distribution: Option<Vec<(Move, u32)>> = visit_distribution.map(|x| {
            x.iter()
                .map(|&(mov, visits)| (mov.into(), visits))
                .collect()
        });

        if let Some(dist) = visit_distribution.as_mut() {
            dist.sort_by_key(|(mov, _)| u16::from(*mov));
        }

        Self {
            best_move: best_move.into(),
            score,
            visit_distribution,
        }
    }
}

pub struct MontyFormat {
    pub startpos: Position,
    pub castling: Castling,
    pub result: f32,
    pub moves: Vec<SearchData>,
}

impl MontyFormat {
    pub fn new(startpos: Position, castling: Castling) -> Self {
        Self {
            startpos,
            castling,
            result: 0.0,
            moves: Vec::new(),
        }
    }

    pub fn push(&mut self, position_data: SearchData) {
        self.moves.push(position_data);
    }

    pub fn pop(&mut self) -> Option<SearchData> {
        self.moves.pop()
    }

    pub fn serialise_into_buffer(&self, writer: &mut Vec<u8>) -> std::io::Result<()> {
        if !writer.is_empty() {
            return Err(Error::other("Buffer is not empty!"));
        }

        let compressed = CompressedChessBoard::from(self.startpos);

        for bb in compressed.bbs {
            writer.write_all(&bb.to_le_bytes())?;
        }

        writer.write_all(&compressed.stm.to_le_bytes())?;
        writer.write_all(&compressed.enp_sq.to_le_bytes())?;
        writer.write_all(&compressed.rights.to_le_bytes())?;
        writer.write_all(&compressed.halfm.to_le_bytes())?;
        writer.write_all(&compressed.fullm.to_le_bytes())?;

        for side in self.castling.rook_files() {
            for rook in side {
                writer.write_all(&rook.to_le_bytes())?;
            }
        }

        let result = (self.result * 2.0) as u8;
        writer.write_all(&result.to_le_bytes())?;

        for data in &self.moves {
            if data.score.clamp(0.0, 1.0) != data.score {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Score outside valid range!",
                ));
            }

            let score = (data.score * f32::from(u16::MAX)) as u16;

            writer.write_all(&u16::from(data.best_move).to_le_bytes())?;
            writer.write_all(&score.to_le_bytes())?;

            let num_moves = data
                .visit_distribution
                .as_ref()
                .map(|dist| dist.len())
                .unwrap_or(0) as u8;

            writer.write_all(&num_moves.to_le_bytes())?;

            if let Some(dist) = data.visit_distribution.as_ref() {
                let max_visits = dist
                    .iter()
                    .max_by_key(|(_, visits)| visits)
                    .map(|x| x.1)
                    .unwrap_or(0);
                for (_, visits) in dist {
                    let scaled_visits = (*visits as f32 * 255.0 / max_visits as f32).round() as u8;
                    writer.write_all(&scaled_visits.to_le_bytes())?;
                }
            }
        }

        writer.write_all(&[0; 2])?;
        Ok(())
    }

    pub fn deserialise_from(reader: &mut impl std::io::BufRead) -> std::io::Result<Self> {
        let mut bbs = [0u64; 4];
        for bb in &mut bbs {
            *bb = read_into_primitive!(reader, u64);
        }

        let stm = read_into_primitive!(reader, u8);
        let enp_sq = read_into_primitive!(reader, u8);
        let rights = read_into_primitive!(reader, u8);
        let halfm = read_into_primitive!(reader, u8);
        let fullm = read_into_primitive!(reader, u16);

        let compressed = CompressedChessBoard {
            bbs,
            stm,
            enp_sq,
            rights,
            halfm,
            fullm,
        };
        let startpos = Position::from(compressed);

        let mut rook_files = [[0; 2]; 2];
        for side in &mut rook_files {
            for rook in side {
                *rook = read_into_primitive!(reader, u8);
            }
        }

        let castling = Castling::from_raw(&startpos, rook_files);

        let result = read_into_primitive!(reader, u8) as f32 / 2.0;

        let mut moves = Vec::new();

        let mut pos = startpos;

        loop {
            let best_move = Move::from(read_into_primitive!(reader, u16));

            if best_move == Move::NULL {
                break;
            }

            let score = f32::from(read_into_primitive!(reader, u16)) / f32::from(u16::MAX);

            let num_moves = read_into_primitive!(reader, u8);

            let visit_distribution = if num_moves == 0 {
                None
            } else {
                let mut dist = Vec::with_capacity(usize::from(num_moves));

                pos.map_legal_moves(&castling, |mov| dist.push((mov, 0)));
                dist.sort_by_key(|(mov, _)| u16::from(*mov));

                assert_eq!(
                    dist.len(),
                    usize::from(num_moves),
                    "{}\n{:?}",
                    pos.as_fen(),
                    castling.rook_files(),
                );

                for entry in &mut dist {
                    entry.1 = u32::from(read_into_primitive!(reader, u8));
                }

                Some(dist)
            };

            moves.push(SearchData {
                best_move,
                score,
                visit_distribution,
            });

            pos.make(best_move, &castling);
        }

        Ok(MontyFormat {
            startpos,
            castling,
            result,
            moves,
        })
    }

    pub fn interleave(input_paths: &[String], output_path: &str, seed: u64) -> std::io::Result<()> {
        interleave::<Self>(input_paths, output_path, seed)
    }
}

impl FastDeserialise for MontyFormat {
    fn deserialise_fast_into_buffer(
        reader: &mut impl std::io::BufRead,
        buffer: &mut Vec<u8>,
    ) -> std::io::Result<()> {
        buffer.clear();
        buffer.reserve(GAME_HEADER_SIZE);

        let mut header = [0u8; GAME_HEADER_SIZE];
        reader.read_exact(&mut header)?;
        buffer.extend_from_slice(&header);

        loop {
            let mut move_header = [0u8; 5];
            reader.read_exact(&mut move_header)?;
            buffer.extend_from_slice(&move_header);

            let best_move = Move::from(u16::from_le_bytes([move_header[0], move_header[1]]));
            if best_move == Move::NULL {
                break;
            }

            let move_count = usize::from(move_header[4]);
            if move_count > 0 {
                let start_len = buffer.len();
                buffer.resize(start_len + move_count, 0);
                reader.read_exact(&mut buffer[start_len..])?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Copy)]
pub struct CompressedChessBoard {
    pub bbs: [u64; 4],
    pub stm: u8,
    pub enp_sq: u8,
    pub rights: u8,
    pub halfm: u8,
    pub fullm: u16,
}

impl From<Position> for CompressedChessBoard {
    fn from(board: Position) -> Self {
        let bbs = board.bbs();

        Self {
            bbs: [
                bbs[1],
                bbs[5] ^ bbs[6] ^ bbs[7],
                bbs[3] ^ bbs[4] ^ bbs[7],
                bbs[2] ^ bbs[4] ^ bbs[6],
            ],
            stm: board.stm() as u8,
            enp_sq: board.enp_sq(),
            rights: board.rights(),
            halfm: board.halfm(),
            fullm: board.fullm(),
        }
    }
}

impl From<CompressedChessBoard> for Position {
    fn from(value: CompressedChessBoard) -> Self {
        let qbbs = value.bbs;

        let mut bbs = [0; 8];

        let blc = qbbs[0];
        let rqk = qbbs[1];
        let nbk = qbbs[2];
        let pbq = qbbs[3];

        let occ = rqk | nbk | pbq;
        let pnb = occ ^ qbbs[1];
        let prq = occ ^ qbbs[2];
        let nrk = occ ^ qbbs[3];

        bbs[0] = occ ^ blc;
        bbs[1] = blc;
        bbs[2] = pnb & prq;
        bbs[3] = pnb & nrk;
        bbs[4] = pnb & nbk & pbq;
        bbs[5] = prq & nrk;
        bbs[6] = pbq & prq & rqk;
        bbs[7] = nbk & rqk;

        #[allow(deprecated)]
        Position::from_raw(
            bbs,
            value.stm > 0,
            value.enp_sq,
            value.rights,
            value.halfm,
            value.fullm,
        )
    }
}
