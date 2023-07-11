mod ai_controller;
pub mod element_traits;
pub mod entities;
mod entity_control;
pub mod game_board;
pub mod game_events;
mod interactions;
mod test_utils;
mod tests;
use ai_controller::AIControlled;
use eframe::egui;
use entity_control::{EntityManager, TrackedEntity};
use std::thread::sleep;
use std::{
    sync::{mpsc::Sender, Arc, RwLock},
    time::Duration,
};

use futures::{executor::block_on, future::join_all};
// use async_std;

use element_traits::{Lives, PostProcessResult, Processing, ProcessingContext};
use entities::{Entity, Living, PTUIDisplay};
use game_board::{populate_board, Board, Pos, Tile};
use game_events::GameEvents;

use log::{debug, error, info}; // todo configure logging framework

use rand::{self, Rng};

use crate::game_events::Event;

/// Our sandbox is like our "game engine"
#[derive(Debug)]
pub struct Sandbox {
    /// The game board
    board: Board,
    /// How many ticks we've performed so far.
    clock: usize,
    /// How many times per second (minimum) our game loop should
    tick_rate: f64,
    /// The tick of the last event.
    last_event: usize,
    /// The general entity context.
    entity_context: Arc<RwLock<EntityManager>>,
}

impl Sandbox {
    pub fn new(board: Board, tick_rate: f64, entity_context: Arc<RwLock<EntityManager>>) -> Self {
        Self {
            board,
            clock: 0,
            tick_rate,
            last_event: 0,
            entity_context,
        }
    }

    /// Get a list of all the important entities currently on the board.
    pub fn get_important_entities(&self) -> Vec<Pos> {
        self.entity_context.read().unwrap().get_active_positions()
    }

    fn get_entity_info(&self) -> Vec<String> {
        let mut entities_info = Vec::new();
        for pos in self.get_important_entities() {
            let entity = self
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap();
            match entity {
                Entity::Living(e) => match e {
                    Living::Plants(_) => (),
                    Living::Animals(a) => entities_info.push(format!(
                        "{}: {} Health = {}",
                        a.get_id().unwrap().get_id_val(),
                        a.get_display_char(),
                        a.get_health(),
                    )),
                },
                // Don't care about living entities
                Entity::NonLiving(_) => (),
            }
        }
        entities_info.sort();
        entities_info
    }

    /// Perform some sanity checks in between different segments of the game loop.
    /// These are mostly checks to make sure our invariants are being upheld.
    /// This should probably be removed once we go gold.
    /// after: The step this one followed.
    /// Note that this function will panic if its invariants fail! It's to ensure that we don't end up with bad behavior
    fn sanity_check(&self, after: &str) {
        if !cfg!(debug_assertions) {
            // disable these checks in release
            return;
        }
        let important_entities = self.get_important_entities();
        if important_entities.is_empty() {
            info!("Important entities list is empty!");
        }
        for pos in &important_entities {
            let tile = self.board.get_tile_from_pos(*pos);
            if !tile.is_occupied() {
                panic!("Checking after {after}: {tile:?} at pos {pos:?} at was in the processing list, while its entity was none!")
            }
        }
        let slice = &[&important_entities];
        let duplicates_exist = (1..slice.len()).any(|i| slice[i..].contains(&slice[i - 1])); // borrowed from stack overflow
        if duplicates_exist {
            panic!("Checking after {after}: Duplicate positions exist in the active list!")
        }
    }

    pub fn run_game_loop(
        &mut self,
        tx: Sender<(String, Vec<String>, String, Sender<bool>)>,
        ctx: egui::Context,
    ) {
        let sleep_time = (1000.0 / self.tick_rate).floor() as u64;
        let (loop_tx, loop_rx) = std::sync::mpsc::channel();
        loop {
            let loop_start = std::time::Instant::now();
            self.handle_moves();
            self.sanity_check("moves");
            self.handle_processing();
            self.sanity_check("processing");

            block_on(self.handle_late_processing());
            self.sanity_check("late_processing");

            let entity_info = self.get_entity_info();

            let event = self.handle_events();
            let pause = event.is_some();
            self.sanity_check("Events");

            let time_elapsed = loop_start.elapsed();
            let tickrate_in_ms = (1.0 / self.tick_rate) * 1000.0;
            let tickrate_consumed = ((time_elapsed.as_millis() as f64) / tickrate_in_ms) * 100.0; //

            println!("Event loop took {}ms to execute, given a tickrate of {}hz it consumed {:.4}% of the tick.", time_elapsed.as_millis(), self.tick_rate, tickrate_consumed);

            self.clock += 1;
            sleep(Duration::from_millis(sleep_time));
            if !pause {
                let _ = tx.send((
                    self.board.to_string(),
                    entity_info,
                    String::new(),
                    loop_tx.clone(),
                ));
                ctx.request_repaint();
            } else {
                let _ = tx.send((
                    self.board.to_string(),
                    entity_info,
                    event.as_ref().unwrap().get_event_display().clone(),
                    loop_tx.clone(),
                ));
                ctx.request_repaint();
                'outer: loop {
                    if let Ok(user_inp) = loop_rx.try_recv() {
                        event.unwrap().process_event(user_inp, self);
                        loop {
                            if loop_rx.try_recv().is_ok() {
                                break 'outer;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(sleep_time));
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(sleep_time));
                }
            }
        }
    }

    /// Handle the movement for everything interesting on the board
    fn handle_moves(&mut self) {
        // run through all of our pieces and see where they would like to move
        for pos in &self.get_important_entities() {
            let x = pos.x;
            let y = pos.y;
            let tile = self.board.get_tile(y, x);
            let ctx = ProcessingContext {
                position: *pos,
                entity_context: Arc::clone(&self.entity_context),
            };
            let new_move = match tile.get_entity() {
                None => None, // should this panic?
                Some(ent) => {
                    match ent {
                        Entity::NonLiving(_) => None,
                        Entity::Living(l) => {
                            match l {
                                Living::Plants(_) => None, // plants don't move (yet)
                                Living::Animals(a) => a.get_desired_move(&ctx, &self.board),
                            }
                        }
                    }
                }
            };
            if let Some(new_pos) = new_move {
                // check that the new position is available
                if !self.board.is_valid_pos(new_pos) {
                    println!(
                        "Failed to move {:?}: tried to move out of bounds!",
                        tile.get_entity()
                    );
                    continue;
                }
                let other_tile = self.board.get_tile(new_pos.y, new_pos.x);
                if other_tile.is_occupied() {
                    println!(
                        "Failed to move {:?} from {pos:?}: space {new_pos:?} already occupied by {:?}!",
                        tile.get_entity(),
                        other_tile.get_entity()
                    );
                    continue;
                } else {
                    let tile_mut = self.board.get_tile_mut(y, x);
                    let our_entity = tile_mut.remove_entity();
                    let other_tile_mut = self.board.get_tile_mut(new_pos.y, new_pos.x);
                    let _ = other_tile_mut.add_entity(our_entity.unwrap());
                }
            }
        }
    }

    /// Run processing, possibly on a few different entities across the board.
    fn handle_processing(&mut self) {
        // need this before the loop since we're immutably running over it
        for pos in &self.get_important_entities() {
            let tile = self.board.get_tile_mut_from_pos(*pos);
            // pop the entity out from the tile.
            // we can't get a mutable ref to the board with a mutable borrow of the tile outstanding,
            // so we pull it out and return it later.
            // if it doesn't get returned to some tile, then it'll be automatically dropped from the processing list.
            let mut entity = tile.remove_entity();
            let ctx = ProcessingContext {
                position: *pos,
                entity_context: Arc::clone(&self.entity_context),
            };
            let action_hint = match &mut entity {
                None => panic!("Entity at pos {pos:?} was none!"),
                Some(ent) => {
                    match ent {
                        Entity::NonLiving(_) => None,
                        Entity::Living(l) => {
                            match l {
                                Living::Plants(p) => p.process(&mut self.board, ctx),
                                Living::Animals(a) => a.process(&mut self.board, ctx), // returns an option
                            }
                        }
                    }
                }
            };

            // Use this helper variable to indicate that you want to automatically add the entity back afterwards.
            // Note that since we removed the entity from the board entirely, we'd probably end up in a nasty situation if there are duplicate entities
            let mut add_self_after = true;

            match action_hint {
                None => (),
                Some(h) => {
                    match h {
                        PostProcessResult::TryToAddEntities(_)
                        | PostProcessResult::TryToAddEntitiesAndKillMe(_) => todo!(),
                        PostProcessResult::MarkTheseAsInteresting(these) => {
                            info!("Marked these ({these:?}) as interesting");
                            add_self_after = true;
                        }
                        PostProcessResult::Delete => {
                            info!("entity {entity:?} at {pos:?} was deleted in process");
                            add_self_after = false;
                        }
                        PostProcessResult::ReplaceMeWith(e) => {
                            let tile = self.board.get_tile_mut_from_pos(*pos);
                            let old_e = tile.remove_entity(); // drop it on the floor
                            info!("Replacing {old_e:?} with {e:?}");
                            let _ = tile.add_entity(e); // and slap the new one in
                            add_self_after = false; // but don't manually re-add our entity to the tile. It's gone.
                        }
                    };
                }
            }

            if add_self_after {
                // borrow again, so hopefully the compiler realizes it can drop the old board borrow
                let tile = self.board.get_tile_mut_from_pos(*pos);
                if let Err(ent) = tile.add_entity(entity.unwrap()) {
                    error!("While processing, {:?} ended up on tile {pos:?}, which is occupied by {:?}", tile.get_entity(), ent);
                    error!("{ent:?} will be dropped!");
                }
            }
        }
    }

    /// Helper function to create futures for late-processing entities.
    /// This function takes in an owned entity, runs its late processing, and then returns all the components.
    /// It takes in and returns its position so that we can reconstruct its place later on after we've joined all the futures
    async fn late_process_entity(
        ent: Entity,
        position: Pos,
    ) -> Option<(Entity, Pos, Option<PostProcessResult>)> {
        let mut ent = ent;
        match &mut ent {
            Entity::Living(l) => match l {
                Living::Plants(p) => {
                    let hint = p.late_process().await;
                    Some((ent, position, hint))
                }
                Living::Animals(a) => {
                    let hint = a.late_process().await;
                    Some((ent, position, hint))
                }
            },
            _ => None,
        }
    }

    /// Run all of our late-processing behavior.
    /// Stuff in here should be designed as atomically as possible, and should only involve things that deal with the creature itself.
    async fn handle_late_processing(&mut self) {
        // need this before the loop since we're immutably running over it
        let mut new_important_entites: Vec<Pos> = vec![];
        // Hang onto all the futures we'll be working with.
        let mut futures = vec![];
        // run through all the important entities and slurp all the entities out into futures.
        for pos in &self.get_important_entities() {
            let x = pos.x;
            let y = pos.y;
            let tile = self.board.get_tile_mut(y, x);
            let entity = tile.remove_entity();
            if let Some(e) = entity {
                futures.push(Self::late_process_entity(e, *pos));
            }
        }

        // wait for them all to finish
        let results = join_all(futures);
        let results = async_std::task::spawn(async move { results.await }).await;

        // run through the results. it returns the positions that the new entities are on.
        for res in results {
            let mut re_insert_self = true;
            if res.is_none() {
                continue; // there was nothing of real note here, keep going
            }
            let (ent, pos, hint) = res.unwrap();
            let tile = self.board.get_tile_mut_from_pos(pos);
            // check to see if we're looking at any special behavior
            match hint {
                None => (),
                Some(PostProcessResult::Delete) => {
                    // Since we have a mutable reference, we can just. destroy ourselves
                    let entity = tile.remove_entity();
                    debug!("{entity:?} was deleted.");
                    re_insert_self = false;
                    // don't push to the new important entities list, since we're removing ourselves
                    // make sure we remove ourselves from the processing list, too.
                }
                Some(PostProcessResult::ReplaceMeWith(e)) => {
                    // same as before.
                    // drop the old entity on the floor lol
                    tile.remove_entity();
                    let _ = tile.add_entity(e);
                    new_important_entites.push(pos);
                    re_insert_self = false;
                }
                Some(
                    PostProcessResult::TryToAddEntities(_)
                    | PostProcessResult::TryToAddEntitiesAndKillMe(_),
                ) => (),
                Some(PostProcessResult::MarkTheseAsInteresting(mut interest)) => {
                    new_important_entites.append(&mut interest);
                    if !tile.is_occupied() {
                        error!("Our current entity was none after late processing and marking interesting")
                    }
                }
            }

            if re_insert_self {
                let tile = self.board.get_tile_mut_from_pos(pos);
                if let Err(ent) = tile.add_entity(ent) {
                    error!("While processing, {:?} ended up on tile {pos:?}, which is occupied by {:?}", tile.get_entity(), ent);
                    error!("{ent:?} will be dropped!");
                    // god what do we even do here, delete it? Hopefully this should never happen
                }
                new_important_entites.push(pos);
            }
        }
    }

    /// Determine if an event occurs
    fn handle_events(&mut self) -> Option<GameEvents> {
        let mut rng = rand::thread_rng();
        let event_chance = rng.gen_range(1..=1000);
        dbg!(event_chance + self.last_event);
        // We start with a 1% chance of generating an event
        if event_chance + self.last_event >= 995 {
            // Reset back to 5% chance
            self.last_event = 0;

            // Return a game event
            // I think here we can update it so the event holds the information for the current
            // iteration of the sandbox that it is reliant on.  The problem is that we need to stop
            // the loop when we get an event, otherwise it will keep on computing without
            // the event result.
            return Some(game_events::get_rand_event(rng.gen_range(0..3)));
        } else if self.clock % 10 == 0 {
            // Increase the chance of getting an event by 1%
            self.last_event += 10;
        }
        None
    }
}

/// Initialize a game board.
pub fn initialize_board(
    row: usize,
    col: usize,
    fish: usize,
    crab: usize,
    shark: usize,
    tx: Sender<(String, Vec<String>, String, Sender<bool>)>,
    ctx: egui::Context,
) {
    let entity_manager = EntityManager::new();
    let mut game_board = Board::new(row, col, Arc::clone(&entity_manager));
    let important_entities = populate_board(&mut game_board, fish, crab, shark);

    run_simulation(
        game_board,
        important_entities,
        3.0,
        false,
        entity_manager,
        tx,
        ctx,
    );
}

/// Spin off the simulation in a new thread.
fn run_simulation(
    board: Board,
    _: Vec<Pos>,
    tick_rate: f64,
    _: bool,
    entity_context: Arc<RwLock<EntityManager>>,
    tx: Sender<(String, Vec<String>, String, Sender<bool>)>,
    ctx: egui::Context,
) {
    println!("Starting!");
    println!("{}", board);
    // Spawn the game loop thread
    std::thread::spawn(move || {
        Sandbox::new(board, tick_rate, entity_context).run_game_loop(tx.clone(), ctx);
    });
}
