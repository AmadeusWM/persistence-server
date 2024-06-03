mod config;
mod session;
mod room_listener;
mod room_provider;
mod index_packet;
mod participant;
mod room_announce_pattern;
mod room_packet;
mod rooms;

use anyhow::Context;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use moq_native::quic;

use crate::{config::Config, session::Session};
use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	env_logger::init();
	let tracer = tracing_subscriber::FmtSubscriber::builder()
		.with_max_level(tracing::Level::WARN)
		.finish();
	tracing::subscriber::set_global_default(tracer).unwrap();

    let config = Config::parse();

    let tls = config.tls.load()?;

    let quic = quic::Endpoint::new(quic::Config { bind: config.bind, tls })?;
    let mut server = quic.server.context("missing server certificate")?;

    let mut tasks = FuturesUnordered::new();


    loop {
        tokio::select! {
            res = server.accept() => {
                log::debug!("accepting new connection");
                let config = config.clone();

                let session = res.context("failed to accept QUIC connection")?;
                let session = Session::new(session, config);

                
                tasks.push(async move {
                    session.run().await
                }.boxed());
            },
            res = tasks.next(), if !tasks.is_empty() => { res.unwrap()?; }
        }
    }
}
