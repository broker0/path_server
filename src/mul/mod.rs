pub mod mulreader;
pub mod tiledata;
pub mod mapdata;
pub mod staticdata;
pub mod multidata;
pub mod colordata;

use std::mem;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MulLookupIndexRecord {
    offset: u32,    // must be multiple of the size StaticTile
    length: u32,    // also
    unknown1: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct MulSlice(pub usize, pub usize);

const LOOKUP_IDX_RECORD_SIZE: usize = mem::size_of::<MulLookupIndexRecord>();


pub use tiledata::TileData;
pub use mapdata::Land;
pub use staticdata::Static;
pub use multidata::Multi;
// pub use world::WorldSource;
// pub use colordata::