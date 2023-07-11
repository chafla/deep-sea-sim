use std::sync::Arc;

use async_std::task::block_on;

use crate::{
    entities::{Entity, NonAbstractTaxonomy},
    entity_control::EntityManager,
    game_events, populate_board, Board, Pos, Sandbox,
};

use crate::game_board::test_utils::*;

/// Testbed for building up and iterating on a simple, pre-built sandbox.
pub struct TestBed {
    pub sandbox: Sandbox,
}

#[allow(dead_code)]
impl TestBed {
    /// Create a new testbed with exactly the entities you want.
    pub fn new_with_entities(rows: usize, cols: usize, entities: Vec<(Pos, Entity)>) -> Self {
        let em = EntityManager::new();
        let board = create_board(rows, cols, entities, &em);
        let sandbox = create_sandbox(board, 1.0, &em); // should be manually ticked anyway

        Self { sandbox }
    }

    /// Create a new testbed with some pre-defined entities.
    pub fn new_populated<T>(rows: usize, cols: usize, entities: Vec<(Pos, T)>) -> Self
    where
        T: NonAbstractTaxonomy,
    {
        let true_entities = entities
            .into_iter()
            .map(|(pos, ent)| (pos, ent.create_new(None)))
            .collect();
        Self::new_with_entities(rows, cols, true_entities)
    }

    /// Create a new board the way the game logic would have made it.
    pub fn new_default(rows: usize, cols: usize, fish: usize, crab: usize, shark: usize) -> Self {
        let em = EntityManager::new();
        let mut board = Board::new(rows, cols, Arc::clone(&em));
        populate_board(&mut board, fish, crab, shark);
        Self {
            sandbox: create_sandbox(board, 1.0, &em),
        }
    }

    /// Simplified version to just run n steps without checking
    pub fn run_n_full_steps(&mut self, steps: usize) {
        self.run_n_steps_no_checks(steps, true, true, true, true)
    }

    pub fn run_n_steps_no_checks(
        &mut self,
        steps: usize,
        run_moves: bool,
        process: bool,
        late_process: bool,
        run_events: bool,
    ) {
        let check_1 = |_: &Sandbox| ();
        let check_2 = |_: &Sandbox| ();

        self.run_n_steps(
            steps,
            run_moves,
            process,
            late_process,
            run_events,
            check_1,
            check_2,
        );
    }

    /// Run the game loop for a given amount of steps, as fast as possible, setting up for inspection.
    /// Set each different value to determine the steps that get run in the loop.
    /// Pass the different checks to execute some check on each iteration step, or each loop.
    #[allow(clippy::too_many_arguments)]
    pub fn run_n_steps<F, G>(
        &mut self,
        steps: usize,
        run_moves: bool,
        process: bool,
        late_process: bool,
        run_events: bool,
        post_step_check: F,
        post_loop_check: G,
    ) where
        // remember: closures have different inherent types, so we have to take in two separate types
        F: Fn(&Sandbox),
        G: Fn(&Sandbox),
    {
        for step in 0..steps {
            if run_moves {
                self.sandbox.handle_moves();
                post_step_check(&self.sandbox)
            }
            if process {
                self.sandbox.handle_processing();
                post_step_check(&self.sandbox)
            }

            if late_process {
                block_on(self.sandbox.handle_late_processing());
                post_step_check(&self.sandbox)
            }

            if run_events {
                self.sandbox.handle_events();
                post_step_check(&self.sandbox)
            }

            post_loop_check(&self.sandbox);

            println!("step {step} completed");
        }
    }

    /// Separate function to test events in order to simulate
    /// input and test board update
    pub fn run_n_steps_events<F>(&mut self, steps: usize, post_step_check: F) -> bool
    where
        F: Fn(&mut Sandbox, Option<game_events::GameEvents>) -> bool,
    {
        let mut test_occured;
        for _ in 0..steps {
            let event = self.sandbox.handle_events();
            test_occured = post_step_check(&mut self.sandbox, event);
            if test_occured {
                return true;
            }
        }
        false
    }

    pub fn get_entity_at_pos(&self, pos: Pos) -> Option<&Entity> {
        self.sandbox
            .board
            .get_tile_from_pos(pos)
            .get_entity()
            .as_ref()
    }

    pub fn get_entity_at_pos_mut(&mut self, pos: Pos) -> Option<&mut Entity> {
        self.sandbox
            .board
            .get_tile_mut_from_pos(pos)
            .get_entity_mut()
            .as_mut()
    }
}
