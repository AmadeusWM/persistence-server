use anyhow::Context;
use log::debug;
use moq_transport::{
    serve::{self, Object, ObjectsWriter, TrackWriter},
    session::Publisher,
};
use yrs::{
    updates::{decoder::Decode, encoder::Encode},
    ReadTxn, StateVector, Transact, Update,
};

use crate::{
    room_announce_pattern::RoomAnnouncePattern,
    room_packet::{RoomPacket, SnapshotPacket, StatePacket},
    rooms::Room,
};

pub struct RoomProvider {
    room: Room,
    relay_publisher: Publisher,
    receiver: tokio::sync::mpsc::Receiver<RoomPacket>,
    announce: RoomAnnouncePattern,
    track: String,
}

impl RoomProvider {
    pub fn new(
        room: Room,
        relay_publisher: Publisher,
        receiver: tokio::sync::mpsc::Receiver<RoomPacket>,
        announce: RoomAnnouncePattern,
        track: String,
    ) -> Self {
        Self {
            room,
            relay_publisher,
            receiver,
            announce,
            track,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        let (mut writer, _, reader) = serve::Tracks {
            namespace: self.announce.to_namespace(),
        }
        .produce();
        log::warn!("announcing: {}", self.announce.to_namespace());

        let track = writer.create(&self.track).unwrap();

        let res = tokio::select! {
            res = Self::serve_track(self.receiver, track, self.room) => res.context("failed serving"),
            res = self.relay_publisher.announce(reader) => res.context("provider failed to serve track"),
        };

        log::warn!("finished: {}", self.announce.to_namespace());
        res
    }

    async fn serve_track(
        mut receiver: tokio::sync::mpsc::Receiver<RoomPacket>,
        track: TrackWriter,
        room: Room,
    ) -> anyhow::Result<()> {
        let mut objects = track.objects()?;
        let group_id = 0; // TODO: update for every snapshot
        let mut object_id = 0; // TODO: make incremental
        let mut priority = 0;

        Self::send_initial_snapshot(
            room.clone(),
            &mut objects,
            Object {
                group_id,
                object_id,
                priority,
            },
        )
        .await?;

        while let Some(packet) = receiver.recv().await {
            object_id += 1;
            priority += 1;

            if let RoomPacket::StatePacket(packet) = &packet {
                Self::apply_update(room.clone(), &packet).await?;
            }
            Self::send(
                &mut objects,
                packet,
                Object {
                    group_id,
                    object_id,
                    priority,
                },
            )
            .await?;
        }

        Ok(())
    }

    async fn send_initial_snapshot(
        room: Room,
        writer: &mut ObjectsWriter,
        object: Object,
    ) -> anyhow::Result<()> {
        let room = room.value.lock().await;
        let snapshot = {
            let txn = room.state.transact_mut();
            let update = txn.encode_diff_v1(&StateVector::default());
            SnapshotPacket { update }
        };
        log::info!("sending initial snapshot: {}", snapshot.update.len());
        Self::send(
            writer,
            RoomPacket::StatePacket(StatePacket::DocSnapshot(snapshot)),
            object,
        )
        .await?;
        Ok(())
    }

    async fn send(
        writer: &mut ObjectsWriter,
        packet: RoomPacket,
        object: Object,
    ) -> anyhow::Result<()> {
        log::info!("forwarding packet {:?}", packet);
        let payload = match packet {
            RoomPacket::StatePacket(packet) => serde_json::to_vec(&packet),
            RoomPacket::Other(v) => serde_json::to_vec(&v),
        }?;

        log::info!("\tsize: {}", payload.len());
        writer.write(object, bytes::Bytes::from(payload))?;
        Ok(())
    }

    async fn apply_update(room: Room, packet: &StatePacket) -> anyhow::Result<()> {
        let room_state = room.value.lock().await;
        match packet {
            StatePacket::DocSnapshot(packet) => {
                room_state
                    .state
                    .transact_mut()
                    .apply_update(Update::decode_v1(&packet.update)?);
            }
            StatePacket::DocDelta(packet) => {
                room_state
                    .state
                    .transact_mut()
                    .apply_update(Update::decode_v1(&packet.update)?);
            }
        };
        Ok(())
    }
}
