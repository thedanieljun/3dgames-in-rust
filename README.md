# 3dgames-in-rust

For our group’s final unit project, we completed a single 3D game hitting 5 feature

components. Given the difficulty of designing a 3D game as well as the time frame given to

complete the entire unit, we tried our best to implement all 5 features cohesively, focusing more

on game engine rather than game design. We created a game called “Hole-in-the-Wall”, with the

intent of mimicking the popular television game show, where the player tries to fit themselves

into a literal hole in a wall coming towards them. Our game is nowhere near as complex as the

actual game show, and instead is the “foundational” version of the game. In “Hole-in-the-Wall”,

the player controls a box using QE/WASD/space to rotate, move, and jump (respectively) a

player box into a hole in a wall coming at them. Successfully jumping through the hole adds to

the score count while jumping into the wall causes the wall to break into pieces.

For our first component, we decided to implement menus/modal UI. Because we had

already written code that constructed a menu/modal UI for our 2D games, we decided to reuse

the concept of game states to implement this feature for our 3D game. In our start state, we

essentially have 3 boxes. When the user collides with the left box, we start the actual game. The

right box shows scores and the front box loads our save state with images of text displayed on

each box to denote which is which. The text for the scores are updated for each run of the game

when entering our end state. At the end state, the user is presented with the same set of

options, thus restarting the game and fulfilling our menu/modal UI feature (1). The second

feature we implemented was save/load progress. Pressing enter saves the player position and

wall level as well as keeps track of the highest score, both recorded in a separate file accessible

from our menus. We then just generate a new wall that is the same to the last one recorded

when loading in a save state (2). The third feature we tackled was spatial audio, footfalls, and

3D audio sources, tied into gameplay. Because our game by nature does not deal with

character movement, but instead a box moving left/right and jumping, we handled spatial audio

and 3-D audio sources. Firstly, the player box produces noise that pans corresponding to its

left/right/up movement. As the wall gets closer, the sound of an oncoming train physically feels

as if it is approaching the player with a directional volume increase. Upon player collision with

the wall, the wall makes one of two sounds, a ore wall cracking (diamond wall) or glass

shattering (glass wall). Also depending on where the player position is, the audio of the

"breaking effect" is also 3D, meaning different breaks on different parts of the wall have

directional audio (3). The fourth feature we hit was destructible/modifiable terrain, which was

achieved by having the solid wall break into multiple cubes then rotating and flying all over the

place when the player box makes contact with the wall and not the hole, completed by having

restitution change velocity instead of position (4). The fifth feature we hit was collision beyond

AABBs. Besides the obvious collision that occurs when the player box jumps into the wall we

showed that our collision handles non-axis aligned boxes, so when the boxes rotate as the wall

gets destroyed, collision is still maintained, as the user box cannot force itself into one of the

broken boxes. The boxes bounce on the ground even when they're rotated (5). Aside from the

features, a change that we decided to implement was changing the textures given to us and

replacing them with our own to spice up the game while also not feeling as if we just relied

solely on the engine code provided for us.
