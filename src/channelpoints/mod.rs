use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use log::{info, error};
use neotwitch::TwitchMessage;
use rand::distributions::Alphanumeric;
use rand::prelude::*;
use tinyroute::{Agent, Message, ToAddress};
use tokio::sync::mpsc;
use tokio::time;

use super::twitch::{connect_channel_points, Sink, SinkExt, StreamExt, WsMessage};
use super::Address;

// -----------------------------------------------------------------------------
//     - Nonce -
// -----------------------------------------------------------------------------
fn nonce() -> String {
    thread_rng().sample_iter(&Alphanumeric).map(char::from).take(18).collect()
}

// -----------------------------------------------------------------------------
//     - Twitch sink -
// -----------------------------------------------------------------------------
fn start_sink(mut sink: Sink, response_tx: mpsc::Sender<Result<()>>) -> mpsc::Sender<WsMessage> {
    let (sink_tx, mut sink_rx) = mpsc::channel(100);
    tokio::spawn(async move {
        while let Some(m) = sink_rx.recv().await {
            if let Err(e) = sink.send(m).await {
                let _ = response_tx.send(Err(anyhow!("Sink error: {}", e))).await;
                break;
            }
        }
    });

    sink_tx
}

// -----------------------------------------------------------------------------
//     - Heartbeat loop -
// -----------------------------------------------------------------------------
fn heartbeat(sink_tx: mpsc::Sender<WsMessage>, response_tx: mpsc::Sender<Result<()>>) -> mpsc::Sender<Instant> {
    let (tx, mut rx) = mpsc::channel(100);

    tokio::spawn(async move {
        let heartbeat = serde_json::json!({
            "type": "PING"
        });
        let heartbeat = WsMessage::Text(serde_json::to_string(&heartbeat).expect("Nice valid JSON"));

        let mut time_since_ping = Instant::now();
        let mut time_since_pong = Instant::now();
        loop {
            let jitter = thread_rng().gen_range(10..1000);
            let minutes = 1;
            let millis = 1000 * 60 * minutes - jitter;
            tokio::select! {
                _ = time::sleep(Duration::from_millis(millis)) => {
                    // If more than ten seconds has elapsed
                    // then break and send an error to restart
                    if time_since_ping.elapsed() > time_since_pong.elapsed() {
                        if (time_since_ping.elapsed() - time_since_pong.elapsed()) > Duration::from_secs(10) {
                            let _ = response_tx.send(Err(anyhow!("Over ten seconds since last pong"))).await;
                            break
                        }
                    }

                    time_since_ping = Instant::now();

                    if let Err(_) = sink_tx.send(heartbeat.clone()).await {
                        break;
                    };
                }
                pong_instant = rx.recv() => match pong_instant {
                    Some(pong) => {
                        time_since_pong = pong;
                        // If more than ten seconds has elapsed
                        // then break and send an error to restart
                        if (time_since_ping.elapsed() - time_since_pong.elapsed()) > Duration::from_secs(10) {
                            let _ = response_tx.send(Err(anyhow!("Over ten seconds since last pong"))).await;
                            break
                        }
                    }
                    None => break,
                }
            };
        }
    });

    tx
}

// Agent here is used to reive commands to shut down,
// but also to pass on test data.
// This is a poor design
pub async fn run(mut agent: Agent<(), Address>, config: &crate::config::Config) -> Result<()> {
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

    let listen_to_topics = WsMessage::Text(serde_json::to_string(&data).expect("Nice valid JSON"));

    let mut subscribers = Vec::new();

    let mut reconnect_count = 0;
    'reconnect: loop {
        reconnect_count += 1;

        if reconnect_count > super::MAX_RETRIES {
            break 'reconnect;
        }

        let (sink, mut stream) = match connect_channel_points().await {
            Ok(s) => {
                reconnect_count = 0;
                s
            }
            Err(_) => {
                error!("Failed to connect to Twitch IRC via websockets");
                time::sleep(Duration::from_secs(reconnect_count)).await;
                continue;
            }
        };

        let (response_tx, mut response) = mpsc::channel(100);

        let sink_tx = start_sink(sink, response_tx.clone());
        let heartbeat_tx = heartbeat(sink_tx.clone(), response_tx.clone());

        if let Err(e) = sink_tx.send(listen_to_topics.clone()).await {
            error!("Websocket TX error: {}", e);
            break;
        };

        loop {
            tokio::select! {
                response = response.recv() => {
                    match response {
                        None => break,
                        Some(Err(e)) => {
                            error!("Channel points websocket closed: {}", e);
                            break;
                        }
                        Some(Ok(())) => continue,
                    }
                }
                ws_msg = stream.next() => {
                    match ws_msg {
                        None => {
                            error!("Channel points websocket closed");
                            break;
                        }
                        Some(Err(e)) => {
                            error!("Websocket error: {}", e);
                            break;
                        }
                        Some(Ok(WsMessage::Text(msg))) => {
                            let bytes = msg.as_bytes();
                            match serde_json::from_slice::<TwitchMessage>(&bytes) {
                                Ok(twitch_data) => {
                                    match twitch_data {
                                        TwitchMessage::Pong => {
                                            if let Err(e) = heartbeat_tx.send(Instant::now()).await {
                                                error!("Heartbeat error: {}", e);
                                                break
                                            }
                                        },
                                        TwitchMessage::Reconnect => break,
                                        _ => {}
                                    }
                                    // Log twith payload (maybe not?)
                                    let s = serde_json::to_string(&twitch_data).expect("this was successfully serialize before, stop complaining");
                                    info!("{}", s);
                                }
                                Err(e) => {
                                    error!("Failed to serialize: {}", e);
                                    continue;
                                }
                            }

                            agent.send_remote(subscribers.iter().copied(), bytes).await?;
                        }
                        Some(Ok(_)) => continue,
                    }
                }
                agent_msg = agent.recv() => {
                    let msg = agent_msg?;
                    match msg {
                        Message::RemoteMessage { sender, host, bytes } => {
                            info!("{}@{} > {:?}", sender.to_string(), host, bytes);

                            match bytes.as_ref() {
                                b"shutdown" => agent.shutdown_router().await,
                                b"sub" => {
                                    if !subscribers.contains(&sender) {
                                        info!("{} subscribed to channelpoint events", sender.to_string());
                                        subscribers.push(sender.clone());
                                        agent.track(sender).await?;
                                    }
                                }
                                // If it's nor shutdown or sub then it's probably some test data
                                bytes if serde_json::from_slice::<TwitchMessage>(&bytes).is_ok() => {
                                    agent.send_remote(subscribers.iter().copied(), bytes).await?;
                                }
                                _ => {
                                    eprintln!("{:?}", "failed to serialize data");
                                }
                            }

                        }
                        Message::AgentRemoved(sender) => {
                            info!("{} subscribed to channelpoint events", sender.to_string());
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
