use seed_core::codec::{encode_event_for_signing, SignableEvent};
use seed_core::event::EventKind;
use seed_core::identity::{DeviceId, IdentityId};
use seed_core::space::SpaceId;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

const RELAY_TARGET: u8 = 7;

fn canonical_event() -> [u8; 119] {
    let event = SignableEvent {
        space: SpaceId::from_bytes([0x22; 32]),
        author: IdentityId::from_bytes([0x33; 32]),
        device: DeviceId::from_bytes([0x44; 32]),
        kind: EventKind::MESSAGE,
        payload: b"hello",
    };

    let mut output = [0u8; 119];
    let written = encode_event_for_signing(&event, &mut output).expect("fixed Event must encode");
    assert_eq!(written, output.len());
    output
}

fn write_frame(stream: &mut TcpStream, frame: &[u8]) -> io::Result<()> {
    let len = u32::try_from(frame.len())
        .map_err(|_| io::Error::new(ErrorKind::InvalidInput, "frame exceeds u32 length"))?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(frame)?;
    stream.flush()
}

fn read_frame(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes)?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    let mut frame = vec![0u8; len];
    stream.read_exact(&mut frame)?;
    Ok(frame)
}

fn direct_round_trip(frame: &[u8]) -> io::Result<Vec<u8>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;
    let server = thread::spawn(move || -> io::Result<Vec<u8>> {
        let (mut stream, _) = listener.accept()?;
        read_frame(&mut stream)
    });

    let mut client = TcpStream::connect(address)?;
    write_frame(&mut client, frame)?;

    server
        .join()
        .map_err(|_| io::Error::other("direct server thread panicked"))?
}

fn spawn_relay() -> io::Result<(SocketAddr, thread::JoinHandle<io::Result<()>>)> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let address = listener.local_addr()?;

    let handle = thread::spawn(move || -> io::Result<()> {
        let (mut sender, _) = listener.accept()?;

        let mut op = [0u8; 2];
        sender.read_exact(&mut op)?;
        if op != [b'P', RELAY_TARGET] {
            return Err(io::Error::new(ErrorKind::InvalidData, "invalid relay PUT"));
        }
        let stored = read_frame(&mut sender)?;

        let (mut receiver, _) = listener.accept()?;
        receiver.read_exact(&mut op)?;
        if op != [b'G', RELAY_TARGET] {
            return Err(io::Error::new(ErrorKind::InvalidData, "invalid relay GET"));
        }
        write_frame(&mut receiver, &stored)
    });

    Ok((address, handle))
}

fn relay_round_trip(frame: &[u8]) -> io::Result<Vec<u8>> {
    let (address, relay) = spawn_relay()?;

    {
        let mut sender = TcpStream::connect(address)?;
        sender.write_all(&[b'P', RELAY_TARGET])?;
        write_frame(&mut sender, frame)?;
    }

    let received = {
        let mut receiver = TcpStream::connect(address)?;
        receiver.write_all(&[b'G', RELAY_TARGET])?;
        receiver.flush()?;
        read_frame(&mut receiver)?
    };

    relay
        .join()
        .map_err(|_| io::Error::other("relay thread panicked"))??;
    Ok(received)
}

fn exercise_transport() -> io::Result<()> {
    let frame = canonical_event();

    let direct = direct_round_trip(&frame)?;
    assert_eq!(direct, frame);

    let relay = relay_round_trip(&frame)?;
    assert_eq!(relay, frame);

    Ok(())
}

fn main() -> io::Result<()> {
    exercise_transport()?;
    println!("Seed Phase 0 transport spike");
    println!("direct_tcp=ok");
    println!("relay_mailbox=ok");
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn direct_and_relay_deliver_same_envelope() {
        super::exercise_transport().unwrap();
    }
}
