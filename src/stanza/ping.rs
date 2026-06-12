use super::Stanza;

/// An outgoing XEP-0199 ping request.
///
/// Serialises to `<iq type='get'><ping xmlns='urn:xmpp:ping'/></iq>`. When
/// `to` is `None` the ping targets the user's own server (the usual choice for
/// a keep-alive or liveness probe); otherwise it is addressed to the given JID.
pub struct PingRequest
{
    id: String,
    to: Option<String>,
}

impl PingRequest
{
    pub fn new(id: String, to: Option<String>) -> Self
    {
        return Self { id, to };
    }
}

impl Stanza for PingRequest
{
    fn to_xml(&self) -> String
    {
        match &self.to
        {
            Some(to) => format!(
                "<iq type='get' to='{}' id='{}'><ping xmlns='urn:xmpp:ping'/></iq>",
                quick_xml::escape::escape(to),
                quick_xml::escape::escape(&self.id)
            ),
            None => format!(
                "<iq type='get' id='{}'><ping xmlns='urn:xmpp:ping'/></iq>",
                quick_xml::escape::escape(&self.id)
            ),
        }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn outgoing_ping_to_server()
    {
        let req = PingRequest::new("ping_1".to_string(), None);
        assert_eq!(
            req.to_xml(),
            "<iq type='get' id='ping_1'><ping xmlns='urn:xmpp:ping'/></iq>"
        );
    }

    #[test]
    fn outgoing_ping_to_target()
    {
        let req = PingRequest::new("ping_2".to_string(), Some("peer@example.com".to_string()));
        assert_eq!(
            req.to_xml(),
            "<iq type='get' to='peer@example.com' id='ping_2'><ping xmlns='urn:xmpp:ping'/></iq>"
        );
    }
}
