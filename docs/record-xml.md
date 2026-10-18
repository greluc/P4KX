# Record XML

Record XML is the XML form of one Record of DataForge, written by `p4kx dcb export --format xml`
and by the `RecordXml` writer of `p4kx-dataforge`. It serialises the canonical Record tree, the
same tree JSON and the Comparison use; References are links and never embedded
([ADR-0004](adr/0004-own-record-xml-with-references.md)). It is not compatible with the XML of
unp4k.

The same Record always gives the same bytes, so two exports can be compared with ordinary diff
tools.

## Example

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Ship xmlns:p4kx="https://github.com/krt-profit/p4k-extractor/record-xml" guid="cbee34fd-6441-4000-b99f-924f7f26e19f" path="libs/foundry/records/ships/avenger" mass="25000.5" size="Large" label="@vehicle_NameAEGS_Avenger" crew="2">
  <p4kx:field name="path" value="hangar"/>
  <p4kx:error field="name" reason="offset 4294967295 in the text table is not a valid string"/>
</Ship>
```

## Document

- UTF-8 without byte order mark, LF line endings, a final line ending.
- The first line is `<?xml version="1.0" encoding="UTF-8"?>`.
- One root element per Record, named after the Record's type (its struct), e.g.
  `EntityClassDefinition`.
- Child elements are indented by two spaces. A root without child elements is self-closing.

## Root attributes

In this order:

1. `xmlns:p4kx`, only when the document uses a `p4kx:` element, with the value
   `https://github.com/krt-profit/p4k-extractor/record-xml`.
2. `type`, only on a `p4kx:record` root (see [Names](#names)).
3. `guid`: the Record's GUID, lower case, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`.
4. `path`: the record path without `.xml`, as DataForge stores it.
5. One attribute per scalar field, in declaration order: the fields of the root ancestor first,
   then those of each struct down to the Record's own type.

Attributes are separated by one space, values are in double quotes.

## Values

| Field type | Text |
|------------|------|
| bool | `true` or `false` |
| int8 … int64, uint8 … uint64 | Decimal, with `-` for negative values |
| float, double | The shortest text that reads back to the same value; with an exponent (`1e21`, `1.5e-7`) from 1e21 and below 1e-6 in magnitude, otherwise without (`0.1`, `25000.5`, `3`); `-0` for negative zero, `NaN`, `INF` and `-INF` |
| string | The text |
| localisation key | The key as stored, e.g. `@item_Name`; keys are not resolved |
| GUID | Lower case, `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx` |
| enum | The name of the option |

In attribute values `&`, `<`, `>` and `"` are written as `&amp;`, `&lt;`, `&gt;` and `&quot;`, and
tab, line feed and carriage return as `&#9;`, `&#10;` and `&#13;`, so they survive attribute
normalisation. Characters XML 1.0 cannot hold (control characters other than tab, line feed and
carriage return, U+FFFE and U+FFFF) are written as U+FFFD.

## Child elements

After the attributes, in declaration order:

- `<p4kx:field name="…" value="…"/>` for a scalar field whose name cannot be an attribute (see
  [Names](#names)).
- `<p4kx:error field="…" reason="…"/>` for a value that cannot be read, such as a string offset
  outside the text table, an enum field naming an enum that does not exist, or a bool byte other
  than 0 and 1. The Record is still written; the reason is English text for people, not a stable
  code.

Nested structs, arrays and References are added by later versions; until then such a field is a
`p4kx:error` whose reason says that its kind is not read yet.

## Names

A field is written as an attribute only when its name is an XML name without `:` that does not
start with `xml` in any case, is not `guid`, `path` or `type`, and no earlier field written as an
attribute has the same name. Any other scalar field is a `p4kx:field` element, so no value is lost or
mistaken for another.

When the type name is no such name, the root element is `p4kx:record` and the type is its `type`
attribute.

## Files

`p4kx dcb export` writes one file per Record below the target folder
([ADR-0008](adr/0008-record-files-by-record-path.md)):

- `<record path>.xml` for a Record that is alone at its record path,
- `<record path>/<Record name>.xml` for Records that share a record path, e.g.
  `libs/foundry/records/tagdatabase/tagdatabase.tagdatabase/Tag.Weapon.xml`,
- `<record path>/<Record name>.<GUID>.xml` when such a file collides with another one ignoring
  case, and `<record path>/<GUID>.xml` when a Record name is no plain file name.

Which file a Record gets depends on the whole DataForge, never on what `--filter` selects; folders
that differ only in case are one folder, spelt as the first Record in definition order needs it.
Record paths and names that could leave the target folder are rejected.