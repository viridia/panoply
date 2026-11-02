"""Panoply's own steps, for its plans: `from _panoply_driver import PanoplyDriver`, then
`DRIVER = PanoplyDriver`."""

from bevy_remote_driver import Driver, step

WINDOW = "bevy_window::window::Window"
GOTO = "panoply_core::viewpoint::GotoLocation"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"
READINESS = "panoply.readiness"

# Grid cells for a parcel's first pending stage; a ready parcel is `o` when shown, `.` when hidden.
STAGE_CELLS = {"fx": "f", "ground": "g", "water": "w", "flora": "l", "flora-scenes": "s"}

# A cold start loads and warms every model before the first view is ready.
READY_TIMEOUT = {"seconds": 60}
# Rendering runs a frame behind, so what was built last needs a frame or two to be drawn.
RENDER_FRAMES = 2


class PanoplyDriver(Driver):
    def _attach(self):
        """Titles the window with the plan's name, so someone watching can tell which plan
        is running."""
        super()._attach()
        self.call(
            "world.mutate_components",
            {
                "entity": self.window,
                "component": WINDOW,
                "path": ".title",
                "value": self.out_dir.name,
            },
        )

    @step
    def goto(self, position, distance=None, azimuth=None, realm="playground", wait=True):
        """Moves the viewpoint to `position`, `[x, y, z]` in meters, in `realm`; at camera
        `distance` and facing `azimuth` (radians; 0 faces +z, pi/2 faces +x) when given. Then
        waits until the area around it has loaded and been drawn (`ViewReady`), unless `wait` is
        false, as for a plan that measures the load itself."""
        value = {"realm": realm, "position": list(position)}
        if distance is not None:
            value["camera_distance"] = distance
        if azimuth is not None:
            value["azimuth"] = azimuth
        # Not the `message` and `frames` steps, so a failure report names `goto`.
        self._send(GOTO, value)
        if not wait:
            return

        def check():
            ready = self.call("world.get_resources", {"resource": VIEW_READY})["value"]
            here = (
                ready["realm"] == realm
                and all(abs(a - b) < 1e-3 for a, b in zip(ready["position"], position))
                and (distance is None or abs(ready["camera_distance"] - distance) < 1e-3)
            )
            # On failure, the report adds the survey, to show what is stuck where.
            return here and ready["ready"], lambda: f"{ready}\n{readiness_grid(self.readiness())}"

        self.wait(check, f"the view at {realm} {list(position)} to load", READY_TIMEOUT)
        self._until(self.frame() + RENDER_FRAMES)

    def readiness(self):
        """The readiness survey (`panoply.readiness`): the viewpoint, `ready` and `waiting_on`
        as `ViewReady` has them, and each loaded parcel and precinct with what it still waits
        on. `readiness_grid` prints it."""
        return self.call(READINESS)

    def camera(self):
        """The primary camera's entity."""
        return self.select("PrimaryCamera")[0]


def readiness_grid(survey):
    """The survey as text: the viewpoint's realm's parcels as a grid, rows by z (north at the
    top is -z), one cell per parcel (`STAGE_CELLS`), then each precinct still waiting."""
    realm = survey["viewpoint"]["realm"]
    cells = {}
    for parcel in survey["parcels"]:
        if parcel["realm"] != realm:
            continue
        pending = [code.split(":")[0] for code in parcel["pending"]]
        cell = STAGE_CELLS.get(pending[0], "?") if pending else "o" if parcel["shown"] else "."
        cells[(parcel["x"], parcel["z"])] = cell
    lines = [
        f"{realm} at {survey['viewpoint']['position']}: ready {survey['ready']}, "
        f"waiting on {survey['waiting_on']!r}, models {survey['models']!r}"
    ]
    if cells:
        xs = [x for x, _ in cells]
        zs = [z for _, z in cells]
        lines.append(f"parcels x {min(xs)}..{max(xs)}, z {min(zs)}..{max(zs)}:")
        for z in range(min(zs), max(zs) + 1):
            row = "".join(cells.get((x, z), " ") for x in range(min(xs), max(xs) + 1))
            lines.append(f"  {z:4} {row}")
    for precinct in survey["precincts"]:
        if precinct["pending"] and precinct["realm"] == realm:
            lines.append(
                f"precinct {precinct['x']} {precinct['z']} ({precinct['asset']}): "
                f"{', '.join(precinct['pending'])}"
            )
    return "\n".join(lines)
