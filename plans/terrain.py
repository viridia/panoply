"""Terrain: parcels spawn ground and water meshes for the playground realm.

    python3 crates/bevy_remote_driver/client/run.py plans/terrain.py

Ground meshes wait for their precinct's terrain effects (roads, holes) from scenery (D11).
"""

BIN = "panoply"


def run(driver):
    driver.present("PrimaryCamera")
    # Ground meshes are named "Ground"; any one of them is enough to show the pipeline runs.
    driver.present("Ground", timeout={"frames": 600, "seconds": 30})
    driver.frames(30)
    driver.screenshot("terrain")
    driver.exits("Escape")
