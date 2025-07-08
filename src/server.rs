use bevy::prelude::*;
use bevy::tasks::futures_lite::io;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::Packet;
use bevy_matchbox::prelude::{PeerId, PeerState};
use bevy_replicon::bytes::{Bytes};
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use crate::{create_matchbox_socket,  uuid_to_u64_truncated};

#[derive(Event, Debug, Clone, Serialize, Deserialize)]
pub(super) struct OnHostDefinitionTrigger {
    pub host_peer_id: PeerId,
}

/// Adds a server messaging backend made for examples to `bevy_replicon`.
pub struct RepliconMatchboxServerPlugin;

impl Plugin for RepliconMatchboxServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                set_running.run_if(resource_added::<MatchboxHost>),
                receive_packets.run_if(resource_exists::<MatchboxHost>),
            )
                .chain()
                .in_set(ServerSet::ReceivePackets),
        );
        app.add_systems(
            PostUpdate,
            (
                set_stopped
                    .in_set(ServerSet::Send)
                    .run_if(resource_removed::<MatchboxHost>),
                send_packets
                    .in_set(ServerSet::SendPackets)
                    .run_if(resource_exists::<MatchboxHost>),
                update_client_presence
                    .in_set(ServerSet::SendPackets)
                    .run_if(resource_exists::<MatchboxHost>),
                received_disconnect
                    .in_set(ServerSet::SendPackets)
                    .run_if(resource_exists::<MatchboxHost>),
            ),
        );

        app.add_server_trigger::<OnHostDefinitionTrigger>(Channel::Ordered);
        app.make_trigger_independent::<OnHostDefinitionTrigger>();
    }
}

fn set_stopped(
    mut server: ResMut<RepliconServer>,
) {
    info!("server stopped");
    server.set_running(false);
}

fn set_running(mut server: ResMut<RepliconServer>) {
    server.set_running(true);
}

fn update_client_presence(mut commands: Commands, mut server: ResMut<MatchboxHost>) {
    let Some(local_peer) = server.socket.id() else {
        return;
    };

    for (peer, state) in server.socket.update_peers() {
        match state {
            PeerState::Connected => {
                if server.client_entities.contains_key(&peer) {
                    continue;
                }
                let network_id = NetworkId::new(uuid_to_u64_truncated(peer));
                let client_entity = commands
                    .spawn((
                        ConnectedClient { max_size: 1200 },
                        network_id,
                        MatchboxClientConnection { peer_id: peer },
                    ))
                    .id();
                debug!("new client {:?}: {}", network_id, client_entity);
                server.client_entities.insert(peer, client_entity);
                commands.server_trigger(ToClients {
                    mode: SendMode::Direct(client_entity),
                    event: OnHostDefinitionTrigger {
                        host_peer_id: local_peer,
                    },
                })
            }
            PeerState::Disconnected => {
                let Some(client_entity) = server.client_entities.remove(&peer) else {
                    continue;
                };
                debug!("client disconnected {:?}: {}", peer, client_entity);
                commands.entity(client_entity).despawn();
            }
        }
    }
}
fn receive_packets(
    mut replicon_server: ResMut<RepliconServer>,
    mut server: ResMut<MatchboxHost>,
    channels: Res<RepliconChannels>,
) {
    for (channel_id, _) in channels.client_channels().iter().enumerate() {
        let socket_channel_id = channels.server_channels().len() + channel_id;
        for (id, packet) in server.socket.channel_mut(socket_channel_id).receive() {
            let Some(client_entity) = server.client_entities.get(&id) else {
                continue;
            };

            replicon_server.insert_received(*client_entity, channel_id, Bytes::from(packet));
        }
    }

}

fn send_packets(
    mut replicon_server: ResMut<RepliconServer>,
    mut server: ResMut<MatchboxHost>,
    clients: Query<&MatchboxClientConnection>,
) {
    for (client_entity, channel_id, message) in replicon_server.drain_sent() {
        let Ok(connection) = clients.get(client_entity) else {
            debug!("client {} not connected", client_entity);
            continue;
        };

        let packet: Packet = message.as_ref().into();
        server
            .socket
            .channel_mut(channel_id)
            .send(packet, connection.peer_id);
    }
}

fn received_disconnect(
    mut disconnect_events: EventReader<DisconnectRequest>,
    mut commands: Commands,
    mut server: ResMut<MatchboxHost>,

    client_connections: Query<&MatchboxClientConnection>,
) {
    for event in disconnect_events.read() {
        let Ok(connection) = client_connections.get(event.client_entity) else {
            continue;
        };
        debug!("disconnecting client `{}` by request", event.client_entity);
        server.client_entities.remove(&connection.peer_id);
        commands.entity(event.client_entity).despawn();
    }
}

// The socket used by the server.
#[derive(Resource)]
pub struct MatchboxHost {
    pub socket: MatchboxSocket,
    pub client_entities: HashMap<PeerId, Entity>,
}

impl MatchboxHost {
    pub fn new(
        room_url: impl Into<String>,
        replicon_channels: &RepliconChannels,
    ) -> io::Result<Self> {
        let reliable_socket = create_matchbox_socket(room_url, replicon_channels);

        Ok(Self {
            socket: reliable_socket,
            // unreliable_socket,
            client_entities: HashMap::new(),
        })
    }
}

#[derive(Component)]
struct MatchboxClientConnection {
    pub peer_id: PeerId,
}
