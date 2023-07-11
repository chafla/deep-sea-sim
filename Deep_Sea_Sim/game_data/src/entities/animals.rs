use std::cmp::{max, min};

use async_trait::async_trait;
use log::info;
use rand::rngs::ThreadRng;
use rand::Rng;

use crate::ai_controller::{
    AIAction, AIConcreteBehaviors, AIControlled, EatAction, IdleAction, MateAction,
};
use crate::element_traits::{
    LifeStatus, Lives, Mobile, OffspringData, PostProcessResult, Processing, ProcessingContext,
    Reproducing,
};
use crate::entity_control::{EntityID, TrackedEntity};
use crate::game_board::Board;
use crate::interactions::{EatResult, Eaten, EatsCreatures, Mates};
use crate::Pos;

use super::NonAbstractTaxonomy;
use super::{
    plants::Plants, Entity, Living, PTUIDisplay, Sex, MAXIMUM_ACTIONS_TO_CONSIDER,
    MAX_PREGNANCY_LEVEL,
};

pub enum ConcreteAnimals {
    Fish,
    Crab,
    Shark,
}

impl NonAbstractTaxonomy for ConcreteAnimals {
    fn create_new(&self, entity_id: Option<EntityID>) -> Entity {
        let new_animal = match self {
            Self::Fish => {
                let new_animal = AnimalType::new("fish", 100, 300, 5, 100, entity_id, 1, 1, None);
                Animals::Fish(new_animal)
            }
            Self::Crab => {
                let new_animal = AnimalType::new(
                    "crab",
                    150,
                    1000,
                    3,
                    200,
                    entity_id,
                    3,
                    1,
                    Some(Sex::Neutral),
                );
                Animals::Crab(new_animal)
            }
            Self::Shark => {
                // live fast die young
                let new_animal = AnimalType::new("shark", 200, 125, 10, 50, entity_id, 3, 3, None);
                Animals::Shark(new_animal)
            }
        };

        Entity::Living(Living::Animals(new_animal))
    }

    /// Get whether this specific type matches the passed-in entity.
    fn same_kind(&self, entity: &Entity) -> bool {
        match entity {
            Entity::NonLiving(_) => false,
            Entity::Living(l) => match l {
                Living::Animals(a) => match a {
                    Animals::Crab(_) => matches!(self, ConcreteAnimals::Crab),
                    Animals::Fish(_) => matches!(self, ConcreteAnimals::Fish),
                    Animals::Shark(_) => matches!(self, ConcreteAnimals::Shark),
                },
                _ => false,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Animals {
    Fish(AnimalType),
    Crab(AnimalType),
    Shark(AnimalType),
}

impl Animals {
    pub fn should_consider_eating(&self) -> bool {
        match self {
            Self::Shark(a) | Self::Crab(a) | Self::Fish(a) => {
                matches!(a.hunger, HungerLevel::Hungry | HungerLevel::Starving)
            }
        }
    }

    pub fn starving(&self) -> bool {
        match self {
            Self::Shark(a) | Self::Crab(a) | Self::Fish(a) => {
                matches!(a.hunger, HungerLevel::Starving)
            }
        }
    }

    /// Get a position that's a random walk from our current step.
    pub fn random_walk<T: Rng>(&self, start: Pos, rng: &mut T, board: &Board) -> Option<Pos> {
        let mut new_pos = start;
        for _ in 0..5 {
            match self {
                Animals::Fish(a) | Animals::Crab(a) | Animals::Shark(a) => {
                    let mut new_x_offset =
                        rng.gen_range(-(a.max_x_movespeed as i64)..=(a.max_x_movespeed as i64));
                    let mut new_y_offset =
                        rng.gen_range(-(a.max_y_movespeed as i64)..=(a.max_y_movespeed as i64));

                    // println!("moving to {new_x_offset}, {new_y_offset}");
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

    pub fn get_all_possible_actions(
        &self,
        board: &Board,
        ctx: &ProcessingContext,
    ) -> Vec<(Pos, AIConcreteBehaviors)> {
        let our_position = ctx.position;

        let mut concrete_behaviors = vec![];

        concrete_behaviors.push((
            our_position,
            AIConcreteBehaviors::Idle(IdleAction::new(true, true)),
        ));
        // that's a mouthful
        // run over all our active entities and see if there are any actions that we might want to perform on them
        for (_, pos) in ctx
            .entity_context
            .read()
            .unwrap()
            .get_active_entries()
            .iter()
        {
            // don't go looking forever
            if concrete_behaviors.len() > MAXIMUM_ACTIONS_TO_CONSIDER {
                break;
            }
            let tile = board.get_tile_from_pos(*pos);

            // ignore dead stuff
            if !tile.is_occupied() || matches!(tile.get_entity(), Some(Entity::NonLiving(_))) {
                continue;
            }

            if self.should_consider_eating() {
                let eat_behavior = AIConcreteBehaviors::Eating(EatAction::new(self.starving()));
                if eat_behavior.is_valid(self, ctx, board) {
                    // println!("Gonna eat");
                    concrete_behaviors.push((*pos, eat_behavior))
                }
            }

            if self.can_mate() {
                let mate_behavior = AIConcreteBehaviors::Mating(MateAction::new());
                if mate_behavior.is_valid(self, ctx, board) {
                    concrete_behaviors.push((*pos, mate_behavior))
                }
            }
        }

        concrete_behaviors
    }

    /// Get the best possible action for us at this moment.
    fn get_best_possible_behavior(
        &self,
        all_behaviors: Vec<(Pos, AIConcreteBehaviors)>,
        ctx: &ProcessingContext,
    ) -> Option<(Pos, AIConcreteBehaviors)> {
        // first, filter out the highest priority event
        if all_behaviors.is_empty() {
            return None;
        }
        let highest_priority = all_behaviors
            .iter()
            .map(|(_, b)| b.priority())
            .max()
            .unwrap();
        let highest_priority_elements = all_behaviors
            .iter()
            .filter(|(_, b)| b.priority() == highest_priority)
            .collect::<Vec<&(Pos, AIConcreteBehaviors)>>();

        if highest_priority_elements.len() == 1 {
            return Some(highest_priority_elements[0].to_owned());
        }
        // we have a few high-priority tasks
        // let's just pick the first closest one we find
        // yes this will have a preferential orrder, but it'd be a coin flip otherwise
        if let Some(res) = highest_priority_elements
            .into_iter()
            .min_by_key(|(p1, _)| ctx.position.dist_to(p1))
        {
            let actual_result = res.to_owned(); // TODO we can do better than cloning heres
            Some(actual_result)
        } else {
            None
        }
    }
}

impl PTUIDisplay for Animals {
    fn get_display_char(&self) -> char {
        match &self {
            Self::Fish(_) => '🐠',
            Self::Shark(_) => '🐬',
            Self::Crab(_) => '🐚',
        }
    }
}

#[async_trait]
impl Processing for Animals {
    fn will_process(&self) -> bool {
        match self {
            Self::Fish(_) | Self::Crab(_) | Self::Shark(_) => true,
        }
    }

    fn will_process_late(&self) -> bool {
        match self {
            Self::Fish(_) | Self::Crab(_) | Self::Shark(_) => true,
        }
    }

    fn will_ever_process(&self) -> bool {
        match self {
            Self::Fish(_) | Self::Crab(_) | Self::Shark(_) => self.will_ever_live(),
        }
    }

    fn process(&mut self, board: &mut Board, ctx: ProcessingContext) -> Option<PostProcessResult> {
        if self.is_dead() {
            return None; // cleanup after the tick
        }
        if self.ready_to_reproduce() {
            let new_important_positions = self.create_offspring(board, ctx.position);
            println!(
                "{:?} has given birth to {} new creatures!",
                &self,
                new_important_positions.len()
            );
            // new_important_positions.push(position);  // make sure our current position stays important
            return None;
        }

        self.update_behavior(&ctx, board);

        // augh I hate this pattern and I know it's an antipattern but I really can't think of anything better
        // in a perfect world with more time this would be better thought out

        // Pull out our current behavior, and clone it.
        // We do this, similar to how we handle normal processing, because tick() requires a mutable reference to self,
        // but of course current behavior is also owned by self.
        // At the very least they're (generally) pretty cheap.

        let mut current_bhvr = self.get_current_behavior_mut().clone();
        // just so we aren't spammed
        if !cfg!(test) {
            println!(
                "{} ({:?}) is {}.",
                self.get_display_char(),
                ctx.position,
                current_bhvr.get_action_desc()
            );
        }
        current_bhvr.tick(self, &ctx, board);

        self.set_current_behavior(current_bhvr);

        None
    }

    /// Take ourselves in, owned (so we can async process), then return ourselves again
    async fn late_process(&mut self) -> Option<PostProcessResult> {
        self.life(); // run this stuff late
        match self {
            Self::Fish(_) | Self::Crab(_) | Self::Shark(_) => (),
        }
        if self.delete_on_death() && matches!(self.get_life_status(), LifeStatus::Dead) {
            Some(PostProcessResult::Delete)
            // return Some(PostProcessResult::Delete);
        } else {
            None
        }
        // None
        // self
    }
}

impl Lives for Animals {
    fn will_ever_live(&self) -> bool {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => !a.has_died,
        }
    }

    fn get_health(&self) -> i64 {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => a.hp,
        }
    }

    fn get_life_status(&self) -> crate::element_traits::LifeStatus {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => {
                if !a.has_died {
                    LifeStatus::Alive
                } else {
                    LifeStatus::Dead
                }
            }
        }
    }

    fn process_health(&mut self) {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => {
                let heal_rate = match a.hunger {
                    HungerLevel::Full => 2,
                    HungerLevel::Hungry => 1,
                    HungerLevel::Starving => 0, // todo things don't die yet
                    HungerLevel::Famished => -2,
                };
                self.modify_health(heal_rate, "hunger");
            }
        }
    }

    fn modify_health(&mut self, delta: i64, cause: &str) {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => {
                a.hp = min(max(a.hp + delta, 0), a.hp_max);

                if a.hp == 0 {
                    self.die(cause);
                }
            }
        }
    }

    fn die(&mut self, cause: &str) {
        match self {
            Self::Fish(a) | Self::Crab(a) | Self::Shark(a) => {
                a.has_died = true;
                println!("{:?} has died of {cause}!", a)
            }
        }
    }

    fn process_hunger(&mut self) {
        // println!("Hunger processed");
        match self {
            Self::Fish(a) => {
                a.hunger_level -= 2;
                a.hunger = HungerLevel::from(a.hunger_level);
            }
            Self::Shark(a) => {
                a.hunger_level -= 3;
                a.hunger = HungerLevel::from(a.hunger_level);
            }
            Self::Crab(a) => {
                a.hunger_level -= 1;
                a.hunger = HungerLevel::from(a.hunger_level);
            }
        }
    }

    fn process_age(&mut self) {
        match self {
            Self::Fish(a) | Self::Shark(a) | Self::Crab(a) => {
                a.age += 1;
                if a.age >= a.max_age {
                    self.die("old age");
                }
            }
        }
        // TODO
    }

    fn process_life_misc(&mut self) {
        self.process_mating()
    }
}

impl Mobile for Animals {
    fn max_speeds(&self) -> (usize, usize) {
        match self {
            Self::Fish(a) | Self::Shark(a) | Self::Crab(a) => {
                (a.max_x_movespeed, a.max_y_movespeed)
            }
        }
    }
}

impl EatsCreatures<Plants> for Animals {
    fn restore_hunger(&mut self, target: &Plants) {
        let hunger_restored = self.hunger_restored(target);
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                a.hunger_level += hunger_restored as i64
            }
        };
    }

    fn hunger_restored(&self, target: &Plants) -> usize {
        match target {
            Plants::Kelp(_) => 100, // full kelp is very fulfilling
            Plants::KelpLeaf(_) => 25,
            Plants::KelpSeed(_) => 10, // barely worth it
        }
    }

    fn can_eat(&self, target: &Plants) -> bool {
        if target.is_dead() {
            return false;
        }
        match self {
            Self::Shark(_) => false, // sharks never eat plants, they're carnivores
            Self::Crab(a) => matches!(a.hunger, HungerLevel::Hungry | HungerLevel::Starving),
            Self::Fish(a) => matches!(a.hunger, HungerLevel::Starving | HungerLevel::Hungry), // save it for the crabs
        }
    }

    fn get_attack(&self, _: &Plants) -> usize {
        1
    }
}

impl Eaten for Animals {
    fn on_eat(&mut self, attack: usize) -> Option<Vec<EatResult>> {
        // unlike plants, animals just Die when they are eaten
        self.modify_health(-(attack as i64), "eaten");

        if self.is_dead() {
            Some(vec![EatResult::Eaten])
        } else {
            Some(vec![
                EatResult::DealDamage(self.get_retaliation_damage()),
                EatResult::Eaten,
            ])
        }
    }

    fn get_retaliation_damage(&self) -> usize {
        match self {
            Self::Shark(_) => 100,
            Self::Crab(_) => 50,
            Self::Fish(_) => 25,
        }
    }
}

impl EatsCreatures<Animals> for Animals {
    fn can_eat(&self, target: &Animals) -> bool {
        if target.is_dead() {
            return false;
        }
        if *self == *target {
            return false;
        }
        match (self, target) {
            (Self::Shark(_), Self::Shark(_)) => false,
            (Self::Shark(_), _) => true, // sharks can eat anything that isn't themselves
            (Self::Fish(_), Self::Crab(_)) => true, // fish can eat crabs, but they might be killed by them in the process!
            // fish are a bit more careful about trying to eat other fish, and will only do it if absolutely necessary!
            (Self::Fish(a), Self::Fish(_)) => match a.hunger {
                HungerLevel::Famished => true,
                HungerLevel::Starving => a.hp > target.get_retaliation_damage() as i64,
                HungerLevel::Hungry => false,
                HungerLevel::Full => false,
            },
            // matches!(a.hunger, HungerLevel::Starving) && a.hp > target.get_retaliation_damage() as i64,
            _ => false, // anything else is a crabshoot
        }
    }

    fn restore_hunger(&mut self, target: &Animals) {
        let hunger_restored = self.hunger_restored(target);
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                a.hunger_level += hunger_restored as i64
            }
        };
    }

    fn hunger_restored(&self, target: &Animals) -> usize {
        match target {
            Animals::Crab(_) => 50,
            Animals::Fish(_) => 100,
            Animals::Shark(_) => 500,
        }
    }

    fn get_attack(&self, _: &Animals) -> usize {
        match self {
            Self::Shark(_) => 100,
            Self::Crab(_) => 50,
            Self::Fish(_) => 25,
        }
    }
}

impl Reproducing for Animals {
    fn ready_to_reproduce(&self) -> bool {
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                matches!(a.sex, Sex::Female | Sex::Neutral)
                    && a.pregnancy_level >= MAX_PREGNANCY_LEVEL
            }
        }
    }

    fn get_offspring_data(&self) -> Option<OffspringData> {
        match self {
            // For now, just a single
            Self::Crab(_) | Self::Fish(_) | Self::Shark(_) => Some(OffspringData {
                min_offspring: 1,
                max_offspring: 1,
                percent_chance_per_tile: 0.0,
            }),
        }
    }

    fn have_child(&mut self, tile: &mut crate::Tile, _: Pos, _: usize) {
        let new_child = match self {
            Self::Crab(_) => ConcreteAnimals::Crab.create_new(None),
            Self::Shark(_) => ConcreteAnimals::Shark.create_new(None),
            Self::Fish(_) => ConcreteAnimals::Fish.create_new(None),
        };
        // ids populated by tile
        let _ = tile.add_entity(new_child);
    }

    fn on_offspring_created(&mut self) {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                a.pregnant = false;
                a.pregnancy_level = 0;
                a.ticks_since_last_mating = 0; // we'll just set this here so there's a bit of a cooldown between having a child and trying to make more
            }
        }
    }
}

impl Mates for Animals {
    fn compatible_mate(&self, target: &Self) -> bool {
        let able_to_mate = self.can_mate() && target.can_mate();
        let compatible = match (self, target) {
            (Animals::Shark(a), Animals::Shark(b)) | (Animals::Fish(a), Animals::Fish(b)) => {
                a.sex != b.sex
            }
            (Animals::Crab(_), Animals::Crab(_)) => true, // crabs don't need gender
            _ => false,
        };

        able_to_mate && compatible
    }

    fn can_mate(&self) -> bool {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                !a.pregnant && a.ticks_since_last_mating >= a.mating_cooldown
            }
        }
    }

    fn slow_mate(&mut self, factor: f64) {
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                let less_growth = a.ticks_since_last_mating as f64 / factor;
                if factor < 1.0 {
                    a.ticks_since_last_mating = less_growth.ceil() as usize;
                } else {
                    a.ticks_since_last_mating -= less_growth.ceil() as usize;
                }
            }
        }
    }

    fn on_successful_mate(&mut self) {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                if matches!(a.sex, Sex::Female | Sex::Neutral) {
                    a.pregnant = true;
                }
                a.ticks_since_last_mating = 0;
            }
        }
    }

    fn process_mating(&mut self) {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => {
                a.ticks_since_last_mating += 1;
                if !a.pregnant {
                    return;
                }
                a.pregnancy_level += a.pregnancy_step;
            }
        }
    }
}

impl TrackedEntity for Animals {
    fn tracked(&self) -> bool {
        self.get_id().is_some()
    }

    fn register(&mut self, id: EntityID) -> Result<(), EntityID> {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => a.id = Some(id),
        }
        Ok(())
    }

    fn get_id(&self) -> Option<EntityID> {
        match self {
            // For now, just a single
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => a.id,
        }
    }
}

impl AIControlled<Animals> for Animals {
    fn get_possible_concrete_actions(
        &self,
        _: ProcessingContext,
        _: &Board,
    ) -> Vec<AIConcreteBehaviors> {
        // let possible_behaviors: Vec<AIConcreteBehaviors> = vec![];

        // Create idle behavior
        // possible_behaviors.push(AIConcreteBehaviors::Idle(IdleAction {  }));

        // for behavior in &[AIConcreteBehaviors::]
        // self.get_all_possible_actions(board, ctx)
        todo!()
    }

    fn get_next_action(
        &self,
        ctx: &ProcessingContext,
        board: &Board,
    ) -> Option<AIConcreteBehaviors> {
        let all_possible_actions = self.get_all_possible_actions(board, ctx);
        let best_possible_action = self.get_best_possible_behavior(all_possible_actions, ctx);

        // if let Some((p, act)) = &best_possible_action {
        //     // println!("Best possible action for {:?} at {:?} is \n {} at {p:?}", self.get_id(), ctx.position, act.get_action_desc());
        // }

        let cur_behavior = self.get_current_behavior();

        if let Some((_, new_action)) = best_possible_action {
            if cur_behavior.completed() || !cur_behavior.is_valid(self, ctx, board) {
                info!("New action was completed!");
                return Some(new_action);

            // } else if cur_behavior. {

            // }
            } else if cur_behavior.priority() >= new_action.priority() {
                // println!("New action {new_action:?} was of too low priority vs {cur_behavior:?}!");
                return None;
            } else {
                return Some(new_action);
            }
        }
        None
    }

    fn get_current_behavior_mut(&mut self) -> &mut AIConcreteBehaviors {
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => &mut a.current_behavior,
        }
    }

    fn get_current_behavior(&self) -> &AIConcreteBehaviors {
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => &a.current_behavior,
        }
    }

    fn set_current_behavior(&mut self, behavior: AIConcreteBehaviors) {
        match self {
            Self::Crab(a) | Self::Fish(a) | Self::Shark(a) => a.current_behavior = behavior,
        }
    }

    fn get_desired_move(&self, ctx: &ProcessingContext, board: &Board) -> Option<Pos> {
        let bhvr = self.get_current_behavior();
        match bhvr {
            // todo this could probably be better placed in AIConcreteBehaviors itself
            AIConcreteBehaviors::Eating(e) => e.get_movement(self, ctx, board),
            AIConcreteBehaviors::Idle(i) => i.get_movement(self, ctx, board),
            AIConcreteBehaviors::Mating(m) => m.get_movement(self, ctx, board),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HungerLevel {
    Full,
    Hungry,
    Starving,
    Famished,
}

impl From<i64> for HungerLevel {
    fn from(value: i64) -> Self {
        match value {
            51..=i64::MAX => HungerLevel::Full,
            1..=50 => HungerLevel::Hungry,
            -24..=0 => HungerLevel::Starving,
            i64::MIN..=-25 => HungerLevel::Famished,
        }
    }
}

/// The raw definition of an animal. One of the possibilities for the bottom of the enum tree.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimalType {
    /// The name of the animal.
    name: String,
    /// Current hitpoints.
    hp: i64,
    /// Maximum number of hit points.
    hp_max: i64,
    /// How many tiles this creature can move per turn in the x direction
    max_x_movespeed: usize,
    /// How many tiles this creature can move per turn in the y direction
    max_y_movespeed: usize,
    hunger: HungerLevel,
    pub hunger_level: i64,
    has_died: bool,
    age: usize,
    max_age: usize, // animals don't live forever,
    sex: Sex,
    // Reproduction stuff
    pregnant: bool,
    /// Pregnancy increases by a given step
    pregnancy_level: usize,
    /// Step by which this creature progresses towards having a child
    pregnancy_step: usize,
    /// The number of ticks since we last had a child.
    pub ticks_since_last_mating: usize,
    /// How long this species takes between mating
    mating_cooldown: usize,
    /// Our working entity ID
    id: Option<EntityID>,
    /// Our possible behaviors
    current_behavior: AIConcreteBehaviors,
}

impl AnimalType {
    #[allow(clippy::too_many_arguments)] // this is an initializer, it needs this many
    fn new(
        name: &str,
        hp: i64,
        max_age: usize,
        pregnancy_step: usize,
        mating_cooldown: usize,
        id: Option<EntityID>,
        max_movespeed_x: usize,
        max_movespeed_y: usize,
        sex_override: Option<Sex>,
    ) -> Self {
        let mut rng: ThreadRng = rand::thread_rng();
        let chosen_sex = if let Some(sex) = sex_override {
            sex
        } else if rng.gen_bool(0.5) {
            Sex::Male
        } else {
            Sex::Female
        };
        Self {
            name: String::from(name),
            hp_max: hp,
            hp,
            hunger: HungerLevel::Full,
            hunger_level: 100,
            has_died: false,
            age: 0,
            max_age,
            sex: chosen_sex,
            pregnancy_level: 0,
            pregnant: false,
            pregnancy_step,
            mating_cooldown,
            ticks_since_last_mating: 0,
            id,
            max_x_movespeed: max_movespeed_x,
            max_y_movespeed: max_movespeed_y,
            current_behavior: AIConcreteBehaviors::Idle(IdleAction::new(true, true)),
        }
    }

    /// Get the maximum movespeeds in the (x, y) directions.
    pub fn get_max_movespeed(&self) -> (usize, usize) {
        (self.max_x_movespeed, self.max_y_movespeed)
    }
}

impl PTUIDisplay for AnimalType {
    fn get_display_char(&self) -> char {
        return self.name.chars().next().unwrap();
    }
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use super::{Animals, ConcreteAnimals, HungerLevel};
    use crate::{
        ai_controller::{AIConcreteBehaviors, AIControlled},
        element_traits::{Lives, ProcessingContext},
        entities::{plants::ConcretePlants, Entity, Living, NonAbstractTaxonomy, Sex},
        entity_control::{EntityID, TrackedEntity},
        game_board::Pos,
        test_utils::TestBed,
    };

    // todo would be nice to verify all these against all animal types, just don't have the time
    #[test]
    fn verify_animal_life() {
        let creature = ConcreteAnimals::Crab.create_new(None);
        let mut health = 0;
        let mut hunger = 0;

        let mut testbed = TestBed::new_with_entities(3, 3, vec![(Pos { x: 1, y: 1 }, creature)]);

        let tile = testbed.sandbox.board.get_tile(1, 1);

        if let Some(Entity::Living(Living::Animals(a))) = &tile.get_entity() {
            health = a.get_health();
            if let Animals::Crab(c) = &a {
                hunger = c.hunger_level;
            }
        }

        testbed.run_n_full_steps(2);

        let tile = testbed.sandbox.board.get_tile(1, 1);

        if let Some(Entity::Living(Living::Animals(a))) = &tile.get_entity() {
            assert!(!a.is_dead());
            if let Animals::Crab(c) = a {
                assert_eq!(a.get_health(), health);
                dbg!(c.hunger_level);
                dbg!(hunger);
                assert!(c.hunger_level < hunger);
            }
        }
    }

    #[test]
    fn verify_death() {
        let pre_verify = |a: &mut Animals| {
            a.die("testing");
        };
        let post_verify = |a: Option<&mut Animals>| assert!(a.is_none());

        verify_animal_after_n_steps(1, pre_verify, post_verify, false, true);
    }

    #[allow(dead_code)]
    /// Helper to verify animal behavior before and after a number of steps occur, in a controlled board.
    fn verify_animal_after_n_steps<F, G>(
        n_steps: usize,
        pre_check: F,
        post_check: G,
        process: bool,
        late_process: bool,
    ) where
        F: FnOnce(&mut Animals),
        G: FnOnce(Option<&mut Animals>),
    {
        let creature = ConcreteAnimals::Crab.create_new(None);
        let mut testbed = TestBed::new_with_entities(3, 3, vec![(Pos { x: 1, y: 1 }, creature)]);
        let tile = testbed.sandbox.board.get_tile_mut(1, 1);

        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity_mut() {
            pre_check(a);
        };

        testbed.run_n_steps_no_checks(n_steps, false, process, late_process, false);

        let tile = testbed.sandbox.board.get_tile_mut(1, 1);

        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity_mut() {
            post_check(Some(a));
        } else {
            post_check(None)
        }
    }

    #[test]
    fn verify_starvation() {
        let pre_verify = |a: &mut Animals| {
            let (Animals::Crab(c) | Animals::Fish(c) | Animals::Shark(c)) = a;
            c.hunger_level = -5;
            c.hunger = HungerLevel::Starving;
        };
        let post_verify = |a: Option<&mut Animals>| assert!(a.is_none());

        verify_animal_after_n_steps(100, pre_verify, post_verify, false, true);
    }

    #[test]
    fn verify_old_age() {
        let pre_verify = |a: &mut Animals| {
            if let Animals::Crab(c) = a {
                c.age = 5000;
            };
        };
        let post_verify = |a: Option<&mut Animals>| assert!(a.is_none());

        verify_animal_after_n_steps(5, pre_verify, post_verify, false, true);
    }

    #[test]
    fn test_behaviors() {
        let creature = ConcreteAnimals::Crab.create_new(None);
        let plant = ConcretePlants::Kelp.create_new(None);
        let mut testbed = TestBed::new_with_entities(
            5,
            8,
            vec![(Pos { x: 1, y: 1 }, creature), (Pos { x: 7, y: 4 }, plant)],
        );

        // make em hungry
        let tile = testbed.sandbox.board.get_tile_mut(1, 1);
        if let Some(Entity::Living(Living::Animals(Animals::Crab(a)))) = tile.get_entity_mut() {
            a.hunger_level = -1;
            a.hunger = HungerLevel::Starving;
        }

        let tile = testbed.sandbox.board.get_tile(1, 1);

        let ctx = ProcessingContext {
            entity_context: Arc::clone(&testbed.sandbox.entity_context),
            position: Pos { x: 1, y: 1 },
        };

        // everything should start out idling
        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity() {
            assert!(matches!(
                a.get_current_behavior(),
                AIConcreteBehaviors::Idle(_)
            ));

            let behaviors = a.get_all_possible_actions(&testbed.sandbox.board, &ctx);
            dbg!(&behaviors);
            assert_eq!(behaviors.len(), 2);

            for (pos, behavior) in behaviors {
                match (pos, behavior) {
                    (_, AIConcreteBehaviors::Idle(_)) => (),
                    (Pos { x: 7, y: 4 }, AIConcreteBehaviors::Eating(_)) => (),
                    _ => assert!(false),
                }
            }
        };

        // ensure it isn't sticky

        // make em
        let tile = testbed.sandbox.board.get_tile_mut(1, 1);
        if let Some(Entity::Living(Living::Animals(Animals::Crab(a)))) = tile.get_entity_mut() {
            a.hunger_level = 100;
            a.hunger = HungerLevel::Full;
        }

        let tile = testbed.sandbox.board.get_tile(1, 1);

        // everything should start out idling
        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity() {
            let behaviors = a.get_all_possible_actions(&testbed.sandbox.board, &ctx);
            // dbg!(&behaviors);
            assert_eq!(behaviors.len(), 1);

            // should now only have the idle behavior
            for (pos, behavior) in behaviors {
                match (pos, behavior) {
                    (_, AIConcreteBehaviors::Idle(_)) => (),
                    _ => assert!(false),
                }
            }
        };

        // but checking for behavior should have two possibilities
    }

    #[test]
    fn test_behaviors_activation() {
        // verify that behaviors are properly activated and deactivated automatically in the presence (or lack thereof) of certain stimuli

        let creature = ConcreteAnimals::Crab.create_new(None);
        let plant = ConcretePlants::Kelp.create_new(None);
        let mut testbed = TestBed::new_with_entities(5, 8, vec![(Pos { x: 1, y: 1 }, creature)]);

        let ctx = ProcessingContext {
            entity_context: Arc::clone(&testbed.sandbox.entity_context),
            position: Pos { x: 1, y: 1 },
        };

        let mut entity_id: Option<EntityID> = None;

        let creature = testbed.get_entity_at_pos(Pos { x: 1, y: 1 });
        if let Some(Entity::Living(Living::Animals(a))) = creature {
            assert_eq!(
                a.get_all_possible_actions(&testbed.sandbox.board, &ctx)
                    .len(),
                1
            );
            entity_id = a.get_id();
        }

        testbed.run_n_steps_no_checks(1, false, true, true, false);

        // should be no change
        let creature = testbed.get_entity_at_pos(Pos { x: 1, y: 1 });
        if let Some(Entity::Living(Living::Animals(a))) = creature {
            assert_eq!(
                a.get_all_possible_actions(&testbed.sandbox.board, &ctx)
                    .len(),
                1
            )
        }

        // make hungry
        let tile = testbed.sandbox.board.get_tile_mut(1, 1);
        if let Some(Entity::Living(Living::Animals(Animals::Crab(a)))) = tile.get_entity_mut() {
            a.hunger_level = 0;
            a.hunger = HungerLevel::Starving;
        }

        // insert plant
        let tile = testbed.sandbox.board.get_tile_mut(4, 6);
        tile.add_entity(plant).unwrap();

        testbed.run_n_steps_no_checks(1, false, true, true, false);

        // now we need to use the entity ID to get the crab position
        {
            let ctx_lock = ctx.entity_context.read().unwrap();
            let new_pos = ctx_lock
                .get_active_entries()
                .get(&entity_id.unwrap())
                .unwrap();

            let creature = testbed.get_entity_at_pos(*new_pos);
            if let Some(Entity::Living(Living::Animals(a))) = creature {
                assert_ne!(
                    a.get_all_possible_actions(&testbed.sandbox.board, &ctx)
                        .len(),
                    1
                );
                assert!(matches!(
                    a.get_current_behavior(),
                    AIConcreteBehaviors::Eating(_)
                ))
            }
        }
    }

    #[test]
    fn verify_mating() {
        let mut creature = ConcreteAnimals::Crab.create_new(None);
        if let Entity::Living(Living::Animals(Animals::Crab(c))) = &mut creature {
            c.sex = Sex::Male;
            c.ticks_since_last_mating = 1000;
        }
        let mut creature_2 = creature.clone();

        if let Entity::Living(Living::Animals(Animals::Crab(c))) = &mut creature_2 {
            c.sex = Sex::Female;
        }

        let mut testbed = TestBed::new_with_entities(
            4,
            4,
            vec![
                (Pos { x: 1, y: 2 }, creature),
                (Pos { x: 1, y: 1 }, creature_2),
            ],
        );

        let ctx = ProcessingContext {
            entity_context: Arc::clone(&testbed.sandbox.entity_context),
            position: Pos { x: 1, y: 1 },
        };

        let tile = testbed.sandbox.board.get_tile(1, 1);
        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity() {
            assert!(
                a.get_all_possible_actions(&testbed.sandbox.board, &ctx)
                    .len()
                    == 2
            )
        }

        testbed.run_n_steps_no_checks(10, false, true, true, false);

        let tile = testbed.sandbox.board.get_tile(1, 1);
        if let Some(Entity::Living(Living::Animals(a))) = tile.get_entity() {
            assert!(
                a.get_all_possible_actions(&testbed.sandbox.board, &ctx)
                    .len()
                    == 1
            )
        }

        // the female one should now be pregnant

        // wait another n turns
        testbed.run_n_steps_no_checks(100, false, true, true, false);

        // and the baby should have popped out

        let entities = testbed.sandbox.get_important_entities();
        assert!(entities.len() > 2);
    }
}
