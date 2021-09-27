use std::collections::VecDeque;

use anyhow::Result;
use log::{error, info};
use tinyroute::{Agent, Message, ToAddress};

use super::Address;

#[derive(Debug, serde::Serialize)]
pub enum Level {
    Info,
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
        ).expect("Router has died");
    }

    pub fn info<T: Send + 'static>( agent: &Agent<T, Address>, msg: impl Into<String>) {
        Self::send(agent, msg, Level::Info);
    }

    pub fn error<T: Send + 'static>( agent: &Agent<T, Address>, msg: impl Into<String>) {
        Self::send(agent, msg, Level::Error);
    }

    fn to_string(&self) -> String {
        format!("{:15.15} | {}", self.sender.to_string(), self.message)
    }
}

const LOG_LEN: usize = 1024;

pub async fn run(mut agent: Agent<LogMessage, Address>) -> Result<()> {
    let mut log = VecDeque::with_capacity(LOG_LEN);
    while let Ok(msg) = agent.recv().await {
        match msg {
            Message::Value(log_msg, _) => {
                match log_msg.level {
                    Level::Info => info!("{}", log_msg.to_string()),
                    Level::Error => error!("{}", log_msg.to_string()),
                }

                if log.len() == LOG_LEN {
                    log.pop_front();
                }

                log.push_back(log_msg);
            }
            Message::RemoteMessage { .. } => {
                // TODO: send the log to the sender
            }
            Message::Shutdown => break,
            _ => {}
        }
    }

    info!("Shut down the logger");
    Ok(())
}
