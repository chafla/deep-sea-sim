# Skittering Crustaceans

Team members:

- Eric Barron
- Matthew Thompson

## Demo Video

![Preview of our Demo](Demo.mov)

## Summary Description

Our program is a deep-sea simulation game where the aquatic creature have to survive within the confinds of their resources and environment.  As the simulation progresses the user is able to follow the colony through a GUI.  Furthermore, the user is to set starting parameters for the simulation and respond to randomly generated events as the simulation progresses.  The creatures confined to the habitat will move around with the motivation to stay alive, with the primary factor being moving towards food and away from predadtors.  As the simulation progresses, animals and plants reproduce, and eventually die off as the colony loses resources.

## Project Execution Summary

### Architecture Design

Our architecture has, from the start, been designed to be as Rusty as possible. Designing what amounts to a game engine was a bit of a challenge that at first may have seemed like biting off more than we could chew, and certainly did present its fair share of trouble, but we’re fairly confident in our design. Our design forceed us to confront some of the realities and difficulties often presented by Rust's Ownership and Borrow System, but in trying to work with these difficulties, our architecture was significantly improved.

### General Program Structure

Our general program structure is more or less as follows, from the bottom to the top:

#### Entities

On the lowest level, we have our `Entity` tree. These consist of a large hierarchy of enums which distinguish the highest-level distinctions between anything present on the board. This allows for filtering and categorization without the need for trait objects, while still allowing access to specific behavior through matching down the hierarchy.

The structure forms a tree, like so:

```rs
Entity::
    Living::
        Plants::
            KelpSeed(Plant)
            KelpLeaf(Plant)
            Kelp(Plant)
        Animals::
            Fish(AnimalType)
            Crab(AnimalType)
            Shark(AnimalType)
    NonLiving::
        Rock(Decoration)
        Shell(Decoration)
```

The outer items are enums, and only the innermost items in parentheses are structs which actually hold data. Across this enum tree, we have a large variety of traits attached at different levels to define similar behaviors (particularly between `Plants` and `Animals`), something we will cover a bit later.

#### Game Board and Tiles

Our architecture is tile-based, with an overarching `Board` that holds a 2-D vector of owned `Tile` structs. These `Tile`s maintain ownership of any `Entity` that is "on their space", and as a result all direct access to game entities must go through the Tiles they are located on. These tiles can be found by querying the board for the tile at a given (x, y) position, and our module structure requires other modules to use specific accessor methods to interact with the entities on them. This helps us define specific behavior for when Entities are added to or removed from these tiles.

#### Sandbox

On the highest level, we have the `Sandbox`. This is more or less the game engine, and it has the mechanisms for executing the game loop itself, as well as keeping track of the current game state.
The sandbox has ownership of the game's `Board` struct, and controls access to it by passing it through to entities as needed, as well as an `EntityManager` struct that keeps tabs on entities that should be processed.

As a result, our entity relation structure works out to form a neat tree:

```c
Sandbox
    Board
        Tile
            Entity
```

### The game loop

This loop consists of a few different steps which altogether make up each turn:

- **Movement**: Handling creature movement
- **Processing**: Handling each creature's behavior that could impact the board
- **Late Processing**: *Asynchronously* handling behavior that only impacts each individual living thing
- **Events**: Handling global game events

As for game-specific data, the sandbox keeps track of two things in particular: the `Board`, and an `EntityManager`. The board is as described, but the `EntityManager` is a game-global manager to keep track of entities that are in some way "important" to the game loop. Only entities marked as important (read: entities with some processing behavior, so not rocks or anything `NonLiving`) will be iterated over and have their behavior executed, rather than searching through the whole board.

Through the *Movement*, *Processing*, and *GameEvents* steps, entities are performing actions that could impact the `Board` state and other entities (such as eating other Entities, moving, or reproducing). As a result, every entity is given an `&mut Board`, and every entity is processed sequentially.

In *Late Processing*, however, entities are required to perform events that can only impact themselves, something reinforced by them being given an immutable `&Board`. As these can only ever be individual functions, we decided to make them asynchronous. The late processing function itself is `async`, as are every entity's own `late_process()` function. They are all added to tasks, which are run simultaneously and joined on by the event loop.

All of these actions require the Sandbox's processing functions to temporarily take ownership of the entities they process, run their processing functions, and then (optionally) return them back to their `Tile` afterwards. If an entity isn't returned to its tile after processing (something it can specifically request by returning a special hint), it is not re-added to the processing list, and dropped.

While the processing list was originally just a `Vec<Pos>` of entity positions, it eventually evolved to associate a position with a program-global `EntityID`[1], using the `EntityManager`. This manager is always wrapped in an `Arc<Mutex>`, which allows it to be passed around fairly easily without worry about concurrent access or race conditions. This manager introduced some strange arrows in our dependency graph, but we were able to make use of our existing structure to keep our ownership situation sane. Every `Tile` gets a clone of the manager, and uses it to add or remove entities from the processing list when they're added to or removed from tiles, while also registering new entities that don't yet have an ID.

### The GUI

To create the GUI we used the `egui` crate.  The GUI is responsible for displaying information to the user as well as passing the user inputs that are needed in order to create the `Sandbox` and start the game loop.  Since `egui` provides an immediate mode GUI, when running the game loop we did so on a separate thread, keeping the GUI on the main thread and passed information (read: board state after a tick, animal health after a tick, user event decisions) between the two threads through channels.

### Architectural Decisions + Keeping Things Rusty

#### Trait structure

Something we really wanted to emphasize from the outset was the use of traits to define different behavior for different creatures. We started out with a large list of different traits defining each and every interaction, but pared it down to only the most crucial ones.
`Processing`, for instance, is implemented by both `Plants` and `Animals`, and defines their behavior in each step of the processing loop, from moving to late processing.
Another trait, `Lives`, is also implemented by `Plants` and `Animals`, and provides a common interface for life functionality such as HP, death, and other individual life aspects.
Some other fun ones are `Reproducing` and `Growing`, which both handle different aspects of life, and more interactive ones like `Mates`, `EatsCreatures<T>`, and `Eaten`.

We even made use of a `PTUIDisplay` trait that entities could use to themselves indicate the character/emoji they could use to represent themselves in our Plain Text UI -- and this representation ended up making it through to our GUI as well.

We really fell in love with the power presented by Rust's trait system throughout this project, and I think it shows through the sheer degree of use they had.

#### Entity structure

This degree of abstraction and separation may seem a little counter-intuitive, but it has provided a fair degree of control when matching.
To take an example from our idle behavior code:

```rust
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
```

We can specifically exclude non-living creatures here, as nobody would like to eat or mate with a rock. We can then drill down and extract specific behavior between the type of creature we are interacting with. All the while, all these entities, whether they be a rock or a crab, can all be stored in collections and the like without requiring trait objects or dynamic dispatch.

[1] These IDs were pretty much just wrapped `usize`s, and were originally intended to be used for AI purposes and entity target tracking, though we didn't really have time to make that happen. Regardless, they are useful in distinguishing between different entities through their lifetime, something that also made its way to the GUI.



### Ownership Hurdles and Considerations

The Ownership system shaped most of our key architectural foundations from very early on. We put a lot of our early thought into how we could build up an effective system that would not have us running into severe borrow checker errors partway through development, which could have forced a complete re-architecting of our system. Since entities were the most "mobile" parts of the system, a lot of effort went into deciding how to best manage them.

#### Entity Management

One of the biggest questions from early on was who would own entities. We had already come up with the idea of a processing list, and we thought it might make sense to store entities there, but that brought up a whole other set of possible challenges and inefficiencies, *especially* with regards to entity-entity interactions. How could we hold a mutable reference to two entities in the same list without wrapping both in `Rc`s or extracting the entities, holding their place in the list, and then returning them to that same spot in the list afterwards? In a language like Java where ownership isn't a thing, perhaps this would have made sense, though it would have made any kind of async processing a real pain.

Idiomatically, it made sense for us to have entities located on `Tile`s within the `Board`. They are distinct slots for entities, and provide both a unique and defined location for where entities can be found (an (x, y) position) as well as a clean interface for controlled access. These entity's positions (stored internally as `Pos` structs, which are essentially just `(x, y)` coordinates) are then stored within the processing list.

As these (x, y) positions (stored internally as `Pos` structs) are both `Copy` and independent from both the `Board` and `Tile`, they can easily be used to query the board for a tile, which then *may or may not* have an entity on it.
This almost forms a level of weal references, but without all the overhead and lack of mutability an `Rc` may have provided. This decision let us steer fully clear from borrowing concerns, while also making it really easy to keep track of which entities we were concerned about.

Even still, our processing system still had to face the facts with the borrow checker. As noted earlier, every entity is removed from their tile before their processing behavior is called.
This emerged from the OBS (Ownership and Borrow System) and our architectural hierarchy. As every possible step (movement, processing, and late processing) may involve changing the active Entity in some way, we need to -- at the very least -- borrow `&mut Entity`. If this `Entity` is still owned by a `Tile` (which is *always* owned by the `Board`), we would be unable to borrow the `Board` mutably *or* immutably due to this outstanding borrow further down the hierarchy. As a result, this pattern emerged from necessity, but ended up having some benefits. It meant that we wouldn't have to worry as much about accidentally having entities interact with themselves, and also made keeping track of active entities almost idiomatic.

The ownership and borrow system may have caused us some headaches, but we believe that we were able to effectively get ahead of it causing us any major problems from the get-go.
Working with Rust has truly taught us that you need to have a very strong and coherent model of such a system before implementing it in Rust. If we had gone ahead gung-ho with certain ideas, we may have reached an impasse and gotten stuck with some behavior that might have required either significant refactoring or a full restructuring.

#### AI Behavior

Our AI behavior is a modular set of behaviors that creatures can start or stop given some conditions. Behaviors control an entity's movement and processing actions each turn they're active. We currently have three behaviors: `Idle`, `Eating`, and `Mating`.

These behaviors' methods are mostly defined by a trait which provides an interface for some standard behavior functions. These include checking if the board is in a valid state to start a given behavior, whether it should still be active, or what the next move action should be.

```rust
pub trait AIAction<T>
where
    T: Lives + Debug + Clone
{ ... }
```

AI movement is generally either randomly walking (as in `Idle`), or pathfinding towards an entity (as in `Eating` and `Mating`). In the latter, creatures find the closest valid entity that matches what they need (either something they can eat, or another creature with whom they can mate).

## Additional Details

### Dependencies

- `game_data`
  - `egui`/`eframe`: GUI tools (needed to signal to the `GUI` updated infor was available)
  - `log`: Managing runtime logs
  - `async-trait`: Allowing us to define trait functions (`late_process()`) as async
  - `futures`/`async-std`: Async support frameworks/runtimes
  - `rand`: Random number generator source for randomness
- `display`
  - `egui`/`eframe`: GUI tools
  - `egui_extras`: More GUI tools
  - `game_data`: To allow for communication
  - `image`: To place a background image on the GUI

### Rustiness

One of the most interesting implementations of creature traits was `EatsCreatures<T>`, where `T` is generic on the type of creature being eaten.

```rust
pub trait EatsCreatures<T: Lives + Eaten>: Lives { ... }
```

This trait being generic allowed us to define different behavior for different creatures being eaten by others, so Animals could have different behavior when eating Plants vs other Animals, and those different behaviors would be backed up by Rust's type system.

```rust
impl EatsCreatures<Plants> for Animals { ... }
impl EatsCreatures<Animals> for Animals { ... }

// then, later

impl Eaten for Plants { ... }
impl Eaten for Animals { ... }

```

In addition, the trait bounds made sure that we could only eat something that had defined behavior for being alive *and* had defined behavior for being itself eaten!

### Difficulties in Expression

#### Enum Structure

Our nested enum structure, while it worked well for our needs, may not have always been the most idiomatic way to approach things. At some times, it felt like we were trying to tack Object-Oriented sensibilities onto Rust where they may have not been the most ideal approach, but we felt like it accomplished what we needed it to.

Since every entity enum only had variants with children, entities could only be matched against if they were properly and fully instantiated with a base-level struct. This required the addition of `NonAbstractTaxonomy`, a trait defined on a set of enums that mirrored the `Entity` structure enums, and specifically defined entities that could be instantiated, rather than representing non-leaf nodes in the entity tree. These could then be matched on, and used to construct a real entity, while also being where our canonical definitions for entities' instantiation lie.

We considered two alternate approaches along the way before arriving at this structure.
First, we considered implementing every entity as their own distinct structs that just implemented traits, and wrapping everything up in trait objects. This might have been nice in some regards and ensured a consistent and clean interface, but would have required dynamic dispatch on just about every entity's function calls, and would have required a massive set of trait definitions for any collection that handled entities. I think that this would have probably been the most future-proof approach, though would have probably take a lot of work, and would have made it hard to provide the same behavior for different entities.

Another approach might have involved keeping the enum hierarchy but defining traits at every level in the hierarchy, and passing the function calls down to the next level (as was done with the AI controller enums, which is another instance that could probably be made more Rusty). We attempted that at first, but it quickly became clear that it was a bit of a nightmare.

In the end, digging through the entity tree to get to a lower level just to call some trait function was sometimes a bit annoying, but having this structure did still provide us with a fair degree of flexibility, especially with matching.

#### GUI Testing

We had set out to design a test suite for the GUI, but had to abandon the effort due to issues we encountered.  The first issue came right at the start when trying to render the GUI from a test function.  It turns out that `egui` must be executed on the main thread, and the standard test implementation does not allow for test to be ran on the main thread.  We learned that the default test harness was the issue, and as such we had to change it.  We did so by creating a separate crate for the GUI tests, and placing the following in the Cargo.toml file.

```rust
[[test]]
name = "guitests"
path = "guitests/main.rs"
harness = false
```

This allows for the main function in the crate to be ran through cargo test, but also means that we had to implement our own functionailty for a test suite.  We dug through this, eventually getting the makings of a test where we were able to render the GUI through our custom test.  Now that we were able to get the GUI running in a test, the next step was to simulate input.  To do this we found the `enigo` crate.  Much like the game loop, we wanted to be able to execute inputs along side the GUI which meant that we would have to run our inputs on a separate thread.  Unfortunately, we found that `enigo` much like `egui` had to be ran on the main thread, which in effect meant that we could not execute out automatic inputs in parallel with the GUI.  We were then left with two options.  The first option would be to have a test that executed and was dependant on manual input, but at this point the test just became manual testing.  The second option would be to create new individual functions that copied sections from the GUI and test those individually, but at that point we were no longer testing the actual GUI code.  As such, we felt it best to stop our attempts at desiging tests for the GUI, and rely on the manual testing we had done.  Along the way we were able to get a better understanding of how Rust tests work.  It was interesting to desing our own test implementation, and while we were not able to get the actual testing to work, it was still a good experience.  Futhermore, we were exposed to the limitations of `egui`.  Since `egui` blocks the main thread, it limits what can be done at the same time.  If there was an option to simulate inputs directly through `egui`, it would have made testing easier.

### Rubric Thoughts

We feel that we've done an effective job at completing the aspects noted on the rubric.

This projecet was a sizable undertaking for our group that signficantly challenged our abilities in Rust and certainly forced us to learn more about the language. We put a lot of work and thought into our architecture, and were able to incorporate a large amount of both basic and advanced material that was both covered in the course, as well as stuff that we had to pick up ourselves. The display crate, with its message-sending system (and the GUI framework et. al.) made great use of more advanced features, while the core foundation of the system was a lot of good ol' foundational Rust.

While we might not have realized it at the start, our project was attempting to do something that is fundamentally difficult in Rust, and we believe that we were able to emerge with a very strong (and fun!) project after it.

We were able to have our teammates contributing to the project, at least insofar as the team we had left by the end of the semester. We did lose a team member fairly late in the development process, which required a good degree of catch-up and may have shown in some regards (such as unit testing being a little weaker), but we put forth our best effort as a team.

As for Style/Design, we feel we made strong use of both stdlib traits and new traits, structs and enums, and were able to make good and effective use of the type system (particularly the borrow checker) to ensure invariants were upheld.
For organization, the `game_data` crate is divided into modules which effectively separates out concerns, while also providing some useful privacy. `Tile` structs, for instance, have their `entity` field private, which allows for things in the same module to interact with it, while requiring that anything outside of the module interact solely with the public getters.

We also have our code split across two crates, which helped to distinguish between the view and the model.

For Correctness, we have verified our code and cleaned up warnings presented by `cargo check` / `cargo clippy`, and have cleaned up our code with `cargo fmt`.

Altogether, we feel that the goals of our project were accomplished. We were able to create a fun wildlife simulation with entities that interacted with each other in complex ways, while having a visually impressive GUI for interacting with the simulation itself. We accomplished all the goals we intended to from our proposal, and while we didn't quite make all of our stretch goals, we certainly have the modularity to get to some of them from where we are now without too much effort.

We have made a fair effort towards incorporating unit testing, including a fair amount of helpers to run some sample simulations. As our final product is a simulation with a significant amount of interactions, this higher-level testing seems to work fairly well, especially when combined with unit testing for some more fundamental components. While we don't have full code coverage, most of the important code paths are covered, and some sanity checks in the program are able to help catch unexpected behavior.
