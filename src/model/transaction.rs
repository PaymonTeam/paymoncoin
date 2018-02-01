extern crate rustc_serialize;

use storage::hive::Error;

pub const MIN_WEIGHT_MAGNITUDE : i8 = 9;

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

impl Transaction {
    pub fn address_to_string(bytes: [u8; 21]) -> String {
        let strs: Vec<String> = bytes.iter()
            .map(|b| format!("{:02X}", b))
            .collect();
        format!("P{}", strs.join(""))
    }

    pub fn address_to_bytes(address: String) -> Result<[u8; 21], Error> {
        use self::rustc_serialize::hex::{FromHex, FromHexError};

        if !address.starts_with("P") {
            return Err(Error::InvalidAddress);
        }

        match address[1..].to_string().from_hex() {
            Err(_) => return Err(Error::InvalidAddress),
            Ok(vec) => {
                let bytes:&[u8] = vec.as_ref();
                let mut ret_bytes = [0u8; 21];

                if bytes.len() == 21 {
                    ret_bytes.copy_from_slice(&bytes);
                    return Ok(ret_bytes)
                } else {
                    return Err(Error::InvalidAddress);
                }
            }
        };
//        let bytes = address[1..].to_string().from_hex().unwrap_or(|| return Err(Error::InvalidAddress));

    }
}