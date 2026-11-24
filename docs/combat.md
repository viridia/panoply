# Combat

Panoply's combat borrows its conventions from Bollywood and Hong Kong martial arts films, where a
fight is staged for the audience, not for realism ([goals.md](goals.md)). Each convention below could
also be a mechanic.

This is a brainstorm, not requirements: none of these ideas is adopted. Many are film camera work
(slow motion, replays from several angles, entrance shots) and may not survive Panoply's overhead
camera, which looks down on the action from a fixed distance as Ultima's did.

## Conventions that are mechanics

- **Time scale follows significance.** Henchmen resolve fast, almost as a montage, one hit sending
  them flying. Rivals get full exchanges. A killing blow, a reversal or a critical choice drops into
  deep slow motion, with a replay from several angles. The system judges significance (a big swing
  in health, a story-flagged enemy, a near-death state), not the player, and keeps it scarce.
- **The circle.** Enemies engaged with any one character are capped, at about two. The rest
  posture, circle, taunt, pick up weapons or reposition: busy, but not attacking. When an engaged
  enemy falls, the next steps in. The screen shows a crowd fight; the player thinks about a small
  exchange.
- **Tempo.** Enemies act only as fast as the player does: each player action yields action points,
  which the enemies spend. Every action then has two prices, what it does and the points it hands
  over, so a heavy blow is stronger and also gives the enemies more to spend, and a weak action that
  hands over almost nothing can be the right one. Precedents: Superhot, where time moves only when
  the player does; roguelike energy systems (Angband, DCSS); Slay the Spire's intents. Skills that
  work on the points:
  - rate: a stance that hands over fewer points per action, at a cost in damage or reach;
  - direction: a taunt that gives the points to one enemy, choosing who in the circle acts;
  - denial: a stun or interrupt that destroys an enemy's banked points;
  - banking: an enemy saving points for a big action, which is what a support role's telegraphed
    beat is; the player drains it, dodges it or races it;
  - conversion: points an enemy spends on a hit feed the hero's Resolve, putting the comeback on the
    same economy.

  Open:
  - whether the enemies share one pool, acting as a group whose spenders the circle chooses, or each
    has its own, which makes taunts and redirection matter more;
  - a floor on every action's points, or a cooldown on the cheap ones, since an action that hands
    over none is free turns without end;
  - nothing of the player's recovers over real time, since waiting is safe and would refill it:
    recovery follows actions, or the enemies' points;
  - whether followers' actions hand over points, and to whom;
  - how the points show: a bar over each enemy reads plainly but breaks the cinematic look; posture,
    the wind-up that is the bank, fits "impact effects are information" but is harder to read in a
    crowd.
- **Support roles.** Henchmen who are not engaged act on the fight instead of on the hero:
  - grappler: holds the hero so an engaged enemy hits freely; breaking free is a choice;
  - weapon passer: feeds weapons or items to the attackers, and the throw can be intercepted;
  - blocker: closes exits and props, shrinking the arena and the options;
  - hostage-taker: threatens an ally or bystander;
  - saboteur: sets fires, cuts ropes, smashes lights, changing the arena over time;
  - hype man: raises the others' morale while he stands, and demoralizes them when he falls;
  - spotter: reveals the hero's position, weakening feints and stealth.

  These come from films set in modern times. Fantasy adds many more ways to stack the deck: healers,
  buffers, enchanters, cursers, and small tricks such as a bandit throwing sand in the hero's eyes or
  an orc lobbing a stink bomb into the arena.

  Support actions are slower than attacks and telegraphed a beat ahead ("he's circling behind you
  with a rope"), so the player chooses whether to deal with them rather than reacting.
- **The comeback.** Taking damage, or seeing an ally or someone protected threatened, builds
  Resolve instead of only draining the hero. At a threshold a short cinematic fires (a flashback,
  the hero's theme, a stance change) and the hero enters a comeback state: new moves, faster, heavier
  hits. The trigger reads the character's relationships and story, such as bonds and oaths.
- **The stand-off.** Before the fight, taunts and intimidation are actions. Their outcome sets
  initiative, morale, and whether henchmen run.
- **Props.** Arenas hold objects (ladders, chairs, chains, oil drums) that offer contextual actions,
  rewarding attention to the space rather than to menus.
- **Stances and styles.** Changing stance is a choice of tempo and commitment, not only of stats.
- **Arriving mid-fight.** A party member arrives or tags in partway through, with a slow-motion
  entrance: reinforcements, or a way to rotate who is active in a large battle.
- **Clearing the arena.** When the boss steps up, the remaining henchmen pull back to watch, and the
  fight becomes a duel: a clean phase for boss design.
- **Impact effects are information.** Hit-stop, speed ramping, loud foley and repeated angles signal
  a critical, a broken guard or a stagger.

## The action bar

A deck-building game's hand, without the cards. The bar has two parts: on the left, plain actions
always available (stab, block); on the right, stronger actions that are not always available.

- **Available by situation, not by chance.** A random draw from a curated deck rewards thinning it
  to the best few cards, until the deck is a fixed bar again. Opportunity actions instead appear
  when their condition holds, as a riposte after a parried thrust: thinning gains nothing, and the
  power lies in waiting for the moment.
- **A loadout budget**, chosen before an encounter, limits what can appear: slots or points, as
  Guild Wars' eight-skill bar with one elite.
- **Against a single best path:**
  - Encounters vary and can be read beforehand (at the stand-off), so no loadout suits every mix of
    enemies: archers want gap closers, a healer behind the line wants interrupts, a crowd wants area
    attacks.
  - Rivals who see the same move twice learn to read it.
  - Few actions modify other actions; most broken combos come from those.
  - Six to eight right-hand actions prove the bar before it grows.
- **A headless simulator** runs encounters thousands of times with simple bot policies, reporting
  win rates and how often each action is chosen and used; an action chosen almost always or almost
  never is a balance problem. Abilities are assets and formulas (X53, [saga.md](saga.md)), so this
  needs only combat resolution that runs without presentation. Depth is the margin by which a bot
  that searches a few beats ahead beats a greedy one, which takes the best immediate payoff: if
  greedy play nearly matches search, there are no tactical choices to make, and a subtle action is
  one that search picks and greedy does not.

## Signals

What the player must read (an enemy's bank, who is in the circle, which opportunity actions are
near) is carried by the presentation, never by numbers in the scene: particles, glows, emanations,
outlines, sound, the camera and time. Anime and fighting games already use effects this way, for
blows as for spells, so the vocabulary is one players know.

- **A channel suits the fact's shape.** What builds up shows continuously: an aura thickening, dust
  kicking up at an enemy's feet, a rising hum, as its bank grows. A threshold shows as an event: the
  eye glint before a strike, a sound sting, a beat of hit-stop, at the moment a decision is due. A
  state shows steadily: rim light, outline weight or framing for the enemies in the circle, the
  others visible but quiet.
- **Outlines are a channel** few games have: their width, color or pulse on the enemy about to act
  are cheap, already the look, and read in a crowd better than particles.
- **Tells belong to the creature.** The fact is shared (morale falling), its tell is each kind's
  own, named with the creature as data. Human henchmen take manga's emotion symbols (the anger vein,
  the sweat drop, the "!"), sweating when the hype man falls; insects chitter, clack their mandibles
  or flatten their antennae; a worm coils, pulls back or flushes color, as a cuttlefish does.
- **A salience budget.** Most channels are quiet most of the time, and the most urgent fact gets the
  strongest one. Only the engaged enemies and a telegraphing support role signal at all. A fact that
  can lose the fight shows on two channels, one of them sound, so a crowded screen cannot hide it.
- **Exact on demand.** The scene stays cinematic; numbers are the action bar's, when the player
  asks. Hovering an action flares the enemies' auras by the points it would hand them, previewing
  its tempo price with no number in the world.
- **Tested by a stranger.** The simulator cannot judge readability. A staged screenshot shown to
  someone who does not know the rules: who strikes next, and which of the hero's moves is cheap now?
  If the picture answers, the channels work.

How effects are built is [particles.md](particles.md)'s.

## Thematic skills

Each virtue owns a kind of moment in combat, and its combat skills are opportunity actions that
answer it ([virtues.md](virtues.md), Combat skills).
