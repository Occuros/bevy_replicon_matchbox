use crate::{SYSTEM_CHANNEL_ID, add_marker, create_matchbox_socket, strip_marker, SystemChannelMessage, from_packet};
use bevy::prelude::*;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
use bevy_replicon::prelude::*;
use std::io;
use bevy_matchbox::prelude::PeerState;

/// Adds a client messaging backend made for examples to `bevy_replicon`.
pub(super) struct RepliconMatchboxClientPlugin;

impl Plugin for RepliconMatchboxClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                receive_packets.run_if(resource_exists::<MatchboxClient>),
                receive_system_channel_packets.run_if(resource_exists::<MatchboxClient>),
                update_peers.run_if(resource_exists::<MatchboxClient>),

            )
                .chain()
                .in_set(ClientSet::ReceivePackets),
        );

        app.add_systems(
            PostUpdate,
            (
                set_disconnected
                    .in_set(ClientSet::Send)
                    .run_if(resource_removed::<MatchboxClient>),
                send_packets
                    .in_set(ClientSet::SendPackets)
                    .run_if(not(no_host_defined).and(resource_exists::<MatchboxClient>)),
            ),
        );
    }
}

fn no_host_defined(client: Option<Res<MatchboxClient>>) -> bool {
    if let Some(client) = client {
        return client.host_peer_id.is_none();
    }
    true
}

fn set_disconnected(mut replicon_client: ResMut<RepliconClient>) {
    replicon_client.set_status(RepliconClientStatus::Disconnected);
}


fn update_peers(mut client: ResMut<MatchboxClient>, mut commands: Commands) {
    let Ok(peers) = client.socket.try_update_peers() else {
        commands.remove_resource::<MatchboxClient>();
        return;
    };

    let Some(host_peer_id) = client.host_peer_id else {
        return;
    };
    for (peer_id, state) in peers {
        if matches!(state, PeerState::Disconnected) && peer_id != host_peer_id {
            trace!("host {} disconnected", peer_id);
            commands.remove_resource::<MatchboxClient>();
            return;
        }
    }
}


fn receive_system_channel_packets(
    mut commands: Commands,
    mut client: ResMut<MatchboxClient>, mut replicon_client: ResMut<RepliconClient>) {
    if client.socket.all_channels_closed() {
        trace!("matchbox socket was closed");
        return;
    }
    let Ok(channel) = client.socket.get_channel_mut(SYSTEM_CHANNEL_ID) else {
        error!("system channel not found!");
        return;
    };
    for (peer_id, packet) in channel.receive() {
        let Ok(message )= from_packet(&packet) else {
            error!("failed to deserialize system message {}", packet.len());
            continue;
        };
        trace!("client received system message {:?} from peer {}",message, peer_id);

        match message {
            SystemChannelMessage::ConnectedToHost => {
                client.host_peer_id = Some(peer_id);
                replicon_client.set_status(RepliconClientStatus::Connected);
            }
            SystemChannelMessage::Disconnect => {
                info!("disconnected by server");
                commands.remove_resource::<MatchboxClient>();
            }
        }


    }
}

fn receive_packets(
    mut client: ResMut<MatchboxClient>,
    mut replicon_client: ResMut<RepliconClient>,
    channels: Res<RepliconChannels>,
) {
    if client.socket.all_channels_closed() {
        trace!("matchbox socket was closed");
        return;
    }

    for (channel_id, _) in channels.server_channels().iter().enumerate() {
        //server socket channels are the same as the channel id +1 for the system channel
        let socket_channel_id = 1 + channel_id;
        let Ok(channel) = client.socket.get_channel_mut(socket_channel_id) else {
            continue;
        };
        for (id, packet) in channel.receive() {
            trace!(
                "client received packet from peer {}, c:{} size {}",
                id,
                channel_id,
                packet.len()
            );
            replicon_client.insert_received(channel_id, strip_marker(packet.as_ref()));
        }
    }

}

fn send_packets(
    mut client: ResMut<MatchboxClient>,
    mut replicon_client: ResMut<RepliconClient>,
    channels: Res<RepliconChannels>,
) {
    if client.socket.any_channel_closed() {
        trace!("matchbox socket was closed");
        return;
    }

    let Some(host_peer_id) = client.host_peer_id else {
        error!("set connected before host was defined");
        return;
    };
    for (channel_id, message) in replicon_client.drain_sent() {
        //client socket channels are offset by the server channel length + 1 for the system channel
        let socket_channel_id = 1 + channels.server_channels().len() + channel_id;
        client
            .socket
            .channel_mut(socket_channel_id)
            .send(add_marker(message.as_ref()), host_peer_id);
    }
}

#[derive(Resource)]
pub struct MatchboxClient {
    pub socket: MatchboxSocket,
    pub host_peer_id: Option<PeerId>,
}

impl MatchboxClient {
    pub fn new(
        room_url: impl Into<String>,
        replicon_channels: &RepliconChannels,
    ) -> io::Result<Self> {
        let socket = create_matchbox_socket(room_url, replicon_channels);
        Ok(Self {
            socket,
            host_peer_id: None,
        })
    }

    pub fn is_connected(&self) -> bool {
        self.host_peer_id.is_some()
    }

    pub fn disconnect(&mut self) {
        self.socket.close();
        self.host_peer_id = None;
    }
}
