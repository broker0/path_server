use crate::mul;
use mul::mulreader::*;
use crate::mul::{LOOKUP_IDX_RECORD_SIZE, MulLookupIndexRecord};

use std::fs::File;
use std::io::{BufReader, Error, ErrorKind, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::mul::colordata::ColorData;

pub struct ArtSprite {
    pub width: u16,
    pub height: u16,
    pub pixels: Vec<Option<(u8, u8, u8)>>,
}

impl ArtSprite {
    pub fn get_pixel(&self, x: usize, y: usize) -> Option<(u8, u8, u8)> {
        self.pixels.get(y * self.width as usize + x).copied().flatten()
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: Option<(u8, u8, u8)>) {
        match self.pixels.get_mut(y * self.width as usize + x) {
            Some(pixel) => {
                *pixel = color;
            }
            
            None => {
                
            }
        }
    }
}



pub struct ArtLoader {
    data_path: PathBuf,
    pub index: Vec<Option<(u64, usize)>>,   // (pos, size) like LOOKUP_IDX_RECORD_SIZE
}


impl ArtLoader {
    pub fn read(data_path: &Path) -> Result<Self, Error> {
        let mut result = Self {
            data_path: PathBuf::from(data_path),
            index: vec![],
        };

        let f = File::open(data_path.join("artidx.mul"))?;
        let file_len = f.metadata()?.len();
        assert_eq!(file_len as usize % LOOKUP_IDX_RECORD_SIZE, 0);
        let lookup_count = file_len as usize / LOOKUP_IDX_RECORD_SIZE;
        let f = &mut BufReader::new(f);

        for _i in 0..lookup_count {
            let idx = MulLookupIndexRecord {
                offset: mul_read_u32(f)?,
                length: mul_read_u32(f)?,
                unknown1: mul_read_u32(f)?,
            };

            if idx.offset != 0xFFFF_FFFF && idx.length != 0xFFFF_FFFF {
                result.index.push(Some((idx.offset as u64, idx.length as usize)));    // valid entry
            } else {
                result.index.push(None);    // empty entry
            }

            // let last = result.index.last().copied().flatten();
            // if !last.is_none() {
            //     println!("{_i} {:?}", last);
            // }


        }

        Ok(result)
    }


    pub fn read_art(&self, art_idx: u32) -> Result<ArtSprite, Error> {
        let entry = self.index.get(art_idx as usize).copied().flatten();
        let (pos, size) = match entry {
            None => return Err(ErrorKind::InvalidData.into()),
            Some(v) => v,
        };

        let f = File::open(self.data_path.join("art.mul"))?;
        let f = &mut BufReader::new(f);
        f.seek(SeekFrom::Start(pos))?;

        let _flag = mul_read_u32(f)?;   // useless, couldn't determine raw/run from it

        // let mut pixels = vec![];

        let sprite = if art_idx < 0x4000 {
            // "RAW sprite", terrain image of fixed size of 44x44 pixel
            // corners are transparent, main image in central part - square rotated by 45 degrees
            const TILE_SIZE: u16 = 44;

            let (width, height) = (TILE_SIZE, TILE_SIZE);
            let mut sprite = ArtSprite {
                width,
                height,
                pixels: vec![],
            };

            // left and right margin of transparent pixels in current row
            // starts with 2 central pixel
            let mut left = TILE_SIZE /2-1;
            let mut right = TILE_SIZE /2-0;

            // over all pixels in height
            for y in 0..height {
                // over all pixels of row
                for x in 0..width {
                    let pixel = if x < left || x > right {
                        None    // if x in left/right margin, then pixel is transparent
                    } else {
                        Some(ColorData::get_rgb(mul_read_u16(f)?))  // else read pixel from file
                    };

                    sprite.pixels.push(pixel);     // add pixel to result
                }

                // decrease transparent margins for first-half of rows
                if y < TILE_SIZE /2-1 {
                    left -= 1;
                    right += 1;
                } else if y > TILE_SIZE /2-1 {
                    left += 1;  // in second-half of rows increase transparent margins back
                    right -= 1;
                }
            }

            debug_assert_eq!((TILE_SIZE*TILE_SIZE) as usize, sprite.pixels.len());
            // after all pixels are read, check that margins are remain correct
            debug_assert_eq!(left, TILE_SIZE/2-0);
            debug_assert_eq!(right, TILE_SIZE/2-1);

            sprite
        } else {
            let (width, height) = (mul_read_u16(f)?, mul_read_u16(f)?);
            let mut sprite = ArtSprite {
                width,
                height,
                pixels: vec![None; width as usize * height as usize],
            };

            if width == 0 || width >= 1024 || height == 0 || height >= 1024 {
                return Err(ErrorKind::InvalidData.into());
            }
            // println!("art_idx {art_idx} {width}x{height}");

            // relative offsets from art_base for each image row
            let lookups: Vec<u16> = (0..height)
                .map(|_| mul_read_u16(f).unwrap())
                .collect();

            let art_base = f.stream_position()?;

            for y in 0..height {    // for each row
                f.seek(SeekFrom::Start(art_base + lookups[y as usize] as u64*2))?;
                let mut x = 0;  // current x position

                while x < width {
                    let xoffset = mul_read_u16(f)?; // offset from last x position
                    let run = mul_read_u16(f)?;     // length of current fragment

                    if xoffset + run >= 2048 {
                        println!("Too large values for decoding: {xoffset}+{run}={}", xoffset+run);
                        break;
                    }                    
                    
                    match (xoffset, run) {
                        (0, 0) => { // end of line
                            break
                        },

                        (xoffset, run) => {
                            x += xoffset;
                            // println!("x: {x}+{run}, y: {y}");
                            for _ in 0..run {   // draw part of line
                                let val = mul_read_u16(f)?;
                                if val != 0 {
                                    // println!("x={x}, y={y}");
                                    sprite.set_pixel(x as usize, y as usize, Some(ColorData::get_rgb(val)));
                                } else {
                                    println!("Transparent color 0!");
                                }

                                x += 1;
                            }
                        }
                    }
                }
            }

            sprite
        };

        Ok(sprite)
    }
}