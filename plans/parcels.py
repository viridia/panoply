"""Parcel cache: parcels outside the view are hidden, and so are their ground meshes.

    bevy-remote-driver plans/parcels.py

Teleports to the ablation spot, counts parcels (cached, hidden) and visible ground meshes, and
prints the readiness survey's grid, which must show nothing pending; then teleports far away
and checks that the first spot's parcels went hidden but stayed cached.
"""

from _panoply_driver import PanoplyDriver, readiness_grid

BIN = "panoply"
DRIVER = PanoplyDriver
NAME = "bevy_ecs::name::Name"
VISIBILITY = "bevy_camera::visibility::Visibility"
INHERITED = "bevy_camera::visibility::InheritedVisibility"
DISTANCE = 20.0
FIRST = [110.0, 0.0, 150.0]
SECOND = [400.0, 0.0, 400.0]


def named(driver, components):
    rows = driver.call(
        "world.query",
        {"data": {"components": [NAME] + components}, "filter": {"with": [NAME]}},
    )
    return [(row["components"][NAME], row) for row in rows]


def parcels(driver):
    """Map of parcel name to its `Visibility`."""
    return {
        name: row["components"][VISIBILITY]
        for name, row in named(driver, [VISIBILITY])
        if name.startswith("Parcel:")
    }


def visible_ground(driver):
    return sum(
        1
        for name, row in named(driver, [INHERITED])
        if name == "Ground" and row["components"][INHERITED]
    )


def report(driver, label):
    cached = parcels(driver)
    hidden = sum(1 for v in cached.values() if v == "Hidden")
    print(
        f"{label}: {len(cached)} parcels cached, {hidden} hidden, "
        f"{visible_ground(driver)} ground meshes visible"
    )
    return cached


def run(driver):
    driver.present("PrimaryCamera")
    driver.goto(FIRST, DISTANCE)
    first = report(driver, "first spot")
    in_view = {name for name, v in first.items() if v != "Hidden"}
    assert in_view, "no parcels in view"
    # Once `goto` has waited for the view, the survey shows nothing left to do, and the same
    # parcels shown as the query above.
    survey = driver.readiness()
    print(readiness_grid(survey))
    stuck = [p for p in survey["parcels"] + survey["precincts"] if p["pending"]]
    assert survey["ready"] and not stuck, f"still pending after goto: {stuck[:5]}"
    shown = sum(1 for p in survey["parcels"] if p["shown"])
    assert shown == len(in_view), f"survey shows {shown} parcels, the query {len(in_view)}"
    driver.screenshot("first")

    driver.goto(SECOND, DISTANCE)
    second = report(driver, "second spot")
    stale = [name for name in in_view if name in second]
    still_shown = [name for name in stale if second[name] != "Hidden"]
    print(f"first spot's parcels still cached: {len(stale)}, of which shown: {len(still_shown)}")
    assert not still_shown, f"out-of-view parcels still shown: {still_shown[:5]}"
    driver.screenshot("second")
    driver.exits("Escape")
