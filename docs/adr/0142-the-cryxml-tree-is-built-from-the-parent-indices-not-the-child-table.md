# The CryXml tree is built from the parent indices, not the child table

Decided for issue #263. Builds on the `p4kx-cryxml` reader and the measurement in
`docs/cryxml-tree.md`.

## Decision

**`p4kx-cryxml` keeps building the element tree from each node's parent index.** The child table
is not used to build the tree. Where the two disagree the parent indices win, and the
disagreement is reported, never raised:

- `compare_trees` returns a `TreeComparison` that counts, per node, the child lists that name the
  same children in another order, name an equal copy of a child, name other nodes, or lie outside
  the child table, the nodes whose attributes do not start where the previous node's end, and
  whether the tree read from the child table, starting at the root, equals the parent-index
  tree. It returns the `CryXmlError` of `parse` for a file that cannot be read. Its work is
  bounded by the node count (a child list of another length is classified without sorting, and
  the comparisons of subtrees share one step budget), so a crafted file cannot make it slow, and
  nothing in it panics on corrupt input.
- `parse` and `read` are unchanged: a file the child table contradicts still converts, from the
  parent indices.
- The real-Archive test `cryxml_trees` runs the comparison over every CryXml Entry, containers
  included. It fails when a file is rejected or unreadable, when a container does not open, when
  a child table lists the same children in another order, when a child range lies outside the
  table, when an attribute does not start where the previous node's end, or when the tree read
  from the child table differs from the parent-index tree in any file.

## Reasons

- **Reading the child table would change no output.** On LIVE the tree read from the
  child table, walked from the root, has the same names, contents, attributes and shape as the
  parent-index tree in every one of the 169 946 CryXml files. The 16.8 per cent of
  files whose child table disagrees (28 516, all `.entxml` Entities of interiors)
  all list a sibling a second time where the parent indices give a distinct node, and that sibling
  is an equal copy (same names, contents, attributes and shape of the subtree). In 39 files per
  Channel a duplicated node also has a `firstChild` of 0, which would give it the root's children;
  the child table never reaches that node, because its parent lists the earlier equal copy twice,
  so the walk does not visit it.
- **Criterion 4 of the issue has nothing to act on.** It asks for the child order from the child
  table, falling back to the parent indices when the table is empty or inconsistent. A child
  list in another order than the parent indices occurs in no file. Every file that disagrees
  lists a sibling twice and so is inconsistent, and the fallback picks the parent indices.
  Implementing it would change no output on either Channel.
- **Parent indices never disagree with themselves.** They are checked to precede their node, so
  the tree is finite and every element appears once. No file of either Channel violates that.
- **Attribute positions need no change.** In every file each node's first attribute is where the
  attributes of the nodes before it end, so reading them in sequence and by `firstAttribute` give
  the same attributes.

## Consequences

- A tool that builds from the child table, as the external decoders reviewed for the issue do,
  gets the same output as P4KX on every file of both Channels; it differs only on a file whose
  child table is wrong where the parent indices are right.
- A later Channel can change the counts. The real-Archive test fails when a child-table tree
  differs, and then this decision is revisited.
