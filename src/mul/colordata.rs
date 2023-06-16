use crate::mul;
use mul::mulreader::*;
use std::fs::File;
use std::io::Error;
use std::io::BufReader;
use std::path::Path;
use log::trace;


/// ColorData stores information about the colors of all tiles to display on the map.
/// Allows you to get the color of any tile in rgba8 format
/// first 16384 colors matched to land tiles colors
/// starting at element 16384, colors for static tiles begin.
pub struct ColorData {
    colors: Vec<(u8, u8, u8)>,
}


// color format is |15|14|13|12|11|10| 9| 8| 7| 6| 5| 4| 3| 2| 1| 0|
//                 |  | R| R| R| R| R| G| G| G| G| G| B| B| B| B| B|
// 5 bit per r/g/b components, high bit is unused

const RED_MASK: u16 = 0b0_11111_00000_00000;
const RED_SHIFT: usize = 10;

const GREEN_MASK: u16 = 0b0_00000_11111_00000;
const GREEN_SHIFT: usize = 5;

const BLUE_MASK: u16 = 0b0_00000_00000_11111;
const BLUE_SHIFT: usize = 0;


impl ColorData {
    /// Tries to read data from a file
    pub fn read(data_path: &Path) -> Result<Self, Error> {
        trace!("ColorData::read");
        let mut result = Self {
            colors: Vec::with_capacity(65536),
        };

        let f = File::open(data_path.join("Radarcol.mul"))?;
        let file_len = f.metadata()?.len();
        let mut f = BufReader::new(f);
        let f = &mut f;


        for _ in 0..file_len/2 {
            let color = mul_read_u16(f)?;   // 5/5/5 RGB packed to u16
            result.colors.push(Self::get_rgb(color));
        }

        Ok(result)
    }

    /// returns color for land tile in rgba8 format
    pub fn get_land_color(&self, tile: u16) -> (u8, u8, u8, u8) {
        let (r,g,b) = self.colors[tile as usize];
        (r, g, b, 255)
    }

    /// returns color for static tile in rgba8 format, tile must me less that 49151
    pub fn get_static_color(&self, tile: u16) -> (u8, u8, u8, u8) {
        // debug_assert!(tile <= 65535 - 16384, "tile number too large");
        let (r,g,b) = self.colors[tile as usize + 16384];
        (r, g, b, 255)
    }

    fn get_rgb(color: u16) -> (u8, u8, u8) {
        let r = (color & RED_MASK) >> RED_SHIFT;
        let g = (color & GREEN_MASK) >> GREEN_SHIFT;
        let b = (color & BLUE_MASK) >> BLUE_SHIFT;

        ((r*255/31) as u8, (g*255/31) as u8, (b*255/31) as u8)
    }
}