"""The editor plans' shared steps and names: `from _editor import ...`."""

import difflib
from pathlib import Path

NAME = "bevy_ecs::name::Name"
TRANSFORM = "bevy_transform::components::transform::Transform"
GLOBAL = "bevy_transform::components::global_transform::GlobalTransform"
MESH = "bevy_mesh::components::Mesh3d"
VISIBILITY = "bevy_camera::visibility::Visibility"
INHERITED = "bevy_camera::visibility::InheritedVisibility"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"
PANEL = "panoply_editor::shell::EditorPanelSettings"
INSET = "panoply_core::viewport::ViewportInset"
GAME_MODE = "panoply_core::game_mode::GameMode"
EDITOR_MODE = "panoply_editor::mode_selector::EditorMode"
TERRAIN_TOOL = "panoply_editor::mode_selector::TerrainTool"
SCENERY_TOOL = "panoply_editor::mode_selector::SceneryTool"
SELECTION = "panoply_editor::parcel_selection::ParcelSelection"
MOTION = "panoply_actors::actor_physics::ActorMotion"
BUTTON_VARIANT = "bevy_feathers::controls::button::ButtonVariant"
COMPUTED = "bevy_ui::ui_node::ComputedNode"
UI_GLOBAL = "bevy_ui::ui_transform::UiGlobalTransform"
TEXT = "bevy_ui::widget::text::Text"
DISABLED = "bevy_ui::interaction_states::InteractionDisabled"
CHECKED = "bevy_ui::interaction_states::Checked"
NUMERIC = "bevy_ui_widgets::numeric::NumericValue"
NODE = "bevy_ui::ui_node::Node"
SCENERY_CHOICES = "panoply_editor::exemplar_list::SceneryChoices"
MODE_BUTTONS = ["RealmMode", "TerrainMode", "SceneryMode", "CatalogsMode", "PlayMode"]
WAIT = {"frames": 900, "seconds": 45}
ASSETS = Path(__file__).resolve().parent.parent / "assets"
# Forest, shoreline, water, grass and rocks.
SPOT = [110.0, 0.0, 150.0]
# The parcel under `SPOT`, 16 m on a side.
SPOT_PARCEL = [6, 9]
# The nav point "warehouse", from `world.locations.json`.
WAREHOUSE = [41.59800604609949, 0.0, 41.48915667709594]


def view(driver):
    return driver.get_resource(VIEW_READY)


def hold(driver, code, seconds):
    """Holds `code` for `seconds`; `key(hold=...)` returns before the release."""
    driver.press(code)
    driver.seconds(seconds)
    driver.release(code)
    driver.frames(2)


def named(driver, components=(), with_=()):
    """Rows of every named entity with `with_`, with `components` where present."""
    return driver.call(
        "world.query",
        {
            "data": {"components": [NAME], "option": list(components)},
            "filter": {"with": [NAME, *with_]},
        },
    )


def by_name(driver, prefix, components=(), with_=()):
    return [
        row
        for row in named(driver, components, with_)
        if row["components"][NAME].startswith(prefix)
    ]


def hidden(driver, prefix):
    """The names of the entities named `prefix…` whose `Visibility` is hidden."""
    rows = by_name(driver, prefix, [VISIBILITY])
    return [row["components"][NAME] for row in rows if row["components"][VISIBILITY] == "Hidden"]


def open_editor(driver, mode=None):
    """Shows the editor panel, which a plan starts with hidden, and clicks `mode`'s button, as
    `"Scenery"`, unless it is not given, leaving the panel on Realm."""
    driver.present("PrimaryCamera")
    driver.key("KeyT")
    driver.frames(10)
    assert driver.get_resource(PANEL, at="visible") is True, "T did not show the panel"
    if mode is not None:
        driver.click(f"{mode}Mode")
        wait_mode(driver, editor_mode, mode)


def game_mode(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{GAME_MODE}>")


def editor_mode(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{EDITOR_MODE}>")


def terrain_tool(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{TERRAIN_TOOL}>")


def scenery_tool(driver):
    return driver.get_resource(f"bevy_state::state::resources::State<{SCENERY_TOOL}>")


def wait_mode(driver, read, expected):
    """Waits until `read` (`game_mode`, `editor_mode` or a tool's) gives `expected`."""

    def check():
        found = read(driver)
        return found == expected, lambda: f"in {found}"

    driver.wait(check, f"the mode {expected}", {"seconds": 2})


def expect_highlight(driver, chosen, buttons=MODE_BUTTONS):
    """Of a segmented button's `buttons`, the mode selector's unless given, only `chosen` is
    highlighted."""
    driver.frames(2)
    for button in buttons:
        variant = driver.get(button, BUTTON_VARIANT)
        expected = "Primary" if button == chosen else "Normal"
        assert variant == expected, f"{button} is {variant}, not {expected}"


def expect_marker(driver, selector, component, wanted, words):
    """Waits until the entity `selector` has the marker `component`, or lacks it if not `wanted`;
    `words` name the two states, as ("disabled", "enabled")."""

    def check():
        (entity,) = driver.select(selector)
        found = driver.call(
            "world.get_components", {"entity": entity, "components": [component], "strict": False}
        )
        has = component in found["components"]
        return has == wanted, lambda: words[0] if has else words[1]

    driver.wait(check, f"{selector} {words[0] if wanted else words[1]}", {"seconds": 2})


def expect_disabled(driver, selector, disabled):
    """Waits until the widget `selector` is disabled, or enabled."""
    expect_marker(driver, selector, DISABLED, disabled, ("disabled", "enabled"))


def expect_checked(driver, selector, checked):
    """Waits until the checkbox `selector` is checked, or unchecked."""
    expect_marker(driver, selector, CHECKED, checked, ("checked", "unchecked"))


def expect_display(driver, selector, display):
    """Waits until the node `selector`'s display is `display`, as "Flex" or "None"."""

    def check():
        found = driver.get(selector, NODE)["display"]
        return found == display, lambda: f"display {found}"

    driver.wait(check, f"{selector} display {display}", {"seconds": 2})


def expect_choice(driver, tool, expected):
    """Waits until scenery mode's saved choice for `tool` is `expected`."""

    def check():
        found = driver.get_resource(SCENERY_CHOICES)[tool]
        return found == expected, lambda: f"{found}"

    driver.wait(check, f"{tool} choice {expected}", {"seconds": 2})


def choose(driver, tool, name):
    """Chooses the exemplar `name` in the current tool's list, through its filter."""
    driver.click("Exemplars/ClearFilter")
    driver.click("Exemplars/Filter")
    driver.type(name)
    driver.click(f"Exemplars/{name}")
    expect_choice(driver, tool, name)


def retype(driver, selector, text):
    """Replaces the text in the text input `selector` with `text`."""
    driver.click(selector)
    driver.key("End")
    for _ in range(60):
        driver.key("Backspace")
    driver.type(text)


def undo(driver, redo=False):
    """Presses Ctrl+Z, or with `redo` Ctrl+Shift+Z."""
    driver.press("ControlLeft")
    if redo:
        driver.press("ShiftLeft")
    driver.key("KeyZ")
    if redo:
        driver.release("ShiftLeft")
    driver.release("ControlLeft")


def save(driver):
    """Clicks Save and waits for the file watcher's reload, which the barrier holds edits off for."""
    driver.click("Save")
    expect_disabled(driver, "Save", True)
    driver.absent("SaveBarrier")


def restore(path, original):
    """Writes `original`, text or bytes, back to `path`, unless the file already holds it: a
    needless write is reloaded by the file watcher, and the reload can arrive after the next save,
    which takes it for that save's own (PAN101)."""
    data = original.encode() if isinstance(original, str) else original
    if path.read_bytes() != data:
        path.write_bytes(data)


def saved_diff(driver, catalog, row, expected):
    """Saves, checks that `catalog` changed by the diff lines `expected` alone, then undoes the
    edit, choosing `row` to take the keys, and saves again, leaving the file as it was. The
    original goes back however the checks end."""
    original = catalog.read_text()

    def changed():
        return [
            line
            for line in difflib.unified_diff(
                original.splitlines(), catalog.read_text().splitlines(), lineterm="", n=0
            )
            if line[:1] in "+-" and line[:3] not in ("+++", "---")
        ]

    try:
        save(driver)
        assert changed() == expected, changed()
        driver.click(row)
        undo(driver)
        save(driver)
        assert not changed(), f"saving the undone edit changed the catalog: {changed()}"
    finally:
        restore(catalog, original)


def arrived(driver, spot, how):
    """The view is at `spot`, x and z, and the Go To dialog closed."""
    driver.absent("GotoDialog")
    driver.frames(2)
    here = view(driver)["position"]
    print(f"{how}: at {here}")
    assert abs(here[0] - spot[0]) < 0.01 and abs(here[2] - spot[2]) < 0.01, f"{how}: at {here}"
