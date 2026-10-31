"""Exemplar loader: catalogs load, and an exemplar's `extends` resolves to its parent.

    bevy-remote-driver plans/exemplars.py

Relies on the temporary `run_exemplar_check` in main.rs, which spawns `ExemplarCheckPassed` once
the exemplars it loads have checked out. The log (target/remote-driver/panoply/exemplars/app.log)
has a line per exemplar.
"""

from _panoply_driver import PanoplyDriver

BIN = "panoply"
DRIVER = PanoplyDriver


def run(driver):
    driver.present("ExemplarCheckPassed")
    driver.exits("Escape")
