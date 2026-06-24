# libxmpp

An async XMPP client library for Rust, built on Tokio.

## Features

- TCP + STARTTLS connection handling
- SASL authentication (SCRAM-SHA-512, SCRAM-SHA-256, SCRAM-SHA-1, PLAIN)
- Resource binding
- One-to-one chat messages
- Multi-User Chat (MUC) rooms: join, leave, send and receive group messages
- Presence updates and presence-error reporting
- XEP-0199 ping for keep-alives and connection liveness probing, and
  automatically answering pings from other entities
- XEP-0030 service discovery (querying other entities, and automatically
  answering info and items queries from others)

## Public API

The crate exposes these top-level items:

- [`XmppClient`](src/lib.rs) — the client handle. Created with
  `XmppClient::new(jid, password).await`, which returns the client
  and an `mpsc::Receiver<XmppEvent>` for incoming events.
- [`XmppEvent`](src/lib.rs) — the events delivered on that receiver
  (connection lifecycle, room joins/leaves, messages, presence
  errors).
- [`RoomMember`](src/lib.rs) — describes an occupant of a MUC room.
- [`DiscoInfo`](src/lib.rs), [`DiscoIdentity`](src/lib.rs) and
  [`DiscoItem`](src/lib.rs) — the results of XEP-0030 service
  discovery queries.

`XmppClient` provides:

| Method | Purpose |
| --- | --- |
| `new(jid, password)` | Connect, upgrade to TLS, authenticate, and bind a resource. |
| `get_jid()` | Return the full JID assigned by the server. |
| `join_room(room_jid, nick)` | Join a MUC room. |
| `leave_room(room_jid, nick)` | Leave a MUC room. |
| `send_room_message(room_jid, body)` | Send a group chat message. |
| `send_message(to, body)` | Send a one-to-one chat message. |
| `ping(to, timeout)` | Send an XEP-0199 ping and await the reply, returning the round-trip time. `to = None` pings the user's own server. |
| `disco_info(to, node, timeout)` | Send an XEP-0030 service discovery info query, returning the entity's advertised identities and features. |
| `disco_items(to, node, timeout)` | Send an XEP-0030 service discovery items query, returning the items the entity hosts. |
| `set_disco_info(info)` | Replace the identities and features advertised when answering incoming service discovery info queries. |
| `add_disco_feature(var)` | Advertise an additional feature namespace in replies to service discovery info queries. |
| `set_disco_items(items)` | Replace the items advertised when answering incoming service discovery items queries. |
| `close()` | Shut down the reader task and close the socket. |

Incoming service discovery queries and XEP-0199 pings from other
entities are answered automatically by the background reader task. Disco
replies use the advertised configuration above; by default a client
reports a single `client`/`bot` identity and the features it answers
(the two service-discovery namespaces and `urn:xmpp:ping`).

See the rustdoc on `src/lib.rs` for detailed documentation of each
item, including the meaning of every `XmppEvent` variant.

## Example

```rust
use xmpp::{XmppClient, XmppEvent};

#[tokio::main]
async fn main() -> Result<(), xmpp::XmppError>
{
    let (mut client, mut events) =
        XmppClient::new("user@example.com", "password").await?;

    client.join_room("room@conference.example.com", "my-nick").await?;

    while let Some(event) = events.recv().await
    {
        match event
        {
            XmppEvent::RoomJoined { room, members } =>
            {
                println!("joined {} ({} members)", room, members.len());
            }
            XmppEvent::RoomMessage { room, nick, body, .. } =>
            {
                println!("[{}] {}: {}", room, nick, body);
            }
            XmppEvent::DirectMessage { from, body, .. } =>
            {
                println!("{}: {}", from, body);
            }
            XmppEvent::PresenceError { from, condition, .. } =>
            {
                eprintln!("presence error from {}: {}", from, condition);
                break;
            }
            _ => {}
        }
    }

    client.close().await;
    return Ok(());
}
```

## Building

```sh
cargo build
cargo test
cargo doc --open
```

## License

See [LICENSE](LICENSE).
