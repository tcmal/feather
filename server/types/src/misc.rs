use feather_core::anvil::entity::{EntityData, EntityDataKind};
use fecs::EntityBuilder;

pub type BumpVec<'bump, T> = bumpalo::collections::Vec<'bump, T>;

pub trait EntityLoaderFn:
    Fn(EntityData) -> anyhow::Result<EntityBuilder> + Send + Sync + 'static
{
}

impl<F> EntityLoaderFn for F where
    F: Fn(EntityData) -> anyhow::Result<EntityBuilder> + Send + Sync + 'static
{
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Weather {
    Clear,
    Rain,
    Thunder,
}

/// A registration for a function to convert an `EntityData`
/// to an `EntityBuilder` for spawning into the world. The
/// registration must provide the `EntityDataKind` it handles
/// to determine which `EntityData`s to pass to this function.
pub struct EntityLoaderRegistration {
    /// The loader function.
    pub f: &'static dyn EntityLoaderFn,
    /// The kind of `EntityData` which this loader
    /// function will accept.
    pub kind: EntityDataKind,
}

impl EntityLoaderRegistration {
    pub fn new(kind: EntityDataKind, f: &'static dyn EntityLoaderFn) -> Self {
        Self { f, kind }
    }
}

inventory::collect!(EntityLoaderRegistration);

/// Wrapper around the send/receive channels which will be used to
/// notify server thread of shutdown due to ctrl+C or /stop command.
pub struct ShutdownChannels {
    pub tx: crossbeam::channel::Sender<()>,
    pub rx: crossbeam::channel::Receiver<()>,
}

impl ShutdownChannels {
    pub fn new() -> Self {
        let (tx, rx) = crossbeam::bounded(1);
        Self { tx, rx }
    }
}

impl Default for ShutdownChannels {
    fn default() -> Self {
        Self::new()
    }
}
