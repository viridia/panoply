"""The editor's property panes, below the scenery tools' lists: one representative field, and the
catalog entries a floor or terrain effect edits, each saved and restored.

    bevy-remote-driver plans/properties.py

- Floors: the floor tool lists the floor catalog's entries, and the chosen one's fields show in a
  pane with no aspects button. Raising a floor type rebuilds the floors of that type, undoably,
  and Save writes that one line into its catalog (restored afterwards).
- Terrain effects: the terrain effect tool lists the effects, the hole among them. Raising the
  tilled rows rebuilds the ground under them, and Save writes that one line.
- Exemplars: the chosen actor's properties show below the list, its own pane, then one per aspect.
  A number typed there changes it, undoably, its drag rounded, and a color shows in its swatch.
  Save writes that one line into its catalog, the reload brings the edit back, and the undone edit
  saved leaves the file as it was. Inherited aspects show dim, and an aspect's required aspects
  that the exemplar lacks follow it; + adds one, the pencil overrides an inherited one, and ×
  reverts it.

The other fields' edits, and the lines each saves, are checked by `cargo test`
(`properties/edit.rs`, `panoply_exemplar`'s `save.rs`). Settings start empty, so the panel starts
hidden.
"""

from _editor import (
    ASSETS,
    NUMERIC,
    SPOT,
    TEXT,
    WAIT,
    choose,
    expect_choice,
    expect_disabled,
    expect_marker,
    open_editor,
    restore,
    retype,
    save,
    saved_diff,
    scenery_tool,
    undo,
    wait_mode,
)
from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TAGS = ["editor"]
PRECISION = "bevy_feathers::controls::number_input::NumberInputPrecision"
SWATCH = "bevy_feathers::controls::color_swatch::ColorSwatchValue"
SCROLL = "bevy_ui::ui_node::ScrollPosition"
CHARACTERS = ASSETS / "exemplars" / "characters.exem.json"
FLOORS = ASSETS / "catalog" / "floors.floor.json"
TERRAIN_EFFECTS = ASSETS / "catalog" / "terrain-effects.terrainfx.json"
# Inside the playground's precinct (0, 0), among its floors.
FLOORS_SPOT = [8.0, 0.0, 8.0]
# The tilled rows (`Tilled.Z`) in the playground's precinct (1, 0).
TILLED_SPOT = [73.0, 0.0, 26.0]


def run(driver):
    driver.goto(FLOORS_SPOT, 15.0)
    open_editor(driver, "Scenery")
    driver.caption("floors")
    floor_entries(driver)
    driver.caption("terrain effects")
    terrain_effects(driver)
    driver.caption("exemplars")
    driver.click("PlaceActor")
    wait_mode(driver, scenery_tool, "PlaceActor")
    exemplar_properties(driver)
    driver.exits("Escape")


def floor_entries(driver):
    """The floor tool lists the floors, catalog entries, and the chosen one's fields show below the
    list, in a pane with no aspects button. Raising a floor type rebuilds the floors of that type at
    the playground's origin, undoably, and Save writes the edit into that entry alone."""
    driver.present("Exemplars/MossyPaversBordered", timeout=WAIT)
    driver.present("Exemplars/WoodWalnut")
    driver.absent("Exemplars/CottageWall")
    choose(driver, "floor", "MossyPaversBordered")
    raise_input = "Properties/raise/input"
    driver.expect(raise_input, NUMERIC, {"F64": 0.0})
    driver.absent("PaneHeader/aspects")
    driver.screenshot("floor_entry")
    retype(driver, raise_input, "0.5")
    driver.key("Enter")
    driver.expect(raise_input, NUMERIC, {"F64": 0.5})
    driver.frames(10)
    driver.screenshot("floor_raised")
    row = "Exemplars/MossyPaversBordered"
    driver.click(row)
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set MossyPaversBordered.raise")
    driver.expect(raise_input, NUMERIC, {"F64": 0.0})
    undo(driver, redo=True)
    driver.expect(raise_input, NUMERIC, {"F64": 0.5})
    saved_diff(driver, FLOORS, row, ["-    }", "+    },", '+    "raise": 0.5'])
    driver.click("Exemplars/ClearFilter")


def terrain_effects(driver):
    """The terrain effect tool lists the effects, catalog entries, the hole among them, and the
    chosen one's variant and fields show below the list. Raising the tilled rows in the
    playground's precinct (1, 0) rebuilds the ground under them, and Save writes the edit into that
    entry alone."""
    driver.click("DrawTerrainFx")
    wait_mode(driver, scenery_tool, "DrawTerrainFx")
    driver.goto(TILLED_SPOT, 15.0)
    driver.present("Exemplars/Tilled.Z", timeout=WAIT)
    driver.present("Exemplars/Hole")
    driver.absent("Exemplars/MossyPaversBordered")
    choose(driver, "terrain_fx", "Tilled.Z")
    elevation = "Properties/elevation/input"
    driver.expect(elevation, NUMERIC, {"F64": 0.3})
    driver.absent("PaneHeader/aspects")
    driver.screenshot("terrain_effect_entry")
    retype(driver, elevation, "1")
    driver.key("Enter")
    driver.expect(elevation, NUMERIC, {"F64": 1.0})
    driver.frames(10)
    driver.screenshot("terrain_effect_raised")
    saved_diff(
        driver,
        TERRAIN_EFFECTS,
        "Exemplars/Tilled.Z",
        ['-      "elevation": 0.3,', '+      "elevation": 1,'],
    )
    driver.click("Exemplars/ClearFilter")
    driver.goto(SPOT, 20.0)


def exemplar_properties(driver):
    """The chosen exemplar's properties show below the list: its own pane, then one per aspect. A
    number typed into one changes the exemplar, undoably."""
    driver.present("Exemplars/FeralDogAlpha", timeout=WAIT)
    driver.click("Exemplars/Filter")
    driver.type("FeralDogAlpha")
    driver.click("Exemplars/FeralDogAlpha")
    expect_choice(driver, "actor", "FeralDogAlpha")
    driver.present("Properties/extends")
    scale = "Properties/ActorScale/input"
    # An f32 field's input holds its shortest decimal (I39).
    loaded = {"F64": 1.2}
    edited = {"F64": 1.5}
    driver.expect(scale, NUMERIC, loaded)
    # Its `Precision` rounds a drag to two places.
    expect_marker(driver, scale, PRECISION, True, ("rounded", "unrounded"))
    # A color's swatch shows it (I48's workaround): Fur is #a68d6b, its channels as `f32`s.
    fur = {
        "Srgba": {
            "red": 0.6509804129600525,
            "green": 0.5529412031173706,
            "blue": 0.41960784792900085,
            "alpha": 1.0,
        }
    }
    driver.expect("Properties/Colors/Fur/value/input/swatch", SWATCH, fur)
    driver.screenshot("exemplar_properties")
    driver.click(scale)
    driver.key("End")
    for _ in range(8):
        driver.key("Backspace")
    driver.type("1.5")
    driver.key("Enter")
    driver.expect(scale, NUMERIC, edited)
    # Undo goes back to the value before the edit, and redo forward again. The input keeps the
    # keys, which the history waits for: the list takes them back.
    driver.click("Exemplars/FeralDogAlpha")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Set ActorScale")
    driver.expect(scale, NUMERIC, loaded)
    undo(driver, redo=True)
    driver.expect(scale, NUMERIC, edited)

    # Save writes the edit into the catalog, changing that line and no other. The original goes
    # back however the checks end.
    original = CHARACTERS.read_bytes()
    try:
        expect_disabled(driver, "Save", False)
        save(driver)
        changed = [
            (before, after)
            for before, after in zip(
                original.decode().splitlines(), CHARACTERS.read_text().splitlines()
            )
            if before != after
        ]
        assert changed == [('      "ActorScale": 1.2', '      "ActorScale": 1.5')], changed
        # The reload brings the edit back from the file.
        driver.expect(scale, NUMERIC, edited)
        # Undone and saved again, the file is as it was.
        driver.click("Exemplars/FeralDogAlpha")
        undo(driver)
        driver.expect(scale, NUMERIC, loaded)
        save(driver)
        assert CHARACTERS.read_bytes() == original, "saving the undone edit changed the catalog"
    finally:
        restore(CHARACTERS, original)
    driver.click("Exemplars/ClearFilter")

    # The skirmisher's panes: those it inherits, Skin among them, dim and collapsed; after an
    # aspect, those it requires that the exemplar lacks, each a header with a +: `Combatant`
    # requires `Health`. Checked by eye in `aspect_panes.png`. + adds Health, and undo takes it
    # out; the pencil overrides Skin, and × reverts it. Saved, the catalog is as it was.
    driver.click("Exemplars/Filter")
    driver.type("CrimsonWatchSkirmisher")
    row = "Exemplars/CrimsonWatchSkirmisher"
    driver.click(row)
    expect_choice(driver, "actor", "CrimsonWatchSkirmisher")
    driver.present("Properties/Health/header/add")
    driver.present("Properties/Skin/header/override")
    # Scrolled to the end, which layout clamps to, where the missing Health follows Combatant.
    driver.set("Properties/Rows", SCROLL, [0.0, 10000.0])
    driver.frames(2)
    driver.screenshot("aspect_panes")
    original = CHARACTERS.read_bytes()
    try:
        driver.click("Properties/Health/header/add")
        driver.present("Properties/Health/header/remove", timeout=WAIT)
        driver.click(row)
        undo(driver)
        driver.expect("Toast/ToastText", TEXT, "Undo: Add CrimsonWatchSkirmisher.Health")
        driver.present("Properties/Health/header/add", timeout=WAIT)
        driver.click("Properties/Skin/header/override")
        driver.present("Properties/Skin/header/remove", timeout=WAIT)
        driver.screenshot("aspect_overridden")
        driver.click("Properties/Skin/header/remove")
        driver.present("Properties/Skin/header/override", timeout=WAIT)
        save(driver)
        assert CHARACTERS.read_bytes() == original, "an undone add and a revert changed the catalog"
    finally:
        restore(CHARACTERS, original)
    driver.click("Exemplars/ClearFilter")
