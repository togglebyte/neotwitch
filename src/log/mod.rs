use std::collections::VecDeque;
use std::time::Instant;

use log::{error, info, warn};
use tinyroute::{Agent, Message, ToAddress};

use super::Address;

fn convert_bytes_to_timestamp(bytes: tinyroute::Bytes) -> Instant {
    todo!()
}

#[derive(Debug, serde::Serialize)]
pub enum Level {
    Info,
    Warning,
    Error,
}

#[derive(Debug, serde::Serialize)]
pub struct LogMessage {
    sender: Address,
    message: String,
    level: Level,
}

impl LogMessage {
    fn send<T: Send + 'static>(
        agent: &Agent<T, Address>,
        msg: impl Into<String>,
        level: Level,
    ) {
        agent.send(
            Address::Log,
            Self {
                sender: agent.address().clone(),
                message: msg.into(),
                level,
            },
        );
    }

    pub fn info<T: Send + 'static>( agent: &Agent<T, Address>, msg: impl Into<String>) {
        Self::send(agent, msg, Level::Info);
    }

    pub fn warn<T: Send + 'static>( agent: &Agent<T, Address>, msg: impl Into<String>) {
        Self::send(agent, msg, Level::Warning);
    }

    pub fn error<T: Send + 'static>( agent: &Agent<T, Address>, msg: impl Into<String>) {
        Self::send(agent, msg, Level::Error);
    }

    fn to_string(&self) -> String {
        format!("{:10.10} > {}", self.sender.to_string(), self.message)
    }
}

const LOG_LEN: usize = 1024;

pub async fn run(mut agent: Agent<LogMessage, Address>) {
    let mut log = VecDeque::with_capacity(LOG_LEN);
    while let Ok(msg) = agent.recv().await {
        match msg {
            Message::Value(log_msg, sender) => {
                match log_msg.level {
                    Level::Info => info!("{}", log_msg.to_string()),
                    Level::Warning => warn!("{}", log_msg.to_string()),
                    Level::Error => error!("{}", log_msg.to_string()),
                }

                if log.len() == LOG_LEN {
                    log.pop_front();
                }

                log.push_back(log_msg);
            }
            Message::RemoteMessage { bytes, sender, .. } => {
                let timestamp = convert_bytes_to_timestamp(bytes);
                // for message in log.iter().filter(|m| m.timestamp >= timestamp) {
                //     agent.send_remote(
                // }
            }
            Message::Shutdown => break,
            _ => {}
        }
    }

    info!("Shut down the logger");
}
