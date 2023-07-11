use std::{
    fmt::Debug,
    sync::{Arc, RwLock},
};

use rand::Rng;

use async_trait::async_trait;

use crate::game_board::Board;
use crate::{entities::Entity, entity_control::EntityManager, Pos, Tile};

use log::info;

/// Provide some sort of hint to the game controller about any special kind of behavior after processing.
pub enum PostProcessResult {
    /// If returned, remove the entity from the processing list, with nothing else in its place.
    Delete,
    /// If returned, drop the current entity, and leave this in its place.
    ReplaceMeWith(Entity),
    /// If returned, try to add these e ntities around us. If we fail to add any, it's okay, we'll just drop them.
    TryToAddEntities(Vec<(Pos, Entity)>),
    /// If returned, try to add these entities around us. If we fail to add any, it's okay, we'll just drop them.
    TryToAddEntitiesAndKillMe(Vec<(Pos, Entity)>),
    /// If returned, mark the given tiles as new interesting ones
    MarkTheseAsInteresting(Vec<Pos>),
    // TODO honestly we should allow for some of these to be returned in a vector together or something
}

/// Anything implementing this can move on its own.
pub trait Mobile {
    /// Maximum movespeeds in the x and y direction
    fn max_speeds(&self) -> (usize, usize);
}

/// This trait indicates things that will process.
/// Everything on our board will need to have this, it's just a matter of whether or not will_process() will return true.
#[async_trait]
pub trait Processing {
    /// Returns true if the object needs to perform some processing this turn.
    fn will_process(&self) -> bool;

    /// Returns true if the object has a late processing step to execute that can run independently of anything else.
    fn will_process_late(&self) -> bool;

    /// Returns true if the object will ever process, or if we can always ignore it
    fn will_ever_process(&self) -> bool;

    /// Perform some turn's worth of processing onto something else.
    fn process(&mut self, board: &mut Board, ctx: ProcessingContext) -> Option<PostProcessResult>;

    /// Perform some late processing. Anything performed here should be atomic and thread-safe.
    async fn late_process(&mut self) -> Option<PostProcessResult>;
}

/// A helper data structure passed into processing elements.
pub struct ProcessingContext {
    pub position: Pos,
    pub entity_context: Arc<RwLock<EntityManager>>,
}

/// Defines your life status.
#[derive(PartialEq)]
pub enum LifeStatus {
    /// You're alive, and will continue to act.
    Alive,
    /// You're dead. Unless handled specially, you'll probably either become a corpse (once that's implemented) or delete.
    Dead,
}

/// Anything with this trait can or will live or die.
pub trait Lives: Processing {
    /// If false, this represents something dead, and we never need to worry about it.
    fn will_ever_live(&self) -> bool;

    /// Run the life loop.
    fn life(&mut self) {
        if !self.will_ever_live() {
            return;
        }

        self.process_health();
        self.process_hunger();
        self.process_age();
        self.process_life_misc();
    }

    /// Get the amount of health
    fn get_health(&self) -> i64;

    /// Apply a positive or negative delta to the amount of health we have, possibly killing if we dip below zero.
    fn modify_health(&mut self, delta: i64, cause: &str);

    /// Get our current life status.
    fn get_life_status(&self) -> LifeStatus;

    /// Process our health.
    fn process_health(&mut self);

    /// Process our hunger
    fn process_hunger(&mut self);

    /// Handle anything related to us aging.
    fn process_age(&mut self);

    /// Process anything else necessary in a life tick.
    fn process_life_misc(&mut self) {}

    /// Irrevocably die. Once this gets called, you should not be able to come back to life.
    fn die(&mut self, cause: &str);

    /// If this is true, you won't leave behind a corpse and will just be deleted once you die.
    fn delete_on_death(&self) -> bool {
        true
    }

    // Little helper function
    fn is_dead(&self) -> bool {
        matches!(self.get_life_status(), LifeStatus::Dead)
    }
}

/// Little informative struct created by things that can reproduce to help inform the offspring finder
pub struct OffspringData {
    /// Minimum allowable offspring, assuming we can fill all the spaces around us (which we may not be able to).
    pub min_offspring: usize,
    /// Maximum allowable offspring.
    pub max_offspring: usize,
    /// Percent chance to spawn an offspring on each empty tile, up to max_offspring
    pub percent_chance_per_tile: f64,
}

/// Defines something that will gradually grow and change forms into something new.
pub trait Growing
where
    Self: Lives + Debug,
{
    /// Grow into...something. Return the type of entity that we'd like to change into.
    /// Note that this doesn't take ownership of self, since that's kind of a pain given how this gets used.
    /// Make sure to clean up ourselves after.
    fn grow_into(&self) -> Option<Entity>;

    /// Perform a growth step.
    fn grow_step(&mut self);

    /// Apply a slow-factor to growth.
    fn slow_growth(&mut self, factor: usize);

    /// If this is true, then we are ready to grow into our next form.
    fn ready_to_grow_into(&self) -> bool;
}

/// Defines something that can reproduce and create new children.
pub trait Reproducing
where
    Self: Lives + Debug,
{
    /// Return whether this is currently ready to reproduce.
    fn ready_to_reproduce(&self) -> bool;

    /// Should be called when offspring are actually born.
    fn on_offspring_created(&mut self);

    /// Create some children out of ourselves, optionally destroying ourselves in the process.
    fn create_offspring(&mut self, board: &mut Board, pos: Pos) -> Vec<Pos> {
        let children_so_far = 0;
        let mut rng = rand::thread_rng();
        let offspring_data = self.get_offspring_data();
        if offspring_data.is_none() {
            // error!("Offspring data was none for something that tried to reproduce!");
            panic!("Offspring data was none for something that tried to reproduce!");
        }
        let offspring_data = offspring_data.unwrap();
        let mut positions_spread = Vec::new();
        let all_valid_tiles = board.range(1, false, pos);
        let mut necessary_children = offspring_data.min_offspring;
        let empty_tiles = all_valid_tiles
            .into_iter()
            .filter(|p| !board.get_tile_from_pos(*p).is_occupied())
            .collect::<Vec<Pos>>();
        if empty_tiles.is_empty() {
            info!("There were no valid tiles for reproduction around {self:?}");
            return positions_spread;
        } else if empty_tiles.len() < offspring_data.min_offspring {
            info!("Not enough empty tiles around for minimum offspring around {self:?}, doing our best");
            necessary_children = empty_tiles.len();
        }

        // fill necessary children
        while necessary_children > 0 {
            let selected = rng.gen_range(0..empty_tiles.len());
            let selected = empty_tiles[selected];
            // todo this is a bit inefficient, could probably be better than randomly choosing one
            if positions_spread.contains(&selected) {
                continue;
            }
            let new_tile = board.get_tile_mut_from_pos(selected);
            if new_tile.is_occupied() {
                continue;
            }
            // have that child
            self.have_child(new_tile, pos, children_so_far);
            necessary_children -= 1;
            positions_spread.push(selected);
        }

        // now, go through to see how many other children we might have.

        let still_empty_pos = empty_tiles
            .into_iter()
            .filter(|p| !board.get_tile_from_pos(*p).is_occupied())
            .collect::<Vec<Pos>>();

        for pos in still_empty_pos {
            if positions_spread.len() > offspring_data.max_offspring {
                break;
            }
            if rng.gen_bool(offspring_data.percent_chance_per_tile) {
                let new_tile = board.get_tile_mut_from_pos(pos);
                self.have_child(new_tile, pos, children_so_far);
                positions_spread.push(pos);
            }
        }

        self.on_offspring_created();

        positions_spread
    }

    /// Get data on how new offspring should be created.
    fn get_offspring_data(&self) -> Option<OffspringData>;

    /// Create a child on a given tile.
    fn have_child(&mut self, tile: &mut Tile, pos: Pos, children_so_far: usize);
}
