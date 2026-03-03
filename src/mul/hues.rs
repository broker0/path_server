use crate::mul;
use mul::mulreader::*;
use std::fs::File;
use std::io::Error;
use std::io::BufReader;
use std::path::Path;
use log::trace;
use crate::mul::colordata::ColorData;

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MulHueEntry {
    color_table: [u16; 32],
    table_start: u16,
    table_end: u16,
    name: [u8; 20],
}

#[derive(Debug, Copy, Clone)]
#[repr(C, packed)]
struct MulHueGroup {
    header: u32,
    entries: [MulHueEntry; 8],
}

const MUL_HUE_GROUP_SIZE: usize = std::mem::size_of::<MulHueGroup>();

#[derive(Debug)]
pub struct HueData {
    name: String,
    hues: Vec<(u8, u8, u8)>,
}

#[derive(Debug)]
pub struct HuesData {
    pub hues: Vec<HueData>,
}


impl HuesData {
    pub fn read(data_path: &Path) -> Result<Self, Error> {
        trace!("HuesData::read");
        let mut result = Self {
            hues: Vec::with_capacity(65536),
        };

        let f = File::open(data_path.join("hues.mul"))?;
        let file_len = f.metadata()?.len();
        let f = &mut BufReader::new(f);

        assert_eq!(file_len as usize % MUL_HUE_GROUP_SIZE, 0);
        let groups_count = file_len as usize / MUL_HUE_GROUP_SIZE;
        // println!("{file_len}, {groups_count}");


        for _group in 0..groups_count {
            // read MulHueGroup without creating structure
            let _header = mul_read_u32(f)?;
            for _entry in 0..8 { // entries
                let entry = MulHueEntry {
                    color_table: std::array::from_fn(|_| mul_read_u16(f).unwrap()),
                    table_start: mul_read_u16(f)?,
                    table_end: mul_read_u16(f)?,
                    name: mul_read_fixed_str20(f)?,
                };

                let mut hue_data = HueData {
                    hues: vec![], 
                    name: String::from_utf8_lossy(&entry.name).to_owned().parse().unwrap(),
                };

                // unpack packed rgb values
                for color in entry.color_table {
                    hue_data.hues.push(ColorData::get_rgb(color))
                }
                result.hues.push(hue_data);
            }
        }

        // println!("{:?}", result);

        Ok(result)
    }


}