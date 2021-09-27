use std::collections::HashMap;

use neotwitch::{Irc, IrcMessage};

struct Cursor<'msg> {
    pos: usize,
    src: &'msg str,
}

impl<'msg> Cursor<'msg> {
    pub fn new(src: &'msg str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn next_pat(
        &mut self,
        pat: &str,
        include_pat: bool,
    ) -> Option<&'msg str> {
        let pos = &self.src[self.pos..].find(pat)?;
        let old_pos = self.pos;
        self.pos += pos;
        if include_pat {
            self.pos += 1;
        }
        let ret_val = &self.src[old_pos..self.pos];
        Some(ret_val)
    }

    pub fn skip(&mut self, pat: &str) -> Option<()> {
        self.next_pat(pat, true)?;
        Some(())
    }
}

const ACTION: &str = "\u{1}ACTION ";

pub fn parse<'msg>(raw: &'msg str) -> Option<Irc<'msg>> {
    let mut cursor = Cursor::new(raw);

    // Tags
    let tags = parse_tags(&mut cursor);

    // Prefix
    let prefix = cursor.next_pat(" ", false)?;
    cursor.skip(" ");

    // Command
    let command = cursor.next_pat(" ", false)?;
    cursor.skip(" ");

    let params_and_trailing = cursor.next_pat("\r\n", false)?;

    let (param, trailing) = match params_and_trailing.find(':') {
        Some(pos) => {
            let (param, trailing) = params_and_trailing.split_at(pos + 1);
            (&param[1..pos - 1], trailing)
        },
        None => (params_and_trailing, ""),
    };

    match command {
        "CLEARCHAT" => Some(Irc::ClearChat),
        "PRIVMSG" => Some(Irc::Message(parse_msg(prefix, param, trailing, tags)?)),
        _ => None,
    }
}

fn parse_msg<'msg>(
    prefix: &'msg str,
    param: &'msg str,
    trailing: &'msg str,
    tags: HashMap<String, String>,
) -> Option<IrcMessage<'msg>> {

    // Username
    let mut cursor = Cursor::new(prefix);
    cursor.skip(":");
    let username = cursor.next_pat("!", false)?;

    // Channel
    let channel = param;

    // Message
    let message = trailing;

    // cursor.skip(":")?;
    let (msg, is_action) = {
        let mut msg = message;
        let is_action = msg.starts_with(ACTION);
        if is_action {
            msg = &msg[ACTION.len()..msg.len() - 1];
        }
        (msg, is_action)
    };

    Some(IrcMessage::new(username, channel, msg, is_action, tags))
}

fn parse_tags(cursor: &mut Cursor) -> HashMap<String, String> {
    let mut key_values = HashMap::new();
    match cursor.src.starts_with('@') {
        true => cursor.src = &cursor.src[1..],
        false => return key_values,
    }

    let tags = match cursor.next_pat(" ", true) {
        Some(t) => t,
        None => return key_values,
    };

    for tags in tags.split(';') {
        let mut tag = tags.splitn(2, '=');
        let key = tag.next().expect("The key should always be there");
        let val = match tag.next() {
            Some(t) => t,
            None => continue,
        };

        key_values.insert(key.into(), val.into());
    }

    key_values
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_message_with_tags() {
        let input = "@badge-info=subscriber/17;badges=broadcaster/1,subscriber/3009;client-nonce=d72eefc2eb1c108b085ecb9c2492afa4;color=#5F9EA0;display-name=togglebit;emotes=;flags=;id=0136c224-4c6d-4554-b5ad-17b6a70ad96d;mod=0;room-id=474725923;subscriber=1;tmi-sent-ts=1632681408560;turbo=0;user-id=474725923;user-type= :togglebit!togglebit@togglebit.tmi.twitch.tv PRIVMSG #togglebit :test\r\n";
        let msg = match parse(input).unwrap() {
            Irc::Message(msg) => msg,
            _ => panic!("Incorrect message type")
        };
        assert_eq!(msg.user, "togglebit");
        assert_eq!(msg.tags["color"], "#5F9EA0");
    }

    #[test]
    fn test_parse_message_without_tags() {
        let input = ":randomuser!randomuser@randomuser.tmi.twitch.tv PRIVMSG #togglebit :some random message\r\n";
        let msg = parse(input).unwrap();
        assert!(matches!(msg, Irc::Message(IrcMessage { user: Cow::Borrowed("randomuser"), .. })));
    }

    #[test]
    fn parse_command() {
        let input = "@room-id=474725923;tmi-sent-ts=1632686729261 :tmi.twitch.tv CLEARCHAT #togglebit\r\n";
        let msg = parse(input).unwrap();
        assert!(matches!(msg, Irc::ClearChat));
    }

    #[test]
    fn parse_action() {
        let input = "@badge-info=subscriber/17;badges=broadcaster/1,subscriber/3009;color=#5F9EA0;display-name=togglebit;emotes=;flags=;id=4c4205a5-cce1-497f-8ea7-aed18a3b113e;mod=0;room-id=474725923;subscriber=1;tmi-sent-ts=1632729819621;turbo=0;user-id=474725923;user-type= :togglebit!togglebit@togglebit.tmi.twitch.tv PRIVMSG #togglebit :\u{1}ACTION test\u{1}\r\n";
        let msg = parse(input).unwrap();
        assert!(matches!(msg, Irc::Message(IrcMessage { user: Cow::Borrowed("togglebit"), action: true, .. })));
    }
}
