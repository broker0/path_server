use std::num::Wrapping;

#[repr(C, packed)]
pub struct UopHeader {
    pub magic: u32,
    pub version: u32,
    pub timestamp: u32,
    pub next_block_offset: u64,
    pub block_size: u32,
    pub entry_count: u32,
}


#[repr(C, packed)]
pub struct UopEntryHeader {
    pub entry_count: u32,
    pub next_block_offset: u64,
}

#[repr(C, packed)]
pub struct UopEntry {
    pub data_offset: u64,
    pub header_length: u32,
    pub compressed_length: u32,
    pub decompressed_length: u32,
    pub entry_hash: u64,
    pub crc: u32,
    pub is_compressed: u16,
}


pub fn uop_hash(mut src: &[u8]) -> u64 {
    let mut a = Wrapping((src.len() as u32).wrapping_add(0xdeadbeef));
    let mut b = a;
    let mut c = a;

    while src.len() > 12 {
        a += partial_read_u32(src);
        b += partial_read_u32(&src[4..]);
        c += partial_read_u32(&src[8..]);

        a = (a - c) ^ ((c << 4) | (c >> 28));
        c += b;
        b = (b - a) ^ ((a << 6) | (a >> 26));
        a += c;
        c = (c - b) ^ ((b << 8) | (b >> 24));
        b += a;
        a = (a - c) ^ ((c << 16) | (c >> 16));
        c += b;
        b = (b - a) ^ ((a << 19) | (a >> 13));
        a += c;
        c = (c - b) ^ ((b << 4) | (b >> 28));
        b += a;

        src = &src[12..];
    }

    if src.len() > 0 {
        a += partial_read_u32(src);
        b += partial_read_u32(&src[4..]);
        c += partial_read_u32(&src[8..]);

        c = (c ^ b) - ((b << 14) | (b >> 18));
        a = (a ^ c) - ((c << 11) | (c >> 21));
        b = (b ^ a) - ((a << 25) | (a >> 7));
        c = (c ^ b) - ((b << 16) | (b >> 16));
        a = (a ^ c) - ((c << 4) | (c >> 28));
        b = (b ^ a) - ((a << 14) | (a >> 18));
        c = (c ^ b) - ((b << 24) | (b >> 8));
    }

    ((b.0 as u64) << 32) | (c.0 as u64)
}

fn partial_read_u32(s: &[u8]) -> Wrapping<u32> {
    let l = s.len();
    let mut v = 0;

    if l > 0 {
        v |= s[0] as u32;
    }

    if l > 1 {
        v |= (s[1] as u32) << 8;
    }

    if l > 2 {
        v |= (s[2] as u32) << 16;
    }

    if l > 3 {
        v |= (s[3] as u32) << 24;
    }

    Wrapping(v)
}

