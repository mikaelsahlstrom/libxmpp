use super::Stanza;
use serde::Deserialize;

pub struct MucJoinPresence
{
    room_jid: String,
    nick: String,
}

pub struct MucGroupMessage
{
    to: String,
    body: String,
}

impl MucGroupMessage
{
    pub fn new(to: String, body: String) -> Self
    {
        Self { to, body }
    }
}

impl Stanza for MucGroupMessage
{
    fn to_xml(&self) -> String
    {
        format!(
            "<message type='groupchat' to='{}'><body>{}</body></message>",
            self.to,
            quick_xml::escape::escape(&self.body)
        )
    }
}

impl MucJoinPresence
{
    pub fn new(room_jid: String, nick: String) -> Self
    {
        Self { room_jid, nick }
    }
}

impl Stanza for MucJoinPresence
{
    fn to_xml(&self) -> String
    {
        format!(
            "<presence to='{}/{}'><x xmlns='http://jabber.org/protocol/muc'/></presence>",
            self.room_jid, self.nick
        )
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename = "presence")]
pub struct MucPresence
{
    #[serde(rename = "@from")]
    pub from: String,
    #[serde(rename = "@type", default)]
    pub presence_type: Option<String>,
    #[serde(default)]
    pub x: Option<MucUserX>,
}

#[derive(Deserialize, Debug)]
pub struct MucUserX
{
    #[serde(default)]
    pub item: Option<MucItem>,
    #[serde(rename = "status", default)]
    pub status: Vec<MucStatus>,
}

#[derive(Deserialize, Debug)]
pub struct MucItem
{
    #[serde(rename = "@affiliation", default)]
    pub affiliation: Option<String>,
    #[serde(rename = "@role", default)]
    pub role: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct MucStatus
{
    #[serde(rename = "@code")]
    pub code: String,
}

impl MucPresence
{
    pub fn from_xml(xml: &str) -> Result<Self, String>
    {
        quick_xml::de::from_str(xml).map_err(|e| e.to_string())
    }

    pub fn room_and_nick(&self) -> Option<(&str, &str)>
    {
        let slash = self.from.find('/')?;
        Some((&self.from[..slash], &self.from[slash + 1..]))
    }

    pub fn is_self_presence(&self) -> bool
    {
        self.x.as_ref()
            .map(|x| x.status.iter().any(|s| s.code == "110"))
            .unwrap_or(false)
    }
}
