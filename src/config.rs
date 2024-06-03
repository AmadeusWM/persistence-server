use clap::Parser;
use moq_native::tls;
use std::net::SocketAddr;

#[derive(Parser, Clone)]
pub struct Config {
    /// Webserver Address
    #[arg(long, default_value = "[::]:4440")]
    pub bind: SocketAddr,

    /// Relay on which to listen for participants
    #[arg(long, default_value = "https://localhost:4443")]
    pub relay: url::Url,

    /// TLS configuration
    #[command(flatten)]
    pub tls: tls::Cli,

    #[arg(long, default_value=".")]
    pub index_namespace: String,

    /// The persistence server will subscribe on all publishers with this prefix,
    /// and then publish a room for these participants to subscribe on.
    #[arg(long, default_value="room.participant.")]
    pub participant_prefix: String,

    /// The prefix of the announcement on which participants should subscribe
    #[arg(long, default_value="room.provider.")]
    pub provider_prefix: String,

    /// The track on which to publish/subscribe for room data
    #[arg(long, default_value=".doc")]
    pub track: String,
}
