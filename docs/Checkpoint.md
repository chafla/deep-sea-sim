# Skittering Crustacean

Team members:

- Matthew Thompson
- Austin Couch
- Eric Barron

## Summary Description

Deep-sea simulation where the aquatic creature have to work and survive within the confinds of their resources and environment.  As the simulation progresses the user will be able to follow the progress of the simulation through a GUI.  Furthermore, the user will be able to set starting parameters for the simulation and respond to randomly generated events as the simulation progresses.  The user will also be able to interact with the simulation in other ways, such as choosing to introduce a new creature.

## Checkpoint Progress Summary

Currently we have based the structure of our project around a model-view-controller architecture to provide specialization for the different crates.  Beyond setting up the structure, we have been able to implement basic functionality of our program.  To start, users have the ability to specify the dimensions of the board as well as the starting creatures for the simulation based on the size of the board.  From that we are able to instantiate the required amount of creatures, place them on the board randomly, and display the information back to the user.  From there, the creatures in our simulation can currently move to different locations on the board, and their hunger will tick down eventually approaching zero. Once their hunger gets low enough, they start to lose health until their HP reaches zero, at which point they die and stop moving.  We have yet to implement other interactions, such as animals eating each other or other types of organisms, animals moving with a set goal, and repopulation, but we have the basic frameworks set up to support other more complex functionalities.

## Additional Details

- List any external Rust crates required for the project (i.e., what
  `[dependencies]` have been added to `Cargo.toml` files).
  - The `game_data` crate is dependant on:
    - `log`, which we use for logging and controlling debug outputs (particularly since we want to keep our PTUI display relatively clean).
    - `rand`, which we use for RNG functionality.
    - `clap`, which we use for command line arguments
  - No other crate has external dependencies at this time

- Briefly describe the structure of the code (what are the main components, the
  module dependency structure).
  - Currently, `game_data` is responsible for holding the information of our game, from the board to the creature states.  As such it is dependant on both the `display` and `process_user` crates.  The program is started through `game_data`, which will handle user input through `process_user`, and provide the user information through `display`.  Within the `game_data` crate, `lib.rs` is responsible for the execution of the game loop through the `Sandbox` instance that it holds, which fundamentally represents our game engine.

- Pose any questions that you may have about your project and/or request
  feedback on specific aspects of the project.
  - Any suggestions for GUI frameworks?
  - Is the structure of our entities optimal?  We would appreciate general feedback on how we setup our entity enums.
