# Skittering Crustaceans

A deep-sea simulator developed in Rust over the course of `Advanced Programming Skills - Rust (CSCI-541)` at RIT, instructed by Professor Matthew Fluet. 

Inspired by colony sims such as Dwarf Fortress and Rimworld. Spawn in a set of creatures such as fish, sharks, and (of course) crabs, and watch them go about their lives and reproduce. 
It's not all fun and games for the creatures, though, who eventually grow hungry. 
Fish and crabs can get a quick snack from the plants on the ocean floor, but too many creatures might finish them off before they get the chance to spread seeds and repopulate.
Eventually, crabs and fish may try to attack each other for a bite. Sharks, naturally, won't even bother with plants when they get hungry.
On top of that, the ocean is a dangerous place subject to random events, which you get a say in.

Like most colony sims, how you play is up to you. You can treat it as a sandbox and try different combinations of creatures to watch them putt around, or strive to find the perfect equilibrium that keeps the colony alive as long as possible.

---

This repository is a copy of the original at the time of submission, and reflects the team's collective effort over the semester.
The only files omitted were the professor's comments/feedback regarding our progress, in an effort to respect his privacy, as well as the demo video,
which is now linked below to save space.

Design documentation and justifications can be found under `docs/`, and the program itself can be found under `deep-sea-sim`. 
You can run the program yourself by calling `cargo run` inside of `deep-sea-sim`, assuming you have rust installed.

[Demo video](https://drive.google.com/file/d/104fz6bFhs84-dQusnmJZjWk2T79JGYjY/view?usp=sharing)

Final team members*:

- Matthew Thompson
	- Developed most of the underlying game logic, including the main processing loop, the AI controllers, and the main board setup.
- Eric Barron
	- Developed the UI/front-end components of the application, as well as some of the game logic.

\* Due to extenuating circumstances, a third team member had to leave partway through the project.
