use bytes::{Bytes, BytesMut};
use mysql_common::proto::codec::error::PacketCodecError;
use mysql_common::proto::codec::PacketCodec;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::*;
use warpgate_database_protocols::io::Encode;

use crate::tls::{MaybeTlsStream, MaybeTlsStreamError, UpgradableStream};

use stream_reconnect::{UnderlyingStream, ReconnectStream};
use std::future::Future;
use std::io;
use std::pin::Pin;
use tokio_tungstenite::{connect_async, MaybeTlsStream as WsMaybeTlsStream, WebSocketStream};
use tokio_tungstenite::tungstenite::{Message, error::Error as WsError};
use futures::{Sink, SinkExt, Stream, StreamExt};
use std::task::{Context, Poll};

#[derive(thiserror::Error, Debug)]
pub enum MySqlStreamError {
    #[error("packet codec error: {0}")]
    Codec(#[from] PacketCodecError),
    #[error("I/O: {0}")]
    Io(#[from] std::io::Error),
}

pub struct MySqlStream<TS>
where
    TS: AsyncRead + AsyncWrite + Unpin,
{
    stream: MaybeTlsStream<TcpStream, TS>,
    codec: PacketCodec,
    inbound_buffer: BytesMut,
    outbound_buffer: BytesMut,
}

impl<TS> MySqlStream<TS>
where
    TS: AsyncRead + AsyncWrite + Unpin,
{
    pub fn new(stream: MaybeTlsStream<TcpStream, TS>) -> Self {
        Self {
            stream,
            codec: PacketCodec::default(),
            inbound_buffer: BytesMut::new(),
            outbound_buffer: BytesMut::new(),
        }
    }

    pub fn push<'a, C, P: Encode<'a, C>>(
        &mut self,
        packet: &'a P,
        context: C,
    ) -> Result<(), MySqlStreamError> {
        let mut buf = vec![];
        packet.encode_with(&mut buf, context);
        self.codec.encode(&mut &*buf, &mut self.outbound_buffer)?;
        Ok(())
    }

    pub async fn flush(&mut self) -> std::io::Result<()> {
        trace!(outbound_buffer=?self.outbound_buffer, "sending");
        self.stream.write_all(&self.outbound_buffer[..]).await?;
        self.outbound_buffer = BytesMut::new();
        self.stream.flush().await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> Result<Option<Bytes>, MySqlStreamError> {
        let mut payload = BytesMut::new();
        loop {
            {
                let got_full_packet = self.codec.decode(&mut self.inbound_buffer, &mut payload)?;
                if got_full_packet {
                    trace!(?payload, "received");
                    return Ok(Some(payload.freeze()));
                }
            }
            let read_bytes = self.stream.read_buf(&mut self.inbound_buffer).await?;
            if read_bytes == 0 {
                return Ok(None);
            }
            trace!(inbound_buffer=?self.inbound_buffer, "received chunk");
        }
    }

    pub fn reset_sequence_id(&mut self) {
        self.codec.reset_seq_id();
    }

    pub async fn upgrade(
        mut self,
        config: <TcpStream as UpgradableStream<TS>>::UpgradeConfig,
    ) -> Result<Self, MaybeTlsStreamError> {
        self.stream = self.stream.upgrade(config).await?;
        Ok(self)
    }

    pub fn is_tls(&self) -> bool {
        match self.stream {
            MaybeTlsStream::Raw(_) => false,
            MaybeTlsStream::Tls(_) => true,
            MaybeTlsStream::Upgrading => false,
        }
    }

    pub fn into_inner(self) -> MaybeTlsStream<TcpStream, TS> {
        self.stream
    }
}

// Define the MyWs struct
struct MyWs(WebSocketStream<WsMaybeTlsStream<TcpStream>>);

// Implement Stream for MyWs
impl Stream for MyWs {
    type Item = Result<Message, WsError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let stream = self.get_mut();
        Pin::new(&mut stream.0).poll_next(cx)
    }
}

// Implement Sink for MyWs
impl Sink<Message> for MyWs {
    type Error = WsError;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let stream = self.get_mut();
        Pin::new(&mut stream.0).poll_ready(cx)
    }

    fn start_send(self: Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let stream = self.get_mut();
        Pin::new(&mut stream.0).start_send(item)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let stream = self.get_mut();
        Pin::new(&mut stream.0).poll_flush(cx)
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let stream = self.get_mut();
        Pin::new(&mut stream.0).poll_close(cx)
    }
}

// Implement UnderlyingStream for MyWs
impl UnderlyingStream<String, Result<Message, WsError>, WsError> for MyWs {
    fn establish(addr: String) -> Pin<Box<dyn Future<Output = Result<Self, WsError>> + Send>> {
        Box::pin(async move {
            // Connect to the WebSocket endpoint
            let ws_connection = connect_async(addr).await?.0;
            Ok(MyWs(ws_connection))
        })
    }

    fn is_write_disconnect_error(&self, err: &WsError) -> bool {
        matches!(
            err,
            WsError::ConnectionClosed
                | WsError::AlreadyClosed
                | WsError::Io(_)
                | WsError::Tls(_)
                | WsError::Protocol(_)
        )
    }

    fn is_read_disconnect_error(&self, item: &Result<Message, WsError>) -> bool {
        if let Err(e) = item {
            self.is_write_disconnect_error(e)
        } else {
            false
        }
    }

    fn exhaust_err() -> WsError {
        WsError::Io(io::Error::new(io::ErrorKind::Other, "Exhausted"))
    }
}

type ReconnectWs = ReconnectStream<MyWs, String, Result<Message, WsError>, WsError>;

#[tokio::main]
async fn main() {
    let mut ws_stream: ReconnectWs = ReconnectWs::connect(String::from("wss://localhost:8000")).await.unwrap();
    ws_stream.send(Message::text(String::from("hello world!"))).await.unwrap();
}
