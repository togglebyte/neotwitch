use super::IrcMessage;

fn parse_next<'a>(src: &'a str, pat: &str) -> (&'a str, usize) {
    let pos = src.find(pat).unwrap();
    (&src[..pos], pos)
}

struct Cursor<'msg> {
    pos: usize,
    src: &'msg str,
}

impl<'msg> Cursor<'msg> {
    pub fn new(src: &'msg str) -> Self {
        Self { src, pos: 0 }
    }

    pub fn next_pat(&mut self, pat: &str) -> Option<&'msg str> {
        let pos = &self.src[self.pos..].find(pat)?;
        let old_pos = self.pos;
        self.pos += pos + 1;
        let ret_val = &self.src[old_pos..self.pos];
        Some(ret_val)
    }

    pub fn skip(&mut self, pat: &str) -> Option<()> {
        self.next_pat(pat)?;
        Some(())
    }

    pub fn remaining(self) -> &'msg str {
        &self.src[self.pos..]
    }
}

const ACTION: &str = "\u{1}ACTION ";

// :<user>!<user>@<user>.tmi.twitch.tv PRIVMSG #<channel> :This is a sample message
// :randomuser!randomuser@randomuser.tmi.twitch.tv PRIVMSG #togglebit :some random message
pub fn parse<'msg>(raw: &'msg str) -> Option<IrcMessage<'msg>> {
    let mut cursor = Cursor::new(raw);
    cursor.skip(":")?;
    let username = cursor.next_pat("!")?;
    cursor.skip("#")?;
    let channel = cursor.next_pat(" ")?;
    cursor.skip(":")?;
    let (msg, is_action) = {
        let mut msg = cursor.remaining().trim_end();
        let is_action = msg.starts_with(ACTION);
        if is_action {
            msg = &msg[ACTION.len()..msg.len() - 1];
        }
        (msg, is_action)
    };

    Some(IrcMessage::new(username, channel, msg, is_action, raw))
}


