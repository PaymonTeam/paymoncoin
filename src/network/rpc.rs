use network::packet::{Packet, SerializedBuffer};

pub struct KeepAlive {
    stream: SerializedBuffer,
}

impl KeepAlive {
    pub const SVUID : i32 = 2;
}

impl Packet for KeepAlive {
    fn read_params(&self, stream: &SerializedBuffer, error: bool) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(KeepAlive::SVUID);
    }
}
