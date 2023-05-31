use crate::{mul, MulSlice};
use mul::mulreader::*;
use std::fs::File;
use std::io::{Error, Read};
use std::io::BufReader;
use std::mem;
use log::{debug, trace};
use crate::mul::{LOOKUP_IDX_RECORD_SIZE, MulLookupIndexRecord};


#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct MulMultiPart {
    static_tile: u16,
    x: i16,
    y: i16,
    z: i16,
    flags: u32,
}

#[derive(Debug, Copy, Clone)]
pub struct MulMultiPart7090 {
    pub static_tile: u16,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub flags: u32,
    pub unknown: u32,
}


const MULTI_PART_SIZE: usize = mem::size_of::<MulMultiPart>();
const MULTI_PART7090_SIZE: usize = mem::size_of::<MulMultiPart7090>();

#[derive(Debug, Copy, Clone)]
pub struct MultiPart {
    pub static_tile: u16,
    pub x: i16,
    pub y: i16,
    pub z: i16,
    pub flags: u32,
}


pub struct Multi {
    parts: Vec<MultiPart>,
    multis: Vec<Option<MulSlice>>,
}

impl Multi {
    pub fn read() -> Result<Self, Error> {
        trace!("Multi::read");
        let f = File::open("multi.mul")?;
        let f_size = f.metadata()?.len() as usize;
        let f = &mut BufReader::new(f);

        // read data file with information about tiles
        let fi = File::open("multi.idx")?;
        let fi_size = fi.metadata()?.len();
        let fi = &mut BufReader::new(fi);

        // calculate count of index records and MultTile in files
        let multi_idx_count = fi_size as usize / LOOKUP_IDX_RECORD_SIZE;

        let mut raw_idx = Vec::new();
        let mut pre7090 = true;
        let mut is7090 = true;

        for i in 0..multi_idx_count {
            let idx = MulLookupIndexRecord {
                offset: mul_read_u32(fi)?,
                length: mul_read_u32(fi)?,
                unknown1: mul_read_u32(fi)?,
            };

            raw_idx.push((idx.offset, idx.length));

            if idx.offset != 0xFFFF_FFFF {
                pre7090 &= (idx.length as usize % MULTI_PART_SIZE) == 0;
                is7090 &= (idx.length as usize % MULTI_PART7090_SIZE) == 0;
            }
        }

        if pre7090 && is7090 || (!pre7090 && !is7090){
            panic!("unable to determine version of multi.mul");
        }

        let part_size = if is7090 { MULTI_PART7090_SIZE } else { MULTI_PART_SIZE };
        let multi_tiles_count = f_size as usize / part_size;
        let mut result = Self {
            multis: Vec::with_capacity(multi_idx_count),
            parts: Vec::with_capacity(multi_tiles_count),
        };

        debug!("entry size of multi.mul is {part_size} bytes, total entries {part_size}, is7090={is7090}");

        for (o, l) in raw_idx {
            let value = if o != 0xFFFF_FFFF {
                // convert file offset and length in bytes to index and count
                let index = o as usize / part_size;
                let count = l as usize / part_size;

                Some(MulSlice(index, count))
            } else {
                None
            };

            result.multis.push(value);
        }

        for _ in 0..multi_tiles_count {
            let tile = MulMultiPart {
                static_tile: mul_read_u16(f)?,
                x: mul_read_i16(f)?,
                y: mul_read_i16(f)?,
                z: mul_read_i16(f)?,
                flags: mul_read_u32(f)?,
            };
            if is7090 {
                mul_read_u32(f)?;   // unknown flag in new format
            }

            result.parts.push(MultiPart {
                static_tile: tile.static_tile,
                x: tile.x,
                y: tile.y,
                z: tile.z,
                flags: tile.flags
            });
        }

        Ok(result)
    }

    pub fn multi_parts(&self, multi_id: u16) -> &[MultiPart] {
        match self.multis[multi_id as usize] {
            None => &[],
            Some(MulSlice(index, count)) => &self.parts[index..(index+count)]
        }
    }
}
