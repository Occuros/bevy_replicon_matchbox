use crate::server::{OnDisconnectClient, OnHostDefinitionTrigger};
use bevy::prelude::*;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
use bevy_replicon::bytes::{Bytes};
use bevy_replicon::prelude::*;
use std::io;
use crate::{create_matchbox_socket};

/// Adds a client messaging backend made for examples to `bevy_replicon`.
pub(super) struct RepliconMatchboxClientPlugin;

impl Plugin for RepliconMatchboxClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                update_peers.run_if(resource_exists::<MatchboxClient>),
                disconnect_client.run_if(resource_exists::<MatchboxClient>),
                set_connected.run_if(resource_added::<MatchboxClient>),
                receive_packets.run_if(resource_exists::<MatchboxClient>),
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
                    .run_if(resource_exists::<MatchboxClient>),
            ),
        );

        app.add_observer(on_connected_to_host);
        app.add_observer(on_disconnected_by_server);
    }
}

fn set_disconnected(mut replicon_client: ResMut<RepliconClient>) {
    replicon_client.set_status(RepliconClientStatus::Disconnected);
}

fn set_connected(mut replicon_client: ResMut<RepliconClient>) {
    replicon_client.set_status(RepliconClientStatus::Connected);
}

fn update_peers(mut client: ResMut<MatchboxClient>, mut commands: Commands) {
    if client.socket.try_update_peers().is_err() {
        commands.remove_resource::<MatchboxClient>();
    }
}

fn on_disconnected_by_server(_: Trigger<OnDisconnectClient>, mut client: ResMut<MatchboxClient>) {
    info!("disconnected by server");
    client.disconnect();
}

fn on_connected_to_host(
    trigger: Trigger<OnHostDefinitionTrigger>,
    mut client: ResMut<MatchboxClient>,
) {
    info!("connected to host {}", trigger.host_peer_id);
    client.host_peer_id = Some(trigger.host_peer_id);
}

fn disconnect_client(client: Res<MatchboxClient>, mut commands: Commands) {
    if client.socket.all_channels_closed() {
        info!("matchbox socket was closed, removing client");
        commands.remove_resource::<MatchboxClient>();
    }
}

fn receive_packets(
    mut client: ResMut<MatchboxClient>,
    mut replicon_client: ResMut<RepliconClient>,
    channels: Res<RepliconChannels>,
) {
    if client.socket.any_channel_closed() {
        debug!("matchbox socket was closed");
        return;
    }
    for (channel_id, _) in channels.server_channels().iter().enumerate() {
        let socket_channel_id = channel_id; //server socket channels are the same as the channel id
        for (id, packet) in client.socket.channel_mut(socket_channel_id).receive() {
            debug!("client received packet from peer {}", id);
            replicon_client.insert_received(channel_id, Bytes::from(packet));
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
        return;
    };
    for (channel_id, message) in replicon_client.drain_sent() {
        //client socket channels are offset by the server channel length
        let socket_channel_id = channels.server_channels().len() + channel_id;
        client
            .socket
            .channel_mut(socket_channel_id)
            .send(message.as_ref().into(), host_peer_id);
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
        !self.socket.connected_peers().count() > 1
    }

    pub fn disconnect(&mut self) {
        self.socket.close();
        self.host_peer_id = None;
    }
}
