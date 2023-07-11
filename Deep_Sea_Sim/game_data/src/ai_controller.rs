// an AI controller that decides the current type of action that an AI-driven creature is performing

use std::{
    collections::{HashMap, VecDeque},
    fmt::Debug,
};

use log::{debug, info};
use rand::{rngs::ThreadRng, Rng};

use crate::{
    element_traits::{LifeStatus, Lives, Mobile, PostProcessResult, ProcessingContext},
    entities::{Entity, Living, PTUIDisplay},
    interactions::{EatsCreatures, Mates},
};

use crate::entities::animals::Animals;

use crate::game_board::{Board, Pos};

/// Similar to other concrete implementations, this allows for easy categorization and initialization of different behaviors.
#[derive(Debug, Clone, PartialEq)]
pub enum AIConcreteBehaviors {
    Idle(IdleAction),
    Eating(EatAction),
    Mating(MateAction),
}

// please look the other way for this impl
// this was the nicest way I could string this together given the amount of time to implement it

impl AIAction<Animals> for AIConcreteBehaviors {
    fn priority(&self) -> usize {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.priority(),
            Self::Idle(i) => i.priority(),
            Self::Mating(m) => m.priority(),
        }
    }

    fn completed(&self) -> bool {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.completed(),
            Self::Idle(i) => i.completed(),
            Self::Mating(m) => m.completed(),
        }
    }

    fn tick(
        &mut self,
        actor: &mut Animals,
        ctx: &ProcessingContext,
        board: &mut Board,
    ) -> Option<PostProcessResult> {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.tick(actor, ctx, board),
            Self::Idle(i) => i.tick(actor, ctx, board),
            Self::Mating(m) => m.tick(actor, ctx, board),
        }
    }

    fn initialize(&mut self) {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.initialize(),
            Self::Idle(i) => i.initialize(),
            Self::Mating(m) => m.initialize(),
        }
    }

    fn get_action_desc(&self) -> String {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.get_action_desc(),
            Self::Idle(i) => i.get_action_desc(),
            Self::Mating(m) => m.get_action_desc(),
        }
    }

    fn is_valid(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> bool {
        match self {
            Self::Eating(e) => e.is_valid(actor, ctx, board),
            Self::Idle(i) => i.is_valid(actor, ctx, board),
            Self::Mating(m) => m.is_valid(actor, ctx, board),
        }
    }

    fn end(self, actor: &mut Animals) {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.end(actor),
            Self::Idle(i) => i.end(actor),
            Self::Mating(m) => m.end(actor),
        }
    }

    fn get_movement(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> Option<Pos> {
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.get_movement(actor, ctx, board),
            Self::Idle(i) => i.get_movement(actor, ctx, board),
            Self::Mating(m) => m.get_movement(actor, ctx, board),
        }
    }

    fn is_valid_target(_: &Animals, _: &Entity, _: &ProcessingContext, _: &Board) -> bool {
        unimplemented!("Call this on the base behavior itself!")
    }

    fn untargeted(&self) -> bool {
        // unimplemented!("Call this on the base behavior itself!");
        match self {
            // TODO FIND A PRETTIER WAY TO DO THIS
            Self::Eating(e) => e.untargeted(),
            Self::Idle(i) => i.untargeted(),
            Self::Mating(m) => m.untargeted(),
        }
    }
}

/// Something with this trait has an AI state machine deciding its next moves
pub trait AIControlled<T>: Lives + Debug {
    /// Get all real, possible actions
    fn get_possible_concrete_actions(
        &self,
        ctx: ProcessingContext,
        board: &Board,
    ) -> Vec<AIConcreteBehaviors>;

    /// Get the action that we should perform during a tick.
    /// This will either return the behavior that we are currently performing,
    /// or a new behavior that's of higher priority than our current behavior.
    /// Return None to keep using the existing behavior.
    fn get_next_action(
        &self,
        ctx: &ProcessingContext,
        board: &Board,
    ) -> Option<AIConcreteBehaviors>;

    /// Get the entity's current behavior, mutably
    fn get_current_behavior_mut(&mut self) -> &mut AIConcreteBehaviors;

    /// Get the entity's current behavior
    fn get_current_behavior(&self) -> &AIConcreteBehaviors;

    /// Update an entity's current behavior
    fn set_current_behavior(&mut self, behavior: AIConcreteBehaviors);

    /// Get the tile that we would like to move to, based on our behavior.
    /// Note that this does need to be mutable as it will provide the next move, possibly reaching the destination
    fn get_desired_move(&self, ctx: &ProcessingContext, board: &Board) -> Option<Pos>;

    fn update_behavior(&mut self, ctx: &ProcessingContext, board: &mut Board) {
        let next_bhvr = self.get_next_action(ctx, board);
        if let Some(action) = next_bhvr {
            info!("{self:?} is switching behaviors to {action:?}");
            self.set_current_behavior(action);
        }
    }
}

/// Defines the behavior for an AI behavior.
pub trait AIAction<T>
where
    T: Lives + Debug + Clone,
{
    /// Create the action, and fill out any important fields.
    fn initialize(&mut self);

    /// Get the priority of this action: the highest priority option at a given time will be selected.
    fn priority(&self) -> usize;

    /// Get a description of the action.
    fn get_action_desc(&self) -> String;

    /// If true, the target would be a valid item to chase down.
    /// Note that this matches the signature animals expect when parsing their behaviors.
    fn is_valid_target(actor: &T, target: &Entity, ctx: &ProcessingContext, board: &Board) -> bool;

    /// Property for this action; if true, this action will never search for a target.
    fn untargeted(&self) -> bool;

    /// Process anything this action in particular might try to do.
    fn tick(
        &mut self,
        actor: &mut T,
        ctx: &ProcessingContext,
        board: &mut Board,
    ) -> Option<PostProcessResult>;

    /// Check to determine if this action's conditions are still met.
    fn is_valid(&self, actor: &T, ctx: &ProcessingContext, board: &Board) -> bool;

    /// Finish the action, completing any necessary cleanup tasks.
    fn end(self, actor: &mut T);

    /// Get the next position that this target would like to move to
    fn get_movement(&self, actor: &T, ctx: &ProcessingContext, board: &Board) -> Option<Pos>;

    /// If true, we should be dropped and never tried again.
    fn completed(&self) -> bool;

    #[allow(dead_code)] // I would like to use this again eventually
    /// Utility function to check if anything of a given type exists on the board.
    fn any_available_matches<F>(actor: &T, board: &Board, ctx: &ProcessingContext, check: F) -> bool
    where
        F: Fn(&T, &Entity, &ProcessingContext, &Board) -> bool,
    {
        let ent_ctx = ctx.entity_context.read().unwrap();

        for position in ent_ctx.get_active_positions() {
            if position == ctx.position {
                continue;
            }
            if Self::specific_pos_matches(actor, position, board, ctx, &check) {
                return true;
            }
        }

        false
    }

    /// Similar to the previous function, except this one checks specifically if such an entity exists on the desired tile
    fn specific_pos_matches<F>(
        actor: &T,
        position: Pos,
        board: &Board,
        ctx: &ProcessingContext,
        check: F,
    ) -> bool
    where
        F: Fn(&T, &Entity, &ProcessingContext, &Board) -> bool,
    {
        let tile = board.get_tile_from_pos(position);
        match tile.get_entity() {
            None => (),
            Some(e) => {
                if check(actor, e, ctx, board) {
                    return true;
                }
            }
        };
        false
    }
}

/// Provides a simple interface for slapping movement into things.
pub trait Pathfinder {
    /// Get the next viable node for something to move to.
    /// start: The starting position of the entity.
    /// end: The ending position of the entity.
    /// board: The board on which to navigate.
    /// max_x: The maximum distance that the entity can travel in the x direction for one step.
    /// max_y: The maximum distance that the entity can travel in the y direction for one step.
    /// method: A pathfinding method that takes in some information and possibly returns a path.
    fn get_next_node<F, T>(
        start: Pos,
        board: &Board,
        max_x: usize,
        max_y: usize,
        method: F,
        check: T,
    ) -> Option<Pos>
    where
        F: FnOnce(Pos, &Board, T) -> Option<Vec<Pos>>,
        T: Fn(Pos, &Board) -> bool,
    {
        let mut last_good_pos = None;
        let res = method(start, board, check);
        // dbg!(&res);
        if let Some(res) = res {
            for path_pos in res {
                if path_pos.x.abs_diff(start.x) <= max_x
                    && path_pos.y.abs_diff(start.y) <= max_y
                    && path_pos != start
                {
                    last_good_pos = Some(path_pos)
                } else {
                    return last_good_pos;
                }
            }
        }
        last_good_pos
    }

    // a star would be really sweet but I can't really reason it out rn
    fn find_path_bfs<T>(start: Pos, board: &Board, check: T) -> Option<Vec<Pos>>
    where
        T: Fn(Pos, &Board) -> bool,
    {
        let (x, y) = board.dims();
        let y_max = y - 1;
        let x_max = x - 1;

        let mut visited: HashMap<Pos, Option<Pos>> = HashMap::new();
        let mut horizon: VecDeque<Pos> = VecDeque::new();

        horizon.push_back(start);
        visited.insert(start, None);

        let mut next_to_visit = None;
        let mut found_goal = false;

        info!("Starting bfs from {start:?}");

        while !horizon.is_empty() {
            next_to_visit = horizon.pop_front(); // again, safe because we just verified it isn't empty
            let cur_pos = next_to_visit.unwrap();
            if check(cur_pos, board) && start != cur_pos {
                info!("{cur_pos:?} is the goal!");
                found_goal = true;
                break;
            }

            // it isn't our goal
            let tile = board.get_tile_from_pos(cur_pos);
            if tile.is_occupied() && cur_pos != start {
                continue;
            }

            // println!("Considering {cur_pos:?}");

            for neighbor in Self::get_adjacent(cur_pos, x_max, y_max) {
                // println!("Pushing back neighbor {neighbor:?}");
                if visited.contains_key(&neighbor) {
                    continue;
                }
                // If we haven't hit our goal and this tile is occupied, don't consider it.

                // let working_state = neighbor;
                let resulting_entry = visited.entry(neighbor).or_insert(Some(cur_pos));

                if resulting_entry.is_some() {
                    horizon.push_back(neighbor);
                }
            }
        }

        if !found_goal {
            println!("Gave up in bfs");
            return None;
        } else {
            println!("Found our entity at {next_to_visit:?}")
        }

        let mut path: Vec<Pos> = Vec::new();
        let original_end_node = next_to_visit.unwrap();
        let mut parent = &original_end_node;
        // backtrack
        while let Some(Some(next_pos)) = visited.get(parent) {
            path.push(*next_pos);
            parent = next_pos;
        }

        path.reverse();

        // dbg!(path);

        Some(path)
    }

    // again, an iterator here would be lovely
    // but that's a lot of work
    fn get_adjacent(pos: Pos, x_max: usize, y_max: usize) -> Vec<Pos> {
        let x_min = (pos.x as i64 - 1).max(0) as usize;
        let y_min = (pos.y as i64 - 1).max(0) as usize;
        let x_max = (pos.x + 1).min(x_max);
        let y_max = (pos.y + 1).min(y_max);

        let mut ret = vec![];

        for x in x_min..=x_max {
            for y in y_min..=y_max {
                if x != pos.x && y != pos.y {
                    ret.push(Pos { x, y })
                }
            }
        }
        // dbg!(&ret);
        ret
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IdleAction {
    // These vars are mostly just here so we can incorporate them in
    /// If true, and we want to mate and have an adjacent mate, do the do.
    mate_adjacent: bool,
    /// Same for above, but for adjacent food.
    feed_adjacent: bool,
}

impl IdleAction {
    pub fn new(mate_adjacent: bool, feed_adjacent: bool) -> Self {
        IdleAction {
            mate_adjacent,
            feed_adjacent,
        }
    }
}

impl AIAction<Animals> for IdleAction {
    fn initialize(&mut self) {}

    fn is_valid_target(_: &Animals, _: &Entity, _: &ProcessingContext, _: &Board) -> bool {
        true
    }

    fn untargeted(&self) -> bool {
        true // this one can always be called
    }

    fn completed(&self) -> bool {
        false // never finishes
    }

    fn priority(&self) -> usize {
        0 // should only be doing this if you have nothing better to
    }

    fn get_action_desc(&self) -> String {
        "idle".to_owned()
    }

    fn tick(
        &mut self,
        actor: &mut Animals,
        ctx: &ProcessingContext,
        board: &mut Board,
    ) -> Option<PostProcessResult> {
        // TODO maybe random walk

        let mut can_mate = actor.can_mate();
        let mut should_try_to_eat = actor.should_consider_eating();

        if (can_mate && self.mate_adjacent) || (should_try_to_eat && self.feed_adjacent) {
            // one loop immediately around us.
            // TODO SHOULD PROBABLY MAKE SOME KIND OF STATE MACHINE LIKE BEHAVIOR FOR CREATURES
            for p in board.range(1, false, ctx.position) {
                if !can_mate && !should_try_to_eat {
                    break;
                }
                // println!("{self:?} is looking to mate ({can_mate}) and eat ({should_try_to_eat})");
                let tile = board.get_tile_mut_from_pos(p);
                if let Some(e) = tile.get_entity_mut() {
                    match e {
                        Entity::NonLiving(_) => (),
                        Entity::Living(l) => match l {
                            Living::Animals(a) => {
                                if should_try_to_eat && actor.can_eat(a) {
                                    actor.eat(a);
                                    should_try_to_eat = false;
                                }
                                if can_mate && actor.compatible_mate(a) {
                                    info!("Trying to mate!");
                                    actor.mate(a);
                                    can_mate = false;
                                }
                            }
                            Living::Plants(p) => {
                                if should_try_to_eat && actor.can_eat(p) {
                                    info!("{self:?} has eaten a tasty plant!");
                                    actor.eat(p);
                                    should_try_to_eat = false;
                                }
                            }
                        },
                    }
                }
            }
        }
        None
    }

    fn is_valid(&self, _: &Animals, _: &ProcessingContext, _: &Board) -> bool {
        true
    }

    fn end(self, _: &mut Animals) {} // do nothing

    fn get_movement(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> Option<Pos> {
        debug!("Idle moving!");
        let mut new_pos = ctx.position;

        if matches!(actor.get_life_status(), LifeStatus::Dead) {
            return None; // don't dance if you're dead
        }

        let mut rng: ThreadRng = rand::thread_rng();

        if rng.gen_bool(0.3) {
            // 50% chance they will just do nothing
            return None;
        }

        for _ in 0..5 {
            match actor {
                Animals::Fish(a) | Animals::Crab(a) | Animals::Shark(a) => {
                    let (max_x, max_y) = a.get_max_movespeed();
                    let mut new_x_offset = rng.gen_range(-(max_x as i64)..=(max_x as i64));
                    let mut new_y_offset = rng.gen_range(-(max_y as i64)..=(max_y as i64));

                    debug!("moving to {new_x_offset}, {new_y_offset}");
                    // don't underflow bestie
                    if (new_pos.x as i64) + new_x_offset < 0 {
                        new_x_offset = 0;
                    }
                    if (new_pos.y as i64) + new_y_offset < 0 {
                        new_y_offset = 0;
                    }
                    new_pos.x = (new_pos.x as i64 + new_x_offset) as usize;
                    new_pos.y = (new_pos.y as i64 + new_y_offset) as usize;
                    info!("{a:?} moving to {new_pos:?}");
                    if board.is_valid_pos(new_pos) {
                        let target_tile = board.get_tile_from_pos(new_pos);
                        if !target_tile.is_occupied() {
                            return Some(new_pos);
                        }
                    }
                }
            }
        }
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MateAction {
    done: bool,
}

impl Pathfinder for MateAction {}

impl MateAction {
    pub fn new() -> Self {
        Self { done: false }
    }
}

impl Default for MateAction {
    fn default() -> Self {
        Self::new()
    }
}

impl AIAction<Animals> for MateAction {
    fn initialize(&mut self) {
        todo!()
    }

    fn priority(&self) -> usize {
        1
    }

    fn untargeted(&self) -> bool {
        false
    }

    fn is_valid_target(actor: &Animals, target: &Entity, _: &ProcessingContext, _: &Board) -> bool {
        if let Entity::Living(Living::Animals(a)) = target {
            actor.compatible_mate(a) && a != actor
        } else {
            false
        }
    }

    fn get_action_desc(&self) -> String {
        "looking for a mate".to_owned()
    }

    fn tick(
        &mut self,
        actor: &mut Animals,
        ctx: &ProcessingContext,
        board: &mut Board,
    ) -> Option<PostProcessResult> {
        debug!("Tick!");

        if !actor.can_mate() {
            debug!("We should stop trying to mate!");
            self.done = true;
            return None;
        }
        // let lock = ctx.entity_context.lock().unwrap();
        // let entity_pos = lock.get_active_entries().get(&self.target);

        // TODO AFTER DINNER
        // CHASING DOWN ENTITIES FOR THIS IS DUMB AND STUPID
        // JUST FIND THE CLOSEST ONE TO US AND EAT IT

        for pos in board.range(1, false, ctx.position) {
            if self.done {
                return None;
            }

            let tile = board.get_tile_mut_from_pos(pos);
            if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity_mut() {
                if actor.compatible_mate(a) && a != actor {
                    println!("{self:?} has mated with {a:?}!");
                    actor.mate(a);
                    self.done = true;
                }
            }
        }
        None
    }

    fn is_valid(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> bool {
        actor.can_mate()
            && !self.done
            && Self::any_available_matches(actor, board, ctx, Self::is_valid_target)
    }

    fn end(self, _: &mut Animals) {}

    fn get_movement(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> Option<Pos> {
        let mut rng = rand::thread_rng();

        if !self.is_valid(actor, ctx, board) {
            // skip the expensive stuff
            println!("We were trying to move as per our behavior, but it wasn't valid...");
            return actor.random_walk(ctx.position, &mut rng, board);
        }
        // if let Some(p) = entity_pos {
        let (x, y) = actor.max_speeds();

        let check = |pos: Pos, board: &Board| {
            let tile = board.get_tile_from_pos(pos);
            debug!("Checking tile at {pos:?}");
            if let Some(ent) = tile.get_entity() {
                debug!("Checking if we can mate with {ent:?} at {pos:?}");
                if !actor.can_mate() {
                    debug!("...but we aren't ready?")
                }
                match ent {
                    Entity::NonLiving(_) => false,
                    Entity::Living(l) => match l {
                        Living::Animals(a) => actor.compatible_mate(a) && actor.can_mate(),
                        _ => false,
                    },
                }
            } else {
                false
            }
        };

        let ret = Self::get_next_node(ctx.position, board, x, y, Self::find_path_bfs, check);

        if let Some(p) = ret {
            if let Some(ent) = &board.get_tile_from_pos(p).get_entity() {
                println!(
                    "{} is trying to mate with {}!",
                    actor.get_display_char(),
                    ent.get_display_char()
                );
            }
        } else {
            // ugh, there's some weird bug
            return actor.random_walk(ctx.position, &mut rng, board);
        }
        ret
    }

    fn completed(&self) -> bool {
        self.done
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EatAction {
    very_hungry: bool,
    should_keep_chasing: bool,
}

impl Pathfinder for EatAction {}

impl EatAction {
    pub fn new(starving: bool) -> Self {
        Self {
            // target,
            very_hungry: starving,
            should_keep_chasing: true,
        }
    }
}

impl AIAction<Animals> for EatAction {
    fn untargeted(&self) -> bool {
        false
    }

    fn is_valid_target(actor: &Animals, target: &Entity, _: &ProcessingContext, _: &Board) -> bool {
        match target {
            Entity::NonLiving(_) => false,
            Entity::Living(l) => match l {
                Living::Plants(p) => actor.can_eat(p),
                Living::Animals(a) => actor.can_eat(a),
            },
        }
    }

    fn initialize(&mut self) {
        todo!()
    }

    fn completed(&self) -> bool {
        self.should_keep_chasing
    }

    fn priority(&self) -> usize {
        if self.very_hungry {
            2
        } else {
            1
        } // boost priority if we're starving
    }

    fn get_action_desc(&self) -> String {
        "eating".to_string()
    }

    fn tick(
        &mut self,
        actor: &mut Animals,
        ctx: &ProcessingContext,
        board: &mut Board,
    ) -> Option<PostProcessResult> {
        debug!("Tick!");

        if !actor.should_consider_eating() {
            debug!("We should stop trying to eat!");
            self.should_keep_chasing = false;
            return None;
        }

        for pos in board.range(1, false, ctx.position) {
            if !self.should_keep_chasing {
                return None;
            }

            let tile = board.get_tile_mut_from_pos(pos);
            if let Some(ent) = tile.get_entity_mut() {
                match ent {
                    Entity::NonLiving(_) => (),
                    Entity::Living(l) => match l {
                        Living::Animals(a) => {
                            if actor.can_eat(a) && a != actor {
                                info!("{self:?} has eaten an animal!");
                                actor.eat(a);
                                self.should_keep_chasing = false;
                            }
                        }
                        Living::Plants(p) => {
                            if actor.can_eat(p) {
                                info!("{self:?} has eaten a tasty plant!");
                                actor.eat(p);
                                self.should_keep_chasing = false;
                            }
                        }
                    },
                }
                // }
            }
        }
        None
    }

    fn is_valid(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> bool {
        actor.should_consider_eating()
            && Self::any_available_matches(actor, board, ctx, Self::is_valid_target)
    }

    fn end(self, _: &mut Animals) {}

    fn get_movement(&self, actor: &Animals, ctx: &ProcessingContext, board: &Board) -> Option<Pos> {
        // let lock = ctx.entity_context.write().unwrap();

        let mut rng = rand::thread_rng();

        if !self.is_valid(actor, ctx, board) {
            // skip the expensive stuff
            return actor.random_walk(ctx.position, &mut rng, board);
        }
        // if let Some(p) = entity_pos {
        let (x, y) = actor.max_speeds();

        let check = |pos: Pos, board: &Board| {
            let tile = board.get_tile_from_pos(pos);
            debug!("Checking tile at {pos:?}");
            if let Some(ent) = tile.get_entity() {
                debug!("Checking if we can eat {ent:?} at {pos:?}");
                if !actor.should_consider_eating() {
                    debug!("...but we aren't even hungry?")
                }
                match ent {
                    Entity::NonLiving(_) => false,
                    Entity::Living(l) => match l {
                        Living::Animals(a) => actor.can_eat(a) && a != actor,
                        Living::Plants(p) => actor.can_eat(p),
                    },
                }
            } else {
                false
            }
        };

        let ret = Self::get_next_node(ctx.position, board, x, y, Self::find_path_bfs, check);

        if let Some(p) = ret {
            if let Some(ent) = &board.get_tile_from_pos(p).get_entity() {
                println!(
                    "{} is trying to eat {}!",
                    actor.get_display_char(),
                    ent.get_display_char()
                );
            }
        } else {
            // if we didn't move anywhere, just try to go somewhere
            return actor.random_walk(ctx.position, &mut rng, board);
        }
        ret
    }
}
