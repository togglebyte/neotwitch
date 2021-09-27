use std::time::Duration;
use anyhow::Result;
use tinyroute::{Agent, Message, ToAddress};
use tokio::time;

use super::twitch::{
    connect_chat, Sink, SinkExt, Stream, StreamExt, WsMessage,
};
use super::Address;
use crate::log::LogMessage;
use crate::config::Config;

mod parse;

// -----------------------------------------------------------------------------
//     - Irc Sink -
// -----------------------------------------------------------------------------
struct IrcWriter(Sink);

impl IrcWriter {
    async fn send(&mut self, message: impl Into<String>) -> Result<()> {
        let message = message.into();
        Ok(self.0.send(WsMessage::Text(message)).await?)
    }
}

async fn connect_irc(config: &Config, agent: &Agent<(), Address>) -> Result<(IrcWriter, Stream)> {
    let (sink, stream) = connect_chat().await?;
    let mut sink = IrcWriter(sink);

    sink.send(format!("PASS oauth:{}\r\n", config.token)).await?;
    sink.send(format!("NICK {}\r\n", config.nick)).await?;
    sink.send(format!("CAP REQ :twitch.tv/tags twitch.tv/commands\r\n")).await?;

    for channel in &config.irc_channels {
        sink.send(format!("JOIN {}\r\n", channel)).await?;
        LogMessage::info(agent, format!("Joined {}", channel));
    }

    Ok((sink, stream))
}

// -----------------------------------------------------------------------------
//     - Run -
// -----------------------------------------------------------------------------
pub async fn run(
    mut agent: Agent<(), Address>,
    config: &crate::config::Config,
) -> Result<()> {
    let mut subscribers: Vec<Address> = Vec::new();

    let mut reconnect_count = 0;

    'reconnect: loop {
        reconnect_count += 1;

        if reconnect_count > super::MAX_RETRIES {
            break 'reconnect;
        }

        let (mut sink, mut stream) = match connect_irc(config, &agent).await {
            Ok(s) => {
                reconnect_count = 0;
                s
            }
            Err(_) => {
                LogMessage::error(&agent, "Failed to connect to Twitch IRC via websockets");
                time::sleep(Duration::from_secs(reconnect_count as u64)).await;
                continue;
            }
        };

        loop {
            tokio::select! {
                chat_msg = stream.next() => {
                    match chat_msg {
                        None => {
                            LogMessage::error(&agent, "Twitch IRC ws closed");
                            break; // cause a reconnect
                        }
                        Some(Err(e)) => {
                            LogMessage::error(&agent, format!("Failed to receive Twitch chat mesasge: {}", e));
                            break; // cause a reconnect
                        }
                        Some(Ok(WsMessage::Text(msg))) => {
                            eprintln!("{:?}", msg);


                            if msg.starts_with("PING") {
                                LogMessage::info(&agent, "> Ping");
                                if let Err(_) = sink.send("PONG".to_string()).await {
                                    LogMessage::error(&agent, "Failed to pong");
                                    break; // cause a reconnect
                                }
                                LogMessage::info(&agent, "< Pong");
                                continue;
                            }

                            if let Some(msg) = parse::parse(&msg) {
                                let bytes = serde_json::to_vec(&msg).unwrap();
                                agent.send_remote(&subscribers, &bytes)?;
                            }
                        }
                        Some(_) => {} // unsupported message
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
                                        agent.track(sender)?;
                                    }
                                }
                                _ => {}
                            }

                        }
                        Message::AgentRemoved(sender) => {
                            LogMessage::info(&agent, format!("{} unsubscribed from irc", sender.to_string()));
                            subscribers.retain(|s| s != &sender);
                        }
                        Message::Shutdown => return Ok(()),
                        _ =>  {}
                    }
                }
            };
        }
    }

    Ok(())
}
