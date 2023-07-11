use crate::element_traits::Growing;
use crate::element_traits::Lives;
use crate::entities::Entity;
use crate::entities::Living;
use crate::interactions::Mates;
use crate::Sandbox;
use rand::Rng;

/// All events will implement this trait
pub trait Event {
    /// What the event displays to the user
    fn get_event_display(&self) -> String;

    /// Process the event
    fn process_event(&self, user_decision: bool, sb: &mut Sandbox);
}

/// Starting game events
#[derive(Debug, Clone, PartialEq)]
pub enum EventTypes {
    OilSpill,
    InvasiveFish,
    Party,
}

#[derive(Debug, PartialEq)]
pub struct GameEvents {
    pub(crate) kind: EventTypes,
}

#[allow(clippy::format_in_format_args)]
impl Event for GameEvents {
    fn get_event_display(&self) -> String {
        match &self.kind {
            EventTypes::OilSpill => {
                format!("{}\n\n{}\n*{}\n*{}",
                    "Oh no! An oil spill has occurred on the surface of the ocean causing havoc on your colony.", 
                    "The oil spill is going to impact the growth of your ecosystem. How do you wish to respond?\n\t1. Hide under the plants\n\t2. Continue as normal.",
                    format!(
                        "Your fish use the plants for cover, allowing them to survive the brunt of the impact.\nFish reproduction slowed by {}%, Plant reproduction slowed by {}%",
                        20, 33
                    ),
                    format!(
                        "Your fish continue on as normal, however the toxic effects of the oil take their toll.\nFish reproduction slowed by {}%, Plant reproduction slowed by {}%.",
                        33, 20
                    )
                    )
            }
            EventTypes::InvasiveFish => {
                format!("{}\n\n{}\n*{}\n*{}", 
                    "A roaming band of fish has come across your colony. They don't look friendly...",
                    "The invaders are going to do everything in their power to take what is not theirs!\nDo you want your colony to run or fight?\n\t1. Run and live another day!\n\t2. Defend our home!",
                    format!(
                        "Your fish hid from the invaders as best they could, unfortunetly your plants were not so lucky.\nYour colony loses plants."
                    ),
                    format!(
                        "Your colony rose to the challenge and fought valiantly.\nYou were able to protect your resources at the cost of your fishes life.\nYou lost fish.",
                    ))
            }
            EventTypes::Party => {
                format!(
                        "{}\n\n{}\n*{}\n*{}",
                        "Your colony want to throw a party!",
                        "While the party will provide a much needed break for the colony, it might be a considerable cost of resources.\nDo you allow your colony to party?\n\t1. Party like it's 1999!\n\t2. Maybe some other time...",
                        format!(
                            "Your fish threw a grand party that was the envy of all seafolk.\nReproduction rate increased.\nHunger increased."
                        ),
                        format!(
                            "Your fish, albiet sad, continued on as normal."
                        )
                    )
            }
        }
    }

    fn process_event(&self, user_decision: bool, sandbox: &mut Sandbox) {
        match &self.kind {
            EventTypes::OilSpill => match user_decision {
                true => {
                    // We are going to limit animal reproduction more
                    for pos in sandbox.get_important_entities() {
                        let entity = sandbox
                            .board
                            .get_tile_mut_from_pos(pos)
                            .get_entity_mut()
                            .as_mut()
                            .unwrap();
                        match entity {
                            Entity::Living(l) => match l {
                                Living::Plants(plant) => plant.slow_growth(5),
                                Living::Animals(animal) => animal.slow_mate(3.0),
                            },
                            Entity::NonLiving(_) => (),
                        }
                    }
                }
                false => {
                    // We are going to limit plant reproduction more
                    for pos in sandbox.get_important_entities() {
                        let entity = sandbox
                            .board
                            .get_tile_mut_from_pos(pos)
                            .get_entity_mut()
                            .as_mut()
                            .unwrap();
                        match entity {
                            Entity::Living(l) => match l {
                                Living::Plants(plant) => plant.slow_growth(3),
                                Living::Animals(animal) => animal.slow_mate(5.0),
                            },
                            Entity::NonLiving(_) => (),
                        }
                    }
                }
            },
            EventTypes::InvasiveFish => match user_decision {
                false => {
                    // We lose plants
                    for pos in sandbox.get_important_entities() {
                        let entity = sandbox
                            .board
                            .get_tile_mut_from_pos(pos)
                            .get_entity_mut()
                            .as_mut()
                            .unwrap();
                        match entity {
                            Entity::Living(l) => match l {
                                Living::Plants(plant) => {
                                    let mut rng = rand::thread_rng();
                                    if rng.gen_bool(2.0 / 3.0) {
                                        plant.die("thievery!");
                                    }
                                }
                                Living::Animals(_) => (),
                            },
                            Entity::NonLiving(_) => (),
                        }
                    }
                }
                true => {
                    // Fish die
                    for pos in sandbox.get_important_entities() {
                        let entity = sandbox
                            .board
                            .get_tile_mut_from_pos(pos)
                            .get_entity_mut()
                            .as_mut()
                            .unwrap();
                        match entity {
                            Entity::Living(l) => match l {
                                Living::Plants(_) => (),
                                Living::Animals(animal) => {
                                    let mut rng = rand::thread_rng();
                                    if rng.gen_bool(1.0 / 4.0) {
                                        animal.die("a fight!");
                                    }
                                }
                            },
                            Entity::NonLiving(_) => (),
                        }
                    }
                }
            },
            EventTypes::Party => match user_decision {
                false => {
                    for pos in sandbox.get_important_entities() {
                        let entity = sandbox
                            .board
                            .get_tile_mut_from_pos(pos)
                            .get_entity_mut()
                            .as_mut()
                            .unwrap();
                        match entity {
                            Entity::Living(e) => match e {
                                Living::Plants(_) => (),
                                Living::Animals(a) => {
                                    a.slow_mate(0.8);
                                    a.process_hunger();
                                }
                            },
                            Entity::NonLiving(_) => (),
                        }
                    }
                }
                // No party fish sad =(
                true => (),
            },
        }
    }
}

pub fn get_rand_event(rand_num: usize) -> GameEvents {
    // TODO update this when new events are added
    match rand_num {
        0 => GameEvents {
            kind: EventTypes::OilSpill,
        },
        1 => GameEvents {
            kind: EventTypes::InvasiveFish,
        },
        2 => GameEvents {
            kind: EventTypes::Party,
        },
        _ => panic!("Unkown event generated!"),
    }
}
