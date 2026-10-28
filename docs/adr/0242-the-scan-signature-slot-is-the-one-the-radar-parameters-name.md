# The scan signature slot is the one the radar parameters name

Decided for issue #830. Evidence: LIVE 4.10, the 269 `EntityClassDefinition` Records under
`entities/mineable` and the `RadarSystemGlobalParams` Record.

## Decision

- The catalog exports the mineable rocks and deposits as the kind `mineableEntities`, additive to
  schema version 2 (ADR-0054). Facts only: GUIDs, Record names and numbers; no game text.
- A rock states its signatures as an array of eight numbers in
  `Components[SSCSignatureSystemParams].radarProperties.baseSignatureParams.signatures`. The
  DataForge does not name a slot there, but the array `signatureTypeParams` of
  `RadarSystemGlobalParams.signatureSystemParams` has one entry per slot, each with a
  `displayName`. The slot whose
  `displayName` is `@hud_scanning_info_rs_signature` is the scan signature; on LIVE 4.10 it is index
  4, after IR, EM, CS and DB. That the rocks fill only that slot agrees with it.
- The slot is read from the Archive, not fixed at 4: a Build that reorders the types moves the
  slot with them. `scanSignature` is `null` when the Archive has no such entry, and the raw
  `signatures` array is exported as well, so the Basetool is not tied to one slot.
- `scanSignature` is `null` for a Record without signatures and for one whose signatures are all
  zero (the FPS blockers and the pearls); it is `0` only when another slot is non-zero.
- The entity keeps the References of `Components[MineableParams]` and the radar contact type, so
  many entities sharing a composition do not repeat its data. A provider preset reaches an entity
  through the existing `entityGuid`; no field of the presets changes.
- Development leftovers follow the generic rule: the 38 `test` and `template` Records, among them
  `test_miningdestroyable`, are counted as skipped.

## Consequences

- 231 entities on LIVE 4.10, 224 with a scan signature from 3000 to 4900.
- The signature slot is looked up once per build, before the Records are checked.
- A later signature type changes nothing, a renamed key makes `scanSignature` `null` everywhere
  while `signatures` stays correct.
