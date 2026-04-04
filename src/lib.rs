use tokio::sync::Notify;
use tokio::task::JoinHandle;
use std::sync::Arc;

mod tcp_stream;
mod xml_framer;
mod stanza;

use stanza::Stanza;
use xml_framer::XmlFramer;

pub struct XmppClient
{
    shutdown: Arc<Notify>,
    task: JoinHandle<()>,
    writer: tcp_stream::TcpWriter,
    bound_jid: String,
}

impl XmppClient
{
    pub async fn new(jid: &str, password: &str) -> Result<Self, String>
    {
        let (username, domain) = parse_jid(jid)?;

        let mut tcp = tcp_stream::Tcp::new()
            .connect(domain.to_string(), 5222).await?;

        let mut framer = XmlFramer::new();

        log::debug!("Opening initial stream...");

        tcp.send(&stanza::stream::Stream::new(jid.to_string(), domain.to_string()).as_bytes()).await?;
        let (_stream, mut features) = read_stream_and_features(&mut tcp, &mut framer).await?;

        // TLS
        if features.starttls.is_some()
        {
            log::debug!("Starting TLS negotiation...");

            tcp.send(&stanza::stream::StartTlsRequest.as_bytes()).await?;
            let response = read_frame(&mut tcp, &mut framer).await?;

            if response.contains("<failure")
            {
                return Err("STARTTLS failed".to_string());
            }

            tcp = tcp.add_tls().await?;
            framer.reset();

            log::debug!("TLS established");
            log::debug!("Reopening stream over TLS...");

            tcp.send(&stanza::stream::Stream::new(jid.to_string(), domain.to_string()).as_bytes()).await?;
            let (_stream, new_features) = read_stream_and_features(&mut tcp, &mut framer).await?;
            features = new_features;
        }

        // SASL authentication
        if let Some(ref mechs) = features.mechanisms
        {
            if !mechs.mechanism.contains(&"SCRAM-SHA-1".to_string())
            {
                return Err("Server doesn't support SCRAM-SHA-1".to_string());
            }

            log::debug!("Starting SASL SCRAM-SHA-1 authentication...");

            do_sasl(&mut tcp, &mut framer, &username, password).await?;
            framer.reset();

            log::debug!("SASL authentication successful");

            log::debug!("Reopening stream after authentication...");
            tcp.send(&stanza::stream::Stream::new(jid.to_string(), domain.to_string()).as_bytes()).await?;
            let (_stream, new_features) = read_stream_and_features(&mut tcp, &mut framer).await?;
            features = new_features;
        }
        else
        {
            // Require authentication.
            return Err("Server doesn't offer SASL mechanisms".to_string());
        }

        // Resource binding
        let bound_jid = if features.bind.is_some()
        {
            log::debug!("Binding resource...");
            do_bind(&mut tcp, &mut framer).await?
        }
        else
        {
            jid.to_string()
        };

        log::info!("Bound JID: {}", bound_jid);

        // Split and spawn reader loop
        let (mut reader, writer) = tcp.split()?;
        let shutdown = Arc::new(Notify::new());
        let shutdown_clone = shutdown.clone();

        let task = tokio::spawn(async move
        {
            loop
            {
                while let Some(stanza_xml) = framer.try_next()
                {
                    log::debug!("Received stanza: {}", stanza_xml);
                }

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

        return Ok(Self { shutdown, task, writer, bound_jid });
    }

    pub fn jid(&self) -> &str
    {
        return &self.bound_jid;
    }

    pub async fn write(&mut self, data: &[u8]) -> Result<(), String>
    {
        return self.writer.write(data).await;
    }

    pub async fn close(mut self)
    {
        self.shutdown.notify_one();
        let _ = self.task.await;
        self.writer.shutdown().await;
    }
}

fn parse_jid(jid: &str) -> Result<(&str, &str), String>
{
    let at = jid.find('@').ok_or_else(|| format!("Invalid JID (no @): {}", jid))?;
    return Ok((&jid[..at], &jid[at + 1..]));
}

async fn read_frame(tcp: &mut tcp_stream::Tcp, framer: &mut XmlFramer) -> Result<String, String>
{
    loop
    {
        if let Some(frame) = framer.try_next()
        {
            return Ok(frame);
        }

        let data = tcp.recv().await?;
        framer.feed(&data);
    }
}

async fn read_stream_and_features(
    tcp: &mut tcp_stream::Tcp,
    framer: &mut XmlFramer,
) -> Result<(stanza::stream::Stream, stanza::stream::StreamFeatures), String>
{
    let header = read_frame(tcp, framer).await?;
    let (stream, _) = stanza::stream::Stream::from_xml(&header)?;

    log::debug!("Stream opened: {:?}", stream);

    let features_xml = read_frame(tcp, framer).await?;
    let features = stanza::stream::StreamFeatures::from_xml(&features_xml)?;

    log::debug!("Stream features: {:?}", features);

    return Ok((stream, features));
}

async fn do_sasl(
    tcp: &mut tcp_stream::Tcp,
    framer: &mut XmlFramer,
    username: &str,
    password: &str,
) -> Result<(), String>
{
    let mut scram = stanza::sasl::ScramSha1Client::new(username, password);

    // Send <auth>
    tcp.send(scram.auth_xml().as_bytes()).await?;

    // Read <challenge>
    let challenge_xml = read_frame(tcp, framer).await?;
    if stanza::sasl::is_failure(&challenge_xml)
    {
        return Err(format!("SASL auth failed: {}", challenge_xml));
    }

    let challenge_b64 = stanza::sasl::parse_challenge(&challenge_xml)?;

    // Send <response>
    let response_xml = scram.response_xml(&challenge_b64)?;
    tcp.send(response_xml.as_bytes()).await?;

    // Read <success>
    let success_xml = read_frame(tcp, framer).await?;
    if stanza::sasl::is_failure(&success_xml)
    {
        return Err(format!("SASL auth failed: {}", success_xml));
    }

    let success_b64 = stanza::sasl::parse_success(&success_xml)?;
    scram.verify_success(&success_b64)?;

    return Ok(());
}

async fn do_bind(
    tcp: &mut tcp_stream::Tcp,
    framer: &mut XmlFramer,
) -> Result<String, String>
{
    let bind_req = stanza::bind::BindRequest::new("bind_1".to_string(), None);
    tcp.send(&bind_req.as_bytes()).await?;

    let result_xml = read_frame(tcp, framer).await?;
    let result = stanza::bind::BindResult::from_xml(&result_xml)?;

    if result.iq_type != "result"
    {
        return Err(format!("Bind failed: {:?}", result));
    }

    return result.bind
        .and_then(|b| b.jid)
        .ok_or_else(|| "No JID in bind result".to_string());
}
