//! The IRC line transport the actor codes against — the trait prompt 4
//! deferred. It lives in havoc-core (not havoc-transport) because no crate
//! edge exists between the two and the §4.2 boundary forbids creating one;
//! havoc-transport carries client/core transports, not IRC sockets.
//!
//! The scripted-fake role belongs to the transcript tables in
//! `tests/state_machine.rs`; there is deliberately no second fake here.

use std::io;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};

/// Lines in, lines out, lifecycle. CRLF framing is the implementor's job;
/// callers speak bare lines.
pub trait LineTransport {
    fn send_line(&mut self, line: &str) -> impl Future<Output = io::Result<()>> + Send;
    /// `Ok(None)` is orderly EOF — the peer closed.
    fn next_line(&mut self) -> impl Future<Output = io::Result<Option<String>>> + Send;
}

/// Plain TCP. TLS is prompt 6; the loud plaintext opt-in is enforced at the
/// CLI boundary, not here.
pub struct TcpLineTransport {
    reader: BufReader<OwnedReadHalf>,
    writer: OwnedWriteHalf,
}

impl TcpLineTransport {
    pub async fn connect(host: &str, port: u16) -> io::Result<Self> {
        let stream = TcpStream::connect((host, port)).await?;
        let (read, writer) = stream.into_split();
        Ok(Self {
            reader: BufReader::new(read),
            writer,
        })
    }
}

impl LineTransport for TcpLineTransport {
    async fn send_line(&mut self, line: &str) -> io::Result<()> {
        self.writer.write_all(line.as_bytes()).await?;
        self.writer.write_all(b"\r\n").await?;
        self.writer.flush().await
    }

    async fn next_line(&mut self) -> io::Result<Option<String>> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await?;
        if n == 0 {
            return Ok(None);
        }
        while line.ends_with('\n') || line.ends_with('\r') {
            line.pop();
        }
        Ok(Some(line))
    }
}
