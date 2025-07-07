
use bevy::prelude::*;
use bevy::tasks::futures_lite::io;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::prelude::{PeerId, PeerState};
use bevy_replicon::bytes::{Bytes, BytesMut};
use bevy_replicon::prelude::*;
use bevy_replicon::shared::backend::connected_client::NetworkId;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;

pub(crate) const SYSTEM_CHANNEL_ID: usize = 0;
pub(crate) const CHANNEL_ID: usize = 1;

use crate::matchbox_package_service::{
    create_matchbox_socket, create_package, read_package, uuid_to_u64_truncated,
};

#[derive(Event, Debug, Clone, Serialize, Deserialize)]
pub(super) struct OnHostDefinitionTrigger {
    pub host_peer_id: PeerId,
}

/// Adds a server messaging backend made for examples to `bevy_replicon`.
pub struct RepliconExampleServerPlugin;

impl Plugin for RepliconExampleServerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                set_running.run_if(resource_added::<ExampleServer>),
                receive_packets.run_if(resource_exists::<ExampleServer>),
            )
                .chain()
                .in_set(ServerSet::ReceivePackets),
        );
        app.add_systems(
            PostUpdate,
            (
                set_stopped
                    .in_set(ServerSet::PrepareSend)
                    .run_if(resource_removed::<ExampleServer>),
                send_packets
                    .in_set(ServerSet::SendPackets)
                    .run_if(resource_exists::<ExampleServer>),
            ),
        );
    }
}

fn set_stopped(
    mut commands: Commands,
    mut server: ResMut<RepliconServer>,
    mut example_server: ResMut<ExampleServer>,
) {
    info!("server stopped");
    example_server.reliable_socket.close();
    commands.remove_resource::<ExampleServer>();
    server.set_running(false);
}

fn set_running(mut server: ResMut<RepliconServer>) {
    server.set_running(true);
}


fn receive_packets(
    mut commands: Commands,
    mut replicon_server: ResMut<RepliconServer>,
    mut server: ResMut<ExampleServer>,
) {
    let Some(local_peer) = server.reliable_socket.id() else {
        return;
    };
    for (peer, state) in server.reliable_socket.update_peers() {
        // if peer == local_peer { continue; }
        if matches!(state, PeerState::Connected) && !server.client_entities.contains_key(&peer) {
            let network_id = NetworkId::new(uuid_to_u64_truncated(peer));
            let client_entity = commands
                .spawn((
                    ConnectedClient { max_size: 1200 },
                    network_id,
                    ExampleClientConnection { peer_id: peer },
                ))
                .id();
            info!("new client {:?}: {}", network_id, client_entity);
            server.client_entities.insert(peer, client_entity);
            commands.server_trigger(ToClients {
                mode: SendMode::Direct(client_entity),
                event: OnHostDefinitionTrigger {
                    host_peer_id: local_peer,
                },
            })
        }
    }

    for (id, packet) in server.reliable_socket.channel_mut(CHANNEL_ID).receive() {
        let Some(client_entity) = server.client_entities.get(&id) else {
            continue;
        };
        let Ok((channel_id, message)) = read_package(packet) else {
            error!("failed to read package");
            continue;
        };

        replicon_server.insert_received(*client_entity, channel_id, message);
    }
}

fn send_packets(
    mut commands: Commands,
    mut disconnect_events: EventReader<DisconnectRequest>,
    mut replicon_server: ResMut<RepliconServer>,
    mut server: ResMut<ExampleServer>,
    clients: Query<&ExampleClientConnection>,
) {
    for (client_entity, channel_id, message) in replicon_server.drain_sent() {

        let Ok(connection) = clients.get(client_entity) else {
            info!("client {} not connected", client_entity);
            continue;
        };

        let Ok(package) = create_package(channel_id, message) else {
            error!("failed to create package");
            continue;
        };
        server
            .reliable_socket
            .channel_mut(CHANNEL_ID)
            .send(package, connection.peer_id);
    }
    for event in disconnect_events.read() {
        debug!("disconnecting client `{}` by request", event.client);
        commands.entity(event.client).despawn();
    }
}

// The socket used by the server.
#[derive(Resource)]
pub struct ExampleServer {
    pub reliable_socket: MatchboxSocket,
    // unreliable_socket: MatchboxSocket,
    // host_peer_id: PeerId,
    pub client_entities: HashMap<PeerId, Entity>,
}

impl ExampleServer {
    /// Opens an example server socket on the specified port.
    pub fn new(port: u16) -> io::Result<Self> {
        let reliable_socket = create_matchbox_socket();

        Ok(Self {
            reliable_socket,
            // unreliable_socket,
            client_entities: HashMap::new(),
        })
    }
}

// A connected for a client.
#[derive(Component)]
struct ExampleClientConnection {
    pub peer_id: PeerId,
}
