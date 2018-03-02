use network::packet::{Packet, SerializedBuffer};

enum RPC {

}

pub struct KeepAlive {
//    stream: SerializedBuffer,
}

impl KeepAlive {
    pub const SVUID : i32 = 2;
}

impl Packet for KeepAlive {
    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(KeepAlive::SVUID);
    }
}

pub struct GetInfo {
    pub name: String
}

impl GetInfo {
    pub const SVUID : i32 = 342834823;
}

impl Packet for GetInfo {
    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(GetInfo::SVUID);
        stream.write_string(self.name.clone());
    }

    fn read_params(&mut self, stream: &mut SerializedBuffer, error: bool) {
        self.name = stream.read_string();
    }
}

