use anyhow::Result;
use futures_util::stream::{SplitSink, SplitStream};
use tokio::net::TcpStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

pub use futures_util::{SinkExt, StreamExt};
pub use tokio_tungstenite::tungstenite::protocol::Message as WsMessage;

const PUBSUB_URL: &'static str = "wss://pubsub-edge.twitch.tv";
const CHAT_URL: &'static str = "wss://irc-ws.chat.twitch.tv/";

pub type Sink = SplitSink<WebSocketStream<MaybeTlsStream<TcpStream>>, WsMessage>;
pub type Stream = SplitStream<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub async fn connect_channel_points() -> Result<(Sink, Stream)> {
    connection(PUBSUB_URL).await
}

pub async fn connect_chat() -> Result<(Sink, Stream)> {
    connection(CHAT_URL).await
}

async fn connection(url: &str) -> Result<(Sink, Stream)> {
    let (ws, _) = connect_async(url).await?;
    Ok(ws.split())
}
