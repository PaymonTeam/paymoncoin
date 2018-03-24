use network::packet::{Serializable, SerializedBuffer};

pub struct KeepAlive {}

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

pub struct GetNodeInfo {}

impl GetNodeInfo {
    pub const SVUID : i32 = 3;
}

impl Serializable for GetNodeInfo {
    fn read_params(&mut self, stream: &mut SerializedBuffer) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
    }
}
pub struct NodeInfo {

}

impl NodeInfo {
    pub const SVUID : i32 = 3;
}

impl Serializable for NodeInfo {
    fn read_params(&mut self, stream: &mut SerializedBuffer) {
    }

    fn serialize_to_stream(&self, stream: &mut SerializedBuffer) {
        stream.write_i32(Self::SVUID);
    }
}