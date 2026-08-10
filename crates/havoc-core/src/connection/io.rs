//! The IRC line transport the actor codes against — the trait prompt 4
//! deferred. It lives in havoc-core (not havoc-transport) because no crate
//! edge exists between the two and the §4.2 boundary forbids creating one;
//! havoc-transport carries client/core transports, not IRC sockets.
//!
//! TLS is the default path (NORTH-STAR §2.3): rustls over tokio-rustls, roots
//! from webpki-roots, with `--tls-ca` appending one named extra anchor.
//! Verification is never switchable off — there is deliberately no
//! skip-verify anywhere: you say what you trust, you never stop verifying.

use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use rustls_pki_types::CertificateDer;
use rustls_pki_types::pem::PemObject;
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

/// How a connection is secured. `Plaintext` is the loud opt-in, enforced
/// loopback-only at the CLI boundary.
#[derive(Debug, Clone)]
pub enum Security {
    Tls {
        /// The name presented for SNI and certificate verification — the
        /// dialed host, never an override.
        server_name: String,
        /// Extra trust anchor (PEM), appended to webpki-roots — for local
        /// ergo's generated cert. Verification stays on.
        ca_file: Option<PathBuf>,
    },
    Plaintext,
}

/// Lines in, lines out, lifecycle. CRLF framing is the implementor's job;
/// callers speak bare lines.
pub trait LineTransport {
    fn send_line(&mut self, line: &str) -> impl Future<Output = io::Result<()>> + Send;
    /// `Ok(None)` is orderly EOF — the peer closed.
    fn next_line(&mut self) -> impl Future<Output = io::Result<Option<String>>> + Send;
}

/// One concrete transport for the actor to hold — an enum rather than a
/// generic because `LineTransport`'s RPITIT methods make the enum cheaper.
pub enum AnyLineTransport {
    Tcp(TcpLineTransport),
    Tls(Box<TlsLineTransport>),
}

impl AnyLineTransport {
    pub async fn connect(security: &Security, host: &str, port: u16) -> io::Result<Self> {
        match security {
            Security::Plaintext => Ok(Self::Tcp(TcpLineTransport::connect(host, port).await?)),
            Security::Tls {
                server_name,
                ca_file,
            } => Ok(Self::Tls(Box::new(
                TlsLineTransport::connect(host, port, server_name, ca_file.as_deref()).await?,
            ))),
        }
    }
}

impl LineTransport for AnyLineTransport {
    async fn send_line(&mut self, line: &str) -> io::Result<()> {
        match self {
            Self::Tcp(t) => write_line(&mut t.writer, line).await,
            Self::Tls(t) => write_line(&mut t.writer, line).await,
        }
    }

    async fn next_line(&mut self) -> io::Result<Option<String>> {
        match self {
            Self::Tcp(t) => read_line(&mut t.reader).await,
            Self::Tls(t) => read_line(&mut t.reader).await,
        }
    }
}

/// Plain TCP — the loud loopback-only opt-in.
pub struct TcpLineTransport {
    reader: BufReader<ReadHalf<TcpStream>>,
    writer: WriteHalf<TcpStream>,
}

impl TcpLineTransport {
    pub async fn connect(host: &str, port: u16) -> io::Result<Self> {
        let stream = TcpStream::connect((host, port)).await?;
        let (read, writer) = tokio::io::split(stream);
        Ok(Self {
            reader: BufReader::new(read),
            writer,
        })
    }
}

impl LineTransport for TcpLineTransport {
    async fn send_line(&mut self, line: &str) -> io::Result<()> {
        write_line(&mut self.writer, line).await
    }

    async fn next_line(&mut self) -> io::Result<Option<String>> {
        read_line(&mut self.reader).await
    }
}

/// TLS via rustls. Roots: webpki-roots, plus at most one named extra anchor.
pub struct TlsLineTransport {
    reader: BufReader<ReadHalf<TlsStream<TcpStream>>>,
    writer: WriteHalf<TlsStream<TcpStream>>,
}

impl TlsLineTransport {
    pub async fn connect(
        host: &str,
        port: u16,
        server_name: &str,
        ca_file: Option<&std::path::Path>,
    ) -> io::Result<Self> {
        let mut roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        if let Some(path) = ca_file {
            for cert in CertificateDer::pem_file_iter(path)
                .map_err(|e| io::Error::other(format!("reading {}: {e}", path.display())))?
            {
                let cert = cert.map_err(|e| io::Error::other(format!("bad PEM: {e}")))?;
                roots
                    .add(cert)
                    .map_err(|e| io::Error::other(format!("bad CA cert: {e}")))?;
            }
        }
        let config = rustls::ClientConfig::builder()
            .with_root_certificates(roots)
            .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(config));
        let name = rustls_pki_types::ServerName::try_from(server_name.to_owned())
            .map_err(|e| io::Error::other(format!("invalid server name: {e}")))?;

        let tcp = TcpStream::connect((host, port)).await?;
        let stream = connector.connect(name, tcp).await?;
        let (read, writer) = tokio::io::split(stream);
        Ok(Self {
            reader: BufReader::new(read),
            writer,
        })
    }
}

async fn write_line<W: AsyncWrite + Unpin>(writer: &mut W, line: &str) -> io::Result<()> {
    writer.write_all(line.as_bytes()).await?;
    writer.write_all(b"\r\n").await?;
    writer.flush().await
}

async fn read_line<R: AsyncRead + Unpin>(reader: &mut BufReader<R>) -> io::Result<Option<String>> {
    let mut line = String::new();
    let n = reader.read_line(&mut line).await?;
    if n == 0 {
        return Ok(None);
    }
    while line.ends_with('\n') || line.ends_with('\r') {
        line.pop();
    }
    Ok(Some(line))
}
