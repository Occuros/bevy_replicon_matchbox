use bevy::app::{PluginGroup, PluginGroupBuilder};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{ChannelConfig, Packet};
use bevy_replicon::postcard;
use bevy_replicon::prelude::{Channel, RepliconChannels};
use bytes::Bytes;
use serde::{Deserialize, Serialize};

//Required to communicate which peer is the host before we start using replicon
pub(super) const SYSTEM_CHANNEL_ID: usize = 0;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub(super) enum SystemChannelMessage {
    ConnectedToHost,
    Disconnect,
}

/// Plugin group for all replicon example backend plugins.
///
/// Contains the following:
/// * [`RepliconMatchboxServerPlugin`] - with feature `server`.
/// * [`RepliconMatchboxClientPlugin`] - with feature `client`.
pub struct RepliconMatchboxPlugins;

impl PluginGroup for RepliconMatchboxPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        #[cfg(feature = "server")]
        {
            use crate::server::RepliconMatchboxServerPlugin;
            group = group.add(RepliconMatchboxServerPlugin);
        }

        #[cfg(feature = "client")]
        {
            use crate::client::RepliconMatchboxClientPlugin;
            group = group.add(RepliconMatchboxClientPlugin);
        }

        group
    }
}
pub(crate) trait RepliconChannelsExt<'a> {
    type Iter: Iterator<Item = &'a Channel>;

    fn all_channels(&'a self) -> Self::Iter;
}

impl<'a> RepliconChannelsExt<'a> for RepliconChannels {
    type Iter = std::iter::Chain<std::slice::Iter<'a, Channel>, std::slice::Iter<'a, Channel>>;
    fn all_channels(&'a self) -> Self::Iter {
        self.server_channels()
            .iter()
            .chain(self.client_channels().iter())
    }
}

pub(super) fn create_matchbox_socket(
    room_url: impl Into<String>,
    replicon_channels: &RepliconChannels,
) -> MatchboxSocket {
    let mut web_rtc_socket = bevy_matchbox::matchbox_socket::WebRtcSocketBuilder::new(room_url);
    //add system channel
    web_rtc_socket = web_rtc_socket.add_reliable_channel();
    for &channel in replicon_channels.all_channels() {
        match channel {
            Channel::Unreliable => {
                web_rtc_socket = web_rtc_socket.add_unreliable_channel();
            }
            Channel::Unordered => {
                web_rtc_socket = web_rtc_socket.add_channel(ChannelConfig {
                    ordered: false,
                    max_retransmits: None,
                });
            }
            Channel::Ordered => {
                web_rtc_socket = web_rtc_socket.add_reliable_channel();
            }
        };
    }
    let socket = web_rtc_socket.build();
    MatchboxSocket::from(socket)
}

#[cfg(feature = "server")]
use bevy_matchbox::matchbox_socket::PeerId;
#[cfg(feature = "server")]
pub(super) fn uuid_to_u64_truncated(peer_id: PeerId) -> u64 {
    let bytes = peer_id.0.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().unwrap())
}

///Marker added as matchbox seems to drop 0 sized packages
pub(super) fn add_marker(data: &[u8]) -> Packet {
    let mut payload = Vec::with_capacity(data.len() + 1);
    payload.push(0);
    payload.extend_from_slice(data);
    payload.into()
}

///Marker stripped as matchbox seems to drop 0 sized packages
pub(super) fn strip_marker(packet: &[u8]) -> Bytes {
    Bytes::copy_from_slice(&packet[1..])
}

#[cfg(feature = "server")]
pub(super) fn to_packet<'a, T: Serialize>(msg: &T, buf: &'a mut [u8]) -> &'a [u8] {
    use bevy_replicon::postcard::to_slice;
    to_slice(msg, buf).expect("serialize failed")
}

#[cfg(feature = "client")]
pub(super) fn from_packet<'a, T: Deserialize<'a>>(
    data: &'a [u8],
) -> bevy::prelude::Result<T, postcard::Error> {
    postcard::from_bytes(data)
}

#[test]
fn test_packaging() {
    let messages = [
        SystemChannelMessage::ConnectedToHost,
        SystemChannelMessage::Disconnect,
    ];
    for msg in messages.iter() {
        let mut buf = [0u8; 1];
        let p = to_packet(&msg, &mut buf);
        let deserialized: SystemChannelMessage = from_packet(p).unwrap();
        assert_eq!(*msg, deserialized);
    }
}
