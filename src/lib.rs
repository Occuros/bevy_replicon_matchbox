//! A simple transport intended only for examples.
//! This transport does not implement any reliability or security features.
//! DO NOT USE in a real project
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(feature = "client")]
mod client;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
pub use server::*;

use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::{ChannelConfig, PeerId};
use bevy_replicon::prelude::{Channel, RepliconChannels};

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
            group = group.add(RepliconMatchboxServerPlugin);
        }

        #[cfg(feature = "client")]
        {
            group = group.add(RepliconMatchboxClientPlugin);
        }

        group = group.add(RepliconMatchboxSharedPlugin);

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

struct RepliconMatchboxSharedPlugin;

impl Plugin for RepliconMatchboxSharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, cleanup_matchbox_socket_on_exit);
    }
}

fn cleanup_matchbox_socket_on_exit(
    mut exit_events: EventReader<AppExit>,
    mut server: Option<ResMut<MatchboxHost>>,
    mut client: Option<ResMut<MatchboxClient>>,
) {
    //seems not to work on all platforms
    for _ in exit_events.read() {
        if let Some(client) = &mut client {
            client.socket.close();
        }
        if let Some(server) = &mut server {
            error!("Closing matchbox socket");
            server.socket.close();
            server.client_entities.clear();
        }
    }
}

fn create_matchbox_socket(
    room_url: impl Into<String>,
    replicon_channels: &RepliconChannels,
) -> MatchboxSocket {
    let mut web_rtc_socket = bevy_matchbox::matchbox_socket::WebRtcSocketBuilder::new(room_url);
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

fn uuid_to_u64_truncated(peer_id: PeerId) -> u64 {
    let bytes = peer_id.0.as_bytes();
    u64::from_le_bytes(bytes[0..8].try_into().unwrap())
}
