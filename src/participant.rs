use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use moq_transport::{
    serve::{ObjectReader, ObjectsReader, TrackReader, TrackReaderMode},
    session::Subscriber,
};
use serde_json::Value;

use crate::room_packet::{RoomPacket, StatePacket};

pub struct Participant {
    packet_reader: TrackReader,
    packet_sender: tokio::sync::mpsc::Sender<RoomPacket>,
}

impl Participant {
    pub fn new(
        packet_reader: TrackReader,
        packet_sender: tokio::sync::mpsc::Sender<RoomPacket>,
    ) -> Self {
        Self { packet_reader, packet_sender }
    }

    pub async fn run_recv(self) -> anyhow::Result<()> {
        match self
            .packet_reader
            .mode()
            .await
            .context("failed to get mode")?
        {
            TrackReaderMode::Objects(objects) => self.recv_objects(objects).await,
            _ => Err(anyhow::format_err!(
                "Only object streams are implemented for participants"
            )),
        }?;
        Ok(())
    }

    async fn recv_objects(self, mut reader: ObjectsReader) -> anyhow::Result<()> {
        let mut tasks = FuturesUnordered::new();
        loop {
            tokio::select! {
                res = reader.next() => match res? {
                    Some(object) => {
                        let sender = self.packet_sender.clone();
                        tasks.push(async move {
                            if let Err(err) = Self::serve_object(sender, object).await {
                                log::warn!("Failed serving object: {}", err);
                            }
                        });
                    },
                    None => {
                        break;
                    }
                },
                _res = tasks.next(), if !tasks.is_empty() => {}
            }
        }
        Ok(())
    }

    async fn serve_object(sender: tokio::sync::mpsc::Sender<RoomPacket>, mut object: ObjectReader) -> anyhow::Result<()> {
        let received = object.read_all().await?;
        let received = String::from_utf8_lossy(&received).to_string();

        let received_values: Vec<Value> = serde_json::from_str(&received)?;
        for value in received_values {
            let packet = serde_json::from_value::<StatePacket>(value.clone());
            let received: RoomPacket = match packet {
                Ok(packet) => RoomPacket::StatePacket(packet),
                Err(_) => RoomPacket::Other(value),
            };
            sender.send(received).await?;
        }
        Ok(())
    }
}
