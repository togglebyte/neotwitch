use std::borrow::Cow;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tinyroute::{Agent, Message, ToAddress};

use super::twitch::{
    connect_chat, Sink, SinkExt, Stream, StreamExt, WsMessage,
};
use super::Address;
use crate::log::{Level, LogMessage};
use crate::config::Config;

mod parse;

// -----------------------------------------------------------------------------
//     - Irc message -
// -----------------------------------------------------------------------------
#[derive(Debug, Deserialize, Serialize)]
pub struct IrcMessage<'msg> {
    user: Cow<'msg, str>,
    channel: Cow<'msg, str>,
    message: Cow<'msg, str>,
    is_action: bool,
    raw: Cow<'msg, str>,
}

impl<'msg> IrcMessage<'msg> {
    fn new(
        user: &'msg str,
        channel: &'msg str,
        message: &'msg str,
        is_action: bool,
        raw: &'msg str,
    ) -> Self {
        Self {
            user: Cow::Borrowed(user),
            channel: Cow::Borrowed(channel),
            message: Cow::Borrowed(message),
            is_action,
            raw: Cow::Borrowed(raw),
        }
    }
}

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
    let (sink, mut stream) = connect_chat().await?;
    let mut sink = IrcWriter(sink);

    sink.send(format!("PASS oauth:{}\r\n", config.token)).await;
    sink.send(format!("NICK {}\r\n", config.nick)).await;

    for channel in &config.irc_channels {
        sink.send(format!("JOIN {}\r\n", channel)).await;
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

    'reconnect: loop {
        let (mut sink, mut stream) = connect_irc(config, &agent).await?;

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
                            if msg.starts_with("PING") {
                                LogMessage::info(&agent, "> Ping");
                                if let Err(res) = sink.send("PONG".to_string()).await {
                                    LogMessage::error(&agent, "Failed to pong");
                                    break; // cause a reconnect
                                }
                                LogMessage::info(&agent, "< Pong");
                            }

                            if let Some(msg) = parse::parse(&msg) {
                                let bytes = serde_json::to_vec(&msg).unwrap();
                                agent.send_remote(&subscribers, &bytes);
                            }
                        }
                        Some(_) => {} // unsupported message
                    }

                }
                agent_msg = agent.recv() => {
                    let msg = agent_msg?;
                    match msg {
                        Message::RemoteMessage { sender, host, bytes } => {
                            eprintln!("{}@{} > {:?}", sender.to_string(), host, bytes);

                            match bytes.as_ref() {
                                b"shutdown" => agent.shutdown_router(),
                                b"sub" => {
                                    if !subscribers.contains(&sender) {
                                        LogMessage::info(&agent, format!("{} subscribed to irc", sender.to_string()));
                                        subscribers.push(sender);
                                    }
                                }
                                _ => {}
                            }

                        }
                        Message::Shutdown => {
                            return Ok(())
                        }
                        _ =>  {}
                    }
                }
            };
        }
    }

    Ok(())
}
