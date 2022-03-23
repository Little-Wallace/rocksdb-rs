mod btree;
mod cache;
pub use cache::{CachableEntry, LRUCache};
pub mod hash;
mod test_sync_point;

pub use test_sync_point::*;

pub use btree::{
    ComparableRecord as BtreeComparable, PageIterator, ThreeLevelBTree as BTree,
    ThreeLevelBTreeIterator as BTreeIter,
};

#[inline(always)]
pub fn decode_fixed_uint32(key: &[u8]) -> u32 {
    unsafe { u32::from_le_bytes(*(key as *const _ as *const [u8; 4])) }
}

#[inline(always)]
pub fn decode_fixed_uint16(key: &[u8]) -> u16 {
    unsafe { u16::from_le_bytes(*(key as *const _ as *const [u8; 2])) }
}

#[inline(always)]
pub fn decode_fixed_uint64(key: &[u8]) -> u64 {
    unsafe { u64::from_le_bytes(*(key as *const _ as *const [u8; 8])) }
}

#[inline(always)]
pub fn difference_offset(origin: &[u8], target: &[u8]) -> usize {
    let mut off = 0;
    let len = std::cmp::min(origin.len(), target.len());
    while off < len && origin[off] == target[off] {
        off += 1;
    }
    off
}

#[inline(always)]
pub fn varint_length(mut v: usize) -> usize {
    let mut len = 1;
    while v >= 128 {
        v >>= 7;
        len += 1;
    }
    len
}

#[inline(always)]
pub fn encode_var_uint32(data: &mut [u8], n: u32) -> usize {
    const B: u32 = 128;
    const MASK: u32 = 255;
    if n < (1 << 7) {
        data[0] = n as u8;
        1
    } else if n < (1 << 14) {
        data[0] = ((n | B) & MASK) as u8;
        data[1] = (n >> 7) as u8;
        2
    } else if n < (1 << 21) {
        data[0] = ((n | B) & MASK) as u8;
        data[1] = ((n >> 7 | B) & MASK) as u8;
        data[2] = (n >> 14) as u8;
        3
    } else if n < (1 << 28) {
        data[0] = ((n | B) & MASK) as u8;
        data[1] = ((n >> 7 | B) & MASK) as u8;
        data[2] = ((n >> 14 | B) & MASK) as u8;
        data[3] = (n >> 21) as u8;
        4
    } else {
        data[0] = ((n | B) & MASK) as u8;
        data[1] = ((n >> 7 | B) & MASK) as u8;
        data[2] = ((n >> 14 | B) & MASK) as u8;
        data[3] = ((n >> 21 | B) & MASK) as u8;
        data[4] = (n >> 28) as u8;
        5
    }
}

#[inline(always)]
pub fn put_var_uint32(data: &mut Vec<u8>, n: u32) {
    let mut tmp: [u8; 5] = [0u8; 5];
    let offset = encode_var_uint32(&mut tmp, n);
    data.extend_from_slice(&tmp[..offset]);
}

#[inline(always)]
pub fn encode_var_uint64(data: &mut [u8], mut v: u64) -> usize {
    const B: u64 = 128;
    let mut offset = 0;
    while v >= B {
        data[offset] = ((v & (B - 1)) | B) as u8;
        v >>= 7u64;
        offset += 1;
    }
    data[offset] = v as u8;
    offset + 1
}

#[inline(always)]
pub fn put_var_uint64(data: &mut Vec<u8>, n: u64) {
    let mut tmp: [u8; 10] = [0u8; 10];
    let offset = encode_var_uint64(&mut tmp, n);
    data.extend_from_slice(&tmp[..offset]);
}

#[inline(always)]
pub fn put_varint32varint32(dist: &mut Vec<u8>, v1: u32, v2: u32) {
    let mut tmp: [u8; 10] = [0u8; 10];
    let offset1 = encode_var_uint32(&mut tmp, v1);
    let offset2 = encode_var_uint32(&mut tmp[offset1..], v2) + offset1;
    dist.extend_from_slice(&tmp[..offset2]);
}

#[inline(always)]
pub fn put_varint32varint64(dist: &mut Vec<u8>, v1: u32, v2: u64) {
    let mut tmp: [u8; 15] = [0u8; 15];
    let offset1 = encode_var_uint32(&mut tmp, v1);
    let offset2 = encode_var_uint64(&mut tmp[offset1..], v2) + offset1;
    dist.extend_from_slice(&tmp[..offset2]);
}

#[inline(always)]
pub fn put_varint64varint64(dist: &mut Vec<u8>, v1: u64, v2: u64) {
    let mut tmp: [u8; 20] = [0u8; 20];
    let offset1 = encode_var_uint64(&mut tmp, v1);
    let offset2 = encode_var_uint64(&mut tmp[offset1..], v2) + offset1;
    dist.extend_from_slice(&tmp[..offset2]);
}

#[inline(always)]
pub fn put_varint32varint32varint64(dist: &mut Vec<u8>, v1: u32, v2: u32, v3: u64) {
    let mut tmp: [u8; 20] = [0u8; 20];
    let offset1 = encode_var_uint32(&mut tmp, v1);
    let offset2 = encode_var_uint32(&mut tmp[offset1..], v2) + offset1;
    let offset3 = encode_var_uint64(&mut tmp[offset2..], v3) + offset2;
    dist.extend_from_slice(&tmp[..offset3]);
}

#[inline(always)]
pub fn put_length_prefixed_slice(buf: &mut Vec<u8>, data: &[u8]) {
    put_var_uint32(buf, data.len() as u32);
    buf.extend_from_slice(data);
}

#[inline(always)]
pub fn get_var_uint32(data: &[u8], offset: &mut usize) -> Option<u32> {
    if data.is_empty() {
        return None;
    }
    const B: u8 = 128;
    const MASK: u32 = 127;
    if (data[0] & B) == 0 {
        *offset += 1;
        return Some(data[0] as u32);
    }
    let mut ret: u32 = 0;
    for i in 0..5 {
        if i > data.len() {
            return None;
        }
        if (data[i] & B) > 0 {
            ret |= (data[i] as u32 & MASK) << (i as u32 * 7);
        } else {
            ret |= (data[i] as u32) << (i as u32 * 7);
            *offset += i + 1;
            return Some(ret);
        }
    }
    None
}

#[inline(always)]
pub fn get_var_uint64(data: &[u8], next_offset: &mut usize) -> Option<u64> {
    const B: u8 = 128;
    const MASK: u64 = 127;

    let mut ret: u64 = 0;
    let mut shift = 0;
    let mut offset = 0;
    while shift <= 63 && offset < data.len() {
        if data[offset] & B > 0 {
            ret |= (data[offset] as u64 & MASK) << shift;
        } else {
            ret |= (data[offset] as u64) << shift;
            *next_offset += offset + 1;
            return Some(ret);
        }
        shift += 7;
        offset += 1;
    }
    *next_offset += offset + 1;
    None
}

#[inline(always)]
pub fn get_length_prefixed_slice<'a>(buf: &'a [u8], offset: &mut usize) -> Option<&'a [u8]> {
    let mut l = 0;
    get_var_uint32(buf, &mut l).map(|val| {
        let limit = l + val as usize;
        *offset += limit;
        &buf[l..limit]
    })
}

#[inline(always)]
pub fn next_key(key: &mut Vec<u8>) {
    if *key.last().unwrap() < 255u8 {
        *key.last_mut().unwrap() += 1;
    } else {
        key.push(0);
    }
}

#[inline(always)]
pub fn get_next_key(key: &[u8]) -> Vec<u8> {
    let mut data = key.to_vec();
    if *data.last().unwrap() < 255u8 {
        *data.last_mut().unwrap() += 1;
    } else {
        data.push(0);
    }
    data
}

const MASK_DELTA: u32 = 0xa282ead8u32;

pub fn crc_mask(crc: u32) -> u32 {
    ((crc >> 15) | crc.wrapping_shl(17)).wrapping_add(MASK_DELTA)
}
pub fn crc_unmask(masked_crc: u32) -> u32 {
    let rot = masked_crc.wrapping_sub(MASK_DELTA);
    (rot >> 17) | rot.wrapping_shl(15)
}
