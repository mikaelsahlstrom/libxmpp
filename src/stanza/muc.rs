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
    pub x: Vec<MucUserX>,
}

#[derive(Deserialize, Debug)]
pub struct MucUserX
{
    #[serde(rename = "$value", default)]
    children: Vec<MucUserXChild>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum MucUserXChild
{
    Item(MucItem),
    Status(MucStatus),
}

impl MucUserX
{
    pub fn items(&self) -> impl Iterator<Item = &MucItem>
    {
        self.children.iter().filter_map(|c| match c
        {
            MucUserXChild::Item(i) => Some(i),
            _ => None,
        })
    }

    pub fn statuses(&self) -> impl Iterator<Item = &MucStatus>
    {
        self.children.iter().filter_map(|c| match c
        {
            MucUserXChild::Status(s) => Some(s),
            _ => None,
        })
    }
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

#[derive(Deserialize, Debug)]
#[serde(rename = "message")]
pub struct MucMessage
{
    #[serde(rename = "@from", default)]
    pub from: Option<String>,
    #[serde(rename = "@type", default)]
    pub message_type: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub delay: Option<MessageDelay>,
}

#[derive(Deserialize, Debug)]
pub struct MessageDelay
{
    #[serde(rename = "@stamp", default)]
    pub stamp: Option<String>,
}

impl MucMessage
{
    pub fn from_xml(xml: &str) -> Result<Self, String>
    {
        quick_xml::de::from_str(xml).map_err(|e| e.to_string())
    }

    pub fn room_and_nick(&self) -> Option<(&str, &str)>
    {
        let from = self.from.as_deref()?;
        let slash = from.find('/')?;
        Some((&from[..slash], &from[slash + 1..]))
    }
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

    pub fn muc_user_x(&self) -> Option<&MucUserX>
    {
        self.x.iter().find(|x| !x.children.is_empty())
    }

    pub fn is_self_presence(&self) -> bool
    {
        self.muc_user_x()
            .map(|x| x.statuses().any(|s| s.code == "110"))
            .unwrap_or(false)
    }
}
