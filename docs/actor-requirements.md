# Actor requirements

What actors must do, taken from faery's actor module (`packages/engine/src/world/actors`, with its
`goals/`, and `physics/PhysicsActor.ts`). These are behaviors, not designs: animation, the character
controller and goals are being re-engineered, and state moves onto entities. Numbers are faery's
values, starting points to tune. Decisions already made elsewhere are linked, not restated:
ownership (D24), models and colors (D28), hair (D29), goals as entities (X17), saves (PAN82),
cutaways (PAN76a), footprint readiness (D67). Ids are never reused; a new requirement takes the next
unused number, so ids within a section are not always in order.

## Definition

- **R1** An actor archetype names its skin file, armature file and animation file.
- **R2** An archetype can alias clip names (`actionMap`), so a species whose clips are named
  differently uses the standard animation states.
- **R3** An archetype sets display name, gender (male, female, object, plural), scale (0.6 to
  1.3), color slots and colors, feature slots and features, equip slots, skills, ally group,
  and portrait height and offset.
- **R4** An archetype sets its collision size: height (default 1) and radius (default 0.5).
  The largest actor fits through doors: height 1.4, radius 0.3.
- **R5** An instance sets position, facing, archetype, and optionally realm (global actors),
  ally group (overrides the archetype's), group id, inventory, instance variables and aspects.
- **R6** An instance can be transient (a random monster): created at run time, not authored.

## Lifetime

- **R7** A local actor is defined in its home precinct and despawns with it (D24). A global
  actor (the player, followers) is defined in a global asset and always exists, but its model
  is spawned only near the viewpoint. Transient actors are deleted when their precinct stops
  being simulated.
- **R8** A local actor is simulated only while its precinct is simulated. A global actor far
  from the viewpoint is not simulated: there is no terrain loaded for it to navigate.
- **R9** An actor is shown only while it is simulated, inside a visible area, and phased in.
- **R100** Usually no more than a dozen actors are on screen at once.
- **R10** Phasing: an aspect can make an actor present only in some quest stages. An actor
  that is on screen stays present until it leaves the screen, so it never vanishes in view.
  In the editor, every actor is present.
- **R11** An actor has physics only in the realm being viewed, and only where the colliders under it
  are built (D67).
- **R12** Restoring an actor's initial state (reset) puts back its position, realm, facing,
  animation and goals.

## Timing

Actors sit on many asynchronous processes at different time scales, which overlap and interact:
loading the instance definition, its exemplar and its behaviors; loading the environment it stands
in (D67); goal planning and pathfinding; loading and saving games; speech; animation; physics and
ballistic motion.

- **R104** No process may assume another has finished. Each can be pending, or change while
  another runs (the ground unloads during a path, a save lands mid-step, a line ends during a
  turn), and the actor waits or recovers rather than failing.

## Appearance

- **R13** Model assembly and colors follow D28; hair follows D29; actors are outlined (D14).
- **R14** An equipped item shows its model attached to the bone named by its equip slot (default
  slots `weapon.L`, `weapon.R`, the bones' names in the glTF), and changes as equipment changes. A
  missing bone is reported with the nearest bone name.
- **R15** An actor on a cut-away floor is hidden (PAN76a), and a hidden actor cannot be picked. The
  player's own actor is never pickable.
- **R16** Editing an archetype in the editor updates colors live, and rebuilds the model when
  features change.

## Movement

These hold whatever does the work: the character controller, the pathfinder (constraints on
what counts as a valid route) or behavior. Which part handles each is not yet decided.

- **R17** Paces: walk 1.5 m/s, trot 3, jog 3.5, run 5.
- **R18** Gait: walk below 2 m/s, fast walk below 3.5, run above, falling back to whichever
  clips the species has. Clip playback rate scales with speed against each gait's nominal
  ground speed (walk 1.5, fast walk 3, run 5), so feet do not slide.
- **R19** Changing gait keeps the stride's phase (proportional clip time).
- **R20** Turning: 5 rad/s by default. A path turn sharper than 60° stops and turns in place
  first. An actor turns to face its direction of travel.
- **R21** Path following: an actor follows waypoints, rounding corners rather than hitting
  each point (a lead point one radius ahead on the next segment). Height differences do not
  count toward reaching a waypoint.
- **R22** An actor that cannot advance (moved under 10% of its intended distance for a few
  ticks) plays the blocked state and replans after 1 s.
- **R23** Actors steer apart: under 0.3 m between capsules (0.5 m between allies), movement bends
  away from the other actor. A repulsion, not a collision: it grows as they close, and a forced
  actor can come nearer.
- **R24** Slopes steeper than about 45° (surface normal y under 0.7) are walls. Walking down a
  walkable slope or stairs stays on the ground instead of launching. Stairs collide as ramps;
  from the camera's distance the difference does not show.
- **R25** An actor within 0.1 m of the ground snaps to it. Beyond that it falls: the falling
  state over 0.7 m above ground, and a landing state on touchdown when the species has one.
- **R26** An actor embedded in geometry is pushed out to free space.
- **R27** An actor that falls out of the world is put back on walkable ground. Stopping the fall is
  not enough: faery's slab under the world caught falls but left the actor stuck below it (recovery
  was a TODO). It goes back to the last place it stood, its velocity zeroed; later, to the last save
  point (X52).
- **R28** A pushed actor (not under its own control) moves physically and plays the stumble
  state; its speed is capped at 4 m/s.
- **R29** Direct movement is relative to the player's view: up always means away from the
  camera, and the character turns gradually to match. Backward walks the gait in reverse.
- **R101** Movement and camera ease: no abrupt, mechanical-looking changes in velocity or
  facing. The camera follows the player with a slop zone: within 0.25 m of the camera's
  target it does not move, and beyond that it catches up faster the farther out the player is
  (speed twice the distance past the zone, capped at 5 m past). It jumps when the player is
  20 m away or in another realm. Camera rotation eases too. faery's tuning
  (`PlayerActionController.ts`) felt right; start from it.
- **R102** The camera follows the active entity, usually the current protagonist. A quest may
  switch protagonists, so the player-controlled actor is not fixed. A cut scene without the
  player character has some other active entity, which the player does not control.
- **R92** Click to move gives the player goals, the same kinds NPCs run (faery's
  `PlayerActionController.ts`). Clicking ground travels there by the pathfinder (to within
  0.2 m, at a trot). Clicking an object travels to its interaction distance (default 1.5 m) or
  to its nearest `useFrom` mark, focuses an actor (which halts its patrol, so the player does
  not chase it), then interacts. Clicking an enemy with an enemy-targeted skill starts combat
  mode and stages a plan: pursue to the skill's range, then attack; a second click on the same
  target runs it. A plan clicked while another runs waits as the next. Direct movement
  cancels the goal and the staged plan, as do dialogue, a modal UI, a cut scene and death. The
  target drops when it dies, leaves the realm or is over 12 m away.
- **R103** Clicking stairs climbs or descends them. A stairs exemplar has two use marks with no
  action, one at the top and one at the bottom; only the one at the far end from the
  protagonist is enabled (above or below the stairs' midpoint), so a click is a fixture use
  (R79) with no animation that walks to the other end.
- **R30** Teleporting resets the animation and snaps to the ground. Teleporting the player to
  another realm moves the camera with them.
- **R31** Crossing a portal moves the player into the target realm, keeping their offset and
  velocity through the portal. Only the player crosses portals; NPCs path around them.
- **R83** NPCs stay near home: no NPC travels across the world or between realms, so
  navigation and simulation are needed only around loaded, simulated precincts (R8).
- **R84** Followers go where the player goes, without walking through portals. When the player
  changes realm, followers arrive with them, however far away they were. A follower stuck on
  terrain (pathfinding is imperfect) cheats its way back without being seen: it reappears out of
  view and walks in. Skyrim's followers are the model. *Open:* when a stuck follower cheats
  (distance, time, out of view), and how a quest or scene keeps a follower from reaching the
  player.
- **R85** An NPC who meets the player in several places is a separate placed instance at each,
  each present only in its quest stages (R10); none travels between them. Quest NPCs cannot be
  killed or targeted, so their inventory is never reachable (R74). *Open:* whether the
  instances share an identity (instance variables, the quest role R72 resolving to the present
  one).
- **R86** Followers keep out of the player's personal space and out of each other's way. Both the
  pathfinder and the controller contribute. The pathfinder cuts exclusion zones where other actors
  stand out of the navigation mesh (boolean operations), so routes and destinations avoid them.
  Actors move at the same time, though, so a plan can still cross another actor's path. Between
  actors, a repulsion aura (R23) makes the near miss look natural; with the aura alone and no hard
  collisions between actors, NPCs do not logjam; actors do not collide with each other. *Open:* what
  a crowd converging on one target (eight skeletons rushing the player) needs beyond the aura.
  Candidates, to try in test scenarios: attack slots (places around the target handed out to
  attackers, so only a few close in while the rest wait or circle) and reciprocal velocity obstacles
  (RVO, or its refinement ORCA: each actor picks a velocity that avoids predicted collisions,
  assuming the others do the same).
- **R87** A party has few followers, typically two. *Open:* temporary quest followers (escort
  missions) depend on how many followers fit without crowding; if one must leave the party for
  a while, the game needs a fiction for it.
- **R32** A precise move takes the actor in a straight line to an exact point at a given speed,
  with collisions off. It is one step's motion; getting into a chair or bed chains several
  steps (R82).
- **R98** The navigation mesh has a region type for openable doors, so routes pass through
  them and the actor opens the door on the way (R52). *Open:* locked doors, a separate problem.

## Animation

- **R33** Each animation state names a list of clips; the first the species has is played.
  States: initial, stand, turn, stumble, walk, blocked, falling, landing, combat stance (and
  unarmed), take damage, dying, dead, sleep. Their fallback chains are in faery's
  `animation.ts`.
- **R34** States crossfade, 0.3 s by default, set per state.
- **R35** A state can loop, play once, or play once and hold its last frame (dying, dead,
  sleep).
- **R36** A state can play a sound (again on each loop) and run a particle effect at an
  offset (sleep: snoring).
- **R37** Completing a one-shot state can trigger behavior (landing and taking damage
  re-evaluate goals; a pose action finishes when its animation does).
- **R38** Aspects choose the idle state (for example combat stance while in combat).
- **R39** Speech can drive gesture clips while the line is spoken.
- **R40** Actors off screen do not animate.
- **R41** A missing clip is reported with the clips the species has.

## Actions

The body's vocabulary, which behavior (goals or the player's input) chooses from.

- **R42** An actor runs one body action and one speech action at a time.
- **R43** Body actions: travel along waypoints at a pace (fixed, or computed each tick); move
  in a direction (player); attack or use a skill on a target; park; idle (waiting on the
  engine) and pass (deliberately doing nothing), each with an optional delay; pose (an
  animation, an optional turn with its own rate, an optional precise move, R32).
- **R44** Instant actions: enact (do nothing, finish), deactivate (end the player's focus on
  this actor).
- **R45** An action can report that it finished: travel at the last waypoint, pose when its
  animation ends, idle and pass when the delay runs out, attack and skill when the skill
  completes, speech when the line ends. A higher-priority choice can pre-empt an action, so
  finishing is not guaranteed.
- **R46** faery declared climb (ladders), area-of-effect and converse actions but never
  implemented them. Climbing is wanted, for what it opens up in level design, but can be
  dropped if it proves too hard.

## Behavior

Goals are reworked (X17); these are the capabilities faery's goals provided.

- **R47** Aspects contribute goals; an actor's goals run in priority order. Bands: ultimate
  25, safety 20, mission 15, social 10, health 5, idle 0.
- **R48** A goal reports satisfied, failed, or the action to take. An action can be
  concurrent, letting lower goals act too (speech while walking).
- **R49** Goals are re-evaluated when what they depend on changes, when an action finishes,
  and on a timer a goal requests (wake in N seconds); not every tick.
- **R50** A goal can have a condition; while it is false the goal is skipped, and when it turns
  true again the goal restarts.
- **R51** Composites: sequence (each step once, in order, never back), repeat (its child N times or
  forever, stopping on a failure), random (weighted, re-chosen on restart, optionally never the same
  twice running, as Ivy's work is in R99), ranked (the first that does not fail), contingent (each
  step a prerequisite of the next, rechecked as conditions change, so a step that stops holding is
  returned to; a fallback when one fails). New in Panoply: timeout (fails its child after N s) and
  cooldown (its child cannot restart within N s of finishing).
- **R52** Leaves: travel to a point (preferred distance, minimum distance, tolerance), pursue
  a moving target (replans when it moves 1 m; walks when near, trots under 4 m, runs beyond, or
  always runs when chasing; can open doors; can avoid actors), wander (a random reachable
  point within a marker's radius), wait, pose, look at a point or instance, face a direction,
  mutate state, deactivate, remark, converse, interact (apply a verb), park (R56), use a
  fixture (R79), equip, unequip, attack (player only), target an enemy (NPCs), apply an
  untargeted skill.
- **R53** Remarks: an NPC within 1 to 2 m of the player says a line chosen at random, weighted,
  from remark sets whose conditions hold; then waits at least 5 s; it turns to the player for
  a line with a gesture.
- **R54** NPC goals pause during cut scenes and while the actor is not present. During a cut
  scene the player's actor runs goals, as scripted actions. At other times input drives it,
  by direct movement (R29) or click to move (R92). *Open:* how a cut scene drives NPCs. Whatever is easier
  to author and implement wins; one scene script that sequences every character's actions is
  probably easier to author than a script per actor. The first cut scene could be the
  confrontation with Sentinel Mercy Rosseau in "Up in Arms" (PAN6). In faery it was goals, not
  a cut scene (`cwLeader` in `behaviors/heroncreek.tsx`): within 5 m of the player she speaks
  (`Rosseau.UpInArms`), advances the quest to `confront`, then turns hostile (R91).
- **R99** Daily schedules: a goal's condition can test the game clock's hour range, wrapping
  past midnight (faery's `isHourBetween`). Tambry's NPCs sleep, sit and work by the hour
  (`behaviors/tambry.tsx`); Ivy's work is a looping random choice, never the same twice
  running, among the cook pot, two cutting boards, sweeping and the counter. faery's clock
  (`systems/GameClock.ts`) runs 100 times real time from 8 AM on day one, with a 403-day year,
  and stops in combat mode and under a modal UI. Panoply has no game clock yet.
- **R55** The editor can show each actor's goal tree with the active branch and a one-line
  description of each goal.

## Parking

Sitting in a chair or sleeping in a bed.

- **R56** A fixture's exemplar has marks (`SceneryMarks`, in the fixture's own coordinates):
  `enter`, where an actor stands to get in or out, and `sit` or `sleep`, the parked position.
  Facing comes from the fixture's rotation. The actor travels to the nearest `enter` mark;
  getting out, it uses the nearest `enter` mark it would not collide at.
- **R82** Getting in is a sequence of steps, not one interpolation from standing to the parked
  position, which would tilt the body through the bed. Each step has a pose clip, a duration, a
  facing (the fixture's rotation plus an offset) with a turn rate, and an optional precise move
  (R32). Getting out runs the steps in reverse. From faery (`packages/game/src/goals`):
  - Chair (`Sit.ts`): `SitPrep` turning to 45° off the chair's facing; `SitPrep` turning to
    its facing while moving onto the `sit` mark; then `Sit`, looping. Out: `Sit`, then `SitPrep`
    moving to the `enter` mark, turned 90°.
  - Bed (`Sleep.ts`): `Sit` turning to face away from the `sleep` mark (sitting on the edge);
    `SleepPrep` swinging round to the bed's facing while moving onto the `sleep` mark; then
    `SleepBed`, looping, with snoring particles. Out: `SleepPrep`, then `Sit` moving to the
    `enter` mark and turning toward it.
- **R78** Physics is off while parking, parked and unparking: the capsule does not fit in a
  chair or bed. Moving into place is a precise move (R32).
- **R57** A parked actor unparks before any other body action or speech.
- **R58** Parking state (which fixture, which step, getting in or out) is saved.

## Fixture use

Working at a fixture: Ivy chopping vegetables with a knife, Ivy stirring a stew pot over a
fireplace with a spoon, Poul alternating between the anvil and the forge.

- **R79** A fixture's exemplar records its use marks (`SceneryMarks`, keyed `use`). The actor
  picks the nearest by path distance, travels there, faces the fixture, and plays an animation
  loop.
- **R80** While using a fixture, the actor can wield a tool (a knife, a spoon, a smith's
  hammer), attached to a bone (R14) for the duration.
- **R81** Fixture use composes with other behavior: Poul's alternation is a sequence of two
  uses, and a use can be interrupted by higher-priority goals and resumed. Physics stays on.

## Speech

- **R59** One actor speaks at a time, through the dialogue system.
- **R60** Time since an actor last spoke is tracked (R53 uses it).

## Combat

- **R61** Each actor keeps a threat map: an aggression score per enemy. Dead enemies drop out.
- **R91** Aspects and goals put threat into the map. Typical sources: random monsters are born
  with threat toward the party; territorial monsters gain it toward anyone who comes close
  (within their aggro radius, R90); combatants gain it toward anyone who attacks them; some
  dialogue choices turn an NPC hostile.
- **R93** Leashing, from faery's `territorialAspect` (`aspects/combat.ts`): a territorial
  actor gains threat toward the player within its threat distance (default 13 m), but only
  while it is within 3 m of home; past 20 m from home it clears its threat; with no threat it
  returns home (to within 2 m). The leash keeps a chase near its home precinct (R7, R83).
- **R94** *Open:* whether walls and floors block aggro (line of sight, as R89 wants for
  attacks), depending on cost. faery's check was distance only.
- **R95** Fleeing depends on the aspect (Cowardly, R90). At worst a fleeing monster runs to
  nearby allies, or allies spawn out of view, and they take up the threat.
- **R96** *Open:* followers in combat. Options: (a) followers assist on their own, take no
  commands, and the player changes tactics by changing followers; (b) simple commands, such as
  focus on this target; (c) selecting a follower shows its own action bar, the player picks its
  exact attacks or spells, and it uses its own judgment once the orders run out.
- **R62** Combat mode starts when an actor with threat toward the player is within 4 m and ends
  when none is within 10 m (hysteresis). A player who turns combat on keeps it until a threat
  leaves. Never during cut scenes or when the player is dead.
- **R63** Turn-based mode: each actor has an action clock; the visible living actor
  with the lowest clock acts next. Moving counts against the clock and ends the turn after 1 s.
  A skill advances the clock by its duration. A deliberate pass jumps past the current clock.
  Clocks reset when combat mode changes, and a newly visible actor joins at the global clock.
- **R88** "Bollywood combat": against a group, one or two enemies engage at a time while the
  rest hold back and wait their turn (circling, posturing), and the leader is left for a
  one-on-one fight at the end. This is attack slots (R86) as a combat style, and it also keeps
  a crowd from jamming around the player.
- **R89** Combat positioning depends on the skill or weapon in use: an attacker moves until the
  target is within that skill's range and in line of sight, and no closer than it needs to (a
  ranged attacker does not close to melee). Unless every weapon has the same range, strategy is
  parameterized by range. faery's pursue goal had the parameters (preferred distance, minimum
  distance, tolerance, R52) but not line of sight. *Open:* combat strategy in general is
  undeveloped.
- **R90** Monsters of one type are not all clones, so players cannot learn one exact aggro
  range and camp just outside it. A fraction of spawned monsters get a random variant aspect,
  shown in their name: Vigilant Ogre (larger aggro radius), Tough Ogre, Speedy Ogre, Cowardly
  Ogre. So aggro radius, toughness, speed and nerve are per-actor values that aspects modify.
  *Open:* the fraction, whether variants stack, and the list of variants.
- **R64** A skill has an effect delay (applied only if the user is still alive), a duration
  (ends the turn and the action) and a cooldown (a global cooldown on the actor).
- **R65** NPC targeting picks an enemy by threat plus a random term, among enemy-targeted
  skills that are enabled. Planned but not built: sticky targets, distance, enemy health,
  crowding, path failures, resistances. Skills and NPC strategy are the hard part of combat,
  turn-based or not.
- **R66** Effects (buffs, debuffs) tick every 0.5 s of effect time, which advances only on the
  actor's turn in turn-based mode. They have attach, tick and detach hooks, a duration, and
  can be dispelled by predicate.
- **R67** Being hit plays take damage when standing. A dead actor plays dying then dead, keeps
  its body in place, and can still wait (cut scenes).
- **R97** The player's first death starts a quest that explains, in the game, how respawning
  works. Later deaths return the player to the last safe location, with followers and gear and
  no corpse.
- **R68** Allies share a group, mainly so monsters do not attack their own kind. Allies also
  keep more distance when steering (R23).
- **R69** An actor can be attacked if its interactions offer attack.

## Interaction and queries

- **R70** The player has a target instance (what they are attacking or talking to); setting it
  turns the player toward it.
- **R71** Aspects provide the interactions the player sees on an actor, depending on combat.
- **R72** Quest roles name actors for quests; the quest system finds an actor by role.
- **R73** Queries: actors overlapping a box, actors within a radius of a point (each with a
  predicate), actors in a group, actors with an aspect.

## Inventory

- **R74** Inventory matters only for shopkeepers and for combatants whose corpse can be looted;
  equipment shows on the model (R14). Quest items are not held by NPCs: an item given to the
  player is spawned when needed (often as a dialogue side effect), and an item the player hands
  over is despawned at once.

## Saving

- **R75** Saved per actor (PAN82): realm if not home, inventory if changed, instance variables. An
  actor at home starts at its authored place and its goals move it on (D80); only followers save
  position, facing and parking (R58). A transient actor also saves its archetype, aspects and group,
  so it can be re-created.
- **R76** Loading a save removes transient actors the save does not mention.

## Diagnostics

- **R77** Counts of actors, simulated, visible and with physics, and the global action clock.
