//! A pre-opened Deepgram socket kept warm between dictations.
//!
//! Every dictation used to pay for a TLS + WebSocket handshake (typically
//! 300–600 ms) after the hotkey was pressed. Instead, once a session ends we
//! open the *next* connection in the background and keep it alive with
//! Deepgram's `KeepAlive` message (sockets with no traffic are closed after
//! ~10 s). KeepAlive carries no audio, so an idle standby is not billed.
//!
//! Each dictation still gets its own fresh socket — a standby is used once and
//! then replaced — so results can never bleed from one session into the next.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::Duration,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
    net::TcpStream,
    sync::oneshot,
    time::{interval, sleep, Instant, MissedTickBehavior},
};
use tokio_tungstenite::{tungstenite::Message, MaybeTlsStream, WebSocketStream};

pub type Socket = WebSocketStream<MaybeTlsStream<TcpStream>>;

/// Deepgram drops sockets after ~10 s without audio or a KeepAlive.
const KEEP_ALIVE_EVERY: Duration = Duration::from_secs(4);
/// A standby that is never used is closed after this long, so an idle app
/// doesn't hold a connection open all day. The next dictation after that
/// simply connects cold, exactly as before.
pub const STANDBY_IDLE_LIMIT: Duration = Duration::from_secs(5 * 60);

/// Identifies what a socket was opened for: the request URL (model, language,
/// sample rate, keyterms…) plus the credential. A standby is only reused when
/// the next session would have opened exactly the same connection. Stored as a
/// hash so the standby never retains the API key itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionKey(u64);

impl ConnectionKey {
    pub fn new(url: &str, api_key: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        api_key.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub struct Standby {
    key: ConnectionKey,
    take: oneshot::Sender<oneshot::Sender<Socket>>,
}

impl Standby {
    /// Starts keeping `socket` alive in the background.
    pub fn keep_alive(key: ConnectionKey, socket: Socket) -> Self {
        let (take, take_requests) = oneshot::channel();
        tokio::spawn(run(socket, take_requests));
        Self { key, take }
    }

    pub fn key(&self) -> ConnectionKey {
        self.key
    }

    /// Hands the socket over, or `None` if it has died or expired meanwhile —
    /// the caller then connects cold.
    pub async fn take(self) -> Option<Socket> {
        let (reply, socket) = oneshot::channel();
        self.take.send(reply).ok()?;
        tokio::time::timeout(Duration::from_millis(500), socket)
            .await
            .ok()?
            .ok()
    }
}

async fn run(mut socket: Socket, mut take_requests: oneshot::Receiver<oneshot::Sender<Socket>>) {
    let deadline = sleep(STANDBY_IDLE_LIMIT);
    tokio::pin!(deadline);
    let mut keep_alive = interval(KEEP_ALIVE_EVERY);
    keep_alive.set_missed_tick_behavior(MissedTickBehavior::Delay);
    let opened = Instant::now();
    loop {
        tokio::select! {
            request = &mut take_requests => {
                match request {
                    Ok(reply) => {
                        log::debug!("deepgram standby handed over after {} ms", opened.elapsed().as_millis());
                        let _ = reply.send(socket);
                    }
                    // Replaced or dropped without being used.
                    Err(_) => { let _ = socket.close(None).await; }
                }
                return;
            }
            _ = keep_alive.tick() => {
                if socket.send(Message::Text(r#"{"type":"KeepAlive"}"#.into())).await.is_err() {
                    log::debug!("deepgram standby closed while sending KeepAlive");
                    return;
                }
            }
            message = socket.next() => match message {
                // Nothing meaningful arrives before audio; anything else means
                // the server closed or errored, so stop advertising this socket.
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => {
                    log::debug!("deepgram standby closed by the server");
                    return;
                }
                Some(Ok(_)) => {}
            },
            _ = &mut deadline => {
                log::debug!("deepgram standby expired unused");
                let _ = socket.close(None).await;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ConnectionKey;

    #[test]
    fn keys_match_only_for_identical_connections() {
        let url = "wss://api.deepgram.com/v1/listen?model=nova-3&sample_rate=16000";
        assert_eq!(ConnectionKey::new(url, "key"), ConnectionKey::new(url, "key"));
        assert_ne!(ConnectionKey::new(url, "key"), ConnectionKey::new(url, "other"));
        assert_ne!(
            ConnectionKey::new(url, "key"),
            ConnectionKey::new(&url.replace("16000", "48000"), "key")
        );
    }
}
