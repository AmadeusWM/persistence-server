use std::{collections::HashSet, sync::Arc, time::Duration};

use anyhow::Context;
use futures::{stream::FuturesUnordered, StreamExt};
use moq_transport::{
    serve::{self, TrackReader, TrackReaderMode},
    session::Subscriber,
};
use tokio::{sync::Mutex, time::sleep};

use crate::{
    index_packet::IndexPacket, participant::Participant, room_announce_pattern::RoomAnnouncePattern, room_packet::RoomPacket
};

#[derive(Clone)]
pub struct RoomListener {
    relay: Subscriber,
    sender: tokio::sync::mpsc::Sender<RoomPacket>,
    announce: RoomAnnouncePattern,
    track: String,
    participants: Arc<Mutex<HashSet<String>>>,
}

impl RoomListener {
    pub fn new(
        relay: Subscriber,
        sender: tokio::sync::mpsc::Sender<RoomPacket>,
        announce: RoomAnnouncePattern,
        track: String
    ) -> Self {
        Self {
            relay,
            sender,
            announce,
            track,
            participants: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        // listen for subscribers on index server
        log::debug!("subscribing on index track {}", self.announce.to_index_track());
        let (writer, reader) = serve::Track::new(
            self.announce.index_namespace.clone(),
            self.announce.to_index_track(),
        )
        .produce();

        let mut relay = self.relay.clone();

        let res = tokio::select! {
            res = relay.subscribe(writer) => res.context(format!("index_server subscribe failed: {}", self.announce.to_index_track())),
            res = Self::recv(self.clone(), reader) => res.context("failed receiving index")
        };
        res
    }

    async fn recv(self, reader: TrackReader) -> anyhow::Result<()> {
        match reader.mode().await.context("failed to get mode")? {
            TrackReaderMode::Groups(groups) => self.recv_groups(groups).await,
            _ => Err(anyhow::format_err!(
                "Prefix server should only publish group streams"
            )),
        }?;
        Ok(())
    }

    async fn recv_groups(self, mut groups: serve::GroupsReader) -> anyhow::Result<()> {
        let mut tasks = FuturesUnordered::new();

        while let Some(mut group) = groups.next().await? {
            loop {
                tokio::select! {
                    res = group.read_next() => {
                        if let Some(object) = res? {
                            let this = self.clone();
                            let packet = String::from_utf8_lossy(&object).to_string();
                            let packet = IndexPacket::from(packet);
                            tasks.push(async move {
                                if let Err(err) = Self::handle_packet(this, packet).await {
                                    log::warn!("error while handling index packet: {}", err)
                                }
                            });
                        } else {
                            // No more groups
                            break;
                        }
                    },
                    res = tasks.next(), if !tasks.is_empty() => {
                        res.unwrap();
                        // TODO: stop if no more participants are left, happens when index-server closes?
                        if self.participants.lock().await.len() == 0 {
                            break;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    async fn handle_packet(mut self, packet: IndexPacket) -> anyhow::Result<()> {
        match packet {
            IndexPacket::Insert(id) => {
                self.add_participant(id).await?;
            }
            IndexPacket::Remove(id) => {
                self.remove_participant(&id).await;
            }
            IndexPacket::Snapshot(ids) => {
                // remove participants
                let participants = {
                    self.participants.clone().lock().await.clone()
                };
                for id in participants.iter() {
                    if !ids.contains(&id) {
                        self.remove_participant(&id).await;
                    }
                }
                let mut tasks = FuturesUnordered::new();
                // add participants
                for id in ids.clone() {
                    let mut this = self.clone();
                    tasks.push(async move { this.add_participant(id).await });
                }
                while let Some(res) = tasks.next().await {
                    res?;
                }
            }
        }
        Ok(())
    }

    async fn remove_participant(&mut self, id: &String) {
        self.participants.lock().await.remove(id);
    }

    async fn add_participant(&mut self, id: String) -> anyhow::Result<()> {
        if self.participants.lock().await.contains(&id) {
            return Ok(());
        }
        self.participants.lock().await.insert(id.clone());

        // use same announcement, but change the id
        let mut announce = self.announce.clone();
        announce.publisher_id = id;
        
        let (writer, reader) = serve::Track::new(
            announce.to_namespace(),
            self.track.to_string(),
        )
        .produce();

        let mut relay = self.relay.clone();
        let participant = Participant::new(reader, self.sender.clone());

        tokio::select! {
            res = relay.subscribe(writer) => res.context("index_server subscribe failed"),
            res = participant.run_recv() => res.context("failed receiving index")
        }
    }
}
