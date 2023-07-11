// Managing interactions with others.

use crate::element_traits::{Lives, Reproducing};

/// Possible results of an action. This can be returned in a vector to possibly signal multiple different types of events.
pub enum ActionResult {
    /// Delete entities at a given position.
    DeleteTarget,
}

/// Result for something being eaten.
/// Currently not super implemented, but would allow for extra behavior after something is eaten.
#[allow(dead_code)]
pub enum EatResult {
    /// Don't actually eat the creature, but deal damage to the eater
    DealDamage(usize),
    /// The target was eaten successfully, restore hunger
    Eaten,
}

/// Trait determining behavior for things that can eat other creatures.
/// Generic on the creature type being eaten, so different behavior can be defined for eating different kinds of entities.
pub trait EatsCreatures<T: Lives + Eaten>: Lives {
    /// Try to eat the other entity, possibly dealing damage back to ourselves.
    fn eat(&mut self, target: &mut T) -> Option<crate::interactions::ActionResult> {
        // plant HP
        // let res = target.on_eat();
        match target.on_eat(self.get_attack(target)) {
            None => (),
            Some(res) => {
                for r in res {
                    match r {
                        EatResult::DealDamage(dam) => {
                            self.modify_health(-(dam as i64), "attacked by something")
                        }
                        EatResult::Eaten => self.restore_hunger(target),
                    }
                }
            }
        }
        if target.is_dead() {
            Some(ActionResult::DeleteTarget)
        } else {
            None
        }
    }
    /// Return whether or not we'd try to eat.
    fn can_eat(&self, target: &T) -> bool;

    /// Restore our own health value based on the target eaten.
    fn restore_hunger(&mut self, target: &T);

    /// Get the amount of hunger restored by eating the target.
    fn hunger_restored(&self, target: &T) -> usize;

    /// Get the amount of damage dealt to a creature
    fn get_attack(&self, target: &T) -> usize;
}

/// Trait defining behavior for something eaten, so it can have its own custom behavior.
pub trait Eaten: Lives {
    /// Perform our own logic when we're eaten.
    fn on_eat(&mut self, attack_damage: usize) -> Option<Vec<EatResult>>;

    /// Don't bite off more than you can chew
    fn get_retaliation_damage(&self) -> usize;
}

/// Defining behavior for something that can mate with other similar entities.
pub trait Mates: Lives + Reproducing {
    /// Check if the other target is a compatible mate.
    /// Note that the type bounds restrict us to only be able to mate with something else that lives and reproduces, and is of our own type.
    fn compatible_mate(&self, target: &Self) -> bool;

    /// Don't even bother checking if we can't mate in the first place!
    fn can_mate(&self) -> bool;

    /// Slow growth if events call for it
    fn slow_mate(&mut self, factor: f64);

    /// Do the do
    fn mate(&mut self, target: &mut Self) {
        if !self.compatible_mate(target) {
            return;
        }

        self.on_successful_mate();
        target.on_successful_mate();
    }

    /// Update any pregnancy-related stuff for this creature
    fn process_mating(&mut self);

    /// Function called when something successfully mates with this creature.
    fn on_successful_mate(&mut self);
}
