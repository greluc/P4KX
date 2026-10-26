### Removed

- Loadout changes: `ChangeError::Incompatible` in `p4kx-assembly`; a replacement that does not
  fit its port is applied and returned by `ChangedLoadout::mismatches`. This is breaking for an
  exhaustive `match` on `ChangeError`. It is also breaking for scripts: `--replace` of `entity
  loadout`, `entity glb`, `entity package` and `ui hologram` with an Entity that does not fit the
  port used to exit with code 2 and print no JSON, and now exits with code 0 and lists the
  mismatch in `incompatible`.
