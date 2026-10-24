# Own archive and DataForge core instead of depending on StarBreaker

StarBreaker (diogotr7, MIT) already reads p4k, CryXml and DataForge in Rust, but it is not
published on crates.io, its API follows its own GUI, and every DataForge version bump in a patch
would leave us waiting on its release cadence. We write our own core crate for the Archive, CryXml
and DataForge, and use StarBreaker, unp4k (MIT) and the observed behaviour of the owner's earlier reference tool as
references; code ported from the MIT projects keeps their copyright notice.

## Considered Options

- StarBreaker crates as a git dependency: fastest start, but coupled to a foreign, unversioned API.
- Fork StarBreaker and add our GUI: inherits its structure and leaves upstream merges as ongoing work.
