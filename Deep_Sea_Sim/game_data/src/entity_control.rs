use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use log::warn;

use crate::entities::Entity;
use crate::game_board::Pos;

// use crate::{Pos, entries::Entity};

/// Interface to ensure that the appropriate methods are implemented on anything that interacts with our entity ID system.
pub trait TrackedEntity {
    /// If false, this entity will never appear in the important entities list.
    /// If this is true and this element does not have an ID, then the first tile it is assigned to will give it a new ID.
    fn tracked(&self) -> bool;

    /// Register oneself as a given entity, hanging onto this ID for as long as it lives.
    /// Will return an error if this object is not actually tracked, indicating that the object cannot be registered.
    fn register(&mut self, id: EntityID) -> Result<(), EntityID>;

    /// Get an entity's ID. If this entity is not tracked, it will not return one.
    fn get_id(&self) -> Option<EntityID>;
}

/// An ID tracking an entity.
/// Essentially just a usize, but we're wrapping it in a struct for typing's sake.
#[derive(Clone, Copy, Hash, Debug, PartialEq, Eq)]
pub struct EntityID {
    id: usize,
}
impl EntityID {
    pub fn get_id_val(&self) -> usize {
        self.id
    }
}

/// A struct that's designed to be passed around in an Arc<Mutex>.
/// This keeps track of all the living entities, and the tiles that they're on.
/// These entity IDs are essentially weak references to the entities themselves. This will provide access to their position, but it may become invalid.
/// If you want to update the list of active entities, you need to hold the lock for both the ID.
#[derive(Debug, Clone)]
pub struct EntityManager {
    /// The current largest entity ID. The next entity ID that will be handed out will be this + 1
    current_largest_entity_id: usize,
    /// Map of current entity IDs to their position.
    active_entities: HashMap<EntityID, Pos>,
}

impl EntityManager {
    /// If you want to make a new one, you'll be creating it as an arc<mutex>>. This shouldn't really exist in any other context.
    pub fn new() -> Arc<RwLock<Self>> {
        Arc::new(RwLock::new(Self {
            current_largest_entity_id: 0,
            active_entities: HashMap::new(),
        }))
    }

    /// Add a new entity to the global list.
    pub fn register_new_entity(&mut self, new_position: Pos, entity: &mut Entity) -> EntityID {
        self.current_largest_entity_id += 1;
        let new_ent_id = EntityID {
            id: self.current_largest_entity_id,
        };
        self.active_entities.insert(new_ent_id, new_position);
        if let Err(id) = entity.register(new_ent_id) {
            warn!("Entity {entity:?} was to be given ID {id:?}, but registration failed!")
        }
        new_ent_id
    }

    /// Update the position of an entity.
    /// This should probably be called within a tile.
    pub fn update_position(&mut self, entity: EntityID, new_position: Option<Pos>) {
        match new_position {
            Some(pos) => self.active_entities.insert(entity, pos),
            None => self.active_entities.remove(&entity),
        };
    }

    /// Public accessor for getting the entries in the map, but only as an immutable reference
    pub fn get_active_entries(&self) -> &HashMap<EntityID, Pos> {
        &self.active_entities
    }

    pub fn get_active_positions(&self) -> Vec<Pos> {
        self.active_entities.values().copied().collect()
    }
}
#[cfg(test)]
mod test {
    use super::*;
    use crate::entities::{plants::ConcretePlants, NonAbstractTaxonomy};
    use crate::game_board::Pos;
    use crate::test_utils::*;

    #[test]
    fn test_track_entity_id() {
        let kelp = ConcretePlants::Kelp.create_new(None);
        assert_eq!(kelp.get_id(), None);

        let kelp_position = Pos { x: 1, y: 1 };

        // Create a testbed with kelp in the middle
        let mut testbed = TestBed::new_with_entities(3, 3, vec![(kelp_position, kelp)]);

        let em = Arc::clone(&testbed.sandbox.entity_context);

        let kelp_tile = testbed.sandbox.board.get_tile(1, 1);
        assert!(&kelp_tile.is_occupied());

        // verify that everything stays connected when we insert the entity

        let mut ent_id = None;

        if let Some(ent) = &kelp_tile.get_entity() {
            ent_id = ent.get_id();
            // verify that an ID was registered
            assert!(ent_id.is_some());
            // now, verify that the ID is present in the active entries list
            let raw_ent_id = ent_id.unwrap();
            let em_guard = em.read().unwrap();
            let kelp_pos = em_guard.get_active_entries().get(&raw_ent_id);
            assert!(kelp_pos.is_some());
            let kelp_pos = kelp_pos.unwrap();
            assert_eq!(*kelp_pos, kelp_position);
        } else {
            assert!(false, "Entity wasn't found on the tile!");
        }

        // now, verify that removing the entry from the tile also removes it from the active entries list
        let kelp_tile = testbed.sandbox.board.get_tile_mut(1, 1);
        let ent = kelp_tile.remove_entity().unwrap();
        assert_eq!(ent.get_id(), ent_id);
        let em_guard = em.read().unwrap();
        let kelp_pos = em_guard.get_active_entries().get(&ent.get_id().unwrap());
        assert!(kelp_pos.is_none())
    }
}
