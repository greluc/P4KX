# The MCP audio excerpt is a bounded WAV and the only tool that returns audio

Decided for issue #287, on top of ADR-0038 (the legal notes of the MCP tools) and ADR-0018 (the
audio index).

## Decision

The MCP server answers questions about audio with metadata and paths: `search_audio_triggers`,
`get_entity_sounds`, `resolve_audio_trigger`, `list_soundbank_wems` and `get_wem_info` return
trigger and event names, IDs, soundbank and Entry paths, codec, channels, sample rate and
duration, and never a byte of audio.

`get_audio_excerpt` is the one exception, because the issue asks for it and a model cannot judge a
sound it cannot hear. It is bounded so that it stays an excerpt and not a way to move the sound
out of the Archive:

- at most 10 seconds (5 by default), and the WAV is cut where it would exceed 1 MiB (`truncated`
  says so, `max_bytes` states the cap);
- the audio is a 16-bit PCM WAV in an `audio/wav` content block next to the structured metadata;
  the structured content never holds audio bytes, so the output schema carries none;
- a sound that decodes to more than 64 MiB is refused, so one call cannot hold more than that in
  memory; at most two such reads run at once (the model permit);
- plug-in media and the prefetched start of a streamed sound cannot be decoded and are refused;
  a WEM ID with a loose Entry is read from the Entry, not from the prefetched start;
- the description says that the audio is game media and must not be redistributed, as every tool
  that returns Archive content does.

## Consequences

The audio index (every ATL definition and soundbank) is built once per open Archive and kept with
it, by the first audio call. `ArchiveAudio::with_index` lets a service user keep the index itself.
Audio calls take the model permit because they read whole soundbanks.
