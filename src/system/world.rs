use std::sync::Arc;

use collider::EntityId;
use collider::EntityDatabase;

#[derive(Debug)]
struct WorldInner {
    db: EntityDatabase,
}

#[derive(Clone, Debug)]
struct World {
    inner: Arc<WorldInner>
}

// Impl's 

impl WorldInner {
    fn spawn_entity(&self) -> EntityId {
        todo!()
    }
}

impl World {
    fn new() -> Self {
        let inner = WorldInner {
            db: EntityDatabase::new()
        };

        World {
            inner: Arc::new(inner)
        }
    }

    fn spawn_entity(&self) -> EntityId {
        self.inner().spawn_entity()
    }

    fn inner(&self) -> &WorldInner {
        &self.inner
    }
}

#[cfg(test)]
mod test {
    use super::*;

    /// Cloning a `World` should not clone the underlying data
    #[test]
    fn clone_world() {
        let world = World::new();
        let world_copy = world.clone();

        assert!(std::sync::Arc::ptr_eq(&world.inner, &world_copy.inner));
    }

    #[test]
    fn spawn_entity() {
        let world = World::new();
        let entity = world.spawn_entity();
    }
}
