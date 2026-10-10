"""Exemplar loader: catalogs load, and an exemplar's `extends` resolves to its parent.

    python3 crates/bevy_remote_driver/client/run.py plans/exemplars.py

Relies on the temporary `run_exemplar_check` in main.rs, which spawns `ExemplarCheckPassed` once
the exemplars it loads have checked out. The log (target/remote-driver/panoply/exemplars/app.log)
has a line per exemplar.
"""

BIN = "panoply"


def run(driver):
    driver.present("ExemplarCheckPassed")
    driver.exits("Escape")
