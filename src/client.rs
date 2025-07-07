use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpStream},
    time::Instant,
};
use std::ops::Deref;
use crate::matchbox_package_service::{create_matchbox_socket, create_package, read_package};
use crate::server::{OnHostDefinitionTrigger, CHANNEL_ID};
use bevy::prelude::*;
use bevy_matchbox::MatchboxSocket;
use bevy_matchbox::matchbox_socket::PeerId;
use bevy_replicon::bytes::{BufMut, Bytes, BytesMut};
use bevy_replicon::prelude::*;

/// Adds a client messaging backend made for examples to `bevy_replicon`.
pub struct RepliconExampleClientPlugin;

impl Plugin for RepliconExampleClientPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PreUpdate,
            (
                set_connected.run_if(resource_added::<ExampleClient>),
                receive_packets.run_if(resource_exists::<ExampleClient>),
            )
                .chain()
                .in_set(ClientSet::ReceivePackets),
        );

        app.add_systems(
            PostUpdate,
            (
                set_disconnected
                    .in_set(ClientSet::PrepareSend)
                    .run_if(resource_removed::<ExampleClient>),
                send_packets
                    .in_set(ClientSet::SendPackets)
                    .run_if(resource_exists::<ExampleClient>),
            ),
        );

        app.add_observer(on_connected_to_host);

    }
}

fn set_disconnected(mut replicon_client: ResMut<RepliconClient>) {
    replicon_client.set_status(RepliconClientStatus::Disconnected);
}

fn set_connected(mut replicon_client: ResMut<RepliconClient>) {
    replicon_client.set_status(RepliconClientStatus::Connected);
}

fn on_connected_to_host(trigger: Trigger<OnHostDefinitionTrigger>, mut client: ResMut<ExampleClient>) {
    info!("connected to host {}", trigger.host_peer_id);
    client.host_peer_id = Some(trigger.host_peer_id);
}

fn receive_packets(mut client: ResMut<ExampleClient>, mut replicon_client: ResMut<RepliconClient>) {
    for (id, packet) in client.matchbox_socket.channel_mut(CHANNEL_ID).receive() {
        if client.host_peer_id.is_none() {
            client.host_peer_id = Some(id);
        }

        let Ok((channel_id, message)) = read_package(packet) else {
            error!("error reading package from {id}");
            continue;
        };
        replicon_client.insert_received(channel_id, message);
    }
}

struct SendQueueElement {
    channel_id: usize,
    message: Bytes,
}
fn send_packets(
    mut client: ResMut<ExampleClient>,
    mut replicon_client: ResMut<RepliconClient>,
) {
    let  Some(host_peer_id) = client.host_peer_id else {
        return;
    };
    for (channel_id, message) in replicon_client.drain_sent() {
        let Ok(package) = create_package(channel_id, message) else {
            error!("error creating package");
            continue;
        };
        client
            .matchbox_socket
            .channel_mut(CHANNEL_ID)
            .send(package, host_peer_id);
    }
}

#[derive(Resource)]
pub struct ExampleClient {
    pub matchbox_socket: MatchboxSocket,
    pub host_peer_id: Option<PeerId>,
}



impl ExampleClient {
    pub fn new(port: u16) -> io::Result<Self> {
        let socket = create_matchbox_socket();
        Ok(Self {
            matchbox_socket: socket,
            host_peer_id: None,
        })
    }

    pub fn is_connected(&self) -> bool {
        true
    }
}
