extern crate rustc_serialize;

pub const MIN_WEIGHT_MAGNITUDE : u8 = 4;

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21; // HASH_SIZE + 1 (checksum byte)

#[derive(Debug)]
pub struct Transaction {
    address: [u8; ADDRESS_SIZE],
    attachment_timestamp: u64,
    attachment_timestamp_lower_bound: u64,
    attachment_timestamp_upper_bound: u64,
    branch_transaction: [u8; HASH_SIZE],
    trunk_transaction: [u8; HASH_SIZE],
    bundle: [u8; HASH_SIZE],
    current_index: u32,
    hash: [u8; HASH_SIZE],
    last_index: u32,
    nonce: u64,
    tag: String,
    timestamp: u64,
    value: u32
}

impl Transaction {

}