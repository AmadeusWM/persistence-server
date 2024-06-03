use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use anyhow::Context;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use moq_transport::session::{Announced, Publisher, Subscriber};
use tokio::sync::Mutex;

use crate::{
    config::Config,
    room_announce_pattern::RoomAnnouncePattern,
    room_listener::RoomListener,
    room_packet::RoomPacket,
    room_provider::RoomProvider,
    rooms::{Room, Rooms},
};

#[derive(Clone)]
pub struct Session {
    session: web_transport::Session,
    config: Config,
    rooms: Rooms,
}

impl Session {
    pub fn new(session: web_transport::Session, config: Config) -> Self {
        Self {
            session,
            config,
            rooms: Rooms::new(),
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let (session, publisher, subscriber) = moq_transport::session::Session::accept_role(
            self.session.clone(),
            moq_transport::setup::Role::Both,
        )
        .await?;

        let mut tasks = FuturesUnordered::new();
        tasks.push(async move { session.run().await.map_err(anyhow::Error::new) }.boxed());

        if let (Some(subscriber), Some(relay_publisher)) = (subscriber, publisher) {
            tasks.push(Self::serve(self.clone(), relay_publisher, subscriber).boxed());
        }

        tasks.select_next_some().await?;
        Ok(())
    }

    async fn serve(
        self,
        relay_publisher: moq_transport::session::Publisher,
        mut relay_subscriber: moq_transport::session::Subscriber,
    ) -> anyhow::Result<()> {
        // wait for announce
        // announce =>
        let mut tasks = FuturesUnordered::new();

        loop {
            let subscriber = relay_subscriber.clone();
            let publisher = relay_publisher.clone();
            tokio::select! {
                Some(announce) = relay_subscriber.announced() => {
                    let this = self.clone();
                    tasks.push(async move {
                        if let Err(err) = Self::serve_announce(this, publisher, subscriber, announce).await {
                            log::warn!("failed serving announce: {:?}", err)
                        }
                    })
                },
                res = tasks.next(), if !tasks.is_empty() => res.unwrap(),
            }
        }
    }

    async fn serve_announce(
        self,
        relay_publisher: moq_transport::session::Publisher,
        relay_subscriber: moq_transport::session::Subscriber,
        announce: Announced,
    ) -> anyhow::Result<()> {
        //   - publish events received from publishers (every e.g. 1/60 sec tick)
        let namespace = announce.namespace.clone();
        let announce = RoomAnnouncePattern::parse_announce(
            self.config.index_namespace.clone(),
            self.config.participant_prefix,
            namespace,
        );

        if let Some(announce) = announce {
            let room = self.rooms.get_room(&announce.room_id).await;
            let room = match room {
                Some(room) => room,
                None => {
                    let room = Room::new();
                    self.rooms
                        .insert(announce.room_id.clone(), room.clone())
                        .await;
                    room
                }
            };
            if room.is_active().await {
                return Ok(());
            }
            room.activate().await;
            // TODO: make a scheduler from this: ForwardScheduler
            let (sender, receiver) = tokio::sync::mpsc::channel::<RoomPacket>(1024);
            let listener_announce = announce.clone();
            let room_listener = RoomListener::new(
                relay_subscriber,
                sender,
                listener_announce,
                self.config.track.clone(),
            );

            let uuid = uuid::Uuid::new_v4();
            let provider_announce = RoomAnnouncePattern::new(
                self.config.index_namespace,
                self.config.provider_prefix,
                announce.room_id,
                uuid.to_string()
            );

            let room_provider = RoomProvider::new(
                room.clone(),
                relay_publisher,
                receiver,
                provider_announce,
                self.config.track,
            );

            let result = tokio::select! {
                res = room_listener.run() => res,
                res = room_provider.run() => res
            };

            if let Err(err) = result {
                log::warn!("session error: {:?}", err);
            }

            // create provider, allow provider to write to some state we store in this struct
            // use select for room_listener and room_provider
            room.deactivate().await;
        }

        Ok(())
    }
}
