use std::mem;
use std::io::BufReader;
use std::fs::File;
use std::io::{Error};
use crate::{MulSlice};
use crate::mul::MulLookupIndexRecord;
use crate::mulreader::{mul_read_i8, mul_read_u16, mul_read_u32, mul_read_u8};


#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MulStaticTile {
    static_tile: u16,
    x: u8,  // offset inside block 0..7
    y: u8,
    z: i8,
    unknown1: u16,
}

const STATIC_TILE_SIZE: usize = mem::size_of::<MulStaticTile>();


#[derive(Debug, Copy, Clone, PartialEq)]
pub struct StaticTile {
    pub static_tile: u16,
    pub x: u8,
    pub y: u8,
    pub z: i8,
}


/// Static stores information about static objects in the world.
/// The data is divided into 8x8 blocks, each block can have an arbitrary number of objects.
/// A block is referenced by its index.
pub struct Static {
    statics: Vec<StaticTile>,
    blocks: Vec<Option<MulSlice>>,
}


impl Static {
    pub fn read(idx_path: &str, data_path: &str, x_blocks: usize, y_blocks: usize) -> Result<Self, Error> {
        // read data file with information about tiles
        let f = File::open(data_path)?;
        let f_size = f.metadata()?.len();
        let f = &mut BufReader::new(f);

        assert_eq!(f_size as usize % STATIC_TILE_SIZE, 0);

        // calculate count of tiles in file
        let tiles_count = f_size as usize / STATIC_TILE_SIZE;

        let mut result = Static {
            statics: Vec::with_capacity(tiles_count),
            blocks: Vec::with_capacity(x_blocks as usize * y_blocks as usize),
        };

        for _ in 0..tiles_count {
            // read tile by tile from file
            let mul_tile = MulStaticTile {
                static_tile: mul_read_u16(f)?,  // graphic is number of static tile
                x: mul_read_u8(f)?,         // x,y is offset relative block (in range 0-7)
                y: mul_read_u8(f)?,
                z: mul_read_i8(f)?,         // z coordinate of this tile
                unknown1: mul_read_u16(f)?, // unknown field
            };

            let static_tile = StaticTile{ static_tile: mul_tile.static_tile, x: mul_tile.x, y: mul_tile.y, z: mul_tile.z };
            result.statics.push(static_tile);

            // TODO filter duped tiles. yet doesn't work because deleting tiles breaks the index
            // let last_tile = result.statics.last();
            //
            // if match last_tile {
            //     Some(last_tile) => *last_tile != static_tile,
            //     None => true,
            // } {
            //     result.statics.push(static_tile);
            // } else {
            //     // println!("duped tile {static_tile:?}");
            // }


            // field `unknown1` seems not random
            let u = mul_tile.unknown1;
            if u != 0 {
                // println!("{:?}, {:016b}", result.statics.last(), u);
            }
        }


        // read index file with information about blocks
        let fi = &mut BufReader::new(File::open(idx_path)?);

        // let mut i = 0;
        // let mut n = 0;

        for _ in 0..x_blocks {
            for _ in 0..y_blocks {
                let idx = MulLookupIndexRecord {
                    offset: mul_read_u32(fi)?,
                    length: mul_read_u32(fi)?,
                    unknown1: mul_read_u32(fi)?,
                };

                let offset = idx.offset;
                let length = idx.length;

                let index = offset as usize / STATIC_TILE_SIZE;
                let count = length as usize / STATIC_TILE_SIZE;

                // if offset != 0xFFFF_FFFF && (n != 0 && offset != n) {
                //     println!("Not sequential offset! {}", offset);
                // }

                let block_slice = if offset != 0xFFFF_FFFF {
                    // order this slice by coordinates, for binary search in the future
                    result.statics[index..index+count].sort_by(|a, b| {
                        (a.x, a.y, a.z).cmp(&(b.x, b.y, b.z))
                    });

                    // n = offset + length;
                    // println!("{}: {} {} {:032b}", i, o/STATIC_TILE_SIZE as u32, l/STATIC_TILE_SIZE as u32, u);
                    Some(MulSlice(index, count))
                } else {
                    // miss_counter += 1;
                    None
                };

                result.blocks.push(block_slice);

                // i += 1;
            }
        }

        // println!("Blocks in vector {}", result.blocks.len());
        // println!("Blocks with missed statics - {miss_counter}/{i}");

        Ok(result)
    }

    /// returns a slice corresponding to the whole block,
    /// if there are no static elements in the block, then an empty slice will be returned
    pub fn statics_block(&self, index: usize) -> &[StaticTile] {
        debug_assert!(self.statics.len() > index);
        let block_slice = &self.blocks[index];
        match block_slice {
            Some(b) => &self.statics[b.0..b.0+b.1],
            None => &[],
        }
    }
    /// returns a block slice including static elements
    /// with offsets corresponding to ox, oy,
    /// or an empty slice if there are no matching elements
    /// ox, oy must be strictly in range 0 <= ox, oy <= 7
    pub fn statics_block_tile2(&self, index: usize, ox: u8, oy: u8) -> &[StaticTile] {
        let slice = self.statics_block(index);

        let key = (ox, oy);
        let p = |tile: &StaticTile| (tile.x, tile.y).cmp(&key);

        let left_index = slice.partition_point(|t| p(t) == std::cmp::Ordering::Less);
        let slice = &slice[left_index..];
        let right_index = slice.partition_point(|t| p(t) == std::cmp::Ordering::Equal);
        let slice = &slice[..right_index];

        slice
    }

    // alternative implementation with sequential fetching of tiles after partition
    pub fn statics_block_tile(&self, index: usize, ox: u8, oy: u8) -> &[StaticTile] {
        let slice = self.statics_block(index);

        let key = (ox, oy);
        let p = |tile: &StaticTile| (tile.x, tile.y).cmp(&key);

        let left_index = slice.partition_point(|t| p(t) == std::cmp::Ordering::Less);
        let count = slice[left_index..].iter().take_while(|tile| tile.x==ox && tile.y == oy).count();

        &slice[left_index..(left_index+count)]
    }

}
