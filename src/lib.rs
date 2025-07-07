//! A simple transport intended only for examples.
//! This transport does not implement any reliability or security features.
//! DO NOT USE in a real project
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

#[cfg(feature = "client")]
mod client;
mod matchbox_package_service;
#[cfg(feature = "server")]
mod server;

#[cfg(feature = "client")]
pub use client::*;
#[cfg(feature = "server")]
pub use server::*;

use bevy::asset::AsyncWriteExt;
use bevy::{app::PluginGroupBuilder, prelude::*};
use bevy_replicon::prelude::{Channel, ServerTriggerAppExt};

/// Plugin group for all replicon example backend plugins.
///
/// Contains the following:
/// * [`RepliconExampleServerPlugin`] - with feature `server`.
/// * [`RepliconExampleClientPlugin`] - with feature `client`.
pub struct RepliconExampleBackendPlugins;

impl PluginGroup for RepliconExampleBackendPlugins {
    fn build(self) -> PluginGroupBuilder {
        let mut group = PluginGroupBuilder::start::<Self>();

        #[cfg(feature = "server")]
        {
            group = group.add(RepliconExampleServerPlugin);
        }

        #[cfg(feature = "client")]
        {
            group = group.add(RepliconExampleClientPlugin);
        }

        group = group.add(RepliconExampleBackendSharedPlugin);

        group
    }
}

pub struct RepliconExampleBackendSharedPlugin;

impl Plugin for RepliconExampleBackendSharedPlugin {
    fn build(&self, app: &mut App) {
        app.add_server_trigger::<OnHostDefinitionTrigger>(Channel::Ordered);
        app.make_trigger_independent::<OnHostDefinitionTrigger>();

        app.add_systems(PreUpdate, cleanup_matchbox_socket_on_exit);
        app.add_systems(PostUpdate, cleanup_matchbox_socket_on_exit);


    }
}

fn cleanup_matchbox_socket_on_exit(
    mut exit_events: EventReader<AppExit>,
    mut server: Option<ResMut<ExampleServer>>,
    mut client: Option<ResMut<ExampleClient>>,
) {
    for _ in exit_events.read() {
        if let Some(client) = &mut client {
            client.matchbox_socket.close();
        }
        if let Some(server) = &mut server {
            server.reliable_socket.close();
            server.client_entities.clear();
        }
    }
}
