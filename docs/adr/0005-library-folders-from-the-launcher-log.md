# Library folders come from the RSI Launcher log, not its settings store

Channel detection should find installs outside the default path through the library folder the
RSI Launcher uses. The launcher keeps that setting in `%APPDATA%\rsilauncher\launcher store.json`,
which is encrypted and also holds the session data of the signed-in account. The launcher's log
(`logs\log.log` and `logs\log.old.log`) names every Channel folder it launches, installs or
verifies, e.g. `Launching Star Citizen LIVE from (…\StarCitizen\LIVE)` and
`Installing Star Citizen PTU … at …\StarCitizen (force DP: false)`. Detection reads only these two
log files and takes each named library folder (the parent of a launched Channel folder, or the
folder an install names); the settings store, `loginData.json` and every other launcher file are
never opened. A library folder the launcher has not yet logged is found only after its first
install, verify or launch, or by adding it manually.

## Considered Options

- Decrypt `launcher store.json` with the key embedded in the launcher and keep only the library
  key: exact, but reads credential-bearing data into memory and breaks whenever the launcher
  changes its key or store layout.
- Default path and manual folders only: safe, but misses the installs the launcher setting exists
  for.
