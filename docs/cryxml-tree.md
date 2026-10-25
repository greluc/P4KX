# The CryXml child table against the parent indices

Measured for issue #263; the decision is ADR-0142.

A CryXml node carries a parent index, and the file has a child table whose entries each node
addresses with `firstChild` and `childCount`. `p4kx-cryxml` builds the tree from the parent
indices. `compare_trees` builds the child-table view too and counts where the two differ.

## What is counted

Per file, over every CryXml Entry of the Archive and of the `.socpak` containers inside it
(read to their full depth):

| Count | Meaning |
|---|---|
| rejected | `parse` returns an error (for example `BadParent`) |
| other order | a node's child list names the children its parent indices give, in another order |
| equal copies | a node's child list names a sibling again where the parent indices give an equal one |
| other nodes | a child list names or misses other nodes, and is not covered by equal copies |
| range outside the table | `firstChild` and `childCount` lie outside the child table |
| attributes not sequential | a node's `firstAttribute` is not where the previous node's attributes end |

A file can be in several rows. Equal means the same names, contents, attributes and shape of the
whole subtree.

## Counts

Run with `cargo test --release -p p4kx-extraction --test real_archive -- --ignored cryxml_trees`
on the Archive of `P4KX_DATA_P4K` and, when set, of `P4KX_DATA_P4K_B`:

| | LIVE | PTU |
|---|---:|---:|
| Entries scanned, containers included | 1 662 092 | 1 662 777 |
| `.socpak` containers opened (failed) | 9 668 (0) | 9 674 (0) |
| CryXml files | 169 946 | 170 071 |
| unreadable | 0 | 0 |
| rejected | 0 | 0 |
| child table agrees | 141 430 | 141 555 |
| other order | 0 | 0 |
| equal copies | 28 516 | 28 516 |
| other nodes | 39 | 39 |
| range outside the table | 0 | 0 |
| attributes not sequential | 0 | 0 |

All 28 516 files that disagree are `.entxml` Entities. The child table lists the first of two
equal siblings twice, and in the 39 files that also count under "other nodes" the duplicated node
has `firstChild` 0, so its child list is the root's. The parent indices give the right tree in
every one of them (ADR-0142).
