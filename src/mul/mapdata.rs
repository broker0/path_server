use std::collections::HashMap;
use std::io::{BufReader, Read, Seek};
use std::fs::File;
use std::io::{Error};
use std::io::{SeekFrom};
use std::{fs, mem};
use std::path::Path;
use log::trace;
use crate::mulreader::{mul_read_i8, mul_read_u16, mul_read_u32, mul_read_u64};
use crate::uop_mapdata::{UopHeader, UopEntryHeader, UopEntry, uop_hash};


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

const MUL_MAP_BLOCK_SIZE: usize = mem::size_of::<MulMapBlock>();

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
    pub fn read_mul(data_path: &Path, world: u8, x_blocks: usize, y_blocks: usize) -> Result<Self, Error> {
        trace!("Land::read_mul");
        let f = &mut BufReader::new(File::open(data_path.join(format!("map{world}.mul")))?);

        let mut result = Land {
            blocks: Vec::with_capacity(x_blocks*y_blocks),
        };

        for _ in 0..x_blocks {
            for _ in 0..y_blocks {
                result.read_block(f)?;
            }
        }

        assert_eq!(result.blocks.len(), x_blocks * y_blocks);
        Ok(result)
    }

    pub fn read_uop(data_path: &Path, world: u8, x_blocks: usize, y_blocks: usize) -> Result<Self, Error> {
        trace!("Land::read_uop");
        let f = &mut BufReader::new(File::open(data_path.join(format!("map{world}LegacyMUL.uop")))?);

        let mut result = Land {
            blocks: Vec::with_capacity(x_blocks*y_blocks),
        };

        let entries = Land::read_uop_index(f)?;
        let max_block = x_blocks * y_blocks;

        while result.blocks.len() < max_block {
            let next_block = result.blocks.len();
            let entry_num = next_block >> 12;
            let chunk_name = format!("build/map{world}legacymul/{entry_num:08}.dat");
            let entry_hash = uop_hash(chunk_name.as_bytes());

            if entries.contains_key(&entry_hash) {
                let entry = entries.get(&entry_hash).unwrap();
                let data_offset = entry.data_offset + entry.header_length as u64;
                let data_size = entry.decompressed_length as usize;
                let blocks = data_size / MUL_MAP_BLOCK_SIZE;
                debug_assert_eq!(data_size % MUL_MAP_BLOCK_SIZE , 0, "file will not be read completely");

                f.seek(SeekFrom::Start(data_offset))?;

                let blocks_to_read = blocks.min(max_block-result.blocks.len());

                for _ in 0..blocks_to_read {
                    result.read_block(f)?;
                }
            } else {
                panic!("!! chunk with hash={entry_hash} for block {next_block} not found!!");
            }
        }

        assert_eq!(result.blocks.len(), x_blocks * y_blocks);
        Ok(result)
    }

    #[inline]
    fn read_block<R: Read>(&mut self, reader: &mut R) -> Result<(), Error> {
        let mut block: LandBlock = [[LandTile {land_tile: 0, z: 0}; 8]; 8];

        let _header = mul_read_u32(reader)?; // unused header
        // loop over 8x8 tiles
        for y in 0..8 {
            for x in 0..8 {
                block[x][y] = LandTile {land_tile: mul_read_u16(reader)?, z: mul_read_i8(reader)?};
            }
        }

        // adding filled block to block list
        self.blocks.push(block);

        Ok(())
    }

    pub fn calc_mul_width(data_path: &Path, world: u8, y_blocks: usize) -> usize {
        let meta = fs::metadata(data_path.join(format!("map{world}.mul"))).unwrap();
        let fsize = meta.len() as usize;
        let blocks = fsize / MUL_MAP_BLOCK_SIZE;

        blocks / y_blocks
    }

    pub fn calc_uop_width(data_path: &Path, world: u8, y_blocks: usize) -> usize {
        let f = &mut BufReader::new(File::open(data_path.join(format!("map{world}LegacyMUL.uop"))).unwrap());
        let entries = Land::read_uop_index(f).unwrap();

        let mut entry_num = 0;
        let mut blocks = 0;
        loop {
            let chunk_name = format!("build/map{world}legacymul/{entry_num:08}.dat");
            let entry_hash = uop_hash(chunk_name.as_bytes());

            match entries.get(&entry_hash) {
                None => break,
                Some(entry) => {
                    entry_num += 1;
                    let blocks_in_entry = entry.decompressed_length as usize / MUL_MAP_BLOCK_SIZE;
                    blocks += blocks_in_entry;
                }
            }
        }

        blocks / y_blocks
    }

    fn read_uop_index<R: Read+Seek>(reader: &mut R) -> Result<HashMap<u64, UopEntry>, Error> {
        let uop_header = UopHeader {
            magic: mul_read_u32(reader)?,
            version: mul_read_u32(reader)?,
            timestamp: mul_read_u32(reader)?,
            next_block_offset: mul_read_u64(reader)?,
            block_size: mul_read_u32(reader)?,
            entry_count: mul_read_u32(reader)?,
        };

        let magic = uop_header.magic;
        assert_eq!(magic, 0x0050594D, "file signature is invalid");

        reader.seek(SeekFrom::Start(uop_header.next_block_offset))?;
        UopEntryHeader { // Unused data but needs to be read
            entry_count: mul_read_u32(reader)?,
            next_block_offset: mul_read_u64(reader)?,
        };

        let mut entries = HashMap::new();
        for _ in 0..uop_header.entry_count {
            let uop_entry = UopEntry {
                data_offset: mul_read_u64(reader)?,
                header_length: mul_read_u32(reader)?,
                compressed_length: mul_read_u32(reader)?,
                decompressed_length: mul_read_u32(reader)?,
                entry_hash: mul_read_u64(reader)?,
                crc: mul_read_u32(reader)?,
                is_compressed: mul_read_u16(reader)?,
            };

            entries.insert(uop_entry.entry_hash, uop_entry);
        }

        Ok(entries)
    }

    pub fn land_block(&self, index: usize) -> &LandBlock {
        debug_assert!(self.blocks.len() > index);
        &self.blocks[index]
    }
}

