# The CryXml tree is built from the parent indices, not the child table

Decided for issue #263. Builds on the `p4kx-cryxml` reader and the measurement in
`docs/cryxml-tree.md`.

## Decision

**`p4kx-cryxml` keeps building the element tree from each node's parent index.** The child table
is not used to build the tree. Where the two disagree the parent indices win, and the
disagreement is reported, never raised:

- `compare_trees` returns a `TreeComparison` that counts, per node, the child lists that name the
  same children in another order, name an equal copy of a child, name other nodes, or lie outside
  the child table, and the nodes whose attributes do not start where the previous node's end. It
  returns the `CryXmlError` of `parse` for a file that cannot be read, and nothing in it
  panics on corrupt input.
- `parse` and `read` are unchanged: a file the child table contradicts still converts, from the
  parent indices.
- The real-Archive test `cryxml_trees` runs the comparison over every CryXml Entry, containers
  included, and fails when a file is rejected, when a child table lists the same children in
  another order, when a child range lies outside the table, or when an attribute does not start
  where the previous node's end. It does not fail on the copies below.

## Reasons

- **The child table is the one that is wrong.** On LIVE and PTU 16.8 per cent of the CryXml files
  (28 516 of about 170 000, all `.entxml` Entities of interiors) have a node whose child list
  names a sibling a second time where the parent indices give a distinct node. That sibling is
  an equal copy (same names, contents, attributes and shape of the subtree), so a child-table
  reader gets the same output there. In 39 files per Channel a duplicated node also has a
  `firstChild` of 0 and so lists the children of the root as its own; a child-table reader would
  give that node the wrong children.
- **Parent indices never disagree with themselves.** They are checked to precede their node, so
  the tree is finite and every element appears once. No file of either Channel violates that.
- **Attribute positions need no change.** In every file each node's first attribute is where the
  attributes of the nodes before it end, so reading them in sequence and by `firstAttribute` give
  the same attributes.

## Consequences

- The issue's fourth criterion (take the child order from the child table) does not apply: the
  data gives no case where the child table is right and the parent indices wrong.
- A tool that builds from the child table, as the external decoders reviewed for the issue do,
  builds a wrong tree in the 39 files per Channel whose child range is wrong. P4KX's output
  differs from theirs in exactly those files.
- A later Channel can change the counts. The real-Archive test keeps the guard for the cases
  that would make the parent indices unsound.
