# A graphics teardown runs in named stages and an abandoned one names the stage it stopped in

Decided for issues #947 and #934. It builds on the bounded teardown of the offscreen renderer
(`Offscreen::close`, `Released`) and the exit guard of the window.

## Context

Once, in a load run on the discrete GPU, dropping the offscreen device never returned. The bound
that followed (idle wait, drop on a worker thread, exit guard of the window) keeps the caller
alive, but it does not say what the drop waits on.

## What was tried

On the RTX 5090 (Vulkan) of the development machine, through `p4kx-render`'s ignored real-adapter
tests and the real `p4kx ui hologram`:

- 1 thread, 20 rounds of open, draw, read back, close: every teardown 35 to 60 ms, none abandoned.
- 24 threads at once, 20 rounds each (480 devices created, drawn on and closed concurrently): none
  abandoned.
- The same with the idle wait switched off, so that the drop ran straight after the read-back:
  none abandoned.
- 145 runs of the whole `gpu::` test group on the real adapter, six processes at a time, with
  the idle wait off: none abandoned, none stalled.
- A raw wgpu device with 200 large copies submitted and dropped at once, 30 rounds: each drop
  returned in about 60 ms.

The hang does not reproduce through the paths the renderer uses. It stays a rare event of the
driver, and the evidence above excludes an unfinished readback, an unfinished submission, the
number of devices alive at once and the thread the drop runs on.

## Decision

**The teardown runs in defined stages.** `Gpu::release` waits for the queue to go idle
(`Stage::Idle`), then drops the renderer's resources (`Stage::Renderer`), the queue
(`Stage::Queue`) and the device (`Stage::Device`), each one explicitly and in that order.

**An abandoned teardown names its stage.** `Released::Abandoned` carries the `Stage` the worker was
in when the limit passed. `p4kx ui hologram` prints it in its warning, and the real-adapter tests
print it on failure, so the next occurrence says which driver call it stopped in.

**The bounds stay.** `TEARDOWN_LIMIT` (15 s) for the caller, the worker thread for the drop, and
the exit guard of the window (status 4) for the device that eframe drops after the exit hook.

## Consequences

The root cause stays open as a driver behaviour until an occurrence names its stage. #947 asked
for a clean, bounded teardown that cannot block the viewer, and that holds; #934 stays open for the
stalls on the software adapter that were never explained. The real-adapter tests
(`P4KX_REAL_GPU_RUNS`, ignored by default) are the way to look for the hang again.
