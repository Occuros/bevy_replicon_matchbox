
use std::net::{Ipv4Addr, SocketAddrV4};
use bevy::log::{Level, LogPlugin};
use bevy::prelude::*;
use bevy_replicon::prelude::*;
use serde::{Deserialize, Serialize};
use test_log::test;
use bevy_replicon_matchbox_backend::{MatchboxClient, MatchboxHost, RepliconMatchboxPlugins};

#[test]
fn connect_disconnect() {
        let mut server_app = App::new();
        let mut client_app = App::new();
        for app in [&mut server_app, &mut client_app] {
            app.add_plugins((
                MinimalPlugins,
                RepliconPlugins.set(ServerPlugin {
                    tick_policy: TickPolicy::EveryFrame,
                    ..Default::default()
                }),
                RepliconMatchboxPlugins,
            ))
                .finish();
        }

        setup(&mut server_app, &mut client_app);
        assert!(server_app.world().resource::<RepliconServer>().is_running());

        let matchbox_server = server_app.world().resource::<MatchboxHost>();
        let connected_clients = matchbox_server.connected_clients();
        info!("connected clients: {}", connected_clients);
        assert_eq!(connected_clients, 1, "one client connected expected but got {}", connected_clients);




    client_app.world_mut().resource_mut::<MatchboxClient>().disconnect();

    let mut clients = server_app
        .world_mut()
        .query::<&ConnectedClient>();
    assert_eq!(clients.iter(server_app.world()).len(), 1);

    let replicon_client = client_app.world().resource::<RepliconClient>();
    assert!(replicon_client.is_connected());

    let mut matchbox_client = client_app.world_mut().resource_mut::<MatchboxClient>();
    assert!(matchbox_client.is_connected());

    matchbox_client.disconnect();

    client_app.update();
    server_app.update();

    info!("connected clients: {}", clients.iter(server_app.world()).len());

    assert_eq!(clients.iter(server_app.world()).len(), 0);

    let matchbox_server = server_app.world().resource::<MatchboxHost>();
    info!("connected clients: {}", matchbox_server.connected_clients());

    assert_eq!(matchbox_server.connected_clients(), 0);

    let replicon_client = client_app.world().resource::<RepliconClient>();
    assert!(replicon_client.is_disconnected());


}

#[test]
fn disconnect_request() {
    let mut server_app = App::new();
    let mut client_app = App::new();

    for app in [&mut server_app, &mut client_app] {
        let log_plugin = LogPlugin {
            level: Level::INFO,
            filter: "bevy_replicon_matchbox=debug,wgpu=error,bevy_matchbox=error,webrtc_ice=error,webrtc=error"
                .into(),
            ..default()
        };
        app.add_plugins((
            MinimalPlugins,
            log_plugin,
            RepliconPlugins.set(ServerPlugin {
                tick_policy: TickPolicy::EveryFrame,
                ..Default::default()
            }),
            RepliconMatchboxPlugins,
        ))
            .add_server_event::<TestEvent>(Channel::Ordered)
            .make_event_independent::<TestEvent>()
            .finish();
    }

    setup(&mut server_app, &mut client_app);

    server_app.world_mut().spawn(Replicated);
    server_app.world_mut().send_event(ToClients {
        mode: SendMode::Broadcast,
        event: TestEvent {
            message: "Hello".into(),
        },
    });

    let events = client_app.world().resource::<Events<TestEvent>>();
    let mut clients = server_app
        .world_mut()
        .query_filtered::<Entity, With<ConnectedClient>>();
    let client_entity = clients.single(server_app.world()).unwrap();
    server_app
        .world_mut()
        .send_event(DisconnectRequest { client_entity });



    server_app.update();

    let events = client_app.world().resource::<Events<TestEvent>>();
    assert_eq!(clients.iter(server_app.world()).len(), 0);

    server_app.update(); // Requires additional update to let transport process the disconnect.
    client_app.update();

    let events = client_app.world().resource::<Events<TestEvent>>();

    assert!(
        client_app.world().resource::<MatchboxClient>().is_connected(),
        "matchbox client disconnects only on the next frame"
    );

    client_app.update();


    let client = client_app.world().resource::<RepliconClient>();
    assert!(client.is_disconnected());

    let events = client_app.world().resource::<Events<TestEvent>>();
    assert_eq!(events.len(), 1, "last event should be received");

    let mut replicated = client_app.world_mut().query::<&Replicated>();
    info!("replicated: {:?}", replicated.iter(client_app.world()).len());

    // assert_eq!(
    //     replicated.iter(client_app.world()).len(),
    //     1,
    //     "last replication should be received"
    // );
}
//
// #[test]
// fn server_stop() {
//     let mut server_app = App::new();
//     let mut client_app = App::new();
//     for app in [&mut server_app, &mut client_app] {
//         app.add_plugins((
//             MinimalPlugins,
//             RepliconPlugins.set(ServerPlugin {
//                 tick_policy: TickPolicy::EveryFrame,
//                 ..Default::default()
//             }),
//             RepliconRenetPlugins,
//         ))
//             .add_server_event::<TestEvent>(Channel::Ordered)
//             .finish();
//     }
//
//     setup(&mut server_app, &mut client_app);
//
//     server_app.world_mut().spawn(Replicated);
//     server_app.world_mut().send_event(ToClients {
//         mode: SendMode::Broadcast,
//         event: TestEvent,
//     });
//
//     // In renet, it's necessary to explicitly call disconnect before removing
//     // the server resource to let clients receive a disconnect.
//     let mut server = server_app.world_mut().resource_mut::<RenetServer>();
//     server.disconnect_all();
//
//     server_app.update();
//     client_app.update();
//
//     let mut clients = server_app.world_mut().query::<&ConnectedClient>();
//     assert_eq!(clients.iter(server_app.world()).len(), 0);
//     assert!(
//         server_app.world().resource::<RepliconServer>().is_running(),
//         "requires resource removal"
//     );
//     assert!(
//         client_app.world().resource::<RenetClient>().is_connected(),
//         "renet client disconnects only on the next frame"
//     );
//
//     server_app.world_mut().remove_resource::<RenetServer>();
//
//     server_app.update();
//     client_app.update();
//
//     assert!(!server_app.world().resource::<RepliconServer>().is_running());
//
//     let client = client_app.world().resource::<RepliconClient>();
//     assert!(client.is_disconnected());
//
//     let events = client_app.world().resource::<Events<TestEvent>>();
//     assert!(events.is_empty(), "event after stop shouldn't be received");
//
//     let mut replicated = client_app.world_mut().query::<&Replicated>();
//     assert_eq!(
//         replicated.iter(client_app.world()).len(),
//         0,
//         "replication after stop shouldn't be received"
//     );
// }
//
// #[test]
// fn replication() {
//     let mut server_app = App::new();
//     let mut client_app = App::new();
//     for app in [&mut server_app, &mut client_app] {
//         app.add_plugins((
//             MinimalPlugins,
//             RepliconPlugins.set(ServerPlugin {
//                 tick_policy: TickPolicy::EveryFrame,
//                 ..Default::default()
//             }),
//             RepliconRenetPlugins,
//         ))
//             .finish();
//     }
//
//     setup(&mut server_app, &mut client_app);
//
//     server_app.world_mut().spawn(Replicated);
//
//     server_app.update();
//     client_app.update();
//
//     let mut replicated = client_app.world_mut().query::<&Replicated>();
//     assert_eq!(replicated.iter(client_app.world()).len(), 1);
// }
//
// #[test]
// fn server_event() {
//     let mut server_app = App::new();
//     let mut client_app = App::new();
//     for app in [&mut server_app, &mut client_app] {
//         app.add_plugins((
//             MinimalPlugins,
//             RepliconPlugins.set(ServerPlugin {
//                 tick_policy: TickPolicy::EveryFrame,
//                 ..Default::default()
//             }),
//             RepliconRenetPlugins,
//         ))
//             .add_server_event::<TestEvent>(Channel::Ordered)
//             .finish();
//     }
//
//     setup(&mut server_app, &mut client_app);
//
//     server_app.world_mut().send_event(ToClients {
//         mode: SendMode::Broadcast,
//         event: TestEvent,
//     });
//
//     server_app.update();
//     client_app.update();
//
//     let events = client_app.world().resource::<Events<TestEvent>>();
//     assert_eq!(events.len(), 1);
// }
//
// #[test]
// fn client_event() {
//     let mut server_app = App::new();
//     let mut client_app = App::new();
//     for app in [&mut server_app, &mut client_app] {
//         app.add_plugins((
//             MinimalPlugins,
//             RepliconPlugins.set(ServerPlugin {
//                 tick_policy: TickPolicy::EveryFrame,
//                 ..Default::default()
//             }),
//             RepliconRenetPlugins,
//         ))
//             .add_client_event::<TestEvent>(Channel::Ordered)
//             .finish();
//     }
//
//     setup(&mut server_app, &mut client_app);
//
//     client_app.world_mut().send_event(TestEvent);
//
//     client_app.update();
//     server_app.update();
//
//     let client_events = server_app
//         .world()
//         .resource::<Events<FromClient<TestEvent>>>();
//     assert_eq!(client_events.len(), 1);
// }

#[cfg(test)]
const DEFAULT_ROOM: &str = "ws://localhost:7777/TestRoom";
#[cfg(test)]
const DEFAULT_PORT: u16 = 7777;

fn setup(server_app: &mut App, client_app: &mut App) {
    start_signaling_server(server_app);
    setup_server(server_app);
    setup_client(client_app);
    wait_for_connection(server_app, client_app);
}

use bevy_matchbox::{matchbox_signaling::SignalingServer};
fn start_signaling_server(server_app: &mut App) {
    info!("Starting signaling server");
    let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DEFAULT_PORT);
    let signaling_server = bevy_matchbox::MatchboxServer::from(
        SignalingServer::client_server_builder(addr)
            .on_connection_request(|connection| {
                info!("Connecting: {connection:?}");
                Ok(true) // Allow all connections
            })
            .on_id_assignment(|(socket, id)| info!("{socket} received {id}"))
            .on_host_connected(|id| info!("Host joined: {id}"))
            .on_host_disconnected(|id| info!("Host left: {id}"))
            .on_client_connected(|id| info!("Client joined: {id}"))
            .on_client_disconnected(|id| info!("Client left: {id}"))
            .cors()
            // .trace()
            .build(),
    );
    server_app.insert_resource(signaling_server);
}


fn setup_server(app: &mut App) {
    let channels = app.world().resource::<RepliconChannels>();

    let server = MatchboxHost::new(DEFAULT_ROOM, channels).unwrap();

    app.insert_resource(server);
}

fn setup_client(app: &mut App) {
    let channels = app.world().resource::<RepliconChannels>();
    let client = MatchboxClient::new(DEFAULT_ROOM, channels).unwrap();
    app.insert_resource(client);
}

fn wait_for_connection(server_app: &mut App, client_app: &mut App) {
    loop {
        client_app.update();
        server_app.update();
        let host = server_app.world().resource::<MatchboxHost>();
        let client = client_app.world().resource::<MatchboxClient>();
        if host.connected_clients() > 0 && client.is_connected(){
            break;
        }
    }
}

#[derive(Deserialize, Event, Serialize)]
struct TestEvent {
    message: String,
}