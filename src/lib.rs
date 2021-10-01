use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// -----------------------------------------------------------------------------
//     - Irc -
// -----------------------------------------------------------------------------
/// Irc message
#[derive(Debug, Deserialize, Serialize)]
pub enum Irc {
    ClearChat,
    Message(IrcMessage)
}

// -----------------------------------------------------------------------------
//     - Priv message -
// -----------------------------------------------------------------------------
/// Priv message (channel message)
#[derive(Debug, Deserialize, Serialize)]
pub struct IrcMessage {
    pub user: String,
    pub channel: String,
    pub message: String,
    pub action: bool,
    pub tags: HashMap<String, String>,
}

impl IrcMessage {
    pub fn new(
        user: String,
        channel: String,
        message: String,
        action: bool,
        tags: HashMap<String, String>,
    ) -> Self {
        Self {
            user,
            channel,
            message,
            action,
            tags,
        }
    }
}

// -----------------------------------------------------------------------------
//     - Pubsub messages -
//     Thanks to Bare!
// -----------------------------------------------------------------------------

/// Base message
#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TwitchMessage {
    /// Response to a PING sent to Twitch
    Pong,
    /// Clients may receive a RECONNECT message at any time. 
    /// This indicates that the server is about to restart and will disconnect the client within 30 seconds. 
    /// During this time, the client should reconnect to the server.
    Reconnect,
    /// Twitch response to a message
    Response,
    /// Inner message
    Message {
        /// Actual message data
        data: Message,
    },
    /// Unknown message type
    #[serde(other)]
    Unknown,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Message {
    /// Topic the message belongs to (e.g channel points event)
    pub topic: String,
    /// Inner message
    pub message: String,
}

impl Message {
    pub fn topic(&self) -> Option<&str> {
        let pos = self.topic.find('.')?;
        Some(&self.topic[..pos])
    }
}

//----- RESPONSE
#[derive(Deserialize, Debug)]
pub struct TwitchMessageResponse {
    #[serde(rename = "type")]
    pub kind: String,
    pub error: String,
    pub nonce: String,
}

/// Channel follow event
#[derive(Deserialize, Serialize, Debug)]
pub struct FollowEvent {
    /// Display name 
    pub display_name: String,
    /// Username
    pub username: String,
    /// User id
    pub user_id: String,
}

/// Channel subscription
/// ```text
/// topic: channel-subscribe-events-v1
/// ```
#[derive(Deserialize, Serialize, Debug)]
pub struct SubscribeEvent {
    // benefit_end_month: String, // Undocumented field, go figure
    // time: String,
    // channel_id: String,
    // channel_name: String,
    // user_id: String,
    // user_name: String,
    
    /// This doesn't exist on anon but we will fill with "".
    /// Display name of the person who subscribed or sent a gift subscription
    pub display_name: Option<String>, 
    /// Sub plan id.
    /// Prime, 1000, 2000 or 3000
    pub sub_plan: String, // Prime, 1000, 2000, 3000
    // sub_plan_name: String,
    /// Deprecated by Twitch, use `cumulative_months` instead
    pub months: Option<usize>,
    /// Cumulative number of tenure months of the subscription
    pub cumulative_months: Option<usize>,
    /// Denotes the user’s most recent (and contiguous) subscription tenure streak in the channel
    pub streak_months: Option<usize>,
    /// Event type associated with the subscription product, values:
    /// * sub
    /// * resub
    /// * subgift
    /// * anonsubgift
    /// * resubgift
    /// * anonresubgift
    pub context: String, // sub, resub, subgift, anonsubgift, resubgift, anonresubgift
    /// If this sub message was caused by a gift subscription
    pub is_gift: bool,
    // recipient_id: Option<String>,
    // recipient_user_name: Option<String>,
    /// Display name of the person who received the subscription gift
    pub recipient_display_name: Option<String>,
    /// Number of months gifted as part of a single, multi-month gift OR number of months purchased as part of a multi-month subscription
    pub multi_month_duration: Option<usize>,
    /// The body of the user-entered resub message. Depending on the type of message, the message body contains different fields
    // pub sub_message: String,

    // I think this is wrong, it should just be "message"
    pub sub_message: SubscribeMessage,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct SubscribeMessage {
    pub message: String,
    // emotes: Option<>
}

//----- channel-points-channel-v1
/// Channel points event
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data", rename_all = "kebab-case")]
pub enum ChannelPointsEvent {
    /// Channel points event
    RewardRedeemed {
        timestamp: String,
        redemption: ChannelPoints,
    },
    #[serde(other)]
    Unknown,
}

// todo: flatten some of this
#[derive(Deserialize, Serialize, Debug)]
pub struct ChannelPoints {
    id: String,
    pub user: User,
    channel_id: String,
    redeemed_at: String,
    pub reward: Reward,
    status: String,
    pub user_input: Option<String>,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Reward {
    pub id: String,
    channel_id: String,
    pub title: String,
    prompt: String,
    cost: usize,
    pub is_user_input_required: bool,
    is_sub_only: bool,
    image: Option<serde_json::map::Map<String, serde_json::Value>>,
    default_image: Option<serde_json::map::Map<String, serde_json::Value>>,
    background_color: String,
    is_enabled: bool,
    is_paused: bool,
    is_in_stock: bool,
    max_per_stream: Option<serde_json::map::Map<String, serde_json::Value>>,
    should_redemptions_skip_request_queue: bool,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct User {
    id: String,
    login: String,
    pub display_name: String,
}

//----- channel-bits-badge-unlocks
#[derive(Deserialize, Debug)]
struct BitsBadgeUnlockEvent {
    user_id: String,
    user_name: String,
    channel_id: String,
    channel_name: String,
    badge_tier: usize, // 1000, 10000, etc
    chat_message: String,
    time: String,
}

//----- channel-bits-events-v1
//----- channel-bits-events-v2
// key difference here is a few fields in v2 are optional
#[derive(Deserialize, Serialize, Debug)]
pub struct BitsEvent {
    pub data: BitsData,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct BitsData {
    // version: String,
    // message_id: String,
    // message_type: String,
    // is_anonymous: Option<bool>,
    // time: String,
    // user_id: Option<String>,       // Null if anonymous

    /// Login name of the person who used the Bits,
    /// None if anonymous
    pub user_name: Option<String>,
    // display_name: String, // need to verify this exists, docs doesn't show it
    // channel_id: String,
    // channel_name: String,
    /// Chat message sent with the cheer.
    pub chat_message: Option<String>,
    // context: String, // cheer
    /// Number of bits used.
    pub bits_used: usize,
    // total_bits_used: usize,
    // badge_entitlement: Option<serde_json::map::Map<String, serde_json::Value>>, // Null if anonymous or user did not reach new badge level
}

//----- chat_moderator_actions
#[derive(Deserialize, Debug)]
struct ModeratorEvent {
    // ignoring for now, see example in _pubsub_examples
}

//----- whispers
#[derive(Deserialize, Debug)]
struct WhisperEvent {
    // ignoring for now, see example in _pubsub_examples
}

