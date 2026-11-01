"""Panoply's own steps, for its plans: `from _panoply_driver import PanoplyDriver`, then
`DRIVER = PanoplyDriver`."""

from bevy_remote_driver import Driver, step

GOTO = "panoply_core::viewpoint::GotoLocation"
VIEW_READY = "panoply_editor::diagnostics::view_ready::ViewReady"

# A cold start loads and warms every model before the first view is ready.
READY_TIMEOUT = {"seconds": 60}
# Rendering runs a frame behind, so what was built last needs a frame or two to be drawn.
RENDER_FRAMES = 2


class PanoplyDriver(Driver):
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
            return here and ready["ready"], ready

        self.wait(check, f"the view at {realm} {list(position)} to load", READY_TIMEOUT)
        self._until(self.frame() + RENDER_FRAMES)

    def camera(self):
        """The primary camera's entity."""
        return self.select("PrimaryCamera")[0]
