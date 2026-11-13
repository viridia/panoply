# Particles

Visual effects split in two: particles, purely visual, instanced and drawn in bulk (PAN86); and
missiles, entities that collide, light, sound and emit particles (PAN87). This doc records what
particles must do, the engines considered, and the design Panoply leans toward.

## Uses

- Combat spells and magical areas of effect.
- Emotional reactions in a manga style above an actor's head: snoring Z's (R36), and others like
  it.
- Trails behind missiles: a missile owns an emitter that follows it.

faery's shipped effects used only textured billboards, disc and sphere emitters, curves of size,
colour and alpha over age, spin, bursts and continuous emission, and normal or additive blending.

## Requirements

1. Billboarded quads with a texture, and flipbooks (the Z's animate).
2. Mesh particles, aligned to the camera, the direction of travel, or world up.
3. Point, disc, sphere and cone emitters; bursts and continuous rates.
4. Curves over age for size, colour, alpha and spin, with random variance at spawn.
5. Gravity or constant acceleration, and damping.
6. Normal and additive blending, depth-tested against the scene without writing depth.
7. An emitter attaches to an entity at an offset and follows its transform.
8. A stopped emitter lets its particles live out their age, then removes itself.
9. Effects are assets, editable through reflection in the editor.
10. It builds on Bevy main (D34) at a maintenance cost Panoply can carry.

Collision with the world is out: Panoply's terrain is too large to sample from a shader, and
what must collide is a missile. WebGL2 is not targeted (see Platforms).

Swarm, the effects engine of The Sims 2, is a source of ideas rather than requirements: an
effect composed of typed parts (particles, decal, camera, screen), and nearly every parameter a
curve over life. Composition sits above particles: a spell's effect may bundle particles, a ground
decal, a camera shake and a sound. A [modder's
note](https://gist.github.com/ammaraskar/62f4af3a02e82e0ea11be62999854194) lists its parameters.

## Engines considered

- **bevy_hanabi** meets requirements 1–8 and much more: an expression graph compiled to WGSL,
  ribbons, and child spawning on the GPU. It is large, and most of it sits on render internals,
  the part of Bevy main that changes most, so a fork pinned to Panoply's commit would need
  migrating at every bump. It needs compute shaders.
- **bevy_enoki** is 2D only: CPU simulation with SIMD, instanced quads, point and circle
  emitters, curves of scale and colour, sprite sheets, RON assets and its own editor. It runs on
  WebGL2. Being 2D rules it out.
- **Panoply's own**, in two layers (below). The recommendation, since Panoply's requirements are
  modest and the code that churns with Bevy main stays small.

## Two layers

The artistic space of particles is too large for any engine to offer every parameter, so
engines curate. The split puts the curating in an upper layer, over a lower layer that is
unopinionated and could serve several upper layers.

- **Lower layer:** particle storage in a layout the upper layer declares; lifetimes and a free
  list; spawn and update dispatches running shader code the upper layer supplies; indirect
  instanced draws; sorting for alpha blending; joining Bevy's views, depth and transparent pass.
  It knows nothing of emitters, curves or forces. It also draws instances simulated on the CPU,
  behind the same draw path.
- **Upper layer:** the attributes, emitter shapes, curves and forces, the asset format and its
  editor. It produces the layout and shader code the lower layer runs.

The lower layer is unopinionated only if two different consumers fit it, so two are built:

- **Curated:** Panoply's own layer, on one uber-shader with baked curves.
- **Compiled:** WGSL generated from a description, warmed up at load. This one tests the
  boundary and is also the escape hatch for a one-off effect.

The first slice is the campfire's smoke on the CPU path: an emitter on a fixture, one rate, one set
of curves, one blend mode. The snoring Z's follow, on an actor.

## Shaders

Blend mode, depth state and vertex layout (billboard or mesh) are pipeline state, so every
approach has a few pipeline variants. Inside the shader there are three choices:

- **Uber-shader:** one shader each for spawn, update and draw, with each effect's parameters in a
  buffer and features switched by flags. A dispatch runs one effect, so its threads all take the
  same branch, and branching is cheap. The cost is registers: the GPU reserves enough for the
  heaviest path, which can lower occupancy. Curves are baked into rows of a small texture (say 64
  samples each) and sampled by age, so any parameter can be a curve at little cost.
- **Compiled per effect:** each effect pays only for what it uses, and expressions are
  arbitrary. Bevy's pipeline cache compiles asynchronously, so a new pipeline doesn't stall the
  frame; instead the effect is missing until it is ready, which loses a one-shot impact. The
  effects a precinct uses are known from its content, so their pipelines are queued while it
  loads.
- **Shader defs:** one source with `#ifdef` switches, and a pipeline per feature set that some
  effect uses. Bevy's materials work this way. Warm-up is still needed, but for fewer pipelines.

## Platforms

Steam and itch.io downloads are native, and browsers can run Panoply through WebGPU. WebGL2 has
no compute shaders and, in Bevy, no storage buffers, which would limit the whole renderer, not
just particles. Keeping the CPU path in the lower layer leaves a WebGL2 build possible.

## Upstream

No single core serves every Bevy user: CPU or GPU simulation, WebGL2 or compute, 2D or 3D, an
uber-shader or generated code, and each choice rules out others. An engine upstream would leave
some users out unless Bevy hosted several cores, so third-party engines keep a place. Panoply's
two layers are offered as an example of how such an engine might be structured, not as code to
upstream.
