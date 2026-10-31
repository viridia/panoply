"""Panoply's own steps, for its plans: `from _panoply_driver import PanoplyDriver`, then
`DRIVER = PanoplyDriver`."""

from bevy_remote_driver import Driver, step

GOTO = "panoply_core::viewpoint::GotoLocation"


class PanoplyDriver(Driver):
    @step
    def goto(self, position, distance=None, azimuth=None, realm="playground"):
        """Moves the viewpoint to `position`, `[x, y, z]` in meters, in `realm`; at camera
        `distance` and facing `azimuth` (radians; 0 faces +z, pi/2 faces +x) when given. It does
        not wait for the area to load (X33), so follow it with `frames` before looking."""
        value = {"realm": realm, "position": list(position)}
        if distance is not None:
            value["camera_distance"] = distance
        if azimuth is not None:
            value["azimuth"] = azimuth
        self.message(GOTO, value)

    def camera(self):
        """The primary camera's entity."""
        return self.select("PrimaryCamera")[0]
