pub struct Transaction {
    address: [u8; 21],
    attachment_timestamp: u64,
    attachment_timestamp_lower_bound: u64,
    attachment_timestamp_upper_bound: u64,
    branch_transaction: [u8; 21],
    trunk_transaction: [u8; 21],
    bundle: [u8; 21],
    current_index: u32,
    hash: [u8; 21],
    last_index: u32,
    nonce: u64,
    tag: String,
    timestamp: u64,
    value: u32
}