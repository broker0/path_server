use std::fs::{self};
use std::io::Error;
use std::io::Read;
use std::mem;
use std::path::{Path, PathBuf};

#[inline]
pub fn mul_read_u8<R: Read>(reader: &mut R) -> Result<u8, Error> {
    type V = u8;
    let mut buff = [0; mem::size_of::<V>()];
    reader.read_exact(&mut buff)?;
    Ok(V::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_i8<R: Read>(reader: &mut R) -> Result<i8, Error> {
    type V = i8;
    let mut buff = [0; mem::size_of::<V>()];
    reader.read_exact(&mut buff)?;
    Ok(V::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_u16<R: Read>(reader: &mut R) -> Result<u16, Error> {
    let mut buff = [0; mem::size_of::<u16>()];
    reader.read_exact(&mut buff)?;
    Ok(u16::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_i16<R: Read>(reader: &mut R) -> Result<i16, Error> {
    let mut buff = [0; mem::size_of::<i16>()];
    reader.read_exact(&mut buff)?;
    Ok(i16::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_u32<R: Read>(reader: &mut R) -> Result<u32, Error> {
    type V = u32;
    let mut buff = [0; mem::size_of::<V>()];
    reader.read_exact(&mut buff)?;
    Ok(V::from_le_bytes(buff))
}

#[allow(dead_code)]
#[inline]
pub fn mul_read_i32<R: Read>(reader: &mut R) -> Result<i32, Error> {
    let mut buff = [0; mem::size_of::<i32>()];
    reader.read_exact(&mut buff)?;
    Ok(i32::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_u64<R: Read>(reader: &mut R) -> Result<u64, Error> {
    type V = u64;
    let mut buff = [0; mem::size_of::<V>()];
    reader.read_exact(&mut buff)?;
    Ok(V::from_le_bytes(buff))
}

#[inline]
pub fn mul_read_fixed_str20<R: Read>(reader: &mut R) -> Result<[u8; 20], Error> {
    let mut buff = [0; 20];
    reader.read_exact(&mut buff)?;
    Ok(buff)
}

/// Resolves a filename relative to `dir` case-insensitively.
/// If nothing matches, it returns the original path
pub fn get_file_path_ci(dir: &Path, filename: &str) -> PathBuf {
    let path = dir.join(filename);
    if path.exists() {
        return path;
    }

    let lower = filename.to_lowercase();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().to_lowercase() == lower {
                return entry.path();
            }
        }
    }
    
    path
}

/// Resolves a world-specific filename (e.g., map1.mul) relative to `dir`.
/// If the requested `world` is 1 and the file does not exist, it falls back to world 0.
pub fn get_world_file_path(dir: &Path, prefix: &str, world: u8, extension: &str) -> PathBuf {
    let filename = format!("{prefix}{world}{extension}");
    let path = get_file_path_ci(dir, &filename);
    if path.exists() {
        return path;
    }

    if world == 1 {
        let fallback_filename = format!("{prefix}0{extension}");
        let fallback_path = get_file_path_ci(dir, &fallback_filename);
        if fallback_path.exists() {
            return fallback_path;
        }
    }

    path
}

