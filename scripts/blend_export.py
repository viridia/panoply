#!/usr/bin/env python3
"""Export .blend files to the .glb beside each, with the export settings remembered in the file.

    scripts/blend_export.py                  every .blend under artwork/characters/ with a .glb
    scripts/blend_export.py a.blend dir ...  files, or directories
    scripts/blend_export.py --to DIR ...     into DIR instead, to compare before replacing

Runs inside Blender 5.2 or later, headless (see blendtools.py). The settings are the ones File >
Export > glTF 2.0 opens with; a file without Panoply's (see blend_inventory.py) is not exported.
Check the result with glb_compare.py.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import blendtools as bt  # noqa: E402

bt.relaunch(__file__)

import bpy  # noqa: E402


def main():
    bt.require_version(5, 2)
    args = bt.script_args()
    to = None
    if "--to" in args:
        at = args.index("--to")
        to = Path(args[at + 1]).resolve()
        to.mkdir(parents=True, exist_ok=True)
        args = args[:at] + args[at + 2 :]
    exported, refused = 0, 0
    for blend in bt.blend_files(args):
        glb = bt.exported_glb(blend)
        if not glb.exists():
            continue
        bpy.ops.wm.open_mainfile(filepath=str(blend))
        drift = bt.settings_drift(
            bpy.context.scene.get(bt.SETTINGS_KEY), bt.expected_settings(bpy.data.objects)
        )
        name = blend.relative_to(bt.ROOT)
        if drift:
            refused += 1
            print(f"\n{name}: not exported, its export settings are not Panoply's:")
            for line in drift:
                print(f"  {line}")
            continue
        out = to / glb.name if to else glb
        bt.export(out)
        exported += 1
        print(f"\n{name}: exported to {out}")
    print(f"\n{exported} exported, {refused} refused")
    if refused:
        sys.exit(1)


main()
