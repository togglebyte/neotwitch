use anyhow::{Result, anyhow};
use log::{info, warn};
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use tinyroute::{Agent, Message, ToAddress};

use super::twitch::{
    connect_channel_points, Sink, SinkExt, Stream, StreamExt, WsMessage,
};
use super::Address;
use crate::log::LogMessage;

// -----------------------------------------------------------------------------
//     - Nonce -
// -----------------------------------------------------------------------------
fn nonce() -> String {
    thread_rng().sample_iter(&Alphanumeric).map(char::from).take(18).collect()
}

fn parse_twitch_message(msg: String) -> Result<neotwitch::Message> {
    match serde_json::from_str::<neotwitch::TwitchMessage>(&msg) {
        Ok(neotwitch::TwitchMessage::Message { data }) => Ok(data),
        Ok(_) => Err(anyhow!("Not a `Twitch Message` message. Ignoring")),
        Err(e) => Err(anyhow!("Failed to parse twitch message: {:?}", e)),
    }
}

pub async fn run(
    mut agent: Agent<(), Address>,
    config: &crate::config::Config,
) -> Result<()> {
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

    let mut subscribers = Vec::new();

    'reconnect: loop {
        let (mut sink, mut stream) = connect_channel_points().await?;
        if let Err(e) = sink.send(WsMessage::Text(text.clone())).await {
            LogMessage::warn(&agent, format!("Websocket error: {}", e));
        };

        loop {
            tokio::select! {
                ws_msg = stream.next() => {
                    match ws_msg {
                        None => {
                            LogMessage::error(&agent, "Channel points websocket closed");
                            break;
                        }
                        Some(Err(e)) => {
                            LogMessage::error(&agent, format!("Failed to do things: {}", e));
                            break;
                        }
                        Some(Ok(WsMessage::Text(msg))) => match agent.send_remote(&subscribers, msg.as_bytes()) {
                            Ok(()) => continue,
                            Err(e) => {
                                LogMessage::error(&agent, format!("Failed to send Twitch message: {}", e));
                                break 'reconnect;
                            }
                        }
                        Some(Ok(_)) => continue,
                    }
                }
                agent_msg = agent.recv() => { 
                    let msg = agent_msg?;
                    match msg {
                        Message::RemoteMessage { sender, host, bytes } => {
                            LogMessage::info(&agent, format!("{}@{} > {:?}", sender.to_string(), host, bytes));

                            match bytes.as_ref() {
                                b"shutdown" => agent.shutdown_router(),
                                b"sub" => {
                                    if !subscribers.contains(&sender) {
                                        LogMessage::info(&agent, format!("{} subscribed to irc", sender.to_string()));
                                        subscribers.push(sender.clone());
                                        agent.track(sender);
                                    }

                                    eprintln!("there is a total of: {} subscribers", subscribers.len());
                                }
                                _ => {}
                            }

                        }
                        Message::AgentRemoved(sender) => subscribers.retain(|s| s != &sender),
                        Message::Shutdown => return Ok(()),
                        _ =>  {}
                    }
                }
            }
        }
    }

    Ok(())
}
