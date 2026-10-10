# Requirements: `bevy_remote_driver`

> `MUST` / `SHOULD` / `MAY` per RFC 2119. Requirements are numbered `DR<section>.<n>`, and sections
> `DR1`–`DR7`, so a reference to one is never mistaken for one of `bevy_action_map`'s `R` numbers.

A driver for a running Bevy app: it finds entities, sends input to them, waits for the app to
settle, and checks what it shows. Its purpose is automated testing of an app with a window, by a
developer or by an agent, over Bevy's remote protocol (BRP).

**What this document admits.** Normative statements: each one can be violated. How the driver meets
them is [design.md](./design.md).

---

## 1. Placement

**Problem.** BRP already queries, spawns, writes messages, triggers events and streams observed
events. A driver that grows its own transport, or its own copy of any of that, is a second protocol
to maintain.

- **DR1.1 (MUST)** The driver is built on BRP. It adds methods through `RemoteMethods` and adds no
  transport of its own.
- **DR1.2 (MUST)** The driver does not depend on `bevy_action_map`, or on any other crate that an
  app under test happens to use.
- **DR1.3 (MUST)** A capability BRP already provides is used as BRP provides it, rather than being
  wrapped in a driver method.
- **DR1.4 (MUST)** An app under test needs nothing beyond adding the driver's plugin and giving
  stable ids to the entities a test names (DR2).
- **DR1.5 (MUST)** The plugin opens no port unless the app is started for testing. A build that
  contains the plugin behaves as though it did not when started normally.

## 2. Selecting entities

**Problem.** A test that names an entity by its `Entity` id, a screen position or a component that
happens to be unique breaks when the scene changes for any unrelated reason.

- **DR2.1 (MUST)** A test selects entities by `Name`. A selector is a path of names, where each name
  after the first identifies a descendant of an entity the previous name identified, at any depth.
  Unnamed entities in between are skipped.
- **DR2.2 (MUST)** A step that acts on one entity fails when its selector matches none or several,
  and the failure lists what it matched.
- **DR2.3 (MUST)** The ids a test selects by cost nothing a release build would notice. They are not
  stripped from a release build or gated behind a feature.
- **DR2.4 (MUST)** Selection does not treat a `Name` as display text or require it to be unique in
  the world. A path is unique if the test needs it to be.

## 3. Waiting

**Problem.** A test that sleeps for a fixed time where it could wait for a condition is slow when
the app is fast and fails when it is slow. A covered window renders at about half the frame rate of
one in front, so wall-clock time is not a measure of how far the app has run.

- **DR3.1 (MUST)** A test can wait until a selector matches.
- **DR3.2 (MUST)** A test can wait until a scene has finished spawning, including a scene that
  finished before the wait began.
- **DR3.3 (MUST)** A test can wait for a number of frames the app has run.
- **DR3.4 (MUST)** Every wait has a limit, and a wait that reaches it fails the step, naming what it
  was waiting for.
- **DR3.5 (MUST)** Consecutive input steps reach the app on distinct frames, unless the plan asks
  for them in one frame.
- **DR3.6 (MUST)** A test can wait for a stated time, as a margin for work the app does not signal
  the end of, such as assets loaded after a scene is ready.

## 4. Input

**Problem.** Bevy's own example clicks a button by querying its transform, dividing by the window's
scale factor and writing three messages. Every test that clicks something repeats that arithmetic.

- **DR4.1 (MUST)** A test can click a UI node by selector, and the click lands at the node's centre.
- **DR4.2 (MUST)** A test can press and release a key by key code, as separate steps, so a key can
  be held for a stated number of frames.
- **DR4.3 (MUST)** A test can type text. A printable key carries the text and the logical key a real
  keyboard with a US layout would send.
- **DR4.4 (SHOULD)** A test can connect a virtual gamepad and press its buttons and move its sticks.
- **DR4.5 (MUST)** Input the driver sends reaches the app whether or not its window has focus.

## 5. Observation

- **DR5.1 (MUST)** A test can save a screenshot of the window, and the screenshot shows what the
  window shows, even when another window covered it when the step began.
- **DR5.2 (MUST)** A test can check that a selector matches, and that it matches nothing.
- **DR5.3 (SHOULD)** A test can check a component's value on a selected entity.

## 6. Running a test

**Problem.** Each prompt for approval brings the editor to the front and covers the window under
test. A run that needs attention partway through fails because of that.

- **DR6.1 (MUST)** One command builds and launches the app, runs a test, collects its results and
  closes the app. Nothing is asked of the person running it until it finishes.
- **DR6.2 (MUST)** A run closes the app when it passes, fails or is interrupted, and does not leave
  the app running.
- **DR6.3 (MUST)** A failure report names the step, what the step expected and what it found, and
  gives the path of the app's log and of every screenshot taken.
- **DR6.4 (MUST)** Two runs on one machine can use different ports.

## 7. Tests and the client

- **DR7.1 (MUST)** A test can be written as a data file, which the client runs without anything
  being compiled.
- **DR7.2 (MUST)** A test can be written as a Python program that uses the client, reaching every
  step a data file can.
- **DR7.3 (MUST)** The client needs only Python's standard library.
- **DR7.4 (MUST)** The driver ships instructions that an agent follows to write, run and diagnose a
  test.
