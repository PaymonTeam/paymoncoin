use std::collections::HashMap;
use model::transaction::*;
use network::packet::*;

pub struct StateDiffObject {
    pub state: HashMap<Address, i64>
}

impl StateDiffObject {
    pub const SVUID: i32 = 29537194;

    pub fn new() -> Self {
        StateDiffObject {
            state: HashMap::new()
        }
    }
}

impl Serializable for StateDiffObject {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(StateDiffObject::SVUID);
        stream.write_u32(self.state.len() as u32);

        for (addr, value) in &self.state {
            stream.write_bytes(&addr);
            stream.write_i64(*value);
//            addr.serialize_to_stream(stream);
//            value.serialize_to_stream(stream);
        }
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer) {
        let len = stream.read_u32();

        for i in 0..len {
            let mut addr = ADDRESS_NULL;
            addr.read_params(stream);
            let value = stream.read_i64();
            self.state.insert(addr, value);
        }
    }
}

pub struct StateDiff {
    pub state_diff_object: StateDiffObject,
    pub hash: Hash
}

impl StateDiff {
    pub fn from_bytes(mut bytes: SerializedBuffer, hash: Hash) -> Self {
        let mut state_diff_object = StateDiffObject::new();
        state_diff_object.read_params(&mut bytes);

        StateDiff {
            state_diff_object,
            hash
        }
    }
}
