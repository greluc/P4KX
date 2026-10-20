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
archive-issues =
    { $kind ->
        [one] The directory of the Archive has one defect in an Entry. The Entry stays listed, but its data may not be readable; an Extraction reports it if it fails, and `p4kx info` names it.
       *[other] The directory of the Archive has { $count } defects in its Entries. The Entries stay listed, but their data may not be readable; an Extraction reports those that fail, and `p4kx info` names them.
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
extraction-summary-records-failed = Records not written ({ $number })
extraction-summary-record-failure = { $path } ({ $guid }): { $reason }
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
query-indexing = Indexing References…
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
query-completion-more =
    { $count ->
        [one] { $number } more name, keep typing
       *[other] { $number } more names, keep typing
    }
query-save = Save Query…
query-saved-title = Saved Queries
query-saved-none = No saved Queries yet. Save the Query in the editor to rerun it with one click.
query-saved-run = Run the saved Query { $name }
query-saved-rename = Rename
query-saved-delete = Delete
query-save-title = Save Query
query-rename-title = Rename saved Query
query-delete-title = Delete saved Query
query-name-label = Name
query-name-hint = e.g. Heavy ships
query-save-action = Save
query-rename-action = Rename
query-delete-action = Delete
query-delete-question = The saved Query “{ $name }” is deleted. The Query in the editor stays.
query-name-replaces = Replaces the saved Query of this name.
query-name-empty = Enter a name.
query-name-too-long = The name is longer than { $max } characters.
query-name-taken = Another saved Query has this name.
query-name-control = The name must not hold line breaks or other control characters.
query-name-full = The saved Queries are full: at most { $max } Queries and { $size } in all. Delete one first.
query-name-bad-text = The Query in the editor is empty or too long to save.

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
container-opening = Opening…
container-unreadable = Cannot open
container-unreadable-detail = Cannot open { $name }: { $reason }
container-over-budget = Not opened: memory limit
container-over-budget-detail = { $name } was not opened: the open containers already hold { $limit }. Collapse other containers and expand it again.
container-stopped = Opening stopped
container-stopped-detail = Opening { $name } stopped without a result. Expand it again to retry.
container-archive-changed = Archive changed
container-archive-changed-detail = The Archive changed on disk. Reload it to open { $name }.
container-status-entries =
    { $count ->
        [one] Container: { $number } Entry
       *[other] Container: { $number } Entries
    }

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

catalog-action = Basetool catalog…
catalog-file-filter = Basetool catalog
catalog-running-title = Creating the Basetool catalog
catalog-reading = Reading DataForge…
catalog-progress = { $done } of { $total } Records checked, { $percent } %
catalog-writing = Writing { $path }…
catalog-summary-title = Basetool catalog created
catalog-kinds-heading = Kinds
catalog-summary-kind = { $kind ->
    [manufacturers] Manufacturers
    [items] Items
    [ships] Ships
    [commodities] Commodities
    [blueprints] Blueprints
   *[other] { $kind }
}: { $written } written, { $skipped } skipped, { $complete ->
    [yes] complete
   *[no] partial
}
catalog-summary-skipped = Records skipped as development leftovers: { $number }
catalog-summary-failed = Records that could not be read and were left out ({ $number })
catalog-summary-failure = { $path } ({ $guid }): { $reason }
catalog-texts-heading = Texts
catalog-summary-texts = { $language ->
    [en] English
    [de] German
   *[other] { $language }
}: { $resolved } resolved, { $missing } missing, { $placeholders } placeholders
catalog-summary-texts-unavailable = { $language ->
    [en] English
    [de] German
   *[other] { $language }
}: the Archive holds no global.ini, { $missing } missing
catalog-summary-missing = Missing texts: { $number }
catalog-summary-size = Size: { $size } of the upload limit of { $limit }
catalog-near-limit = The catalog is close to the basetool upload limit of { $limit }.
catalog-over-limit = The catalog is over the basetool upload limit of { $limit }; the basetool will reject the upload.
catalog-summary-file = File: { $path }
catalog-cancelled-title = Basetool catalog cancelled
catalog-cancelled = The Basetool catalog was cancelled. No file was written.
catalog-failed-title = Basetool catalog failed
catalog-failed = The Basetool catalog { $path } could not be created: { $reason }
catalog-stopped = The Basetool catalog { $path } stopped without a result.
catalog-output-is-archive = The Basetool catalog cannot be written to { $path }, because that is the open Archive. Choose another file.
preview-tab-xml = XML
xml-converting = Converting { $path } to XML…
xml-too-large = { $path } is { $size }. CryXml Entries larger than { $limit } are not converted in the preview; extract it with Conversion on to read it.
xml-failed = Cannot convert { $path } to XML: { $reason }
xml-stopped = Converting { $path } stopped without a result.
xml-truncated = Truncated: showing the first { $lines } lines, the first { $limit } of the XML.
xml-view = Converted XML

scroll-text = Text lines
scroll-hex = Hexadecimal rows
scroll-image = Picture
scroll-diff = Line differences
scroll-fields = Changed fields
scroll-lines = Record fields
scroll-localization = Localisation rows
scroll-saved-queries = Saved Queries
scroll-soundbank = Embedded WEMs

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
audio-column-id = ID
audio-prefetched = Prefetched
audio-prefetched-note = WEM { $id } is only prefetched in this soundbank and cannot be played here. The loose WEM Entry with the same ID holds the whole sound.
audio-plugin = Plug-in
audio-plugin-note = WEM { $id } is plug-in media, such as a convolution reverb impulse, and holds no sound to play.
audio-bank-summary =
    { $size } · { $count ->
        [one] { $number } WEM
       *[other] { $number } WEMs
    }
audio-bank-empty = { $path } embeds no WEMs.
audio-bank-failed = Cannot read the soundbank { $path }: { $reason }
audio-wem-name = WEM { $id } of { $bank }
audio-wem-row = WEM { $id }, { $codec }, { $duration }
audio-unsupported-wem = The preview cannot play { $codec } sounds ({ $name }).

texture-parts =
    { $count ->
        [one] +{ $number } part
       *[other] +{ $number } parts
    }
texture-expand = Show the parts of { $name }
texture-collapse = Hide the parts of { $name }

preview-tab-record = Record
record-back = Back
record-forward = Forward
record-type = Type
record-guid = GUID
record-name = Name
record-head-value = Value
record-loading = Reading the Record { $path }…
record-failed = Cannot read the Record { $path }: { $reason }
record-stopped = Reading the Record { $path } stopped without a result.
record-archive-changed = The Archive changed on disk. Reload it to read the Record.
record-dangling = The Reference points to the Record { $guid }, which does not exist in this DataForge.
record-follow = Follow the Reference to { $path }
record-row = { $field }: { $value }
record-no-fields = The Record has no fields.
record-xml-failed = The Record XML of { $path } cannot be written: { $reason }

record-language-menu = Record texts
record-language-keys-only = Keys only
record-language-unavailable = The Archive holds no texts in { $language }, so only the keys are shown.
record-language-key-row = { $key }: { $text }
record-language-key-row-missing = { $key } (no text in this language)
record-language-load-failed = The texts in { $language } cannot be read, so only the keys are shown: { $reason }
localization-tab = Localisation
localization-label = Search
localization-hint = Key or text, e.g. item_Name or Aurora
localization-loading = Reading the localisation files…
localization-no-texts = The Archive holds no global.ini.
localization-failed = The localisation files cannot be read: { $reason }
localization-stopped = Reading the localisation files stopped without a result.
localization-archive-changed = The Archive changed on disk. Reload it to read the localisation files.
localization-empty-title = No Archive open
localization-empty-hint = Open an Archive to look up its localisation keys and texts.
localization-column-key = Key
localization-language-en = English
localization-language-de = German
localization-language-fr = French
localization-language-it = Italian
localization-language-es-ES = Spanish (Spain)
localization-language-es-419 = Spanish (Latin America)
localization-language-pt-BR = Portuguese (Brazil)
localization-language-ja = Japanese
localization-language-ko = Korean
localization-language-zh-Hans = Chinese (simplified)
localization-language-zh-Hant = Chinese (traditional)
localization-language-other = Other language
localization-missing = Missing
localization-keys =
    { $count ->
        [one] { $number } key
       *[other] { $number } keys
    }
localization-keys-shown =
    { $count ->
        [one] { $number } of { $total } keys
       *[other] { $number } of { $total } keys
    }
localization-searching = Searching…
localization-no-match = No key or text matches the search.
localization-copy-key = Copy key
localization-copy-text = Copy text
localization-notes = Some global.ini files were not read cleanly:
preview-tab-model = Model
model-reset = Reset view
model-hint = Drag: orbit · right-drag or Shift-drag: pan · wheel: zoom · arrows: orbit, Shift and arrows: pan · + and -: zoom · Home: reset
model-vertices = Vertices
model-triangles = Triangles
model-extent = Size
model-dimensions = { $width } × { $depth } × { $height } m
model-view = 3D view of { $path }: { $vertices } vertices, { $triangles } triangles
model-loading = Reading { $path }…
model-failed = Cannot read { $path }: { $reason }
model-file-too-large = { $path } or its geometry companion is { $size }, above the { $limit } the viewer reads.
model-empty = { $path } has no triangles to show.
model-too-large = { $path } has { $vertices } vertices and { $triangles } triangles, more than the viewer shows: { $max_vertices } vertices and { $max_triangles } triangles.
model-not-finite = { $path } has no usable coordinates.
model-gpu-none = The window has no graphics device for the 3D viewer. Everything else works as usual.
model-gpu-texture-side = The graphics device takes textures of { $limit } pixels a side at most, too few for the 3D viewer. Everything else works as usual.
model-gpu-buffer-size = The graphics device takes buffers of { $limit } bytes at most, too few for the 3D viewer. Everything else works as usual.
model-gpu-setup = The graphics device refused the 3D viewer: { $reason } Everything else works as usual.
model-gpu-lost = The graphics device was lost. Restart P4K-Extractor to see models again.
model-gpu-failed = The graphics device failed to draw the model. Select another model or restart P4K-Extractor.
model-gpu-rejected = { $path } is too large for the graphics device.


preview-tab-incoming = Referenced by
scroll-incoming = Referring Records
incoming-head-field = Field
incoming-head-record = Referring Record
incoming-loading = Finding the Records that refer to { $path }…
incoming-failed = Cannot find the Records that refer to { $path }: { $reason }
incoming-stopped = Finding the Records that refer to { $path } stopped without a result.
incoming-archive-changed = The Archive changed on disk. Reload it to find the Records that refer to a Record.
incoming-none = No Record refers to this Record.
incoming-follow = Open the Record { $path }, which refers to this Record in { $field }
incoming-summary = { $references } from { $records }
incoming-reference-count =
    { $count ->
        [one] { $number } Reference
       *[other] { $number } References
    }
incoming-record-count =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
incoming-truncated = The first { $shown } of { $total } References are listed.
incoming-incomplete = Some Records could not be read or the index is full, so the list may miss References.

preview-key-required = { $path } is encrypted. Enter the Archive key to read it.
preview-key-rejected = The Archive key does not decrypt { $path }. Enter the right key to read it.
key-menu = Archive key…
key-title = Archive key
key-needed = An encrypted Entry needs the key of this Archive. Enter it once; it is stored as a file in your user profile.
key-rejected = The key in use does not decrypt this Archive. Enter the right key.
key-intro = Encrypted Entries can only be read with the key of the Archive. Enter it once; it is stored as a file in your user profile.
key-label = Key
key-hint = 32 hexadecimal digits
key-show = Show the key
key-stored = A key is stored: { $masked }
key-not-stored = No key is stored.
key-save = Save
key-cancel = Cancel
key-remove = Remove stored key
key-error-length = A key has 32 hexadecimal digits; you entered { $digits }.
key-error-character = Character { $position } is not a hexadecimal digit.
key-error-wrong = This key does not decrypt the Archive.
key-error-check = The key could not be checked: { $reason }
key-error-store = The key could not be stored: { $reason }
key-error-remove = The stored key could not be removed: { $reason }
key-checking = Checking the key against the Archive…
key-note-variable = The environment variable P4KX_ARCHIVE_KEY does not hold a valid key and is ignored.
key-note-store = The stored key cannot be used. Enter the key again to replace it. ({ $reason })
