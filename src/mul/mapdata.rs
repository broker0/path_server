use std::io::BufReader;
use std::fs::File;
use std::io::{Error};
use crate::mulreader::{mul_read_i8, mul_read_u16, mul_read_u32};


#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MulMapTile {
    land_tile: u16,
    z: i8,
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MulMapBlock {
    header: u32,
    cells: [[MulMapTile; 8]; 8],
}

#[derive(Debug, Copy, Clone)]
pub struct LandTile {
    pub land_tile: u16,
    pub z: i8,
}

/// LandBlock stores information about the map. For each tile,
/// its number is stored, as well as the z-coordinate of the vertex.
pub type LandBlock = [[LandTile; 8]; 8];

/// Land stores information about the map. For each tile, its number is stored, as well as the z-coordinate of the vertex.
/// Can return a ref to LandBlock for the given index
#[derive(Debug)]
pub struct Land {
    blocks: Vec<LandBlock>,  // blocks in same order as in file
}


impl Land {
    pub fn read(path: &str, x_blocks: usize, y_blocks: usize) -> Result<Self, Error> {
        let f = &mut BufReader::new(File::open(path)?);

        let mut result = Land {
            blocks: Vec::with_capacity(x_blocks as usize*y_blocks as usize),
        };
        let mut block: LandBlock = [[LandTile {land_tile: 0, z: 0}; 8]; 8];

        for _ in 0..x_blocks {
            for _ in 0..y_blocks {
                // read block data
                let _header = mul_read_u32(f)?; // unused header

                // loop over 8x8 tiles
                for y in 0..8 {
                    for x in 0..8 {
                        block[x][y] = LandTile {land_tile: mul_read_u16(f)?, z: mul_read_i8(f)?};  // short version
                    }
                }

                // adding filled block to block list
                result.blocks.push(block);
            }
        }

        assert_eq!(result.blocks.len(), x_blocks * y_blocks);

        Ok(result)
    }

    pub fn land_block(&self, index: usize) -> &LandBlock {
        debug_assert!(self.blocks.len() > index);
        &self.blocks[index]
    }
}