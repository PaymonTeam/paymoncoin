extern crate rustc_serialize;

pub const MIN_WEIGHT_MAGNITUDE : u8 = 4;

pub const HASH_SIZE: usize = 20;
pub const ADDRESS_SIZE: usize = 21; // HASH_SIZE + 1 (checksum byte)

#[derive(Debug)]
pub struct Transaction {
    pub address: [u8; ADDRESS_SIZE],
    pub attachment_timestamp: u64,
    pub attachment_timestamp_lower_bound: u64,
    pub attachment_timestamp_upper_bound: u64,
    pub branch_transaction: [u8; HASH_SIZE],
    pub trunk_transaction: [u8; HASH_SIZE],
    pub bundle: [u8; HASH_SIZE],
    pub current_index: u32,
    pub hash: [u8; HASH_SIZE],
    pub last_index: u32,
    pub nonce: u64,
    pub tag: String,
    pub timestamp: u64,
    pub value: u32
}


impl Transaction{
    pub const SVUID : i32 = 342631123;
}
