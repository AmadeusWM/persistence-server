use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub enum RoomPacket {
    /// Packets that have to be stored
    StatePacket(StatePacket),
    /// Packets that can be forwarded immediately
    Other(Value)
}


#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)] // is this necessary here?
#[serde(tag = "packet_type", rename_all = "snake_case")]
pub enum StatePacket {
    DocSnapshot(SnapshotPacket),
    DocDelta(DeltaPacket),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct SnapshotPacket {
    pub update: Vec<u8>
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeltaPacket {
    pub update: Vec<u8>
}
