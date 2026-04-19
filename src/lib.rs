use tokio::sync::Notify;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

mod tcp_stream;
mod xml_framer;
mod stanza;
mod xmpp;

use stanza::Stanza;
use xml_framer::XmlFramer;

#[derive(Debug, Clone)]
pub enum XmppEvent
{
    Connecting,
    EstablishingTls,
    Authenticating,
    Connected,
    RoomJoined { room: String, members: Vec<RoomMember> },
    RoomLeft(String),
    MemberJoined { room: String, member: RoomMember },
    MemberLeft { room: String, nick: String },
    RoomMessage { room: String, nick: String, body: String, timestamp: Option<String> },
    RoomSubject { room: String, subject: String },
}

#[derive(Debug, Clone)]
pub struct RoomMember
{
    pub nick: String,
    pub affiliation: String,
    pub role: String,
    pub show: Option<String>,
    pub status: Option<String>,
}

pub struct XmppClient
{
    shutdown: Arc<Notify>,
    task: JoinHandle<()>,
    writer: tcp_stream::TcpWriter,
    bound_jid: String,
}

impl XmppClient
{
    pub async fn new(jid: &str, password: &str) -> Result<(Self, mpsc::Receiver<XmppEvent>), String>
    {
        let (event_tx, event_rx) = mpsc::channel(32);

        // Setup TCP connection, TLS, and SASL authentication.
        let (bound_jid, tcp) = xmpp::setup_connection(&event_tx, jid, password).await?;

        // We now have a working xmpp connection, split and spawn reader loop.
        let (mut reader, writer) = tcp.split()?;
        let shutdown = Arc::new(Notify::new());
        let shutdown_clone = shutdown.clone();
        let event_tx_loop = event_tx.clone();

        let task = tokio::spawn(async move
        {
            let mut framer = XmlFramer::new_opened();
            let mut pending_joins: HashMap<String, Vec<RoomMember>> = HashMap::new();
            let mut pending_messages: HashMap<String, Vec<XmppEvent>> = HashMap::new();
            let mut joined_rooms: HashSet<String> = HashSet::new();

            loop
            {
                // Parse all data on the framer and process stanzas.
                while let Some(stanza_xml) = framer.try_next()
                {
                    log::debug!("Received stanza: {}", stanza_xml);
                    xmpp::process_stanza(&stanza_xml, &event_tx_loop, &mut pending_joins, &mut pending_messages, &mut joined_rooms).await;
                }

                // Collect data.
                tokio::select!
                {
                    result = reader.read() =>
                    {
                        match result
                        {
                            Ok(data) =>
                            {
                                framer.feed(&data);
                            }
                            Err(e) =>
                            {
                                log::error!("Read error: {}", e);
                                break;
                            }
                        }
                    }
                    _ = shutdown_clone.notified() =>
                    {
                        break;
                    }
                }
            }
        });

        let _ = event_tx.send(XmppEvent::Connected).await;

        return Ok((Self { shutdown, task, writer, bound_jid }, event_rx));
    }

    pub fn get_jid(&self) -> &str
    {
        return &self.bound_jid;
    }

    pub async fn join_room(&mut self, room_jid: &str, nick: &str) -> Result<(), String>
    {
        let presence = stanza::muc::MucJoinPresence::new(room_jid.to_string(), nick.to_string());
        self.writer.write(&presence.as_bytes()).await
    }

    pub async fn leave_room(&mut self, room_jid: &str, nick: &str) -> Result<(), String>
    {
        let presence = stanza::muc::MucLeavePresence::new(room_jid.to_string(), nick.to_string());
        self.writer.write(&presence.as_bytes()).await
    }

    pub async fn send_room_message(&mut self, room_jid: &str, body: &str) -> Result<(), String>
    {
        let msg = stanza::muc::MucGroupMessage::new(room_jid.to_string(), body.to_string());
        self.writer.write(&msg.as_bytes()).await
    }

    pub async fn close(mut self)
    {
        self.shutdown.notify_one();
        let _ = self.task.await;
        self.writer.shutdown().await;
    }
}
