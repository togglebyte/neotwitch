use std::time::Duration;

use anyhow::Result;
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use tinyroute::{Agent, Message, ToAddress};
use tokio::time;

use super::twitch::{connect_channel_points, SinkExt, StreamExt, WsMessage};
use super::Address;
use crate::log::LogMessage;

// -----------------------------------------------------------------------------
//     - Nonce -
// -----------------------------------------------------------------------------
fn nonce() -> String {
    thread_rng().sample_iter(&Alphanumeric).map(char::from).take(18).collect()
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

    let mut reconnect_count = 0;
    'reconnect: loop {
        reconnect_count += 1;

        if reconnect_count > super::MAX_RETRIES {
            break 'reconnect;
        }

        let (mut sink, mut stream) = match connect_channel_points().await {
            Ok(s) => {
                reconnect_count = 0;
                s
            }
            Err(_) => {
                LogMessage::error(
                    &agent,
                    "Failed to connect to Twitch IRC via websockets",
                );
                time::sleep(Duration::from_secs(reconnect_count)).await;
                continue;
            }
        };
        if let Err(e) = sink.send(WsMessage::Text(text.clone())).await {
            LogMessage::warn(&agent, format!("Websocket error: {}", e));
            continue;
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
                        Some(Ok(WsMessage::Text(msg))) => {
                            eprintln!("{:?}", msg);
                            agent.send_remote(&subscribers, msg.as_bytes())?;
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
                                        LogMessage::info(&agent, format!("{} subscribed to channelpoint events", sender.to_string()));
                                        subscribers.push(sender.clone());
                                        agent.track(sender)?;
                                    }
                                }
                                _ => {}
                            }

                        }
                        Message::AgentRemoved(sender) => {
                            LogMessage::info(&agent, format!("{} unsubscribed from channelpoint events", sender.to_string()));
                            subscribers.retain(|s| s != &sender);
                        }
                        Message::Shutdown => return Ok(()),
                        _ =>  {}
                    }
                }
            }
        }
    }

    Ok(())
}
