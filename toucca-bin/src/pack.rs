pub type Pack = [u8; 36];

pub fn checksum(pack: &[u8]) -> u8 {
    let mut val = 0;
    for byte in pack {
        val ^= byte;
    }
    val
}

pub fn set(pack: &mut Pack, index: usize, value: bool) {
    let byte_index = index / 8;
    let bit_index = index % 8;
    let byte = &mut pack[byte_index];
    if value {
        *byte |= 1 << bit_index;
    } else {
        *byte &= !(1 << bit_index);
    }
}

pub fn prepare(mut pack: Pack) -> Pack {
    pack[0] = 129;
    pack[34] += 1;
    pack[35] = 128;
    pack[35] = super::pack::checksum(&pack);
    if pack[34] > 127 {
        pack[34] = 0;
    }
    pack
}
