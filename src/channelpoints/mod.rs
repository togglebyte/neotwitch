use anyhow::Result;
use log::{info, warn};
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use tinyroute::Agent;

use super::twitch::{
    connect_channel_points, Sink, SinkExt, Stream, StreamExt, WsMessage,
};
use super::Address;

// -----------------------------------------------------------------------------
//     - Nonce -
// -----------------------------------------------------------------------------
fn nonce() -> String {
    thread_rng().sample_iter(&Alphanumeric).map(char::from).take(18).collect()
}

pub async fn run(
    agent: Agent<(), Address>,
    config: &crate::config::Config,
) -> Result<()> {
    let (mut sink, mut stream) = connect_channel_points().await?;

    let topics = [
        format!("channel-bits-events-v2.{}", config.channel_id),
        format!("channel-points-channel-v1.{}", config.channel_id),
        format!("channel-subscribe-events-v1.{}", config.channel_id),
        format!("following.{}", config.channel_id),
    ];

    // -----------------------------------------------------------------------------
    //     - Listen to selected topics -
    // -----------------------------------------------------------------------------
    let data = serde_json::json!({
       "type": "LISTEN",
       "nonce": nonce(),
       "data": {
           "topics": topics,
           "auth_token": config.token,
       }
    });

    let text = serde_json::to_string(&data).unwrap();
    
    if let Err(e) = sink.send(WsMessage::Text(text)).await {
        warn!("Some kind of error: {:?}", e);
    };

    // while let Some(Ok(msg)) = stream.next().await {
    //     eprintln!("{:?}", msg);
    // }


    Ok(())
}
