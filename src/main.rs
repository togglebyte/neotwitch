use anyhow::Result;
use tinyroute::{Router, ToAddress};

mod channelpoints;
mod chat;
mod config;
mod server;
mod twitch;

pub const MAX_RETRIES: u64 = 5;

// -----------------------------------------------------------------------------
//     - Address -
// -----------------------------------------------------------------------------
#[derive(Debug, PartialEq, Eq, Copy, Clone, Hash, serde::Serialize)]
pub enum Address {
    Chat,
    ChannelPoints,
    Server,
    Connection(usize),
}

impl ToAddress for Address {
    fn from_bytes(bytes: &[u8]) -> Option<Self> {
        match bytes {
            b"chat" => Some(Self::Chat),
            b"cpoints" => Some(Self::ChannelPoints),
            _ => None,
        }
    }

    fn to_string(&self) -> String {
        match self {
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
    #[cfg(feature="logging")]
    tinylog::init_logger().await.expect("Failed to connect logger");

    // Setup
    let config = config::Config::new()?;
    let config = Box::new(config);
    let config = Box::leak(config);

    let mut router = Router::new();

    // Agents
    let chat_agent = router.new_agent(None, Address::Chat)?;
    let cpoints_agent = router.new_agent(None, Address::ChannelPoints)?;
    let server_agent = router.new_agent(None, Address::Server)?;

    // Handles, so the application can close properly
    let chat_handle = tokio::spawn(chat::run(chat_agent, config));
    let cpoints_handle = tokio::spawn(channelpoints::run(cpoints_agent, config));
    let server_handle = tokio::spawn(server::run(server_agent, "127.0.0.1:6000"));

    // Run the router
    router.run().await;

    // Wait for the handles to finish before exiting
    chat_handle.await??;
    cpoints_handle.await??;
    server_handle.await??;

    // ... and done
    Ok(())
}
