pub mod animals;
pub mod nonliving;
pub mod plants;

use crate::entity_control::{EntityID, TrackedEntity};

use self::{animals::Animals, nonliving::Decoration, plants::Plants};

/// Once something reaches this pregancy level, they will start trying to have a child if they can.
const MAX_PREGNANCY_LEVEL: usize = 100;

/// The maximum number of actions to consider per turn
const MAXIMUM_ACTIONS_TO_CONSIDER: usize = 5000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sex {
    Male,
    Female,
    #[allow(unused)] // would be nice to introduce this at some point
    Neutral,
}

/// This trait defines a class that can be used to directly create an Entity.
/// More specifically (and the reason for its name), it's not truly in the entity tree by itself, but represents some concrete entity, not just a step in the hierarchy.
pub trait NonAbstractTaxonomy {
    fn create_new(&self, id: Option<EntityID>) -> Entity; // An enum with this trait can create members of a certain type.

    /// Get whether this specific type matches the passed-in entity.
    fn same_kind(&self, entity: &Entity) -> bool;
}

pub trait PTUIDisplay {
    fn get_display_char(&self) -> char;
}

#[derive(Debug, Clone)]
pub enum Entity {
    Living(Living),
    NonLiving(NonLiving),
}

impl PTUIDisplay for Entity {
    fn get_display_char(&self) -> char {
        match &self {
            Entity::Living(l) => l.get_display_char(),
            Entity::NonLiving(n) => n.get_display_char(),
        }
    }
}

impl TrackedEntity for Entity {
    fn tracked(&self) -> bool {
        match self {
            Self::NonLiving(_) => false,
            Self::Living(_) => true,
        }
    }

    fn register(&mut self, id: EntityID) -> Result<(), EntityID> {
        match self {
            Self::NonLiving(_) => Err(id),
            Self::Living(l) => match l {
                Living::Animals(a) => a.register(id),
                Living::Plants(p) => p.register(id),
            },
        }
    }

    fn get_id(&self) -> Option<EntityID> {
        match self {
            Self::NonLiving(_) => None,
            Self::Living(l) => match l {
                Living::Animals(a) => a.get_id(),
                Living::Plants(p) => p.get_id(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum NonLiving {
    Rock(Decoration),
    Shell(Decoration),
}

impl PTUIDisplay for NonLiving {
    fn get_display_char(&self) -> char {
        match &self {
            Self::Rock(_) => '🗿',
            Self::Shell(_) => '🔲',
        }
    }
}

#[derive(Debug, Clone)]
pub enum Living {
    Plants(Plants),
    Animals(Animals),
}

impl PTUIDisplay for Living {
    fn get_display_char(&self) -> char {
        match &self {
            Self::Plants(p) => p.get_display_char(),
            Self::Animals(a) => a.get_display_char(),
            // & => deco.get_display_char(),
        }
    }
}

pub fn generate_creatures<T>(number_to_gen: usize, class: T) -> Vec<Entity>
where
    T: NonAbstractTaxonomy,
{
    let mut resulting_entities = Vec::new();

    for _ in 0..number_to_gen {
        resulting_entities.push(class.create_new(None));
    }

    resulting_entities
}
