# Skittering Crustacean

Team members:
- Matthew Thompson
- Austin Couch
- Eric Barron

## Summary Description

Deep-sea simulation where the aquatic creature have to work and survive within the confinds of their resources and environment.  As the simulation progresses the user will be able to follow the progress of the simulation through a GUI.  Furthermore, the user will be able to set starting parameters for the simulation and respond to randomly generated events as the simulation progresses.  The user will also be able to interact with the simulation in other ways, such as choosing to introduce a new creature.

## Additional Details

- Use case:
  The user would start the application and be presented with a screen allowing them to specify variables such as the
  envrionment to start in, the types of creatures to add to the environment, and their starting resources.  Next, the
  user will start the simulation and watch as it progresses, being able to both respond to random events that occur,
  and add additional creatues to the environment to see their effect.  The simulation ends when either every creature has
  died or the user decides to exit.

- Sketch of intended components:
  Organisms (creatures) - traits describing behaviors and providing information such as HP, damage, needs, etc.  Organisms 
  are defined in an enum, with each organism having a struct to define unique attributes.  Organisms will have differnt 
  interaction depending on their type that they will perform asynchronously. Possible for the enum of organisms
  to expand into multiple enums with different types of creatures.  Food is considered part of organisms with different
  types of food being available such as plants or meat.
  
  Environment - Dictates the availability of resource and fequency of random events.

  Random events- Enum containing the different events.  Each event will have an effect and frequency attached to it through
  a struct.

  GUI - A graphical interface for the user to interact with and follow the progress of the colony.

- Testing:
  - Test being able to establish an environment
  - Test organisms performing properly in a given environment
    - Perform specified interaction
    - Consume appropriate resources
    - Respond accurately to their needs
  - Test how organisms perform with each other in a given environment
    - Compete for resources with proper winner/loser
  - Test random events occur properly with intended impact
  - Test GUI
    - Proper display
    - Proper user interactions allowed


- MVP:
  - Animals able to exist in a sandboxed environment, consuming the proper resources taking ownership of them as they do.
  - Animals able to have basic interactions with each other and environment 
  - User defined initialization of the environment through a GUI, and the simulation appears in the GUI, updating every
     few seconds to demonstrate the simulation occuring.
  - Basic random events that the user have input on

- Stretch:
  - Different types of environments
  - More complex creatures
  - More complex creature interactions
  - More complex random events

- Expected functionality to be completed at the Checkpoint.
  - Basic environment (PTUI) created
  - Basic creature models created and represented in environment
  - Basic creature action (i.e. skittering crustacean)
  - Preliminarly user input (initialization aspects)