"""The editor's terrain mode, over the playground.

    bevy-remote-driver plans/terrain.py

- Palette: terrain mode starts on Raise, and its buttons choose the tool, kept through play.
- Rotation: the rotate buttons wait for a selected parcel. Turning a contour is in `realm.py`,
  in the realm it creates, since an edit to the playground's would have to be saved.

Settings start empty, so the panel starts hidden.
"""

from _editor import (
    SELECTION,
    SPOT,
    SPOT_PARCEL,
    editor_mode,
    expect_disabled,
    expect_highlight,
    game_mode,
    open_editor,
    terrain_tool,
    wait_mode,
)
from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver
TAGS = ["editor"]
TOOL_BUTTONS = [
    "RaiseDraw", "LowerDraw", "FlattenDraw", "RaiseRect", "LowerRect", "FlattenRect",
    "DrawTrees", "DrawShrubs", "DrawHerbs", "EraseFlora", "RotateCcw", "RotateCw",
]


def run(driver):
    driver.goto(SPOT, 20.0)
    open_editor(driver, "Terrain")
    expect_highlight(driver, "TerrainMode")
    driver.present("TerrainPanel")
    assert terrain_tool(driver) == "RaiseDraw", f"started with {terrain_tool(driver)}"
    driver.click("DrawShrubs")
    wait_mode(driver, terrain_tool, "DrawShrubs")
    expect_highlight(driver, "DrawShrubs", TOOL_BUTTONS)
    driver.screenshot("terrain_panel")

    # The rotate buttons wait for a selected parcel. The driver cannot click the ground, so the
    # plan selects through the setting.
    expect_disabled(driver, "RotateCw", True)
    driver.set_resource(SELECTION, {"realm": "playground", "parcel": SPOT_PARCEL})
    expect_disabled(driver, "RotateCw", False)

    # Play and back keeps the mode and its tool.
    driver.click("PlayMode")
    wait_mode(driver, game_mode, "Play")
    driver.key("F2")
    wait_mode(driver, game_mode, "Edit")
    assert editor_mode(driver) == "Terrain", f"F2 returned to {editor_mode(driver)}"
    wait_mode(driver, terrain_tool, "DrawShrubs")
    driver.exits("Escape")
