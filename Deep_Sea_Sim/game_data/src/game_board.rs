use std::fmt::{Display, Write};
use std::sync::{Arc, RwLock};

use log::debug;
use rand::Rng;

use crate::entities::animals::ConcreteAnimals;
use crate::entities::nonliving::ConcreteDecorations;
use crate::entities::plants::ConcretePlants;
use crate::entities::{generate_creatures, Entity, NonAbstractTaxonomy, PTUIDisplay};
use crate::entity_control::{EntityManager, TrackedEntity};

/// Percentage of tiles to fill with decorations after adding creatures.
const DECORATION_PERCENT: f64 = 0.1;

/// Percentage of tiles to fill with plants after adding creatures.
const PLANT_PERCENTAGE: f64 = 0.15;

/// A position somewhere on the board.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

impl From<(usize, usize)> for Pos {
    fn from(value: (usize, usize)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}

impl Pos {
    /// Get euclidean distance to another position
    pub fn dist_to(&self, other: &Self) -> usize {
        let sum =
            (other.x as f64 - self.x as f64).powf(2.0) + (other.y as f64 - self.y as f64).powf(2.0);
        sum.sqrt().floor() as usize
    }
}

/// A tile on the game board.
#[derive(Debug, Clone)]
pub struct Tile {
    /// The entity on a board.
    entity: Option<Entity>,
    /// The entity manager for a board.
    entity_manager: Arc<RwLock<EntityManager>>,
    /// This tile's position.
    position: Pos,
}

impl Tile {
    pub fn is_occupied(&self) -> bool {
        self.entity.is_some()
    }

    pub fn get_entity(&self) -> &Option<Entity> {
        &self.entity
    }

    pub fn get_entity_mut(&mut self) -> &mut Option<Entity> {
        &mut self.entity
    }

    pub fn remove_entity(&mut self) -> Option<Entity> {
        let mut res = self.entity.take();
        if let Some(ent) = &mut res {
            if ent.tracked() {
                let mut em = self.entity_manager.write().unwrap();
                if let Some(id) = ent.get_id() {
                    em.update_position(id, None)
                }
            }
        }

        res
    }

    #[allow(clippy::result_large_err)]
    /// Insert an entry. If we're already occupied, returns an error with the passed value.
    pub fn add_entity(&mut self, mut entity: Entity) -> Result<(), Entity> {
        if self.entity.is_some() {
            Err(entity)
        } else {
            if entity.tracked() {
                let mut em = self.entity_manager.write().unwrap();
                let id = if let Some(ent_id) = entity.get_id() {
                    ent_id
                } else {
                    em.register_new_entity(self.position, &mut entity)
                };

                em.update_position(id, Some(self.position));
            } else {
                debug!("Added an untracked entity to the tile")
            }
            self.entity = Some(entity);
            Ok(())
        }
    }
}

/// The board, holding the 2-D vector representation of the game tiles.
#[derive(Debug)]
pub struct Board {
    /// Game tiles making up the game board.
    board: Vec<Vec<Tile>>,
}

impl Board {
    pub fn new(rows: usize, cols: usize, entity_manager: Arc<RwLock<EntityManager>>) -> Self {
        let mut board = vec![
            vec![
                Tile {
                    entity: None,
                    entity_manager: Arc::clone(&entity_manager),
                    position: Pos { x: 0, y: 0 }
                };
                cols
            ];
            rows
        ];
        // whatever you say, clippy
        for (i, row) in board.iter_mut().enumerate().take(rows) {
            for (j, tile) in row.iter_mut().enumerate().take(cols) {
                tile.position = Pos { x: j, y: i }
            }
        }
        Self {
            // positions are dummy values and will be updated shortly
            board,
        }
    }

    /// Get the dimensions of the game board. Returned as (x, y)
    pub fn dims(&self) -> (usize, usize) {
        let y = self.board.len();
        let x = self.board[0].len();
        (x, y)
    }

    pub fn get_tile(&self, row: usize, col: usize) -> &Tile {
        &self.board[row][col]
    }

    pub fn get_tile_mut(&mut self, row: usize, col: usize) -> &mut Tile {
        &mut self.board[row][col]
    }

    pub fn get_tile_from_pos(&self, pos: Pos) -> &Tile {
        &self.board[pos.y][pos.x]
    }

    pub fn get_tile_mut_from_pos(&mut self, pos: Pos) -> &mut Tile {
        &mut self.board[pos.y][pos.x]
    }

    /// simply check if a given position is valid insofar as it's in bounds. Don't worry about entities.
    pub fn is_valid_pos(&self, pos: Pos) -> bool {
        // no need to check if less than zero because of usize
        pos.y < self.board.len() && pos.x < self.board[0].len()
    }

    pub fn range(&self, radius: usize, include_self: bool, center: Pos) -> Vec<Pos> {
        let mut ret = Vec::<Pos>::new();
        let max_y = self.board.len() - 1;
        let max_x = self.board[max_y].len() - 1;

        // if both are zero and we're in include self then just return ourselves
        // if you're doing this though...why

        // Yes, this is busy.
        // what we're doing is going from at least 0 to at most the maximum range of the board - 1, in a radius around us.
        for i in (f64::max(center.y as f64 - radius as f64, 0.0) as usize)
            ..=usize::min(center.y + radius, max_y)
        {
            for j in (f64::max(center.x as f64 - radius as f64, 0.0) as usize)
                ..=usize::min(center.x + radius, max_x)
            {
                if !include_self && i == center.y && j == center.x {
                    continue;
                }
                ret.push(Pos { x: j, y: i });
            }
        }

        ret
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for y in 0..self.board.len() {
            for x in 0..self.board[y].len() {
                let tile = self.get_tile(y, x);
                f.write_char('\u{200B}')?; // zero width space
                if let Some(ent) = &tile.entity {
                    let ch = ent.get_display_char();
                    f.write_char(ch)?;
                    // some glyphs are (annoyingly) half-size. Try to add a half-space to pad them out.
                    // note: this is why I made rocks moyai in the first place.
                    // if matches!(ch, '🪴' | '🪨') {
                    //     f.write_char(' ')?;
                    // }
                } else {
                    f.write_char('⬛')?;
                }
                // f.write_char(c)?;
            }
            // f.write_char('.')?;  // use periods to mark grid spaces
            f.write_char('\n')?;
        }
        Ok(())
    }
}

/// Attempt to populate the board as best as possible.
/// Returns a vector of the locations of new elements, as well as a usize of the elements we were unable to place in time.
pub fn populate_board(board: &mut Board, fish: usize, crab: usize, shark: usize) -> Vec<Pos> {
    let board_rows = board.board.len();
    let board_cols = board.board[board_rows - 1].len();
    let board_size = board_rows * board_cols;

    if board_size == 0 {
        panic!("Cannot generate a zero-size board!");
    }

    if board_size < fish + crab + shark {
        panic!(
            "More creatures were given ({}) than there are spaces on the board ({})!",
            fish + crab + shark,
            board_size
        )
    }

    let fish = generate_creatures(fish, ConcreteAnimals::Fish);
    let crab = generate_creatures(crab, ConcreteAnimals::Crab);
    let shark = generate_creatures(shark, ConcreteAnimals::Shark);

    let mut rng = rand::thread_rng();

    // let's be clever about this and select a random set of tiles

    let creatures = vec![fish, crab, shark];
    let attempts = 10;
    let mut important_tiles = vec![];

    // set up tiles
    for row in 0..board.board.len() {
        for col in 0..board.board[row].len() {
            let tile = &mut board.board[row][col];
            tile.position = Pos { x: col, y: row };
        }
    }

    for creatures_of_kind in creatures {
        'creature: for creature in creatures_of_kind {
            // try 5 times to place a creature, or give up if we've gotten horribly unlucky.
            for _ in 0..attempts {
                let selected_row = rng.gen_range(0..board.board.len());
                let selected_col = rng.gen_range(0..board.board[selected_row].len());
                if !board.board[selected_row][selected_col].is_occupied() {
                    board.board[selected_row][selected_col]
                        .add_entity(creature)
                        .unwrap();
                    important_tiles.push(Pos::from((selected_col, selected_row))); // x, y
                    continue 'creature;
                }
            }
            // if we failed five times, then just slot it into the first available spot
            for row in 0..board.board.len() {
                for col in 0..board.board[row].len() {
                    let tile = board.get_tile_mut(row, col);
                    if !tile.is_occupied() {
                        tile.add_entity(creature).unwrap();
                        continue 'creature;
                    }
                }
            }
            // if we make it this far, something's really wrong
            panic!("Gave up while trying to place a {creature:?} after {attempts} attempts, and iteration.")
        }
    }

    // With all of the creatures placed that we need, we can start to insert some of the other Things in our game board.
    // of course, this is after everything has been placed, so there's a perfectly good chance that we'll end up with less space for material if there's too many creatures
    for row in 0..board.board.len() {
        for col in 0..board.board[row].len() {
            if board.board[row][col].is_occupied() {
                continue;
            }
            if rng.gen_bool(DECORATION_PERCENT) {
                let decoration = if rng.gen_bool(0.5) {
                    ConcreteDecorations::Rock.create_new(None)
                } else {
                    ConcreteDecorations::Shell.create_new(None)
                };
                board.board[row][col].add_entity(decoration).unwrap(); // we've checked! it's unoccupied.
            } else if rng.gen_bool(PLANT_PERCENTAGE) {
                let plant_life = ConcretePlants::Kelp.create_new(None);
                board.board[row][col].add_entity(plant_life).unwrap();
                important_tiles.push(Pos::from((col, row)))
            }
        }
    }

    important_tiles
}

/// A set of testing utilities for manipulating the board.
pub mod test_utils {
    use super::*;
    use crate::{Pos, Sandbox};

    pub fn get_positions_of_type<T: NonAbstractTaxonomy>(
        board: &Board,
        creature_type: T,
    ) -> Vec<Pos> {
        let mut positions = vec![];
        for row in 0..board.board.len() {
            for col in 0..board.board[row].len() {
                let tile = board.get_tile(row, col);
                if let Some(ent) = &tile.entity {
                    if creature_type.same_kind(ent) {
                        positions.push(Pos { x: col, y: row })
                    }
                }
            }
        }
        positions
    }

    /// Create a simple board, placing entities where they belong.
    pub fn create_board(
        rows: usize,
        cols: usize,
        entities: Vec<(Pos, Entity)>,
        em: &Arc<RwLock<EntityManager>>,
    ) -> Board {
        // let em = EntityManager::new();

        // let it create its own entity manager
        let mut board = Board::new(rows, cols, Arc::clone(em));

        for (pos, entity) in entities {
            let tile = board.get_tile_mut_from_pos(pos);
            tile.add_entity(entity).unwrap();
        }
        board
    }

    pub fn create_sandbox(
        board: Board,
        tick_rate: f64,
        em: &Arc<RwLock<EntityManager>>,
    ) -> Sandbox {
        Sandbox::new(board, tick_rate, Arc::clone(em))
    }
}

#[cfg(test)]
mod tests {
    use crate::{entities::plants::ConcretePlants, test_utils::TestBed};

    use super::*;

    #[test]
    fn test_pos_from() {
        let pos = Pos::from((5, 4));
        assert_eq!(pos.x, 5);
        assert_eq!(pos.y, 4);
    }

    #[test]
    fn test_pos_dist() {
        let p1 = Pos::from((0, 5));
        let p2 = Pos::from((0, 5));
        let p3 = Pos::from((5, 5));
        let p4 = Pos::from((0, 0));

        assert_eq!(p1.dist_to(&p2), 0);
        assert_eq!(p1.dist_to(&p3), 5);
        assert_eq!(p1.dist_to(&p4), 5);
    }

    #[test]
    fn test_board_range() {
        let testbed = TestBed::new_default(6, 6, 0, 0, 0);

        let center = Pos::from((3, 3));

        let range = testbed.sandbox.board.range(1, false, center);

        assert!(range.iter().all(|p| p.dist_to(&center) == 1)); // also checks that ignore center is working
        assert_eq!(range.len(), 8);

        let range = testbed.sandbox.board.range(2, true, center);
        assert_eq!(range.len(), 25);

        assert!(range.iter().all(|p| p.dist_to(&center) <= 2)); // also checks that ignore center is working
    }

    #[test]
    pub fn test_board_range_edge() {
        let testbed = TestBed::new_default(6, 6, 0, 0, 0);
        let center = Pos::from((0, 0));

        let range = testbed.sandbox.board.range(2, false, center);

        assert_eq!(range.len(), 8);
    }

    #[test]
    pub fn test_board_is_occupied() {
        let testbed = TestBed::new_populated(6, 6, vec![(Pos::from((0, 0)), ConcretePlants::Kelp)]);
        assert!(testbed.sandbox.board.board[0][0].is_occupied())
    }

    #[should_panic]
    #[test]
    pub fn test_board_too_many_ents() {
        TestBed::new_default(1, 1, 5, 5, 5);
    }
}
