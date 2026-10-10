"""Terrain: parcels spawn ground and water meshes for the playground realm.

    python3 crates/bevy_remote_driver/client/run.py plans/terrain.py

Until scenery is restored, ground meshes rely on the temporary `release_parcel_terrain_fx` in
main.rs, and have no terrain effects (roads, holes).
"""

BIN = "panoply"


def run(driver):
    driver.present("PrimaryCamera")
    # Ground meshes are named "Ground"; any one of them is enough to show the pipeline runs.
    driver.present("Ground", timeout={"frames": 600, "seconds": 30})
    driver.frames(30)
    driver.screenshot("terrain")
    driver.exits("Escape")
