//! Module for interacting with the chunk worker thread
//! from the server threads.
use crate::chunkworker;
use crossbeam::channel::{Receiver, Sender};
use feather_core::world::{ChunkMap, ChunkPosition};
use shrev::EventChannel;
use specs::{Read, System, World, Write};

pub struct ChunkWorkerHandle {
    sender: Sender<chunkworker::Request>,
    receiver: Receiver<chunkworker::Reply>,
}

impl Default for ChunkWorkerHandle {
    fn default() -> Self {
        let (sender, receiver) = chunkworker::start("world");
        Self { sender, receiver }
    }
}

/// Event which is triggered when a chunk is loaded.
#[derive(Debug, Clone, Copy)]
pub struct ChunkLoadEvent {
    pub pos: ChunkPosition,
}

/// System for receiving loaded chunks from the chunk worker thread.
pub struct ChunkLoadSystem;

impl<'a> System<'a> for ChunkLoadSystem {
    type SystemData = (
        Write<'a, ChunkMap>,
        Write<'a, EventChannel<ChunkLoadEvent>>,
        Read<'a, ChunkWorkerHandle>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (mut chunk_map, mut events, handle) = data;

        while let Ok((pos, result)) = handle.receiver.try_recv() {
            match result {
                Ok(chunk) => {
                    chunk_map.set_chunk_at(pos, chunk);

                    // Trigger event
                    let event = ChunkLoadEvent { pos };
                    events.single_write(event);

                    trace!("Loaded chunk at {:?}", pos);
                }
                Err(err) => {
                    // TODO generate chunk if it didn't exist
                    warn!("Failed to load chunk at {:?}: {}", pos, err);
                }
            }
        }
    }

    fn setup(&mut self, world: &mut World) {
        use specs::prelude::SystemData;

        info!("Starting chunk worker thread");
        let handle = chunkworker::start("world");
        world.insert(handle);

        Self::SystemData::setup(world);
    }
}

/// Asynchronously loads the chunk at the given position.
/// At some point in time after this function is called,
/// the chunk will appear in the chunk map.
///
/// In the event that the requested chunk does not exist
/// in the world save, it will be generated asynchronously.
pub fn load_chunk(handle: &ChunkWorkerHandle, pos: ChunkPosition) {
    // Send request to chunk worker thread
    handle
        .sender
        .send(chunkworker::Request::LoadChunk(pos))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use feather_core::world::chunk::Chunk;
    use feather_core::world::ChunkPosition;
    use specs::{RunNow, World, WorldExt};

    #[test]
    fn test_chunk_system() {
        let (send1, _recv1) = crossbeam::channel::unbounded();
        let (send2, recv2) = crossbeam::channel::unbounded();
        let handle = ChunkWorkerHandle {
            sender: send1,
            receiver: recv2,
        };

        let chunk_map = ChunkMap::new();
        let pos = ChunkPosition::new(0, 0);
        send2.send((pos, Ok(Chunk::new(pos)))).unwrap();

        let event_channel = EventChannel::<ChunkLoadEvent>::new();

        let mut system = ChunkLoadSystem;
        let mut world = World::new();
        world.insert(chunk_map);
        world.insert(handle);
        world.insert(event_channel);

        system.run_now(&world);

        // Confirm that chunk was loaded
        let chunk_map = world.read_resource::<ChunkMap>();
        let chunk = chunk_map.chunk_at(pos);

        assert!(chunk.is_some());
        assert!(chunk.unwrap().position() == pos);
    }

    #[test]
    fn test_load_chunk() {
        let (send1, recv1) = crossbeam::channel::unbounded();
        let (_send2, recv2) = crossbeam::channel::unbounded();
        let handle = ChunkWorkerHandle {
            sender: send1,
            receiver: recv2,
        };

        let pos = ChunkPosition::new(0, 0);

        load_chunk(&handle, pos);

        let recv = recv1.try_recv().unwrap();
        assert_eq!(recv, chunkworker::Request::LoadChunk(pos));
    }
}
