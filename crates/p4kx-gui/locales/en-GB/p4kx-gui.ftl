app-title = P4K-Extractor
menu-archive = Archive
action-open = Open Archive…
action-add-folder = Add folder…
action-add-archive = Add Archive…
menu-recent = Recent Archives
recent-none = No recent Archives
channels-heading = Channels
channels-none = No Channels found. Add a folder or an Archive through the Archive menu.
menu-language = Language
language-de-DE = Deutsch
language-en-GB = English (UK)
menu-help = Help
font-licence = Font licence
update-check = Check for a new version at start
update-available = Version { $version } is available
dialog-archive-filter = Star Citizen Archive
column-name = Name
column-compression = Compression
column-encryption = Encryption
column-size = Size
select-all = Select all shown Entries
select-entry = Select { $name }
compression-store = Stored
compression-deflate = Deflate
compression-zstd = Zstandard
compression-other = Method { $method }
encrypted-yes = Yes
encrypted-no = No
expand-folder = Expand { $name }
collapse-folder = Collapse { $name }
search-label = Search
search-hint = Filter by path: a glob with * or ?, otherwise any part of the path
drop-hint = Drop a Data.p4k here to open it
empty-title = No Archive open
drop-release = Release to open
quick-filters = Quick filters
close-dialog = Close dialog
shortcut-reload = F5
shortcut-extract-selected = Ctrl+E
status-empty = Open an Archive or drop a Data.p4k onto the window.
status-opening = Opening { $path }…
status-failed = Cannot open { $path }: { $reason }
status-opening-stopped = Opening { $path } stopped without a result.
status-entries =
    { $count ->
        [one] { $number } Entry
       *[other] { $number } Entries
    }
status-shown = { $number } shown
status-selected = { $number } selected
status-selected-hidden = { $number } selected, { $hidden } not shown
status-searching = Searching…
status-build = Build { $build }
status-build-unknown = Build unknown
preview-tab-text = Text
preview-tab-hex = Hex
preview-copy = Copy
preview-none = Select an Entry to preview it.
preview-loading = Reading { $path }…
preview-too-large = { $path } is { $size }. Entries larger than { $limit } are not previewed; extract it to look inside.
preview-truncated = Truncated: showing the first { $shown } of { $total } bytes.
preview-failed = Cannot read { $path }: { $reason }
preview-stopped = Reading { $path } stopped without a result.
preview-archive-changed = The Archive changed on disk. Reload it to preview Entries.
archive-changed =
    { $stopped ->
        [yes] { $path } changed on disk, probably updated by the RSI Launcher. A running Extraction was stopped; reload the Archive to extract or preview Entries again.
       *[no] { $path } changed on disk, probably updated by the RSI Launcher. Reload the Archive to extract or preview Entries again.
    }
action-reload = Reload
extract-selected = Extract selected…
extract-shown = Extract shown…
extract-all = Extract all…
extraction-setup-title = Extract Entries
extraction-scope-heading = Entries
extraction-scope-selected = Selected ({ $number })
extraction-scope-shown = Shown ({ $number })
extraction-scope-all = Whole Archive ({ $number })
extraction-target-heading = Target
extraction-target-new = New folder inside
extraction-target-existing = Existing folder
extraction-target-new-name = Creates { $name }
extraction-folder-none = No folder chosen
extraction-choose-folder = Choose folder…
extraction-policy-heading = Files that already exist
extraction-policy-skip = Skip
extraction-policy-overwrite = Overwrite
extraction-policy-if-changed = Overwrite only if changed
extraction-conversion = Convert CryXml and DataForge to XML, textures and sounds
extraction-texture-merge = Merge split textures into one DDS
extraction-texture-png = Export textures as PNG (high-range as OpenEXR)
extraction-texture-mip-largest = Largest mip
extraction-texture-mip-level = Mip level
extraction-texture-raw = Keep the raw texture parts
extraction-audio-label = Sounds as
extraction-audio-ogg = Ogg
extraction-audio-wav = WAV
extraction-audio-raw = WEM, unchanged
extraction-audio-banks = Ogg falls back to WAV for a codec without an Ogg form. Soundbanks are also unpacked into a folder per bank, each WEM named by its ID.
extraction-start = Extract
extraction-close = Close
extraction-confirm-all =
    { $count ->
        [one] Extract the whole Archive, { $number } Entry of { $size }?
       *[other] Extract the whole Archive, { $number } Entries of { $size }?
    }
extraction-confirm-all-action = Extract whole Archive
extraction-back = Back
extraction-running-title = Extracting
extraction-progress = { $done } of { $total } Entries, { $percent } %
extraction-cancelling = Cancelling…
extraction-cancel = Cancel
extraction-summary-title = Extraction finished
extraction-summary-cancelled-title = Extraction cancelled
extraction-summary-changed-title = Extraction stopped: the Archive changed
extraction-summary-changed = The Archive changed on disk during the Extraction. Files written before may be stale; reload the Archive and extract again.
extraction-summary-counts = Extracted { $extracted }, skipped { $skipped }, failed { $failed }, cancelled { $cancelled }
extraction-summary-counts-converted = Extracted { $extracted } ({ $converted } converted), skipped { $skipped }, failed { $failed }, cancelled { $cancelled }
extraction-summary-records = Records written { $written }, skipped { $skipped }, failed { $failed }, cancelled { $cancelled }; { $errors } error values in { $erroneous } Records
extraction-summary-target = Target: { $path }
extraction-summary-failed = Failed ({ $number })
extraction-summary-skipped = Skipped ({ $number })
extraction-summary-cancelled = Cancelled ({ $number })
extraction-summary-unconverted = Not converted ({ $number })
extraction-summary-failure = { $path }: { $reason }
extraction-failed = Cannot prepare the target in { $path }: { $reason }
extraction-stopped = The Extraction into { $path } stopped without a result.
extraction-failed-title = Extraction failed
comparison-tab-archive = Browse
comparison-tab-comparison = Comparison
comparison-old-archive = Old Archive
comparison-new-archive = New Archive
comparison-pick-archive = Choose an Archive
comparison-choose-file = Choose file…
comparison-levels = Levels
comparison-level-entries = Entries
comparison-level-records = Records
comparison-entries-skipped = The Entries were not compared; switch on Entries to list them.
comparison-start = Compare
comparison-cancel = Cancel
comparison-opening = Opening both Archives…
comparison-progress = { $done } of { $total } Entries, { $percent } %
comparison-preparing = Preparing the list…
comparison-cancelled = The Comparison was cancelled.
comparison-failed = The Comparison failed: { $reason }
comparison-stopped = The Comparison stopped without a result.
comparison-empty-title = No Comparison yet
comparison-empty-hint = Choose an old and a new Archive, then compare them.
comparison-fact-old = Old
comparison-fact-new = New
comparison-side = { $channel } · { $build }
comparison-channel-unknown = No Channel
comparison-added = Added
comparison-removed = Removed
comparison-changed = Changed
comparison-directories = By directory ({ $number })
comparison-root = (root)
comparison-column-directory = Directory
comparison-column-change = Change
comparison-column-path = Path
comparison-column-old-size = Old size
comparison-column-new-size = New size
comparison-no-size = –
comparison-save-report = Save report…
comparison-report-filter = JSON report
comparison-report-saving = Saving the report to { $path }…
comparison-report-saved = Report saved to { $path }
comparison-report-failed = Cannot save the report to { $path }: { $reason }
comparison-no-differences = No Entry differs between the Archives.
comparison-no-matches = No change matches the filter.
comparison-shown = { $number } shown
comparison-row = { $change }: { $path }

diff-previous = Previous difference
diff-next = Next difference
diff-position = Difference { $current } of { $total }
diff-head-old = Old
diff-head-new = New
diff-hunk = Lines { $first }–{ $last }
diff-hunk-empty = No lines
diff-loading = Reading the line diff of { $path }…
diff-binary = { $path } is binary, so there is no line diff.
diff-too-large = { $path } is too large for a line diff: a side holds more than { $size } or { $lines } lines.
diff-failed = Cannot diff { $path }: { $reason }
diff-stopped = The line diff of { $path } stopped without a result.
diff-equal = The lines of both sides are equal; only the stored bytes differ.
diff-line-context = Line { $line }: { $text }
diff-line-removed = Removed line { $line }: { $text }
diff-line-added = Added line { $line }: { $text }
diff-marker-removed = −
diff-marker-added = +
query-tab = Query
query-label = Query
query-hint = from EntityClassDefinition at "*/spaceships/*" select Category
query-run = Run Query
query-cancel = Cancel
query-loading = Reading DataForge…
query-selecting = Selecting Records…
query-progress = { $done } of { $total } Records, { $percent } %
query-cancelled = The Query was cancelled.
query-failed = The Query failed: { $reason }
query-stopped = The Query stopped without a result.
query-archive-changed = The Archive changed on disk. Reload it to run Queries.
query-invalid = Line { $line }, column { $column }: { $reason }
query-empty-title = No Archive open
query-empty-hint = Open an Archive to run Queries over its DataForge.
query-prompt = Write a Query and run it with Ctrl+Enter, e.g. from AmmoParams select speed where speed >= 1500
query-no-rows = No Record matches the Query.
query-rows =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
query-rows-failed =
    { $count ->
        [one] { $number } Record, { $failed } failed
       *[other] { $number } Records, { $failed } failed
    }
query-unresolved =
    { $count ->
        [one] , { $number } unresolved Reference
       *[other] , { $number } unresolved References
    }
query-export-csv = Export CSV…
query-export-json = Export JSON…
query-filter-csv = CSV file
query-filter-json = JSON file
query-exporting = Exporting the result to { $path }…
query-exported =
    { $count ->
        [one] { $number } Record exported to { $path }
       *[other] { $number } Records exported to { $path }
    }
query-export-failed = Cannot export the result to { $path }: { $reason }
query-export-cancel = Cancel export
query-export-cancelled = The export to { $path } was cancelled.
query-page = Page { $page } of { $pages }
query-page-first = First page
query-page-previous = Previous page
query-page-next = Next page
query-page-last = Last page

dataforge-node = DataForge
dataforge-node-loading = Loading…
preview-tab-dataforge = DataForge
dataforge-loading = Loading DataForge from { $path }…
dataforge-unsupported = DataForge version { $version } is not supported by this version of P4K-Extractor.
dataforge-failed = Cannot load DataForge from { $path }: { $reason }
dataforge-stopped = Loading DataForge from { $path } stopped without a result.
dataforge-archive-changed = The Archive changed on disk. Reload it to load DataForge.
dataforge-version = Version
dataforge-records = Records
record-search-label = Search Records
record-search-hint = Filter by record path or type: a glob with * or ?, otherwise any part
record-search-prompt = Type part of a record path or type name to find Records.
record-search-no-matches = No Record matches the search.
record-search-matches =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
dataforge-structs = Structs
dataforge-enums = Enums
dataforge-first-records = First Records
column-type = Type
column-record-path = Record path
record-change-progress = { $done } of { $total } Records, { $percent } %
record-change-moved = Moved
record-change-unreadable = Unreadable
record-change-moved-path = { $old } to { $new }
record-change-no-differences = No Record differs between the DataForges.
record-change-head-field = Field
record-change-field-count =
    { $count ->
        [one] { $number } change
       *[other] { $number } changes
    }
record-change-loading = Comparing the fields of { $path }…
record-change-failed = Cannot compare the fields of { $path }: { $reason }
record-change-stopped = Comparing the fields of { $path } stopped without a result.
record-change-only-new = The Record exists only in the new Archive.
record-change-only-old = The Record exists only in the old Archive.
record-change-only-path = Only the record path changed; the fields are equal.
record-change-no-field-changes = No field differs.
record-change-unreadable-old = The old Record cannot be read: { $reason }
record-change-unreadable-new = The new Record cannot be read: { $reason }
record-change-record-name = Record name
record-change-record-type = Record type
record-change-value-missing = –
record-change-value-null = null
record-change-value-struct = { $type } (struct)
record-change-value-array =
    { $count ->
        [one] { $number } element
       *[other] { $number } elements
    }
record-change-value-reference-at = { $path } · { $at }
record-change-value-error = Error: { $reason }
record-change-field-row = { $field }: { $old } to { $new }
change-extraction-action-old = Extract old ({ $number })…
change-extraction-action-new = Extract new ({ $number })…
change-extraction-scope-old = Changed Entries of the old Archive, in their old version ({ $number })
change-extraction-scope-new = Added and changed Entries of the new Archive ({ $number })
change-extraction-scope-filtered = Only the changes matching the Filter { $filter }
change-extraction-summary-changed = The Archive changed on disk during the Extraction. Files written before may be stale; compare the Archives again and extract the changes again.

preview-tab-image = Image
image-mip = Mip
image-format = Format
image-size = Size
image-mips = Mips
image-faces = Cubemap faces
image-dimensions = { $width } × { $height }
image-none = –
image-shown = Mip { $level }: { $width } × { $height } pixels
image-decoding = Decoding mip { $level } of { $path }…
image-unsupported = The preview cannot decode { $format } textures. The Text and Hex tabs still show the data of { $path }.
image-failed = Cannot decode { $path }: { $reason }
image-unshowable = Every mip of { $path } is larger than the preview can show.

menu-export = Export
record-export-action = Record export…
record-export-title = Export Records
record-export-scope-heading = Records
record-export-scope-tree = Selected in the Record tree: { $name } ({ $number })
record-export-scope-tree-none = Selected in the Record tree (nothing selected)
record-export-scope-filter = Record path filter
record-export-filter-hint = A glob with * or ?, otherwise any part of the record path
record-export-form-heading = Form
record-export-form-json = JSON, one file per Record under its record path
record-export-form-ndjson = NDJSON, one file { $name } with one Record per line
record-export-start = Start export
record-export-running-title = Exporting Records
record-export-reading = Reading DataForge…
record-export-selecting = Selecting Records…
record-export-progress = { $done } of { $total } Records, { $percent } %
record-export-summary-title = Record export finished
record-export-summary-cancelled-title = Record export cancelled
record-export-summary-cancelled = The Record export was cancelled before every Record was written.
record-export-summary-counts = Records written { $written }, skipped { $skipped }, failed { $failed }, cancelled { $cancelled }
record-export-summary-errors = Records with error values: { $erroneous }, error values: { $errors }
record-export-summary-no-match = No Record matches the filter.
record-export-summary-target = Target: { $path }
record-export-summary-file-failed = Cannot write { $path }: { $reason }
record-export-summary-failed = Failed ({ $number })
record-export-summary-skipped = Skipped ({ $number })
record-export-summary-failure = { $path } ({ $guid }): { $reason }
record-export-failed-title = Record export failed
record-export-failed = The Record export into { $path } failed: { $reason }
record-export-stopped = The Record export into { $path } stopped without a result.

preview-tab-xml = XML
xml-converting = Converting { $path } to XML…
xml-too-large = { $path } is { $size }. CryXml Entries larger than { $limit } are not converted in the preview; extract it with Conversion on to read it.
xml-failed = Cannot convert { $path } to XML: { $reason }
xml-stopped = Converting { $path } stopped without a result.
xml-truncated = Truncated: showing the first { $lines } lines, the first { $limit } of the XML.
xml-view = Converted XML

preview-tab-audio = Audio
audio-codec = Codec
audio-channels = Channels
audio-sample-rate = Sample rate
audio-duration = Duration
audio-hertz = { $rate } Hz
audio-unknown = –
audio-play = Play
audio-pause = Pause
audio-position = Position
audio-volume = Volume
audio-time = { $position } / { $duration }
audio-decoding = Decoding { $path }…
audio-unsupported = The preview cannot play { $codec } sounds. The Text and Hex tabs still show the data of { $path }.
audio-too-long = { $path } is too long to preview: its decoded sound would exceed { $limit }.
audio-failed = Cannot decode { $path }: { $reason }
audio-output-failed = The sound cannot be played: { $reason }
