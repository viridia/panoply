"""The editor's realm mode and the Go To dialog.

    bevy-remote-driver plans/realm.py

- Realm panel: its filter box does not take the keys; Go travels to the point selected; + asks
  for a name, refusing one in use, and adds the point, which enables Save; Escape with it unsaved
  asks first. Ctrl+Z and Ctrl+Shift+Z undo and redo that, returning to Realm mode from another,
  and do nothing while a text field has the keys; Save writes `world.locations.json` (restored
  afterwards); − removes the point.
- Realms: Create Realm refuses a name in use, writes a new realm (deleted afterwards) and cuts to
  it. There the map size control, turned with the view, grows the map, undoably, and Save writes
  it; the playground's selected parcel does not count there, and a parcel selected there turns,
  undoably. Change Realm lists the new realm and goes back, keeping the view's coordinates.
- Go To dialog: G opens it, centered over a barrier that covers the window, and Esc closes it.
  Its filter box has the keys as it opens: typing filters the list, a held arrow leaves the view
  where it was, Up and Down move the selection, and Enter travels to the point selected, or to
  the first shown if none is. A double-click travels to its point.

The map size's arithmetic and a realm name's rules are checked by `cargo test` (`map_size.rs`,
`new_realm.rs`). Settings start empty, so the panel starts hidden.
"""

from _editor import (
    ASSETS,
    COMPUTED,
    SELECTION,
    SPOT,
    SPOT_PARCEL,
    TEXT,
    TRANSFORM,
    UI_GLOBAL,
    WAREHOUSE,
    arrived,
    editor_mode,
    expect_disabled,
    hold,
    open_editor,
    restore,
    save,
    undo,
    wait_mode,
)
from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TAGS = ["editor"]
MODAL = "bevy_ui_widgets::modal::ModalDialog"
LOCATIONS = ASSETS / "world.locations.json"
# The nav point "test-inn", from `world.locations.json`.
TEST_INN = [56.65786741155058, 0.0, 37.0414409317615]
REALM = "plan-realm"


def run(driver):
    driver.goto(SPOT, 20.0)
    open_editor(driver)
    driver.present("RealmPanel")
    driver.caption("realm panel")
    realm_panel(driver)
    driver.caption("realms")
    create_and_change_realm(driver)
    driver.caption("go to dialog")
    goto_dialog(driver)
    driver.exits("Escape")


def realm_panel(driver):
    driver.present("RealmPanel/warehouse")
    driver.screenshot("realm_panel")

    # The panel's filter box does not take the focus: an arrow still turns the view.
    before = driver.get("PrimaryCamera", TRANSFORM)
    hold(driver, "ArrowLeft", 0.3)
    assert driver.get("PrimaryCamera", TRANSFORM) != before, "the panel's list took the arrows"

    driver.click("RealmPanel/warehouse")
    expect_disabled(driver, "RealmPanel/Go", False)
    driver.click("RealmPanel/Go")
    arrived(driver, WAREHOUSE, "the panel's Go")

    # + refuses a name in use, and adds the point under a new one, selected.
    expect_disabled(driver, "Save", True)
    driver.click("AddNavPoint")
    driver.present("TextEntryDialog/Entry")
    driver.type("warehouse")
    driver.expect("TextEntryDialog/Hint", TEXT, "Name in use")
    expect_disabled(driver, "TextEntryDialog/Ok", True)
    driver.type("-plan")
    expect_disabled(driver, "TextEntryDialog/Ok", False)
    driver.screenshot("new_nav_point")
    driver.click("TextEntryDialog/Ok")
    driver.absent("TextEntryDialog")
    driver.present("RealmPanel/warehouse-plan")
    expect_disabled(driver, "Save", False)

    # With the point unsaved, Escape asks first, and Cancel stays.
    driver.key("Escape")
    driver.present("QuitDialog")
    driver.click("QuitDialog/Cancel")
    driver.absent("QuitDialog")

    # Undo, from another mode, removes the point and returns to the mode of the edit; redo brings
    # it back. Before Save, whose reload would overwrite the edits (PAN57).
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    undo(driver)
    wait_mode(driver, editor_mode, "Realm")
    driver.absent("RealmPanel/warehouse-plan")
    driver.expect("Toast/ToastText", TEXT, "Undo: Remove nav point warehouse-plan")
    undo(driver, redo=True)
    driver.present("RealmPanel/warehouse-plan")
    driver.expect("Toast/ToastText", TEXT, "Redo: Add nav point warehouse-plan")

    # While a text field has the keys, the history does not get them.
    driver.click("AddNavPoint")
    driver.present("TextEntryDialog/Entry")
    undo(driver)
    driver.click("TextEntryDialog/Cancel")
    driver.absent("TextEntryDialog")
    driver.present("RealmPanel/warehouse-plan")

    # Undo cleared the selection, which − needs.
    driver.click("RealmPanel/warehouse-plan")

    # Save writes the file. The original goes back however the checks end; the file watcher then
    # reloads it.
    original = LOCATIONS.read_bytes()
    try:
        save(driver)
        assert '"warehouse-plan"' in LOCATIONS.read_text(), "Save did not write the new point"
        driver.click("RemoveNavPoint")
        driver.absent("RealmPanel/warehouse-plan")
        expect_disabled(driver, "Save", False)
        # Saved again, so that nothing is unsaved when the plan's last Escape quits.
        save(driver)
    finally:
        restore(LOCATIONS, original)


def create_and_change_realm(driver):
    # The playground's selection, which the new realm's terrain mode does not count.
    driver.set_resource(SELECTION, {"realm": "playground", "parcel": SPOT_PARCEL})
    # A realm's files are written at once, so they go however the checks end.
    precincts = ASSETS / f"scenery/precincts/{REALM}"
    files = [
        ASSETS / "realms" / f"{REALM}.realm.json",
        ASSETS / f"terrain/maps/{REALM}.terrain",
        precincts / "index.json",
    ]
    try:
        # Create Realm refuses a name in use.
        driver.click("CreateRealm")
        driver.present("TextEntryDialog/Entry")
        driver.type("playground")
        driver.expect("TextEntryDialog/Hint", TEXT, "Name in use")
        expect_disabled(driver, "TextEntryDialog/Ok", True)
        driver.click("TextEntryDialog/Cancel")
        driver.absent("TextEntryDialog")

        # The view cuts to the new realm, which has no parcels and is still ready.
        driver.click("CreateRealm")
        driver.present("TextEntryDialog/Entry")
        driver.type(REALM)
        driver.click("TextEntryDialog/Ok")
        driver.absent("TextEntryDialog")
        driver.expect("RealmPanel/Realm", TEXT, REALM)
        driver.expect("RealmPanel/Size", TEXT, "0 × 0 parcels")
        assert all(file.exists() for file in files), "Create Realm did not write its three files"
        driver.goto([0.0, 0.0, 0.0], realm=REALM)
        driver.screenshot("new_realm")

        # Turned with the view, the map size control's buttons still take clicks.
        driver.goto([0.0, 0.0, 0.0], azimuth=1.0, realm=REALM)
        driver.click("GrowEast")
        driver.expect("RealmPanel/Size", TEXT, "1 × 1 parcels")
        undo(driver)
        driver.expect("RealmPanel/Size", TEXT, "0 × 0 parcels")
        driver.expect("Toast/ToastText", TEXT, "Undo: Resize map")
        undo(driver, redo=True)
        driver.expect("RealmPanel/Size", TEXT, "1 × 1 parcels")
        driver.screenshot("map_size")
        rotate_contour(driver)
        # Save writes the terrain map, which the reload brings back the same size.
        save(driver)
        driver.expect("RealmPanel/Size", TEXT, "1 × 1 parcels")

        driver.goto(WAREHOUSE, realm=REALM)

        # Change Realm lists the new realm, and goes back. Its filter is the Go To dialog's.
        driver.click("ChangeRealm")
        driver.present(f"ChangeRealmDialog/{REALM}")
        driver.click("ChangeRealmDialog/playground")
        driver.screenshot("change_realm")
        driver.click("ChangeRealmDialog/Go")
        driver.absent("ChangeRealmDialog")
        driver.expect("RealmPanel/Realm", TEXT, "playground")
        arrived(driver, WAREHOUSE, "Change Realm, keeping the position")
    finally:
        for file in files:
            file.unlink(missing_ok=True)
        if precincts.exists():
            precincts.rmdir()


def rotate_contour(driver):
    """In the new realm, whose map is its parcel (0, 0): the playground's selection does not count
    here, and the rotate buttons turn a parcel selected here, undoably."""
    driver.click("TerrainMode")
    wait_mode(driver, editor_mode, "Terrain")
    expect_disabled(driver, "RotateCw", True)
    driver.set_resource(SELECTION, {"realm": REALM, "parcel": [0, 0]})
    expect_disabled(driver, "RotateCw", False)
    driver.click("RotateCw")
    undo(driver)
    driver.expect("Toast/ToastText", TEXT, "Undo: Rotate contour")
    undo(driver, redo=True)
    driver.expect("Toast/ToastText", TEXT, "Redo: Rotate contour")
    driver.click("RealmMode")
    wait_mode(driver, editor_mode, "Realm")


def goto_dialog(driver):
    # Esc closes the dialog, and only the dialog: outside it, Esc quits.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.frames(10)
    # The barrier covers the window, and centers the dialog in it.
    barrier = driver.get("GotoDialog", COMPUTED, at="size")
    barrier_at = driver.get("GotoDialog", UI_GLOBAL)[-2:]
    (modal,) = driver.call(
        "world.query", {"data": {"components": [COMPUTED, UI_GLOBAL]}, "filter": {"with": [MODAL]}}
    )
    dialog = modal["components"][COMPUTED]["size"]
    dialog_at = modal["components"][UI_GLOBAL][-2:]
    print(f"barrier {barrier} at {barrier_at}, dialog {dialog} at {dialog_at}")
    assert barrier_at == [barrier[0] / 2, barrier[1] / 2], "the barrier does not cover the window"
    assert dialog_at == barrier_at and dialog[1] < barrier[1], "the dialog is not centered"
    driver.screenshot("goto_dialog_open")
    driver.key("Escape")
    driver.absent("GotoDialog")

    # Change Realm left the view at the warehouse, so each trip below starts away from its point.
    # A row clicked and Go are checked in the Change Realm dialog, the same chooser.

    # The filter box has focus as it opens: `tein` matches only "test-inn", by its words' starts.
    # It keeps a held arrow from the camera for the whole hold, not just its first frame. With
    # nothing selected, Enter travels to the first point shown.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.absent("GotoDialog/warehouse")
    driver.present("GotoDialog/test-inn")
    driver.screenshot("goto_dialog")
    before = driver.get("PrimaryCamera", TRANSFORM)
    hold(driver, "ArrowLeft", 0.5)
    after = driver.get("PrimaryCamera", TRANSFORM)
    assert before == after, f"a held arrow in the filter box moved the view: {before} to {after}"
    driver.key("Enter")
    arrived(driver, TEST_INN, "Enter")

    # In the filter box, Down and Up move the selection, and Enter travels to it.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("wareh")
    driver.present("GotoDialog/warehouse2")
    driver.key("ArrowDown")
    driver.key("ArrowDown")
    driver.key("ArrowUp")
    driver.key("Enter")
    arrived(driver, WAREHOUSE, "arrows and Enter")

    # A double-click on a point travels to it: the list's focus outline does not take the clicks.
    driver.key("KeyG")
    driver.present("GotoDialog/warehouse")
    driver.type("tein")
    driver.click("GotoDialog/test-inn")
    driver.click("GotoDialog/test-inn")
    arrived(driver, TEST_INN, "double-click")
