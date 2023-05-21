use crate::mul;
use mul::mulreader::*;
use std::fs::File;
use std::io::{Error, Seek};
use std::io::BufReader;
use std::mem;

/*
    mul file raw structures, full set of fields
 */
#[repr(C, packed)]
struct MulLandTile {
    flags: u32,
    texture_id: u16,
    tile_name: [u8; 20],
}

#[repr(C, packed)]
struct MulLandTileFirst7090 {
    flags: u64,
    texture_id: u16,
    tile_name: [u8; 20],
}

#[repr(C, packed)]
struct MulLandTileGroup {
    header: u32,
    tiles: [MulLandTile; 32],
}

#[repr(C, packed)]
struct MulStaticObject {
    flags: u32,
    weight: u8,
    quality: u8,
    quantity: u8,
    anim_id: u16,
    unk1: u16,
    unk2: u8,
    unk3: u8,
    hue: u8,
    unk4: u16,
    height: u8,
    tile_name: [u8; 20],
}

#[repr(C, packed)]
struct MulStaticObject7090 {
    flags: u64,
    weight: u8,
    quality: u8,
    quantity: u8,
    anim_id: u16,
    unk1: u16,
    unk2: u8,
    unk3: u8,
    hue: u8,
    unk4: u16,
    height: u8,
    tile_name: [u8; 20],
}


#[repr(C, packed)]
struct MulStaticGroup {
    header: u32,
    tiles: [MulStaticObject; 32],
}

#[repr(C, packed)]
struct MulStaticGroup7090 {
    header: u32,
    tiles: [MulStaticObject7090; 32],
}


// MUST BE 1188 bytes TODO add tests
const STATIC_TILE_GROUP_SIZE: usize = mem::size_of::<MulStaticGroup>();
const STATIC_TILE_GROUP7090_SIZE: usize = mem::size_of::<MulStaticGroup7090>();


// now the use of these flags requires a cast, usually to u32.
// we need to come up with a more convenient interface or replace it with constants
#[repr(u32)]
#[allow(dead_code)]
pub enum MulTileFlags {
    Background  = 0x0000_0001,
    Weapon      = 0x0000_0002,
    Transparent = 0x0000_0004,
    Translucent = 0x0000_0008,
    Wall        = 0x0000_0010,
    Damaging    = 0x0000_0020,
    Impassable  = 0x0000_0040,
    Wet         = 0x0000_0080,
    Unknown1    = 0x0000_0100,
    Surface     = 0x0000_0200,
    Bridge      = 0x0000_0400,
    Generic     = 0x0000_0800,  // Stackable?
    Window      = 0x0000_1000,
    NoShoot     = 0x0000_2000,
    PrefixA     = 0x0000_4000,
    PrefixAn    = 0x0000_8000,
    Internal    = 0x0001_0000,
    Foliage     = 0x0002_0000,
    PartialHue  = 0x0004_0000,
    Unknown2    = 0x0008_0000,
    Map         = 0x0010_0000,
    Container   = 0x0020_0000,
    Wearable    = 0x0040_0000,
    LightSource = 0x0080_0000,
    Animated    = 0x0100_0000,
    HoverOver   = 0x0200_0000,
    NoDiagonal  = 0x0400_0000,
    Armor       = 0x0800_0000,
    Roof        = 0x1000_0000,
    Door        = 0x2000_0000,
    StairBack   = 0x4000_0000,
    StairRight  = 0x8000_0000,
}


/*
    public structure, refined
 */
pub struct LandTileData {
    pub flags: u32,
}

pub struct StaticTileData {
    pub flags: u32,
    pub height: u8,
}

/// TileData stores information about tiles of the map and tiles representing static objects
pub struct TileData {
    pub land_tiles: Vec<LandTileData>,
    pub static_tiles: Vec<StaticTileData>,
}

impl TileData {
    pub fn read() -> Result<Self, Error> {
        let mut result = TileData {
            land_tiles: Vec::with_capacity(16384),
            static_tiles: Vec::with_capacity(16384),
        };

        let f = File::open("tiledata.mul")?;
        let file_len = f.metadata()?.len();
        let f = &mut BufReader::new(f);

        let is7090 = file_len == 3_188_736;
        let is7000 = file_len == 1_644_544;

        println!("{is7000} {is7090}");

        // the first half of the file (roughly) contains information about MulLandTile
        // 512 block of tile blocks
        // each block contain 32 tiles
        // total 512*32=16384 tiles
        const LAND_TILES: i32 = 0x4000;
        for i in 0..LAND_TILES {
            // TODO make reading different versions more understandable
            if is7090 {
                if i == 1 || i > 0 && (i & 0x1F) == 0 {
                    let _header = mul_read_u32(f)?;   // unknown _header
                }
            } else if (i & 0x1F) == 0 {
                let _header = mul_read_u32(f)?;   // unknown _header
            }

            let tile = MulLandTile{
                flags: if is7090 { mul_read_u64(f)? as u32} else { mul_read_u32(f)?},
                texture_id: mul_read_u16(f)?,
                tile_name: mul_read_fixed_str20(f)?,
            };

            result.land_tiles.push(LandTileData {flags: tile.flags});
        }


        // The second half of the file contains the StaticTile data.
        // tiles also lay in blocks of 32 tile
        // but count of groups calculated from file size and size of MulStaticGroup[7090]
        let left_bytes = file_len - f.stream_position()?;
        let group_size = if is7090 { STATIC_TILE_GROUP7090_SIZE as u64 } else { STATIC_TILE_GROUP_SIZE as u64 };
        let static_objects =  (left_bytes / group_size) * 32;

        // println!("{static_objects} static objects in tiledata {left_bytes}");

        for i in 0..static_objects {
            if i & 0x1F == 0 {
                let _header = mul_read_u32(f)?;   // unknown _header
            }

            let tile = MulStaticObject {
                flags: if is7090 { mul_read_u64(f)? as u32 } else { mul_read_u32(f)? },
                weight: mul_read_u8(f)?,
                quality: mul_read_u8(f)?,
                unk1: mul_read_u16(f)?,   // unknown field
                unk2: mul_read_u8(f)?,   // unknown field
                quantity: mul_read_u8(f)?,
                anim_id: mul_read_u16(f)?,
                unk3: mul_read_u8(f)?,
                hue: mul_read_u8(f)?,
                unk4: mul_read_u16(f)?,
                height: mul_read_u8(f)?,
                tile_name: mul_read_fixed_str20(f)?,
            };

            // println!("{}",  std::str::from_utf8(&tile_name).unwrap());
            // println!("Static tile flags {:032b}", tile.flags);

            result.static_tiles.push(StaticTileData {flags: tile.flags, height: tile.height});
        }

        Ok(result)
    }

    pub fn get_land_tile(&self, land_tile: u16) -> &LandTileData {
        &self.land_tiles[land_tile as usize]
    }

    pub fn get_static_tile(&self, static_tile: u16) -> &StaticTileData {
        &self.static_tiles[static_tile as usize]
    }
}