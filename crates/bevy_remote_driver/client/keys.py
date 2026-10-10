"""What a US keyboard sends for a key: its `KeyCode`, logical key and text (DD5.2)."""

# Character -> (KeyCode, shifted). Built from the rows below.
CHARACTERS = {}
# KeyCode -> unshifted character.
PRINTABLE = {}

for code, plain, shifted in (
    ("Backquote", "`", "~"),
    ("Minus", "-", "_"),
    ("Equal", "=", "+"),
    ("BracketLeft", "[", "{"),
    ("BracketRight", "]", "}"),
    ("Backslash", "\\", "|"),
    ("Semicolon", ";", ":"),
    ("Quote", "'", '"'),
    ("Comma", ",", "<"),
    ("Period", ".", ">"),
    ("Slash", "/", "?"),
    *((f"Digit{d}", str(d), s) for d, s in zip(range(10), ")!@#$%^&*(")),
    *((f"Key{c.upper()}", c, c.upper()) for c in "abcdefghijklmnopqrstuvwxyz"),
):
    CHARACTERS[plain] = (code, False)
    CHARACTERS[shifted] = (code, True)
    PRINTABLE[code] = plain

# KeyCode -> the `Key` variant it produces, for keys that produce no character.
NAMED = {
    "Space": "Space",
    "Enter": "Enter",
    "Tab": "Tab",
    "Escape": "Escape",
    "Backspace": "Backspace",
    "Delete": "Delete",
    "Insert": "Insert",
    "Home": "Home",
    "End": "End",
    "PageUp": "PageUp",
    "PageDown": "PageDown",
    "ArrowUp": "ArrowUp",
    "ArrowDown": "ArrowDown",
    "ArrowLeft": "ArrowLeft",
    "ArrowRight": "ArrowRight",
    "ShiftLeft": "Shift",
    "ShiftRight": "Shift",
    "ControlLeft": "Control",
    "ControlRight": "Control",
    "AltLeft": "Alt",
    "AltRight": "Alt",
    "SuperLeft": "Super",
    "SuperRight": "Super",
    **{f"F{n}": f"F{n}" for n in range(1, 13)},
}

# winit gives Space a named logical key and a character of text.
TEXT = {"Space": " "}


def logical(code):
    """The serialized `Key` and text for an unshifted press of `code`, or `None` if unknown."""
    if code in PRINTABLE:
        return {"Character": PRINTABLE[code]}, PRINTABLE[code]
    if code in NAMED:
        return NAMED[code], TEXT.get(code)
    return None
