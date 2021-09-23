use std::env;
use anyhow::{anyhow, Result};

const NEO_TWITCH_TOKEN: &str = "NEO_TWITCH_TOKEN";
const NEO_TWITCH_CHANNEL: &str = "NEO_TWITCH_CHANNEL";
const NEO_TWITCH_IRC_NICK: &str = "NEO_TWITCH_IRC_NICK";
const NEO_TWITCH_IRC_CHANNELS: &str = "NEO_TWITCH_IRC_CHANNELS";

pub struct Config {
    pub channel_id: String,
    pub token: String,
    pub nick: String,
    pub irc_channels: Vec<String>,
}

fn validate_channel(channel: &str) -> Option<String> {
    if channel.is_empty() {
        return Some("Channel name can not be empty".into());
    }

    if channel == "#" {
        return Some("Channel name missing, specify a channel like so: #channel".into());
    }

    if !channel.starts_with('#') {
        return Some("Channel name has to start with a # char, e.g #mychannel".into());
    }

    if channel.len() > 200 {
        return Some("Channel name can not be longer than 200 characters".into());
    }

    None
}

impl Config {
    pub fn new() -> Result<Self> {
        let channel_id = env::var(NEO_TWITCH_CHANNEL).map_err(|_| anyhow!("Channel id missing{}"))?;
        let token = env::var(NEO_TWITCH_TOKEN).map_err(|_| anyhow!("OAuth token missing"))?;
        let nick = env::var(NEO_TWITCH_IRC_NICK).map_err(|_| anyhow!("Nick is missing"))?;
        let irc_channels = env::var(NEO_TWITCH_IRC_CHANNELS).map_err(|_| anyhow!("Channels are missing"))?;

        let irc_channels = irc_channels
            .split(&[' ', ','][..])
            .map(str::trim)
            .map(String::from)
            .collect::<Vec<_>>();

        // Validate channel names
        for channel in &irc_channels {
            if let Some(err) = validate_channel(channel) {
                return Err(anyhow!("Invalid irc channel name \"{}\": {}", channel, err));
            }
        }

        let inst = Self {
            channel_id,
            token,
            nick,
            irc_channels,
        };

        Ok(inst)
    }
}
