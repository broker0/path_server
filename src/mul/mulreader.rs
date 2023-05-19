use std::io::Read;
use std::mem;
use std::io::Error;

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