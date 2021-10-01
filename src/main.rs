use anyhow::Result;
use tinylog::Logger;
use tinyroute::{Router, ToAddress};
use tinyroute::client::{TcpClient, UdsClient};

mod channelpoints;
mod chat;
mod config;
mod server;
mod twitch;

pub const MAX_RETRIES: u64 = 5;

// -----------------------------------------------------------------------------
//     - Address -
// -----------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Clone, Hash, serde::Serialize)]
pub enum Address {
    Log,
    Chat,
    ChannelPoints,
    Server,
    Connection(usize),
}

impl ToAddress for Address {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"log" => Some(Self::Log),
            b"chat" => Some(Self::Chat),
            b"cpoints" => Some(Self::ChannelPoints),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        match self {
            Self::Log => "Log".to_string(),
            Self::Chat => "Chat".to_string(),
            Self::ChannelPoints => "ChannelPoints".to_string(),
            Self::Server => "Server".to_string(),
            Self::Connection(id) => format!("Connection({})", id),
        }
    }
}

// -----------------------------------------------------------------------------
//     - main -
// -----------------------------------------------------------------------------
#[tokio::main]
async fn main() -> Result<()> {
    // Setup
    pretty_env_logger::init();
    let config = config::Config::new()?;
    let config = Box::new(config);
    let config = Box::leak(config);

    let mut router = Router::new();

    // Agents
    let log_agent = router.new_agent(1024, Address::Log)?;
    let chat_agent = router.new_agent(1024, Address::Chat)?;
    let cpoints_agent = router.new_agent(1024, Address::ChannelPoints)?;
    let server_agent = router.new_agent(1024, Address::Server)?;

    // Setup logging
    // let socket_path = env::var("TINYLOG_SOCKET").map_err(|_| Error::MissingEnvVar)?;
    // // let client = UdsClient::connect(socket_path).await?;
    let client = TcpClient::connect("127.0.0.1:5566").await?;
    let logger = Logger::new(log_agent, client).await?;

    // Handles, so the application can close properly
    let log_handle = tokio::spawn(logger.run());
    let chat_handle = tokio::spawn(chat::run(chat_agent, config));
    let cpoints_handle = tokio::spawn(channelpoints::run(cpoints_agent, config));
    let server_handle = tokio::spawn(server::run(server_agent, "127.0.0.1:6000"));

    // Run the router
    router.run().await;

    // Wait for the handles to finish before exiting
    log_handle.await?;
    chat_handle.await??;
    cpoints_handle.await??;
    server_handle.await??;

    // ... and done
    Ok(())
}
