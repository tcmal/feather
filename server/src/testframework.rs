//! Helper framework for writing unit tests.

use crate::config::Config;
use crate::entity::{EntityComponent, PlayerComponent};
use crate::io::ServerToWorkerMessage;
use crate::network::{NetworkComponent, PacketQueue};
use crate::PlayerCount;
use feather_core::network::packet::{Packet, PacketType};
use feather_core::world::Position;
use feather_core::Gamemode;
use mio_extras::channel::{channel, Receiver, Sender};
use rand::Rng;
use specs::{Builder, Dispatcher, Entity, World, WorldExt};
use std::net::TcpListener;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use uuid::Uuid;

/// Initializes a Specs world and dispatcher
/// using default configuration options and an
/// available server port.
pub fn init_world<'a, 'b>() -> (World, Dispatcher<'a, 'b>) {
    let mut config = Config::default();
    config.server.port = find_open_port().unwrap();

    let config = Arc::new(config);

    let player_count = Arc::new(PlayerCount(AtomicUsize::new(0)));
    let ioman = super::init_io_manager(Arc::clone(&config), Arc::clone(&player_count));

    super::init_world(config, player_count, ioman)
}

pub struct Player {
    pub entity: Entity,
    pub network_sender: Sender<ServerToWorkerMessage>,
    pub network_receiver: Receiver<ServerToWorkerMessage>,
}

/// Adds a player to the world, inserting
/// all the necessary components. Returns
/// a number of useful channels.
pub fn add_player(world: &mut World) -> Player {
    let (ns1, nr1) = channel();
    let (ns2, nr2) = channel();
    let e = world
        .create_entity()
        .with(NetworkComponent::new(ns1, nr2))
        .with(PlayerComponent {
            gamemode: Gamemode::Creative,
            profile_properties: vec![],
        })
        .with(EntityComponent {
            uuid: Uuid::new_v4(),
            on_ground: true,
            position: Position::new(0.0, 0.0, 0.0, 0.0, 0.0),
            display_name: "Test".to_string(),
        })
        .build();

    Player {
        entity: e,
        network_sender: ns2,
        network_receiver: nr1,
    }
}

/// Asserts that the given player has received
/// a packet of the given type, returning the packet.
pub fn assert_packet_received(player: &Player, ty: PacketType) -> Box<Packet> {
    match player.network_receiver.try_recv().unwrap() {
        ServerToWorkerMessage::SendPacket(pack) => {
            assert_eq!(pack.ty(), ty);
            pack
        }
        _ => panic!(),
    }
}

/// Retrieves up to `cap` packets sent to a player, if any.
/// If `cap` is set to `None`, all packets will be read.
///
/// Note that this function consumes messages in
/// the network channel until enough packets have been read.
pub fn received_packets(player: &Player, cap: Option<usize>) -> Vec<Box<Packet>> {
    let mut result = vec![];

    while let Ok(msg) = player.network_receiver.try_recv() {
        if let Some(cap) = cap.as_ref() {
            if result.len() >= *cap {
                break;
            }
        }
        match msg {
            ServerToWorkerMessage::SendPacket(pack) => result.push(pack),
            _ => (),
        }
    }

    result
}

/// Adds a received packet to the packet queue
/// for a given player.
pub fn receive_packet<P: Packet + 'static>(player: &Player, world: &World, packet: P) {
    let queue = world.fetch_mut::<PacketQueue>();
    queue.add_for_packet(player.entity, Box::new(packet));
}

/// Attempts to find an available port.
fn find_open_port() -> Option<u16> {
    let start = rand::thread_rng().gen_range(10000, 30000);
    (start..60000).find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
}

/// Heh... tests for the testing framework.
/// Not sure what the point of this is, since
/// all other tests would fail if the testing
/// framework didn't work.
mod tests {
    use super::*;
    use crate::entity::{EntityComponent, PlayerComponent};
    use crate::network::NetworkComponent;

    #[test]
    fn test_find_open_port() {
        let port = find_open_port().unwrap();
        println!("Found open port: {}", port);
        assert!(TcpListener::bind(("127.0.0.1", port)).is_ok());
    }

    #[test]
    fn test_init_world() {
        // Check that initializing the world doesn't cause
        // a panic.
        let (w, mut d) = init_world();

        // Check that running the dispatcher works fine
        d.dispatch(&w);
    }

    #[test]
    fn test_add_player() {
        let (mut w, _d) = init_world();

        let entity = add_player(&mut w).entity;

        assert!(w.read_component::<PlayerComponent>().get(entity).is_some());
        assert!(w.read_component::<EntityComponent>().get(entity).is_some());
        assert!(w.read_component::<NetworkComponent>().get(entity).is_some());
    }
}
