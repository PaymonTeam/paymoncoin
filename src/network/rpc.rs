use network::packet::{Serializable, SerializedBuffer};

enum RPC {

}

pub struct KeepAlive {
}

impl KeepAlive {
    pub const SVUID : i32 = 2;
}

impl Serializable for KeepAlive {
    fn read_params(&mut self, stream: &mut SerializedBuffer) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(KeepAlive::SVUID);
    }
}
