"""The `Driver`: one method per plan step, each a handful of BRP calls (DD4, DD5)."""

import functools
import http.client
import json
import struct
import time
import urllib.request
import zlib

import keys

DEFAULT_TIMEOUT = {"frames": 300, "seconds": 10}
# How often a wait asks again. A round trip is about a millisecond, and a frame at least eight.
POLL = 0.005

WINDOW = "bevy_window::window::Window"
PRIMARY_WINDOW = "bevy_window::window::PrimaryWindow"
WINDOW_EVENT = "bevy_window::event::WindowEvent"
KEYBOARD_INPUT = "bevy_input::keyboard::KeyboardInput"
GAMEPAD_CONNECTION = "bevy_input::gamepad::GamepadConnectionEvent"
RAW_GAMEPAD = "bevy_input::gamepad::RawGamepadEvent"
SCREENSHOT = "bevy_render::view::window::screenshot::Screenshot"
SCREENSHOT_CAPTURED = "bevy_render::view::window::screenshot::ScreenshotCaptured"
SCENE_READY = "bevy_remote_driver::ready::SceneReady"
APP_EXIT = "bevy_app::app::AppExit"

# `GamepadAxis`'s variants. Every other control name is taken for a `GamepadButton`, and one that is
# neither fails to deserialize, which names the offending word in the step's error.
AXES = frozenset({"LeftStickX", "LeftStickY", "LeftZ", "RightStickX", "RightStickY", "RightZ"})
# What the pad reports itself as. No vendor or product id, so a game that themes its prompts by
# brand sees a pad it does not recognize and falls back — the same answer on every machine, rather
# than one that depends on what is plugged into it.
VIRTUAL_PAD = {"name": "Virtual Gamepad", "vendor_id": None, "product_id": None}


class StepFailed(Exception):
    def __init__(self, expected, found):
        super().__init__(f"expected {expected}, found {found}")
        self.expected = expected
        self.found = found


class BrpError(Exception):
    def __init__(self, method, error):
        super().__init__(f"{method}: {error.get('message')}")
        self.data = error.get("data")


def step(method):
    """Records each call, so a report can name the step that failed."""

    @functools.wraps(method)
    def recorded(self, *args, **kwargs):
        shown = [repr(a) for a in args] + [f"{k}={v!r}" for k, v in kwargs.items()]
        self.steps.append(f"{method.__name__}({', '.join(shown)})")
        return method(self, *args, **kwargs)

    recorded.is_step = True
    return recorded


def path_of(selector):
    return selector.split("/") if isinstance(selector, str) else list(selector)


def shown(matches):
    return ["/".join(m["path"]) for m in matches] or "nothing"


def subset(expected, found):
    """Whether `found` has every key `expected` names, with the same value, at every depth."""
    if isinstance(expected, dict):
        return isinstance(found, dict) and all(
            key in found and subset(value, found[key]) for key, value in expected.items()
        )
    return expected == found


class Driver:
    def __init__(self, port, out_dir):
        self.host = "127.0.0.1"
        self.port = port
        self.out_dir = out_dir
        self.steps = []
        self.screenshots = []
        # Set by `exits`, so closing does not ask an app that has gone to exit again.
        self.exited = False
        # The frame count read after the last input was sent. The next input waits until the count
        # has passed it, so two inputs never land on the same frame (DR3.5).
        self._sent_at = None
        # The virtual pad's entity, spawned by the first `pad` step.
        self._gamepad = None
        windows = self._call(
            "world.query", {"data": {"components": []}, "filter": {"with": [PRIMARY_WINDOW]}}
        )
        self.window = windows[0]["entity"]

    # The steps.

    @step
    def present(self, selector, ready=False, timeout=None):
        """Waits until `selector` matches, and with `ready`, until a match's scene has spawned."""

        def check():
            matches = self._select(selector)
            if not ready:
                return bool(matches), shown(matches)
            done = [m for m in matches if self._has(m["entity"], SCENE_READY)]
            return bool(done), f"{shown(matches)}, {len(done)} of them ready"

        self._wait(check, f"{selector} to match" + (" and be ready" if ready else ""), timeout)

    @step
    def absent(self, selector, timeout=None):
        """Waits until `selector` matches nothing."""

        def check():
            matches = self._select(selector)
            return not matches, shown(matches)

        self._wait(check, f"{selector} to match nothing", timeout)

    @step
    def expect(self, selector, component, value, timeout=None):
        """Waits until `component` on the one entity `selector` matches has `value`, compared as a
        subset: keys `value` leaves out are not compared."""

        def check():
            matches = self._select(selector)
            if len(matches) != 1:
                return False, shown(matches)
            result = self._call(
                "world.get_components",
                {"entity": matches[0]["entity"], "components": [component], "strict": False},
            )
            if component not in result["components"]:
                return False, result["errors"].get(component, "no such component")
            found = result["components"][component]
            return subset(value, found), json.dumps(found)

        self._wait(check, f"{component} on {selector} to be {json.dumps(value)}", timeout)

    @step
    def click(self, selector):
        """Clicks the centre of a UI node, with the left button."""
        try:
            where = self._call("driver.locate", {"path": path_of(selector)})
        except BrpError as err:
            raise StepFailed(f"{selector} to be one UI node", shown(err.data or [])) from None
        window = where["window"]
        # Picking needs to know where the cursor is before the press arrives.
        self._send(
            WINDOW_EVENT,
            {"CursorMoved": {"window": window, "position": where["logical"], "delta": None}},
        )
        for state in ("Pressed", "Released"):
            self._send(
                WINDOW_EVENT,
                {"MouseButtonInput": {"button": "Left", "state": state, "window": window}},
            )

    @step
    def key(self, code, hold=1, logical_key=None, text=None):
        """Presses a key, and releases it `hold` frames later."""
        self._key(code, "Pressed", logical_key, text)
        self._until(self._sent_at + hold)
        self._key(code, "Released", logical_key, text)

    @step
    def press(self, code, logical_key=None, text=None):
        """Presses a key and leaves it held."""
        self._key(code, "Pressed", logical_key, text)

    @step
    def release(self, code, logical_key=None, text=None):
        """Releases a held key."""
        self._key(code, "Released", logical_key, text)

    @step
    def exits(self, code, seconds=10):
        """Presses a key that should quit the app, and waits until the app stops answering.

        No release follows, and no frame count is read after the press, since either would be
        sent to an app that has gone.
        """
        if self._sent_at is not None:
            self._until(self._sent_at + 1)
        known = keys.logical(code)
        if known is None:
            raise StepFailed(f"a key the client knows for {code}", code)
        logical_key, text = known
        value = {
            "key_code": code,
            "logical_key": logical_key,
            "state": "Pressed",
            "text": text,
            "repeat": False,
            "window": self.window,
        }
        self._call("world.write_message", {"message": KEYBOARD_INPUT, "value": value})
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            try:
                self.frame()
            except OSError:
                self.exited = True
                return
            time.sleep(POLL)
        raise StepFailed(f"the app to exit within {seconds} s", "it still answering")

    @step
    def type(self, text):
        """Types a string as a US keyboard would, holding Shift where a character needs it."""
        for char in text:
            if char not in keys.CHARACTERS:
                raise StepFailed("a character on a US keyboard", repr(char))
            code, shifted = keys.CHARACTERS[char]
            if shifted:
                self._key("ShiftLeft", "Pressed")
            self._key(code, "Pressed", {"Character": char}, char)
            self._key(code, "Released", {"Character": char}, char)
            if shifted:
                self._key("ShiftLeft", "Released")

    @step
    def pad(self, control, value=None):
        """Presses a gamepad button, or holds a control at a value.

        With no `value`, presses `control` and releases it on the next frame, as `key` does. With
        one, writes it and leaves it there: a stick pushed over, a trigger part-way down, a button
        held while later steps run. A second call with `0.0` centres it.

        The first call connects the pad, and it stays connected for the rest of the plan.
        """
        if value is not None:
            self._pad(control, value)
            return
        if control in AXES:
            raise StepFailed(f"a value for the axis {control}", "a press")
        self._pad(control, 1.0)
        self._pad(control, 0.0)

    @step
    def frames(self, n):
        """Lets `n` frames pass."""
        self._until(self.frame() + n)

    @step
    def seconds(self, n):
        """Lets `n` seconds pass."""
        time.sleep(n)

    @step
    def screenshot(self, name):
        """Saves a PNG of the primary window as `<name>.png`."""
        # A covered window captures an empty image, so bring it forward and give it a moment.
        self._call(
            "world.mutate_components",
            {"entity": self.window, "component": WINDOW, "path": ".focused", "value": True},
        )
        time.sleep(0.1)
        image = self._capture()
        size = image["texture_descriptor"]["size"]
        width, height = size["width"], size["height"]
        data = bytes(image["data"])
        if not any(data):
            raise StepFailed("a screenshot", "an empty image, as a covered window gives")
        if image["texture_descriptor"]["format"].startswith("bgra"):
            swapped = bytearray(data)
            swapped[0::4], swapped[2::4] = data[2::4], data[0::4]
            data = bytes(swapped)
        path = self.out_dir / f"{name}.png"
        write_png(path, width, height, data)
        self.screenshots.append(path)

    # Reading the app.

    def frame(self):
        """How many frames the app has run."""
        return int(self._call("driver.diagnostics")["frame_count"])

    def exit(self):
        self._call("world.write_message", {"message": APP_EXIT, "value": "Success"})

    def _select(self, selector):
        return self._call("driver.select", {"path": path_of(selector)})

    def _has(self, entity, component):
        result = self._call(
            "world.get_components", {"entity": entity, "components": [component], "strict": False}
        )
        return component in result["components"]

    def _wait(self, check, expected, timeout):
        limit = DEFAULT_TIMEOUT if timeout is None else timeout
        start_frame, start = self.frame(), time.monotonic()
        while True:
            passed, found = check()
            if passed:
                return
            if (
                "frames" in limit and self.frame() - start_frame >= limit["frames"]
            ) or ("seconds" in limit and time.monotonic() - start >= limit["seconds"]):
                raise StepFailed(expected, found)
            time.sleep(POLL)

    def _until(self, frame):
        while self.frame() < frame:
            time.sleep(POLL)

    # Writing to the app.

    def _key(self, code, state, logical_key=None, text=None):
        if logical_key is None:
            known = keys.logical(code)
            if known is None:
                raise StepFailed(f"a key the client knows, or a logical_key for {code}", code)
            logical_key, text = known
        self._send(
            KEYBOARD_INPUT,
            {
                "key_code": code,
                "logical_key": logical_key,
                "state": state,
                "text": text,
                "repeat": False,
                "window": self.window,
            },
        )

    def _pad(self, control, value):
        """One raw gamepad message, connecting the pad first if this is the plan's first."""
        if self._gamepad is None:
            # An entity of its own, as `bevy_input` gives a real pad. The connection is what puts
            # `Gamepad` on it, and `_send` leaves a frame for that before the first control arrives.
            self._gamepad = self._call("world.spawn_entity", {"components": {}})["entity"]
            self._send(
                GAMEPAD_CONNECTION,
                {"gamepad": self._gamepad, "connection": {"Connected": VIRTUAL_PAD}},
            )
        kind, part = ("Axis", "axis") if control in AXES else ("Button", "button")
        self._send(
            RAW_GAMEPAD, {kind: {"gamepad": self._gamepad, part: control, "value": value}}
        )

    def _send(self, message, value):
        if self._sent_at is not None:
            self._until(self._sent_at + 1)
        self._call("world.write_message", {"message": message, "value": value})
        self._sent_at = self.frame()

    def _capture(self):
        # Watch before spawning, so the capture cannot finish before anyone is listening.
        stream = http.client.HTTPConnection(self.host, self.port, timeout=30)
        try:
            stream.request(
                "POST",
                "/",
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 0,
                        "method": "world.observe+watch",
                        "params": {"event": SCREENSHOT_CAPTURED},
                    }
                ),
            )
            response = stream.getresponse()
            spawned = self._call(
                "world.spawn_entity", {"components": {SCREENSHOT: {"Window": "Primary"}}}
            )["entity"]
            for line in response:
                if not line.startswith(b"data: "):
                    continue
                reply = json.loads(line[len(b"data: ") :])
                if "error" in reply:
                    raise BrpError("world.observe+watch", reply["error"])
                for event in reply.get("result") or []:
                    if event.get("entity") == spawned:
                        return event["image"]
            raise StepFailed("a screenshot", "the stream closing before one arrived")
        finally:
            stream.close()

    def _call(self, method, params=None):
        body = {"jsonrpc": "2.0", "id": 0, "method": method}
        if params is not None:
            body["params"] = params
        request = urllib.request.Request(
            f"http://{self.host}:{self.port}/",
            data=json.dumps(body).encode(),
            headers={"Content-Type": "application/json"},
        )
        with urllib.request.urlopen(request, timeout=30) as response:
            reply = json.load(response)
        if "error" in reply:
            raise BrpError(method, reply["error"])
        return reply.get("result")


def write_png(path, width, height, rgba):
    def chunk(tag, data):
        return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data))

    stride = width * 4
    # Each row is prefixed by its filter type, none.
    rows = b"".join(b"\x00" + rgba[y * stride : (y + 1) * stride] for y in range(height))
    header = struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)
    path.write_bytes(
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(rows, 6))
        + chunk(b"IEND", b"")
    )
