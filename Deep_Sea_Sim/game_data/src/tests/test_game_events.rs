#[cfg(test)]
mod tests {
    use crate::{
        element_traits::{LifeStatus, Lives},
        game_events::{self, Event},
        test_utils::TestBed,
        Sandbox,
    };

    #[test]
    /// Verify event generation
    fn verify_generation() {
        // Get first event type (Oil Spill)
        let event = game_events::get_rand_event(0);
        assert_eq!(event.kind, game_events::EventTypes::OilSpill);

        // Get second event type (Invasive Fish)
        let event = game_events::get_rand_event(1);
        assert_eq!(event.kind, game_events::EventTypes::InvasiveFish);

        // Get third event type (Party)
        let event = game_events::get_rand_event(2);
        assert_eq!(event.kind, game_events::EventTypes::Party);
    }

    #[test]
    #[should_panic]
    fn invalid_generation() {
        let _ = game_events::get_rand_event(100);
    }

    #[test]
    fn verify_display() {
        // Get first event type (Oil Spill)
        let event = game_events::get_rand_event(0);
        assert_eq!(event.get_event_display().len(), 540);

        // Get second event type (Invasive Fish)
        let event = game_events::get_rand_event(1);
        assert_eq!(event.get_event_display().len(), 523);

        // Get third event type (Party)
        let event = game_events::get_rand_event(2);
        assert_eq!(event.get_event_display().len(), 396);
    }

    #[test]
    fn verify_events_in_loop() {
        let mut testbed = TestBed::new_default(50, 50, 10, 10, 10);
        let check = |sandbox: &mut Sandbox, event: Option<game_events::GameEvents>| -> bool {
            if event.is_some() {
                event.unwrap().process_event(true, sandbox);
                true
            } else {
                false
            }
        };
        // Run 99 steps as an event has to be generated in that time
        let result = testbed.run_n_steps_events(99, check);
        assert!(result);
    }

    #[test]
    fn verify_event_results() {
        // Oil spill false
        verify_oil_spill(false);
        // Oil spill true
        verify_oil_spill(true);
        // Invasive fish false
        verify_invasive_fish(false);
        // Invasive fish true
        verify_invasive_fish(true);
        // Party false
        verify_party();
    }

    fn verify_oil_spill(input: bool) {
        let mut testbed = TestBed::new_default(10, 10, 1, 1, 1);
        // Get the simulation running
        testbed.run_n_steps_no_checks(20, false, true, true, false);

        // Get initial data
        let mut init_repo_rate = Vec::new();
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(p) => match p {
                        crate::entities::plants::Plants::Kelp(p)
                        | crate::entities::plants::Plants::KelpSeed(p)
                        | crate::entities::plants::Plants::KelpLeaf(p) => {
                            init_repo_rate.push(p.growth_level)
                        }
                    },
                    crate::entities::Living::Animals(a) => match a {
                        crate::entities::animals::Animals::Fish(a)
                        | crate::entities::animals::Animals::Crab(a)
                        | crate::entities::animals::Animals::Shark(a) => {
                            init_repo_rate.push(a.ticks_since_last_mating)
                        }
                    },
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        let event = game_events::get_rand_event(0);
        event.process_event(input, &mut testbed.sandbox);
        // Get new data
        let mut new_repo_rate = Vec::new();
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(p) => match p {
                        crate::entities::plants::Plants::Kelp(p)
                        | crate::entities::plants::Plants::KelpSeed(p)
                        | crate::entities::plants::Plants::KelpLeaf(p) => {
                            new_repo_rate.push(p.growth_level)
                        }
                    },
                    crate::entities::Living::Animals(a) => match a {
                        crate::entities::animals::Animals::Fish(a)
                        | crate::entities::animals::Animals::Crab(a)
                        | crate::entities::animals::Animals::Shark(a) => {
                            new_repo_rate.push(a.ticks_since_last_mating)
                        }
                    },
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        // Verify the entities were affected accordingly
        for i in 0..new_repo_rate.len() {
            assert!(new_repo_rate[i] < init_repo_rate[i]);
        }
    }

    fn verify_invasive_fish(input: bool) {
        let mut testbed = TestBed::new_default(30, 30, 5, 5, 5);
        // Get the simulation running
        testbed.run_n_steps_no_checks(20, false, true, true, false);

        // Get initial data
        let mut init_plant_num = 0;
        let mut init_animal_num = 0;
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(p) => {
                        if p.get_life_status() == LifeStatus::Alive {
                            init_plant_num += 1
                        }
                    }
                    crate::entities::Living::Animals(a) => {
                        if a.get_life_status() == LifeStatus::Alive {
                            init_animal_num += 1
                        }
                    }
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        let event = game_events::get_rand_event(1);
        event.process_event(input, &mut testbed.sandbox);
        // Get new data
        let mut new_plant_num = 0;
        let mut new_animal_num = 0;
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(p) => {
                        if p.get_life_status() == LifeStatus::Alive {
                            new_plant_num += 1;
                        }
                    }
                    crate::entities::Living::Animals(a) => {
                        if a.get_life_status() == LifeStatus::Alive {
                            new_animal_num += 1;
                        }
                    }
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        // Verify the entities were affected accordingly
        if input {
            // There is a small chance that no animals die
            // so don't check it...
            //assert_ne!(new_animal_num, init_animal_num);
            assert_eq!(new_plant_num, init_plant_num);
        } else {
            // There is a small chance that no plants die
            // so don't check it...
            // assert_ne!(new_plant_num, init_plant_num);
            assert_eq!(new_animal_num, init_animal_num);
        }
    }

    fn verify_party() {
        let mut testbed = TestBed::new_default(10, 10, 1, 1, 1);
        // Get the simulation running
        testbed.run_n_steps_no_checks(20, false, true, true, false);

        // Get initial data
        let mut init_repo_rate = Vec::new();
        let mut init_hunger = Vec::new();
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(_) => (),
                    crate::entities::Living::Animals(a) => match a {
                        crate::entities::animals::Animals::Fish(a)
                        | crate::entities::animals::Animals::Crab(a)
                        | crate::entities::animals::Animals::Shark(a) => {
                            init_repo_rate.push(a.ticks_since_last_mating);
                            init_hunger.push(a.hunger_level);
                        }
                    },
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        let event = game_events::get_rand_event(2);
        event.process_event(false, &mut testbed.sandbox);
        // Get new data
        let mut new_repo_rate = Vec::new();
        let mut new_hunger = Vec::new();
        for pos in testbed.sandbox.get_important_entities() {
            match testbed
                .sandbox
                .board
                .get_tile_from_pos(pos)
                .get_entity()
                .as_ref()
                .unwrap()
            {
                crate::entities::Entity::Living(ent) => match ent {
                    crate::entities::Living::Plants(_) => (),
                    crate::entities::Living::Animals(a) => match a {
                        crate::entities::animals::Animals::Fish(a)
                        | crate::entities::animals::Animals::Crab(a)
                        | crate::entities::animals::Animals::Shark(a) => {
                            new_repo_rate.push(a.ticks_since_last_mating);
                            new_hunger.push(a.hunger_level);
                        }
                    },
                },
                crate::entities::Entity::NonLiving(_) => (),
            }
        }
        // Verify the entities were affected accordingly
        for i in 0..new_repo_rate.len() {
            assert!(new_repo_rate[i] > init_repo_rate[i]);
            assert!(new_hunger[i] < init_hunger[i]);
        }
    }
}
