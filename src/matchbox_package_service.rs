use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::Packet;
use bevy_matchbox::prelude::PeerId;
use bevy_replicon::bytes::{Buf, Bytes};
use std::{
    error::Error,
    io::{self},
};

pub(super) fn read_package(packet: Packet) -> io::Result<(u8, Bytes)> {
    if packet.len() < 3 {
        return Err(std::io::ErrorKind::UnexpectedEof.into());
    }

    let bytes = Bytes::from(packet);

    let channel_id = bytes[0];
    let size_bytes = [bytes[1], bytes[2]];
    let message_size = u16::from_le_bytes(size_bytes) as usize;

    let expected_total_size = 3 + message_size;
    if bytes.len() < expected_total_size {
        return Err(std::io::ErrorKind::UnexpectedEof.into());
    }

    let message = bytes.slice(3..expected_total_size); // Zero-copy

    Ok((channel_id, message))
}

pub(super) fn create_package(
    channel_id: usize,
    message: Bytes,
) -> Result<Packet, Box<dyn Error + Send + Sync>> {
    let message_size: u16 = message.len().try_into()?;
    let channel_id: u8 = channel_id.try_into()?;

    let mut buffer = Vec::with_capacity(1 + 2 + message.len());
    buffer.push(channel_id);
    buffer.extend_from_slice(&message_size.to_le_bytes());
    buffer.extend_from_slice(message.as_ref());

    Ok(buffer.into_boxed_slice())
}

pub fn uuid_to_u64_truncated(peer_id: PeerId) -> u64 {
    let bytes = peer_id.0.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().unwrap())
}

pub(super) fn create_matchbox_socket() -> MatchboxSocket {
    let room = "ws://localhost:3536/hello";
    let web_rtc_socket = bevy_matchbox::matchbox_socket::WebRtcSocketBuilder::new(room)
        .add_reliable_channel() //channel 0 system channel(
        .add_reliable_channel() //channel 1 normal messages
        .build();
    MatchboxSocket::from(web_rtc_socket)
}
