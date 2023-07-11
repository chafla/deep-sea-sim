use std::cmp::{max, min};

use async_trait::async_trait;
use log::info;

use crate::{
    element_traits::{
        Growing, LifeStatus, Lives, OffspringData, PostProcessResult, Processing,
        ProcessingContext, Reproducing,
    },
    entity_control::{EntityID, TrackedEntity},
    game_board::Board,
    interactions::{EatResult, Eaten},
    Pos,
};

use super::{Entity, Living, NonAbstractTaxonomy, PTUIDisplay};

// only add the plants we'll see on spawn here
pub enum ConcretePlants {
    Kelp,
    KelpSeed,
    KelpLeaf,
}

impl NonAbstractTaxonomy for ConcretePlants {
    fn create_new(&self, id: Option<EntityID>) -> Entity {
        let new_plant = match self {
            Self::Kelp => {
                Plants::Kelp(Plant::new("kelp".to_owned(), 65, 2, Some(200), id))
                // kelp will last a long time on its own
            }
            Self::KelpLeaf => Plants::KelpLeaf(Plant::new("kelp_leaf".to_owned(), 15, 1, None, id)),
            Self::KelpSeed => Plants::KelpSeed(Plant::new("kelp_seed".to_owned(), 50, 1, None, id)),
        };

        Entity::Living(Living::Plants(new_plant))
    }

    fn same_kind(&self, entity: &Entity) -> bool {
        match entity {
            Entity::Living(Living::Plants(p)) => match p {
                Plants::KelpSeed(_) => matches!(self, Self::KelpSeed),
                Plants::KelpLeaf(_) => matches!(self, Self::KelpLeaf),
                Plants::Kelp(_) => matches!(self, Self::Kelp),
            },
            _ => false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Plants {
    Kelp(Plant),
    KelpSeed(Plant),
    KelpLeaf(Plant),
}

impl Eaten for Plants {
    fn on_eat(&mut self, _: usize) -> Option<Vec<EatResult>> {
        // regardless of attack damage,
        match self {
            Self::Kelp(p) | Self::KelpLeaf(p) | Self::KelpSeed(p) => {
                println!("{p:?} was eaten!");
                p.hp -= 1;
                if p.hp == 0 {
                    self.die("eaten")
                }
            }
        }
        Some(vec![EatResult::Eaten])
    }
    fn get_retaliation_damage(&self) -> usize {
        0
    }
}

impl Growing for Plants {
    /// Grow into a plant.
    /// I'd make this take ownership, but due to our structure that's /kind/ of a bugger
    /// It seems like it'll be easier to just. Hope that the person using us drops us.
    /// Basically, if you ever use this, make sure to return the proper post-process hint.
    fn grow_into(&self) -> Option<Entity> {
        match self {
            // Self::Kelp(_) => Some(ConcretePlants::KelpSeed.create_new()),  // kelp will grow into a seed when it finally gives up the game.
            Self::Kelp(_) => None,
            Self::KelpLeaf(_) => Some(ConcretePlants::Kelp.create_new(self.get_id())),
            Self::KelpSeed(_) => Some(ConcretePlants::KelpLeaf.create_new(self.get_id())),
        }
    }

    /// Increase our growth level
    fn grow_step(&mut self) {
        match self {
            Self::Kelp(p) | Self::KelpLeaf(p) | Self::KelpSeed(p) => p.growth_level += 1,
        }
    }

    fn slow_growth(&mut self, factor: usize) {
        match self {
            Self::Kelp(p) | Self::KelpLeaf(p) | Self::KelpSeed(p) => {
                let less_growth = p.growth_level as f64 / factor as f64;
                p.growth_level -= less_growth.ceil() as usize;
            }
        }
    }

    fn ready_to_grow_into(&self) -> bool {
        match self {
            Self::KelpLeaf(p) | Self::KelpSeed(p) => p.growth_level >= p.max_growth,
            _ => false, // don't let kelp "grow", though TODO it eventually should
        }
    }
}

impl Reproducing for Plants {
    fn ready_to_reproduce(&self) -> bool {
        match self {
            Self::Kelp(p) => p.growth_level % p.max_growth == 0 && p.growth_level > 0,
            _ => false,
        }
    }

    fn on_offspring_created(&mut self) {}

    fn get_offspring_data(&self) -> Option<OffspringData> {
        match self {
            Self::Kelp(_) => {
                // kelp always need to produce 1, but could produce up to 3 if lucky
                Some(OffspringData {
                    min_offspring: 1,
                    max_offspring: 3,
                    percent_chance_per_tile: 0.1,
                })
            }
            _ => None,
        }
    }

    fn have_child(&mut self, tile: &mut crate::Tile, _pos: Pos, _children_so_far: usize) {
        let seed = match self {
            // it'll be assigned its ID when added
            Plants::Kelp(_) => Some(ConcretePlants::KelpSeed.create_new(None)),
            _ => None,
        };
        if let Some(s) = seed {
            tile.add_entity(s).unwrap()
        }
    }
}

impl PTUIDisplay for Plants {
    fn get_display_char(&self) -> char {
        match &self {
            Self::Kelp(_) => '🌳',
            Self::KelpSeed(_) => '🌱',
            Self::KelpLeaf(_) => '🌿',
        }
    }
}

impl Lives for Plants {
    fn will_ever_live(&self) -> bool {
        true // plants never really die
    }

    fn modify_health(&mut self, delta: i64, cause: &str) {
        match self {
            Self::Kelp(p) | Self::KelpSeed(p) | Self::KelpLeaf(p) => {
                p.hp = max(0, min(p.hp_max, p.hp + delta));
                if p.hp == 0 {
                    self.die(cause);
                }
            }
        }
    }

    fn get_health(&self) -> i64 {
        match &self {
            Self::Kelp(p) | Self::KelpSeed(p) | Self::KelpLeaf(p) => p.hp,
        }
    }

    fn get_life_status(&self) -> LifeStatus {
        match &self {
            Self::Kelp(p) | Self::KelpSeed(p) | Self::KelpLeaf(p) => {
                if p.has_died {
                    LifeStatus::Dead
                } else {
                    LifeStatus::Alive
                }
            }
        }
    }

    fn process_life_misc(&mut self) {
        self.grow_step()
    }

    fn process_health(&mut self) {
        // we don't do anything
    }

    fn process_hunger(&mut self) {
        // no hunger
    }

    fn process_age(&mut self) {
        match self {
            Self::Kelp(p) | Self::KelpSeed(p) | Self::KelpLeaf(p) => {
                p.age += 1;
                if let Some(max_age) = p.max_age {
                    if max_age < p.age {
                        self.die("old age")
                    }
                }
            }
        }
    }

    fn die(&mut self, cause: &str) {
        match self {
            Self::Kelp(p) | Self::KelpSeed(p) | Self::KelpLeaf(p) => {
                p.has_died = true;
            }
        }

        info!("{self:?} has died of {cause}!")

        // note! we don't want to delete, at least not immediately, if we're capable of spreading seeds.
    }
}

#[async_trait]
impl Processing for Plants {
    fn will_ever_process(&self) -> bool {
        true
    }

    fn will_process(&self) -> bool {
        match self {
            &Plants::Kelp(_) => true, // only kelp processes
            _ => false,
        }
    }

    fn will_process_late(&self) -> bool {
        match self {
            Plants::KelpLeaf(_) | Plants::KelpSeed(_) | Plants::Kelp(_) => true, // these baddies need to grow (also kelp needs to die lol)
        }
    }

    fn process(&mut self, board: &mut Board, ctx: ProcessingContext) -> Option<PostProcessResult> {
        if !self.will_process() {
            return None;
        }

        if self.ready_to_reproduce() {
            let new_important_positions = self.create_offspring(board, ctx.position);
            // new_important_positions.push(position);  // make sure our current position stays important
            return Some(PostProcessResult::MarkTheseAsInteresting(
                new_important_positions,
            ));
        }

        match self {
            &mut Plants::Kelp(_) => {
                if matches!(self.get_life_status(), LifeStatus::Dead) {
                    // let new_important_positions = self.create_offspring(board, position, false);
                    return Some(PostProcessResult::Delete);
                }
                None
            }
            _ => None,
        }
    }

    async fn late_process(&mut self) -> Option<PostProcessResult> {
        match self.get_life_status() {
            LifeStatus::Alive => {
                self.life();
                if self.ready_to_grow_into() {
                    if let Some(the_next_generation) = self.grow_into() {
                        return Some(PostProcessResult::ReplaceMeWith(the_next_generation));
                    }
                };
                None
            }
            LifeStatus::Dead => {
                info!("{:?} is returning delete in late process", &self);
                Some(PostProcessResult::Delete)
            }
        }
    }
}

impl TrackedEntity for Plants {
    fn tracked(&self) -> bool {
        self.get_id().is_some() // they should always have IDs, but on init there's a brief moment in time where they might not, better safe than sorry ig
    }

    fn register(&mut self, id: EntityID) -> Result<(), EntityID> {
        match self {
            Self::Kelp(p) | Self::KelpLeaf(p) | Self::KelpSeed(p) => p.entity_id = Some(id),
        }
        Ok(())
    }

    fn get_id(&self) -> Option<EntityID> {
        match self {
            Self::Kelp(p) | Self::KelpLeaf(p) | Self::KelpSeed(p) => p.entity_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Plant {
    pub name: String,
    /// Amount that the plant has currently grown
    pub growth_level: usize,
    /// Point at which the plant should consider changing into another species
    max_growth: usize,
    /// Current bites left
    hp: i64,
    /// Number of "HP", or basically the number of times this can be eaten.
    hp_max: i64,
    /// Age, in ticks.
    age: usize,
    /// How old we can possibly be (in ticks) before dying. If None, can't die of old age.
    max_age: Option<usize>,
    /// If we're irrevocably dead
    has_died: bool,
    /// Our ID as a tracked entity.
    entity_id: Option<EntityID>,
}

impl Plant {
    pub fn new(
        name: String,
        max_growth: usize,
        hp: i64,
        max_age: Option<usize>,
        entity_id: Option<EntityID>,
    ) -> Self {
        Plant {
            name,
            growth_level: 0,
            max_growth,
            hp_max: 0,
            hp,
            age: 0,
            max_age,
            has_died: false,
            entity_id,
        }
    }

    pub fn die(&mut self) {
        self.has_died = true;
    }
}

impl PTUIDisplay for Plant {
    fn get_display_char(&self) -> char {
        return self.name.chars().next().unwrap();
    }
}
