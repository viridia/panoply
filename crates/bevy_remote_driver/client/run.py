"""Runs a plan: builds the target, launches it, runs the steps, reports, closes it (DD4.3).

    python3 crates/bevy_remote_driver/client/run.py <plan.json | plan.py>

A JSON plan names its `bin` or its `example`, optionally its `package`, and its `steps`. A Python
plan sets `BIN` or `EXAMPLE`, optionally `PACKAGE`, and defines `run(driver)`.
"""

import importlib.util
import json
import os
import shutil
import socket
import subprocess
import sys
import time
import traceback
import urllib.error
from pathlib import Path

from driver import BrpError, Driver, StepFailed

ROOT = Path(__file__).resolve().parents[3]
STARTUP_SECONDS = 60
EXIT_SECONDS = 3


class PlanError(Exception):
    pass


def target_of(bin_name, example):
    """The plan's target as (kind, name): a binary or an example, exactly one."""
    if (bin_name is None) == (example is None):
        raise PlanError("a plan names exactly one of `bin` and `example`")
    return ("bin", bin_name) if bin_name is not None else ("example", example)


def load(plan_path):
    """The plan's target, package, features, and a function running its steps on a driver."""
    if plan_path.suffix == ".py":
        spec = importlib.util.spec_from_file_location(plan_path.stem, plan_path)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return (
            target_of(getattr(module, "BIN", None), getattr(module, "EXAMPLE", None)),
            getattr(module, "PACKAGE", None),
            getattr(module, "FEATURES", None),
            module.run,
        )
    plan = json.loads(plan_path.read_text())
    steps = plan["steps"]

    def run(driver):
        for index, entry in enumerate(steps):
            names = [key for key in entry if getattr(getattr(Driver, key, None), "is_step", False)]
            if len(names) != 1:
                raise PlanError(f"step {index + 1} names {len(names)} steps: {json.dumps(entry)}")
            name = names[0]
            options = {key: value for key, value in entry.items() if key != name}
            getattr(driver, name)(entry[name], **options)

    target = target_of(plan.get("bin"), plan.get("example"))
    return target, plan.get("package"), plan.get("features"), run


def build(target, package, features):
    """Builds the target, and returns its binary and its package's directory."""
    kind, name = target
    command = ["cargo", "build", f"--{kind}", name, "--message-format=json-render-diagnostics"]
    if package:
        command += ["-p", package]
    # An example declaring `required-features` is skipped rather than refused without them, so
    # leaving these out ends at "the build produced no example named …" below.
    if features:
        command += ["--features", ",".join(features)]
    result = subprocess.run(command, cwd=ROOT, stdout=subprocess.PIPE, text=True)
    if result.returncode != 0:
        raise PlanError(f"`{' '.join(command)}` failed")
    for line in result.stdout.splitlines():
        message = json.loads(line)
        target = message.get("target", {})
        if (
            message.get("reason") == "compiler-artifact"
            and target.get("name") == name
            and kind in target.get("kind", [])
        ):
            return Path(message["executable"]), Path(message["manifest_path"]).parent
    raise PlanError(f"the build produced no {kind} named {name}")


def free_port():
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        return probe.getsockname()[1]


def environment(binary, package_dir, port):
    env = dict(os.environ)
    env["BEVY_REMOTE_DRIVER_PORT"] = str(port)
    # Where Bevy looks for `assets/`, as under `cargo run`.
    env["CARGO_MANIFEST_DIR"] = str(package_dir)
    if sys.platform == "darwin":
        # `dynamic_linking` leaves the binary without an rpath to the toolchain's libstd, or to the
        # Bevy dylib next to it.
        libdir = subprocess.run(
            ["rustc", "--print", "target-libdir"], stdout=subprocess.PIPE, text=True, check=True
        ).stdout.strip()
        paths = [libdir, str(binary.parent.parent / "deps"), env.get("DYLD_FALLBACK_LIBRARY_PATH")]
        env["DYLD_FALLBACK_LIBRARY_PATH"] = ":".join(p for p in paths if p)
    return env


def connect(app, port, out_dir, log_path):
    deadline = time.monotonic() + STARTUP_SECONDS
    while time.monotonic() < deadline:
        if app.poll() is not None:
            raise PlanError(
                f"the app exited during startup with code {app.returncode}; see {log_path}"
            )
        try:
            return Driver(port, out_dir)
        except (urllib.error.URLError, ConnectionError):
            time.sleep(0.1)
    raise PlanError(f"the app did not answer on port {port} in {STARTUP_SECONDS} s; see {log_path}")


def close(app, driver):
    if app.poll() is None and driver is not None and not driver.exited:
        try:
            driver.exit()
            app.wait(EXIT_SECONDS)
        except (OSError, BrpError, subprocess.TimeoutExpired):
            pass
    if app.poll() is None:
        app.kill()
        app.wait()


def failed_step(driver, plan_path, error):
    """Which step failed: its number and text, and for a Python plan, the line that called it."""
    lines = [f"step {len(driver.steps)}: {driver.steps[-1]}" if driver.steps else "before any step"]
    if error is not None and plan_path.suffix == ".py":
        for frame in reversed(traceback.extract_tb(error.__traceback__)):
            if Path(frame.filename).resolve() == plan_path.resolve():
                lines.append(f"at {plan_path}:{frame.lineno}")
                break
    return lines


def main(argv):
    if len(argv) != 2:
        print(__doc__.strip(), file=sys.stderr)
        return 2
    plan_path = Path(argv[1])
    try:
        target, package, features, run = load(plan_path)
    except PlanError as error:
        print(f"could not run: {error}", file=sys.stderr)
        return 2
    out_dir = ROOT / "target" / "remote-driver" / target[1] / plan_path.stem
    shutil.rmtree(out_dir, ignore_errors=True)
    out_dir.mkdir(parents=True)
    log_path = out_dir / "app.log"

    report = [f"plan: {plan_path}"]
    status = 0
    driver = None
    try:
        binary, package_dir = build(target, package, features)
        port = free_port()
        with open(log_path, "wb") as log:
            app = subprocess.Popen(
                [binary],
                cwd=package_dir,
                env=environment(binary, package_dir, port),
                stdout=log,
                stderr=subprocess.STDOUT,
            )
            try:
                driver = connect(app, port, out_dir, log_path)
                run(driver)
                report.append(f"passed: {len(driver.steps)} steps")
            except (StepFailed, BrpError, OSError) as error:
                status = 1
                report.append("FAILED")
                report += failed_step(driver, plan_path, error) if driver else []
                if isinstance(error, StepFailed):
                    report += [f"expected: {error.expected}", f"found: {error.found}"]
                else:
                    report.append(f"error: {error}")
                if app.poll() is not None:
                    report.append(f"the app had exited, with code {app.returncode}")
            except KeyboardInterrupt:
                status = 130
                report.append("interrupted")
                report += failed_step(driver, plan_path, None)[:1] if driver else []
            finally:
                close(app, driver)
    except PlanError as error:
        status = 2
        report.append(f"could not run: {error}")

    report.append(f"log: {log_path}")
    for path in driver.screenshots if driver else []:
        report.append(f"screenshot: {path}")
    text = "\n".join(report) + "\n"
    (out_dir / "report.txt").write_text(text)
    print(text, end="")
    return status


if __name__ == "__main__":
    sys.exit(main(sys.argv))
