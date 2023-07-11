#[cfg(test)]
mod tests {
    use std::{collections::HashSet, sync::Arc};

    use crate::{
        entities::animals::ConcreteAnimals, entity_control::EntityManager, populate_board,
        test_utils::TestBed, Board, Sandbox,
    };

    use crate::game_board::test_utils::get_positions_of_type;

    #[test]
    /// Verify generation at a few different sizes
    fn verify_generation() {
        let sizes = vec![(1, 5), (5, 5), (10, 5), (50, 50)];
        let creature_count = vec![(3, 0, 0), (5, 2, 1), (5, 5, 5), (10, 10, 10)];
        let em = EntityManager::new();
        for i in 0..sizes.len() {
            let (row, col) = sizes[i];
            let (fish, crab, shark) = creature_count[i];
            let mut board = Board::new(row, col, Arc::clone(&em));

            populate_board(&mut board, fish, crab, shark);

            let fish_count = get_positions_of_type(&board, ConcreteAnimals::Fish);
            let crab_count = get_positions_of_type(&board, ConcreteAnimals::Crab);
            let shark_count = get_positions_of_type(&board, ConcreteAnimals::Shark);

            assert_eq!(fish_count.len(), fish);
            assert_eq!(crab_count.len(), crab);
            assert_eq!(shark_count.len(), shark);
        }
    }

    #[test]
    #[should_panic]
    fn invalid_generation() {
        let em = EntityManager::new();
        let mut board = Board::new(0, 5, em);

        populate_board(&mut board, 0, 5, 0);
    }

    #[test]
    fn no_duplicates_in_proc_list() {
        let mut testbed = TestBed::new_default(50, 50, 50, 50, 50);
        let check = |sandbox: &Sandbox| {
            let mut positions_unique = HashSet::new();
            let em = &sandbox.entity_context;
            let active_positions = em.read().unwrap().get_active_positions();
            for pos in active_positions.iter() {
                positions_unique.insert(*pos);
            }

            assert_eq!(active_positions.len(), positions_unique.len());
        };
        testbed.run_n_steps(100, true, true, true, true, check, |_| ());
    }
}
