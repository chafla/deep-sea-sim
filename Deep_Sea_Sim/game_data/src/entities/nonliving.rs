use crate::entity_control::EntityID;

use super::{Entity, NonAbstractTaxonomy, NonLiving, PTUIDisplay};

pub enum ConcreteDecorations {
    Rock,
    Shell,
}

impl NonAbstractTaxonomy for ConcreteDecorations {
    fn create_new(&self, _: Option<EntityID>) -> Entity {
        let new_creature = match self {
            Self::Rock => NonLiving::Rock(Decoration {
                name: "rock".to_owned(),
            }),
            Self::Shell => NonLiving::Shell(Decoration {
                name: "shell".to_owned(),
            }),
        };

        Entity::NonLiving(new_creature)
    }

    fn same_kind(&self, entity: &Entity) -> bool {
        match entity {
            Entity::NonLiving(nl) => match nl {
                NonLiving::Rock(_) => matches!(self, ConcreteDecorations::Rock),
                NonLiving::Shell(_) => matches!(self, ConcreteDecorations::Shell),
            },
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Decoration {
    pub name: String,
}

impl PTUIDisplay for Decoration {
    fn get_display_char(&self) -> char {
        return self.name.chars().next().unwrap();
    }
}
