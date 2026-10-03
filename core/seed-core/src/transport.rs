use std::{
    io::{self, Read, Write},
    net::{SocketAddr, TcpStream},
};

const MAX_FRAME_BYTES: usize = 17 * 1024 * 1024 + 256;

/// Raw byte-frame transport boundary.
///
/// This interface deliberately does not claim authentication or encryption.
/// Secure session establishment and end-to-end encryption must sit above this
/// layer and may use Direct TCP, Relay, or another transport implementation.
pub trait FrameTransport {
    fn send_frame(&mut self, payload: &[u8]) -> Result<(), TransportError>;
    fn recv_frame(&mut self) -> Result<Vec<u8>, TransportError>;
}

#[derive(Debug)]
pub enum TransportError {
    Io(io::Error),
    Closed,
    TruncatedFrame,
    FrameTooLarge,
}

impl From<io::Error> for TransportError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Debug)]
pub struct TcpFrameTransport {
    stream: TcpStream,
}

impl TcpFrameTransport {
    pub fn connect(address: SocketAddr) -> Result<Self, TransportError> {
        let stream = TcpStream::connect(address)?;
        Ok(Self { stream })
    }

    pub fn from_stream(stream: TcpStream) -> Self {
        Self { stream }
    }

    pub fn peer_addr(&self) -> Result<SocketAddr, TransportError> {
        Ok(self.stream.peer_addr()?)
    }

    pub fn set_nodelay(&self, enabled: bool) -> Result<(), TransportError> {
        self.stream.set_nodelay(enabled)?;
        Ok(())
    }
}

impl FrameTransport for TcpFrameTransport {
    fn send_frame(&mut self, payload: &[u8]) -> Result<(), TransportError> {
        if payload.len() > MAX_FRAME_BYTES {
            return Err(TransportError::FrameTooLarge);
        }

        let len = u32::try_from(payload.len()).map_err(|_| TransportError::FrameTooLarge)?;
        self.stream.write_all(&len.to_be_bytes())?;
        self.stream.write_all(payload)?;
        self.stream.flush()?;
        Ok(())
    }

    fn recv_frame(&mut self) -> Result<Vec<u8>, TransportError> {
        let Some(len) = read_frame_len(&mut self.stream)? else {
            return Err(TransportError::Closed);
        };

        let len = len as usize;
        if len > MAX_FRAME_BYTES {
            return Err(TransportError::FrameTooLarge);
        }

        let mut payload = vec![0u8; len];
        self.stream
            .read_exact(&mut payload)
            .map_err(map_frame_read_error)?;
        Ok(payload)
    }
}

fn read_frame_len(stream: &mut TcpStream) -> Result<Option<u32>, TransportError> {
    let mut bytes = [0u8; 4];
    let read = stream.read(&mut bytes)?;

    if read == 0 {
        return Ok(None);
    }

    if read < bytes.len() {
        stream
            .read_exact(&mut bytes[read..])
            .map_err(map_frame_read_error)?;
    }

    Ok(Some(u32::from_be_bytes(bytes)))
}

fn map_frame_read_error(error: io::Error) -> TransportError {
    match error.kind() {
        io::ErrorKind::UnexpectedEof => TransportError::TruncatedFrame,
        _ => TransportError::Io(error),
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    #[test]
    fn loopback_frames_round_trip() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let mut client = TcpFrameTransport::connect(address).unwrap();
        let (server_stream, _) = listener.accept().unwrap();
        let mut server = TcpFrameTransport::from_stream(server_stream);

        client.send_frame(b"seed-ping").unwrap();
        assert_eq!(server.recv_frame().unwrap(), b"seed-ping");

        server.send_frame(b"seed-pong").unwrap();
        assert_eq!(client.recv_frame().unwrap(), b"seed-pong");
    }

    #[test]
    fn clean_peer_close_is_distinguished_from_truncation() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let client = TcpStream::connect(address).unwrap();
        let (server_stream, _) = listener.accept().unwrap();
        drop(client);

        let mut server = TcpFrameTransport::from_stream(server_stream);
        assert!(matches!(server.recv_frame(), Err(TransportError::Closed)));
    }

    #[test]
    fn partial_length_prefix_is_rejected() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();

        let mut client = TcpStream::connect(address).unwrap();
        let (server_stream, _) = listener.accept().unwrap();
        client.write_all(&[0, 1]).unwrap();
        drop(client);

        let mut server = TcpFrameTransport::from_stream(server_stream);
        assert!(matches!(
            server.recv_frame(),
            Err(TransportError::TruncatedFrame)
        ));
    }
}
