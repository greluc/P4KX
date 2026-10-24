app-title = P4KX
menu-archive = Archiv
action-open = Archiv öffnen …
action-add-folder = Ordner hinzufügen …
action-add-archive = Archiv hinzufügen …
menu-recent = Zuletzt geöffnet
recent-none = Noch kein Archiv geöffnet
channels-heading = Channels
channels-none = Keine Channels gefunden. Füge über das Menü Archiv einen Ordner oder ein Archiv hinzu.
menu-language = Sprache
language-de-DE = Deutsch
language-en-GB = English (UK)
menu-help = Hilfe
font-licence = Schriftlizenz
update-check = Beim Start nach neuer Version suchen
update-available = Version { $version } ist verfügbar
dialog-archive-filter = Star-Citizen-Archiv
column-name = Name
column-compression = Kompression
column-encryption = Verschlüsselung
column-size = Größe
select-all = Alle angezeigten Einträge auswählen
select-entry = { $name } auswählen
compression-store = Unkomprimiert
compression-deflate = Deflate
compression-zstd = Zstandard
compression-other = Methode { $method }
encrypted-yes = Ja
encrypted-no = Nein
expand-folder = { $name } aufklappen
collapse-folder = { $name } zuklappen
search-label = Suche
search-hint = Nach Pfad filtern: Glob mit * oder ?, sonst ein beliebiger Teil des Pfads
drop-hint = Data.p4k hier ablegen, um sie zu öffnen
empty-title = Kein Archiv geöffnet
drop-release = Loslassen zum Öffnen
quick-filters = Schnellfilter
close-dialog = Dialog schließen
shortcut-reload = F5
shortcut-extract-selected = Strg+E
status-empty = Öffne ein Archiv oder ziehe eine Data.p4k in das Fenster.
status-opening = Öffne { $path } …
status-failed = { $path } lässt sich nicht öffnen: { $reason }
status-opening-stopped = Das Öffnen von { $path } brach ohne Ergebnis ab.
status-entries =
    { $count ->
        [one] { $number } Eintrag
       *[other] { $number } Einträge
    }
status-shown = { $number } angezeigt
status-selected = { $number } ausgewählt
status-selected-hidden = { $number } ausgewählt, davon { $hidden } nicht angezeigt
status-searching = Suche …
status-build = Build { $build }
status-build-unknown = Build unbekannt
preview-tab-text = Text
preview-tab-hex = Hex
preview-copy = Kopieren
preview-none = Einen Eintrag auswählen, um ihn in der Vorschau zu sehen.
preview-loading = { $path } wird gelesen …
preview-too-large = { $path } ist { $size } groß. Einträge über { $limit } werden nicht in der Vorschau gezeigt; zum Ansehen extrahieren.
preview-truncated = Gekürzt: angezeigt werden die ersten { $shown } von { $total } Bytes.
preview-failed = { $path } kann nicht gelesen werden: { $reason }
preview-stopped = Das Lesen von { $path } brach ohne Ergebnis ab.
preview-archive-changed = Das Archiv hat sich auf der Festplatte geändert. Lade es neu, um Einträge anzusehen.
archive-changed =
    { $stopped ->
        [yes] { $path } hat sich auf der Festplatte geändert, vermutlich durch ein Update im RSI Launcher. Eine laufende Extraktion wurde gestoppt; lade das Archiv neu, um wieder Einträge zu extrahieren oder anzusehen.
       *[no] { $path } hat sich auf der Festplatte geändert, vermutlich durch ein Update im RSI Launcher. Lade das Archiv neu, um wieder Einträge zu extrahieren oder anzusehen.
    }
archive-issues =
    { $kind ->
        [one] Das Verzeichnis des Archivs hat einen Fehler in einem Eintrag. Der Eintrag bleibt aufgelistet, seine Daten sind aber womöglich nicht lesbar; eine Extraktion meldet ihn, wenn er scheitert, und `p4kx info` nennt ihn.
       *[other] Das Verzeichnis des Archivs hat { $count } Fehler in seinen Einträgen. Die Einträge bleiben aufgelistet, ihre Daten sind aber womöglich nicht lesbar; eine Extraktion meldet die gescheiterten, und `p4kx info` nennt sie.
    }
game-running = Star Citizen läuft. Nutze dieses Werkzeug bei beendetem Spiel: Lesen funktioniert weiter, aber das Archiv ändert sich, wenn das Spiel aktualisiert, und das Werkzeug schreibt nie in die Installation.
archive-test-build = Dieses Archiv gehört zum Channel { $channel }, einem Testbuild von Star Citizen. Material aus Testbuilds ist nach den Nutzungsbedingungen von Star Citizen vertraulich: Gib nichts daraus weiter und veröffentliche nichts davon.
action-reload = Neu laden
extract-selected = Ausgewählte extrahieren …
extract-shown = Angezeigte extrahieren …
extract-all = Alle extrahieren …
extraction-setup-title = Einträge extrahieren
extraction-scope-heading = Einträge
extraction-scope-selected = Ausgewählte ({ $number })
extraction-scope-shown = Angezeigte ({ $number })
extraction-scope-all = Ganzes Archiv ({ $number })
extraction-target-heading = Ziel
extraction-target-new = Neuer Ordner in
extraction-target-existing = Vorhandener Ordner
extraction-target-new-name = Legt { $name } an
extraction-target-in-installation = Dieser Ordner liegt in der Star-Citizen-Installation { $installation }. Das Werkzeug schreibt dort nie; wähle einen Ordner an anderer Stelle.
extraction-folder-none = Kein Ordner gewählt
extraction-choose-folder = Ordner wählen …
extraction-policy-heading = Bereits vorhandene Dateien
extraction-policy-skip = Überspringen
extraction-policy-overwrite = Überschreiben
extraction-policy-if-changed = Nur bei Änderung überschreiben
extraction-conversion = CryXml und DataForge in XML, Texturen und Sounds umwandeln
extraction-texture-merge = Geteilte Texturen zu einer DDS zusammenführen
extraction-texture-png = Texturen als PNG exportieren (HDR als OpenEXR)
extraction-texture-mip-largest = Größte Mip-Stufe
extraction-texture-mip-level = Mip-Stufe
extraction-texture-raw = Rohe Texturteile behalten
extraction-audio-label = Sounds als
extraction-audio-ogg = Ogg
extraction-audio-wav = WAV
extraction-audio-raw = WEM, unverändert
extraction-audio-banks = Ohne Ogg-Form des Codecs wird statt Ogg WAV geschrieben. Soundbanks werden zusätzlich in einen Ordner je Bank entpackt, jede WEM nach ihrer ID benannt.
extraction-start = Extrahieren
extraction-close = Schließen
extraction-confirm-all =
    { $count ->
        [one] Das ganze Archiv extrahieren, { $number } Eintrag mit { $size }?
       *[other] Das ganze Archiv extrahieren, { $number } Einträge mit { $size }?
    }
extraction-confirm-all-action = Ganzes Archiv extrahieren
extraction-back = Zurück
extraction-running-title = Extrahiere
extraction-progress = { $done } von { $total } Einträgen, { $percent } %
extraction-cancelling = Breche ab …
extraction-cancel = Abbrechen
extraction-summary-title = Extraktion abgeschlossen
extraction-summary-cancelled-title = Extraktion abgebrochen
extraction-summary-changed-title = Extraktion gestoppt: Das Archiv hat sich geändert
extraction-summary-changed = Das Archiv hat sich während der Extraktion auf der Festplatte geändert. Bereits geschriebene Dateien können veraltet sein; lade das Archiv neu und extrahiere erneut.
extraction-summary-counts = Extrahiert { $extracted }, übersprungen { $skipped }, fehlgeschlagen { $failed }, abgebrochen { $cancelled }
extraction-summary-counts-converted = Extrahiert { $extracted } ({ $converted } umgewandelt), übersprungen { $skipped }, fehlgeschlagen { $failed }, abgebrochen { $cancelled }
extraction-summary-records = Records geschrieben { $written }, übersprungen { $skipped }, fehlgeschlagen { $failed }, abgebrochen { $cancelled }; { $errors } Fehlerwerte in { $erroneous } Records
extraction-summary-target = Ziel: { $path }
extraction-summary-failed = Fehlgeschlagen ({ $number })
extraction-summary-skipped = Übersprungen ({ $number })
extraction-summary-cancelled = Abgebrochen ({ $number })
extraction-summary-unconverted = Nicht umgewandelt ({ $number })
extraction-summary-failure = { $path }: { $reason }
extraction-summary-records-failed = Nicht geschriebene Records ({ $number })
extraction-summary-record-failure = { $path } ({ $guid }): { $reason }
extraction-failed = Das Ziel in { $path } lässt sich nicht anlegen: { $reason }
extraction-stopped = Die Extraktion nach { $path } brach ohne Ergebnis ab.
extraction-failed-title = Extraktion fehlgeschlagen
comparison-tab-archive = Durchsuchen
comparison-tab-comparison = Vergleich
comparison-old-archive = Altes Archiv
comparison-new-archive = Neues Archiv
comparison-pick-archive = Archiv wählen
comparison-choose-file = Datei wählen …
comparison-levels = Ebenen
comparison-level-entries = Einträge
comparison-level-records = Records
comparison-entries-skipped = Die Einträge wurden nicht verglichen; schalte Einträge ein, um sie aufzulisten.
comparison-start = Vergleichen
comparison-cancel = Abbrechen
comparison-opening = Beide Archive werden geöffnet …
comparison-progress = { $done } von { $total } Einträgen, { $percent } %
comparison-preparing = Die Liste wird vorbereitet …
comparison-cancelled = Der Vergleich wurde abgebrochen.
comparison-failed = Der Vergleich ist fehlgeschlagen: { $reason }
comparison-stopped = Der Vergleich endete ohne Ergebnis.
comparison-empty-title = Noch kein Vergleich
comparison-empty-hint = Wähle ein altes und ein neues Archiv und vergleiche sie.
comparison-fact-old = Alt
comparison-fact-new = Neu
comparison-side = { $channel } · { $build }
comparison-channel-unknown = Kein Channel
comparison-added = Hinzugefügt
comparison-removed = Entfernt
comparison-changed = Geändert
comparison-directories = Nach Verzeichnis ({ $number })
comparison-root = (Wurzel)
comparison-column-directory = Verzeichnis
comparison-column-change = Änderung
comparison-column-path = Pfad
comparison-column-old-size = Alte Größe
comparison-column-new-size = Neue Größe
comparison-no-size = –
comparison-save-report = Bericht speichern …
comparison-report-filter = JSON-Bericht
comparison-report-saving = Der Bericht wird in { $path } gespeichert …
comparison-report-saved = Bericht gespeichert in { $path }
comparison-report-failed = Der Bericht kann nicht in { $path } gespeichert werden: { $reason }
comparison-no-differences = Kein Eintrag unterscheidet sich zwischen den Archiven.
comparison-no-matches = Keine Änderung passt zum Filter.
comparison-shown = { $number } angezeigt
comparison-row = { $change }: { $path }

schema-tab = Schema
schema-label-structs = Structs
schema-label-properties = Eigenschaften
schema-label-enums = Enums
schema-label-options = Enum-Optionen
schema-counts = { $label }: { $added } hinzugefügt, { $removed } entfernt, { $changed } geändert
schema-column-kind = Art
schema-column-name = Name
schema-column-old = Alt
schema-column-new = Neu
schema-column-old-records = Records alt
schema-column-new-records = Records neu
schema-kind-struct = Struct
schema-kind-parent = Basis-Struct
schema-kind-property = Eigenschaft
schema-kind-enum = Enum
schema-kind-option = Option
schema-no-differences = Das Schema unterscheidet sich zwischen den DataForges nicht.
schema-failed = Die Schemas konnten nicht verglichen werden: { $reason }
schema-row ={ $change }: { $kind } { $name }

diff-previous = Vorheriger Unterschied
diff-next = Nächster Unterschied
diff-position = Unterschied { $current } von { $total }
diff-head-old = Alt
diff-head-new = Neu
diff-hunk = Zeilen { $first }–{ $last }
diff-hunk-empty = Keine Zeilen
diff-loading = Zeilenvergleich von { $path } wird gelesen …
diff-binary = { $path } ist binär, daher gibt es keinen Zeilenvergleich.
diff-too-large = { $path } ist zu groß für einen Zeilenvergleich: Eine Seite hat mehr als { $size } oder { $lines } Zeilen.
diff-failed = { $path } lässt sich nicht vergleichen: { $reason }
diff-stopped = Der Zeilenvergleich von { $path } endete ohne Ergebnis.
diff-equal = Die Zeilen beider Seiten sind gleich; nur die gespeicherten Bytes unterscheiden sich.
diff-line-context = Zeile { $line }: { $text }
diff-line-removed = Entfernte Zeile { $line }: { $text }
diff-line-added = Hinzugefügte Zeile { $line }: { $text }
diff-marker-removed = −
diff-marker-added = +
query-tab = Abfrage
query-label = Abfrage
query-hint = from EntityClassDefinition at "*/spaceships/*" select Category
query-run = Abfrage ausführen
query-cancel = Abbrechen
query-loading = DataForge wird gelesen …
query-indexing = Referenzen werden indiziert …
query-selecting = Records werden ausgewählt …
query-progress = { $done } von { $total } Records, { $percent } %
query-cancelled = Die Abfrage wurde abgebrochen.
query-failed = Die Abfrage ist fehlgeschlagen: { $reason }
query-stopped = Die Abfrage wurde ohne Ergebnis beendet.
query-archive-changed = Das Archiv wurde auf der Festplatte geändert. Laden Sie es neu, um Abfragen auszuführen.
query-invalid = Zeile { $line }, Spalte { $column }: { $reason }
query-empty-title = Kein Archiv geöffnet
query-empty-hint = Öffnen Sie ein Archiv, um Abfragen über sein DataForge auszuführen.
query-prompt = Schreiben Sie eine Abfrage und führen Sie sie mit Strg+Enter aus, z. B. from AmmoParams select speed where speed >= 1500
query-no-rows = Kein Record passt zur Abfrage.
query-rows =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
query-rows-failed =
    { $count ->
        [one] { $number } Record, { $failed } fehlgeschlagen
       *[other] { $number } Records, { $failed } fehlgeschlagen
    }
query-unresolved =
    { $count ->
        [one] , { $number } nicht aufgelöste Referenz
       *[other] , { $number } nicht aufgelöste Referenzen
    }
query-export-csv = CSV exportieren …
query-export-json = JSON exportieren …
query-filter-csv = CSV-Datei
query-filter-json = JSON-Datei
query-exporting = Das Ergebnis wird nach { $path } exportiert …
query-exported =
    { $count ->
        [one] { $number } Record nach { $path } exportiert
       *[other] { $number } Records nach { $path } exportiert
    }
query-export-failed = Das Ergebnis kann nicht nach { $path } exportiert werden: { $reason }
query-export-cancel = Export abbrechen
query-export-cancelled = Der Export nach { $path } wurde abgebrochen.
query-page = Seite { $page } von { $pages }
query-page-first = Erste Seite
query-page-previous = Vorherige Seite
query-page-next = Nächste Seite
query-page-last = Letzte Seite
query-completion-more =
    { $count ->
        [one] { $number } weiterer Name, weiter tippen
       *[other] { $number } weitere Namen, weiter tippen
    }
query-save = Abfrage speichern …
query-saved-title = Gespeicherte Abfragen
query-saved-none = Noch keine gespeicherten Abfragen. Speichern Sie die Abfrage im Editor, um sie mit einem Klick erneut auszuführen.
query-saved-run = Gespeicherte Abfrage { $name } ausführen
query-saved-rename = Umbenennen
query-saved-delete = Löschen
query-save-title = Abfrage speichern
query-rename-title = Gespeicherte Abfrage umbenennen
query-delete-title = Gespeicherte Abfrage löschen
query-name-label = Name
query-name-hint = z. B. Schwere Schiffe
query-save-action = Speichern
query-rename-action = Umbenennen
query-delete-action = Löschen
query-delete-question = Die gespeicherte Abfrage „{ $name }“ wird gelöscht. Die Abfrage im Editor bleibt.
query-name-replaces = Ersetzt die gespeicherte Abfrage dieses Namens.
query-name-empty = Geben Sie einen Namen ein.
query-name-too-long = Der Name ist länger als { $max } Zeichen.
query-name-taken = Eine andere gespeicherte Abfrage hat diesen Namen.
query-name-control = Der Name darf keine Zeilenumbrüche oder anderen Steuerzeichen enthalten.
query-name-full = Die gespeicherten Abfragen sind voll: höchstens { $max } Abfragen und { $size } insgesamt. Löschen Sie zuerst eine.
query-name-bad-text = Die Abfrage im Editor ist leer oder zu lang zum Speichern.

dataforge-node = DataForge
dataforge-node-loading = Wird geladen …
preview-tab-dataforge = DataForge
dataforge-loading = DataForge wird aus { $path } geladen …
dataforge-unsupported = DataForge-Version { $version } wird von dieser Version von P4KX nicht unterstützt.
dataforge-failed = DataForge lässt sich nicht aus { $path } laden: { $reason }
dataforge-stopped = Das Laden von DataForge aus { $path } brach ohne Ergebnis ab.
dataforge-archive-changed = Das Archiv hat sich auf der Festplatte geändert. Lade es neu, um DataForge zu laden.
dataforge-version = Version
dataforge-records = Records
record-search-label = Records suchen
record-search-hint = Nach Record-Pfad oder Typ filtern: Glob mit * oder ?, sonst ein beliebiger Teil
record-search-prompt = Gib einen Teil eines Record-Pfads oder Typnamens ein, um Records zu finden.
record-search-no-matches = Kein Record passt zur Suche.
record-search-matches =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
dataforge-structs = Structs
dataforge-enums = Enums
dataforge-first-records = Erste Records
column-type = Typ
column-record-path = Record-Pfad
record-change-progress = { $done } von { $total } Records, { $percent } %
record-change-moved = Verschoben
record-change-unreadable = Unlesbar
record-change-moved-path = { $old } zu { $new }
record-change-no-differences = Kein Record unterscheidet sich zwischen den DataForges.
record-change-head-field = Feld
record-change-field-count =
    { $count ->
        [one] { $number } Änderung
       *[other] { $number } Änderungen
    }
record-change-loading = Die Felder von { $path } werden verglichen …
record-change-failed = Die Felder von { $path } lassen sich nicht vergleichen: { $reason }
record-change-stopped = Der Feldvergleich von { $path } endete ohne Ergebnis.
record-change-only-new = Den Record gibt es nur im neuen Archiv.
record-change-only-old = Den Record gibt es nur im alten Archiv.
record-change-only-path = Nur der Record-Pfad hat sich geändert; die Felder sind gleich.
record-change-no-field-changes = Kein Feld unterscheidet sich.
record-change-unreadable-old = Der alte Record lässt sich nicht lesen: { $reason }
record-change-unreadable-new = Der neue Record lässt sich nicht lesen: { $reason }
record-change-record-name = Record-Name
record-change-record-type = Record-Typ
record-change-value-missing = –
record-change-value-null = null
record-change-value-struct = { $type } (Struct)
record-change-value-array =
    { $count ->
        [one] { $number } Element
       *[other] { $number } Elemente
    }
record-change-value-reference-at = { $path } · { $at }
record-change-value-error = Fehler: { $reason }
record-change-field-row = { $field }: { $old } zu { $new }
change-extraction-action-old = Alt extrahieren ({ $number }) …
change-extraction-action-new = Neu extrahieren ({ $number }) …
change-extraction-scope-old = Geänderte Einträge des alten Archivs, in ihrer alten Fassung ({ $number })
change-extraction-scope-new = Hinzugefügte und geänderte Einträge des neuen Archivs ({ $number })
change-extraction-scope-filtered = Nur die Änderungen, die zum Filter { $filter } passen
change-extraction-summary-changed = Das Archiv hat sich während der Extraktion auf der Festplatte geändert. Bereits geschriebene Dateien können veraltet sein; vergleiche die Archive erneut und extrahiere die Änderungen noch einmal.

preview-tab-image = Bild
image-mip = Mip
image-format = Format
image-size = Größe
image-mips = Mips
image-faces = Cubemap-Seiten
image-dimensions = { $width } × { $height }
image-none = –
image-shown = Mip { $level }: { $width } × { $height } Pixel
image-decoding = Mip { $level } von { $path } wird dekodiert …
image-unsupported = Die Vorschau kann Texturen im Format { $format } nicht dekodieren. Die Tabs Text und Hex zeigen die Daten von { $path } weiterhin.
image-failed = { $path } kann nicht dekodiert werden: { $reason }
image-unshowable = Jedes Mip von { $path } ist größer, als die Vorschau zeigen kann.
container-opening = Wird geöffnet …
container-unreadable = Lässt sich nicht öffnen
container-unreadable-detail = { $name } lässt sich nicht öffnen: { $reason }
container-over-budget = Nicht geöffnet: Speichergrenze
container-over-budget-detail = { $name } wurde nicht geöffnet: Die offenen Container belegen bereits { $limit }. Klappe andere Container zu und klappe ihn erneut auf.
container-stopped = Öffnen abgebrochen
container-stopped-detail = Das Öffnen von { $name } brach ohne Ergebnis ab. Klappe ihn erneut auf, um es noch einmal zu versuchen.
container-archive-changed = Archiv geändert
container-archive-changed-detail = Das Archiv hat sich auf der Festplatte geändert. Lade es neu, um { $name } zu öffnen.
container-status-entries =
    { $count ->
        [one] Container: { $number } Eintrag
       *[other] Container: { $number } Einträge
    }

menu-export = Export
record-export-action = Record-Export …
record-export-title = Records exportieren
record-export-scope-heading = Records
record-export-scope-tree = Im Record-Baum ausgewählt: { $name } ({ $number })
record-export-scope-tree-none = Im Record-Baum ausgewählt (nichts ausgewählt)
record-export-scope-filter = Filter nach Record-Pfad
record-export-filter-hint = Glob mit * oder ?, sonst ein beliebiger Teil des Record-Pfads
record-export-form-heading = Form
record-export-form-json = JSON, eine Datei je Record unter seinem Record-Pfad
record-export-form-ndjson = NDJSON, eine Datei { $name } mit einem Record je Zeile
record-export-start = Export starten
record-export-running-title = Exportiere Records
record-export-reading = Lese DataForge …
record-export-selecting = Wähle Records aus …
record-export-progress = { $done } von { $total } Records, { $percent } %
record-export-summary-title = Record-Export abgeschlossen
record-export-summary-cancelled-title = Record-Export abgebrochen
record-export-summary-cancelled = Der Record-Export wurde abgebrochen, bevor alle Records geschrieben waren.
record-export-summary-counts = Records geschrieben { $written }, übersprungen { $skipped }, fehlgeschlagen { $failed }, abgebrochen { $cancelled }
record-export-summary-errors = Records mit Fehlerwerten: { $erroneous }, Fehlerwerte: { $errors }
record-export-summary-no-match = Kein Record passt zum Filter.
record-export-summary-target = Ziel: { $path }
record-export-summary-file-failed = { $path } lässt sich nicht schreiben: { $reason }
record-export-summary-failed = Fehlgeschlagen ({ $number })
record-export-summary-skipped = Übersprungen ({ $number })
record-export-summary-failure = { $path } ({ $guid }): { $reason }
record-export-failed-title = Record-Export fehlgeschlagen
record-export-failed = Der Record-Export nach { $path } ist fehlgeschlagen: { $reason }
record-export-stopped = Der Record-Export nach { $path } brach ohne Ergebnis ab.

catalog-action = Basetool-Katalog …
catalog-file-filter = Basetool-Katalog (JSON)
catalog-file-filter-compressed = Basetool-Katalog (JSON, zstd-komprimiert)
catalog-running-title = Erstelle Basetool-Katalog
catalog-reading = Lese DataForge …
catalog-progress = { $done } von { $total } Records geprüft, { $percent } %
catalog-writing = Schreibe { $path } …
catalog-summary-title = Basetool-Katalog erstellt
catalog-kinds-heading = Arten
catalog-summary-kind = { $kind ->
    [manufacturers] Hersteller
    [items] Items
    [ships] Schiffe
    [commodities] Waren
    [blueprints] Blaupausen
    [resourceGroups] Ressourcengruppen
    [blueprintCategories] Blaupausenkategorien
    [paints] Lackierungen
   *[other] { $kind }
}: { $written } geschrieben, { $skipped } übersprungen, { $complete ->
    [yes] vollständig
   *[no] teilweise
}
catalog-summary-skipped = Als Entwicklungsreste übersprungene Records: { $number }
catalog-summary-failed = Nicht lesbare und weggelassene Records ({ $number })
catalog-summary-failure = { $path } ({ $guid }): { $reason }
catalog-texts-heading = Texte
catalog-summary-texts = { $language ->
    [en] Englisch
    [de] Deutsch
   *[other] { $language }
}: { $resolved } aufgelöst, { $missing } fehlen, { $placeholders } Platzhalter
catalog-summary-texts-unavailable = { $language ->
    [en] Englisch
    [de] Deutsch
   *[other] { $language }
}: das Archiv enthält keine global.ini, { $missing } fehlen
catalog-summary-missing = Fehlende Texte: { $number }
catalog-summary-size = Größe: { $size } vom Upload-Limit { $limit }
catalog-summary-size-compressed = Größe: { $compressed } komprimiert, { $size } unkomprimiert, vom Upload-Limit { $limit }
catalog-near-limit = Der Katalog liegt nahe am Upload-Limit des Basetools von { $limit }.
catalog-over-limit = Der Katalog liegt über dem Upload-Limit des Basetools von { $limit }; das Basetool lehnt den Upload ab.
catalog-near-limit-plain = Der JSON-Katalog liegt nahe am Upload-Limit des Basetools von { $limit }. Schreibe den Katalog für den Upload als .json.zst; reines .json dient der Ansicht.
catalog-over-limit-plain = Der JSON-Katalog liegt über dem Upload-Limit des Basetools von { $limit }; das Basetool lehnt ihn ab. Schreibe den Katalog für den Upload als .json.zst; reines .json dient der Ansicht.
catalog-decompressed-near-limit = Der entpackte Katalog liegt nahe am Limit des Basetools von { $limit } für entpackte Inhalte.
catalog-decompressed-over-limit = Der entpackte Katalog liegt über dem Limit des Basetools von { $limit } für entpackte Inhalte; das Basetool lehnt ihn ab.
catalog-summary-file = Datei: { $path }
catalog-cancelled-title = Basetool-Katalog abgebrochen
catalog-cancelled = Der Basetool-Katalog wurde abgebrochen. Es wurde keine Datei geschrieben.
catalog-failed-title = Basetool-Katalog fehlgeschlagen
catalog-failed = Der Basetool-Katalog { $path } ließ sich nicht erstellen: { $reason }
catalog-stopped = Der Basetool-Katalog { $path } brach ohne Ergebnis ab.
catalog-output-is-archive = Der Basetool-Katalog kann nicht nach { $path } geschrieben werden, weil das das geöffnete Archiv ist. Eine andere Datei wählen.
preview-tab-xml = XML
xml-converting = { $path } wird in XML umgewandelt …
xml-too-large = { $path } ist { $size } groß. CryXml-Einträge über { $limit } werden in der Vorschau nicht umgewandelt; zum Lesen mit Umwandlung extrahieren.
xml-failed = { $path } kann nicht in XML umgewandelt werden: { $reason }
xml-stopped = Das Umwandeln von { $path } brach ohne Ergebnis ab.
xml-truncated = Gekürzt: angezeigt werden die ersten { $lines } Zeilen, die ersten { $limit } des XML.
xml-view = Umgewandeltes XML

scroll-text = Textzeilen
scroll-hex = Hexadezimalzeilen
scroll-image = Bildansicht
scroll-diff = Zeilenunterschiede
scroll-fields = Geänderte Felder
scroll-lines = Record-Felder
scroll-localization = Lokalisierungszeilen
scroll-saved-queries = Gespeicherte Abfragen
scroll-soundbank = Eingebettete WEMs

preview-tab-audio = Audio
audio-codec = Codec
audio-channels = Kanäle
audio-sample-rate = Abtastrate
audio-duration = Dauer
audio-hertz = { $rate } Hz
audio-unknown = –
audio-play = Abspielen
audio-pause = Pause
audio-position = Position
audio-volume = Lautstärke
audio-time = { $position } / { $duration }
audio-decoding = { $path } wird dekodiert …
audio-unsupported = Die Vorschau kann Töne im Codec { $codec } nicht abspielen. Die Tabs Text und Hex zeigen die Daten von { $path } weiterhin.
audio-too-long = { $path } ist zu lang für die Vorschau: der dekodierte Ton wäre größer als { $limit }.
audio-failed = { $path } kann nicht dekodiert werden: { $reason }
audio-output-failed = Der Ton kann nicht abgespielt werden: { $reason }
audio-column-id = ID
audio-prefetched = Vorgeladen
audio-prefetched-note = WEM { $id } ist in dieser Soundbank nur vorgeladen und kann hier nicht abgespielt werden. Der ganze Ton steht im losen WEM-Eintrag mit derselben ID.
audio-plugin = Plug-in
audio-plugin-note = WEM { $id } ist ein Plug-in-Medium, etwa ein Faltungshall-Impuls, und enthält keinen Ton zum Abspielen.
audio-bank-summary =
    { $size } · { $count ->
        [one] { $number } WEM
       *[other] { $number } WEMs
    }
audio-bank-empty = { $path } enthält keine WEMs.
audio-bank-failed = Die Soundbank { $path } kann nicht gelesen werden: { $reason }
audio-wem-name = WEM { $id } aus { $bank }
audio-wem-row = WEM { $id }, { $codec }, { $duration }
audio-unsupported-wem = Die Vorschau kann Töne im Codec { $codec } nicht abspielen ({ $name }).

texture-parts =
    { $count ->
        [one] +{ $number } Teil
       *[other] +{ $number } Teile
    }
texture-expand = Teile von { $name } anzeigen
texture-collapse = Teile von { $name } ausblenden

preview-tab-record = Record
record-back = Zurück
record-forward = Vor
record-type = Typ
record-guid = GUID
record-name = Name
record-head-value = Wert
record-loading = Der Record { $path } wird gelesen …
record-failed = Der Record { $path } kann nicht gelesen werden: { $reason }
record-stopped = Das Lesen des Records { $path } brach ohne Ergebnis ab.
record-archive-changed = Das Archiv wurde auf der Festplatte geändert. Neu laden, um den Record zu lesen.
record-dangling = Die Referenz zeigt auf den Record { $guid }, den es in diesem DataForge nicht gibt.
record-follow = Der Referenz auf { $path } folgen
record-row = { $field }: { $value }
record-no-fields = Der Record hat keine Felder.
record-xml-failed = Das Record-XML von { $path } kann nicht geschrieben werden: { $reason }

record-language-menu = Record-Texte
record-language-keys-only = Nur Schlüssel
record-language-unavailable = Das Archiv enthält keine Texte auf { $language }, deshalb stehen nur die Schlüssel da.
record-language-key-row = { $key }: { $text }
record-language-key-row-missing = { $key } (kein Text in dieser Sprache)
record-language-load-failed = Die Texte auf { $language } können nicht gelesen werden, deshalb stehen nur die Schlüssel da: { $reason }
localization-tab = Lokalisierung
localization-label = Suche
localization-hint = Schlüssel oder Text, z. B. item_Name oder Aurora
localization-loading = Die Lokalisierungsdateien werden gelesen …
localization-no-texts = Das Archiv enthält keine global.ini.
localization-failed = Die Lokalisierungsdateien können nicht gelesen werden: { $reason }
localization-stopped = Das Lesen der Lokalisierungsdateien wurde ohne Ergebnis beendet.
localization-archive-changed = Das Archiv hat sich auf dem Datenträger geändert. Laden Sie es neu, um die Lokalisierungsdateien zu lesen.
localization-empty-title = Kein Archiv geöffnet
localization-empty-hint = Öffnen Sie ein Archiv, um seine Lokalisierungsschlüssel und Texte nachzuschlagen.
localization-column-key = Schlüssel
localization-language-en = Englisch
localization-language-de = Deutsch
localization-language-fr = Französisch
localization-language-it = Italienisch
localization-language-es-ES = Spanisch (Spanien)
localization-language-es-419 = Spanisch (Lateinamerika)
localization-language-pt-BR = Portugiesisch (Brasilien)
localization-language-ja = Japanisch
localization-language-ko = Koreanisch
localization-language-zh-Hans = Chinesisch (vereinfacht)
localization-language-zh-Hant = Chinesisch (traditionell)
localization-language-other = Andere Sprache
localization-missing = Fehlt
localization-keys =
    { $count ->
        [one] { $number } Schlüssel
       *[other] { $number } Schlüssel
    }
localization-keys-shown =
    { $count ->
        [one] { $number } von { $total } Schlüsseln
       *[other] { $number } von { $total } Schlüsseln
    }
localization-searching = Suche läuft …
localization-no-match = Kein Schlüssel und kein Text passt zur Suche.
localization-copy-key = Schlüssel kopieren
localization-copy-text = Text kopieren
localization-notes = Einige global.ini-Dateien wurden nicht sauber gelesen:
tags-tab = Tags
tags-label = Suche
tags-hint = Tag-Pfad, z. B. Weapon.Gun
tags-loading = Die Tags werden gelesen …
tags-no-tags = Das DataForge des Archivs enthält keine Tags.
tags-failed = Die Tags können nicht gelesen werden: { $reason }
tags-stopped = Das Lesen der Tags wurde ohne Ergebnis beendet.
tags-archive-changed = Das Archiv hat sich auf dem Datenträger geändert. Laden Sie es neu, um die Tags zu lesen.
tags-empty-title = Kein Archiv geöffnet
tags-empty-hint = Öffnen Sie ein Archiv, um seinen Tag-Baum zu durchsuchen.
tags-column-tag = Tag
tags-column-guid = GUID
tags-column-legacy = Legacy-ID
tags-count =
    { $count ->
        [one] { $number } Tag
       *[other] { $number } Tags
    }
tags-count-shown =
    { $count ->
        [one] { $number } von { $total } Tags
       *[other] { $number } von { $total } Tags
    }
tags-no-match = Kein Tag-Pfad passt zur Suche.
tags-copy-path = Tag-Pfad kopieren
tags-copy-guid = GUID kopieren
sounds-tab = Sounds
sounds-entity-label = Entity
sounds-entity-hint = Klassenname, GUID oder Record-Pfad einer Entity
sounds-show = Sounds anzeigen
sounds-use-record = Ausgewählten Record verwenden
sounds-search-label = Suche
sounds-search-hint = Name eines Triggers oder Events, * und ? erlaubt
sounds-export = Sounds exportieren …
sounds-empty-title = Kein Archiv geöffnet
sounds-empty-hint = Öffnen Sie ein Archiv, um die Sounds seiner Entities zu durchsuchen.
sounds-loading = Die Audiodaten des Archivs werden gelesen …
sounds-failed = Die Audiodaten des Archivs können nicht gelesen werden: { $reason }
sounds-stopped = Das Lesen der Audiodaten endete ohne Ergebnis.
sounds-archive-changed = Das Archiv hat sich auf dem Datenträger geändert. Laden Sie es neu, um seine Audiodaten zu lesen.
sounds-issues =
    { $count ->
        [one] { $number } Audiodatei konnte nicht gelesen werden; die Trigger und Sounds darin fehlen.
       *[other] { $number } Audiodateien konnten nicht gelesen werden; die Trigger und Sounds darin fehlen.
    }
sounds-list-hint = Zeigen Sie eine Entity an oder suchen Sie nach einem Trigger oder Event, um Audio-Trigger aufzulisten.
sounds-list-loading = Die Audio-Trigger werden aufgelistet …
sounds-not-an-entity = Der ausgewählte Record ist keine Entity.
sounds-list-failed = Die Audio-Trigger können nicht aufgelistet werden: { $reason }
sounds-list-archive-changed = Das Archiv hat sich auf dem Datenträger geändert. Laden Sie es neu, um die Audio-Trigger aufzulisten.
sounds-no-match = Kein Trigger- oder Event-Name passt zur Suche.
sounds-no-triggers = Die Entity nennt keinen Audio-Trigger.
sounds-column-trigger = Trigger
sounds-column-state = Status
sounds-column-events = Wwise-Events
sounds-state-ok = Aufgelöst
sounds-state-no-definition = Ohne Definition
sounds-state-no-event = Ohne Event
sounds-state-no-soundbank = Ohne Soundbank
sounds-count-entity =
    { $count ->
        [one] { $name }: { $number } Audio-Trigger, { $resolved } mit Soundbank
       *[other] { $name }: { $number } Audio-Trigger, { $resolved } mit Soundbank
    }
sounds-count-search =
    { $count ->
        [one] { $number } Trigger
       *[other] { $number } Trigger
    }
sounds-count-search-more = Gezeigt werden die ersten { $number } von { $total } Triggern. Ein längerer Text grenzt die Suche ein.
sounds-scroll-triggers = Audio-Trigger
sounds-scroll-variants = Sounds des Triggers
sounds-select-hint = Wählen Sie einen Trigger, um seine Sounds zu sehen.
sounds-variants-loading = Die Sounds werden aufgelöst …
sounds-undefined = Keine ATL-Definition hat diesen Trigger.
sounds-no-variants = Der Trigger führt zu keinem Sound.
sounds-variants-count =
    { $count ->
        [one] { $name }: { $number } Sound
       *[other] { $name }: { $number } Sounds
    }
sounds-breaks-heading =
    { $count ->
        [one] { $number } Stelle, an der der Weg zu einem Sound abbricht
       *[other] { $number } Stellen, an denen der Weg zu einem Sound abbricht
    }
sounds-column-event = Wwise-Event
sounds-column-wem = WEM-ID
sounds-column-source = Quelle
sounds-column-language = Sprache
sounds-column-soundbank = Soundbank
sounds-source-embedded = Eingebettet
sounds-source-prefetch = Vorgeladen
sounds-source-streamed = Gestreamt
sounds-source-plugin = Plug-in
sounds-source-other = Andere
sounds-language-fallback = { $language } (Ersatz)
sounds-variant-row = { $name }: { $bank }, WEM { $id }
sounds-detail = { $path }, WEM { $id }, { $source }
sounds-select-variant = Wählen Sie einen Sound, um ihn abzuspielen.
sounds-note-prefetch = Von diesem gestreamten Sound ist nur der Anfang eingebettet und der zugehörige WEM-Eintrag fehlt, deshalb lässt er sich nicht abspielen.
sounds-note-plugin = Ein Quell-Plug-in erzeugt diesen Sound; kein WEM enthält ihn.
sounds-note-missing = Weder eine Soundbank noch ein loser WEM-Eintrag enthält dieses WEM.
sounds-export-title = Sounds exportieren
sounds-export-scope = Trigger
sounds-export-selected = Der ausgewählte Trigger: { $name }
sounds-export-selected-none = Der ausgewählte Trigger (keiner ausgewählt)
sounds-export-all =
    { $count ->
        [one] Der Trigger der Liste mit Soundbank
       *[other] Alle { $number } Trigger der Liste mit Soundbank
    }
sounds-export-format = Sounds als
sounds-export-start = Exportieren
sounds-export-running = Sounds werden exportiert
sounds-export-planning = Der Export wird geplant …
sounds-export-progress = { $done } von { $total } Sounds, { $percent } %
sounds-summary-title = Sounds exportiert
sounds-summary-cancelled-title = Sound-Export abgebrochen
sounds-failed-title = Sound-Export fehlgeschlagen
sounds-export-failed = Die Sounds können nicht nach { $path } exportiert werden: { $reason }
sounds-export-stopped = Der Export der Sounds nach { $path } endete ohne Ergebnis.
sounds-summary-cancelled = Der Export wurde abgebrochen. Die noch nicht bearbeiteten Sounds wurden nicht geschrieben.
sounds-summary-counts = Sounds geschrieben { $written }, behalten { $skipped }, fehlgeschlagen { $failed }
sounds-summary-target = Ziel: { $path }
sounds-summary-unconverted =
    { $count ->
        [one] { $number } Sound wurde unverändert geschrieben, weil er sich nicht umwandeln lässt
       *[other] { $number } Sounds wurden unverändert geschrieben, weil sie sich nicht umwandeln lassen
    }
sounds-summary-failed =
    { $count ->
        [one] { $number } Sound wurde nicht exportiert
       *[other] { $number } Sounds wurden nicht exportiert
    }
sounds-summary-file = { $name }: { $reason }
sounds-summary-failure = { $name }, WEM { $id }: { $reason }
sounds-summary-failure-plain = { $name }: { $reason }
tags-irregular =
    { $count ->
        [one] Der Tag-Baum weicht an { $number } Stelle von dem ab, was die TagDatabase auflistet. Der Befehl p4kx dcb tags zählt sie.
       *[other] Der Tag-Baum weicht an { $number } Stellen von dem ab, was die TagDatabase auflistet. Der Befehl p4kx dcb tags zählt sie.
    }
preview-tab-model = Modell
model-reset = Ansicht zurücksetzen
model-hint = Ziehen: drehen · Rechtsklick-Ziehen oder Umschalt-Ziehen: verschieben · Mausrad: zoomen · Pfeile: drehen, Umschalt und Pfeile: verschieben · + und -: zoomen · Pos1: zurücksetzen
model-vertices = Vertices
model-triangles = Dreiecke
model-extent = Größe
model-dimensions = { $width } × { $depth } × { $height } m
model-view = 3D-Ansicht von { $path }: { $vertices } Vertices, { $triangles } Dreiecke
model-loading = { $path } wird gelesen …
model-failed = { $path } kann nicht gelesen werden: { $reason }
model-file-too-large = { $path } oder seine Geometriedatei ist { $size } groß, mehr als die { $limit }, die der Viewer liest.
model-empty = { $path } hat keine Dreiecke zum Anzeigen.
model-too-large = { $path } hat { $vertices } Vertices und { $triangles } Dreiecke, mehr als der Viewer zeigt: { $max_vertices } Vertices und { $max_triangles } Dreiecke.
model-not-finite = { $path } hat keine brauchbaren Koordinaten.
model-gpu-none = Das Fenster hat kein Grafikgerät für den 3D-Viewer. Alles andere funktioniert wie gewohnt.
model-gpu-texture-side = Das Grafikgerät nimmt Texturen mit höchstens { $limit } Pixeln Kantenlänge, zu wenig für den 3D-Viewer. Alles andere funktioniert wie gewohnt.
model-gpu-buffer-size = Das Grafikgerät nimmt Puffer von höchstens { $limit } Byte, zu wenig für den 3D-Viewer. Alles andere funktioniert wie gewohnt.
model-gpu-setup = Das Grafikgerät hat den 3D-Viewer abgelehnt: { $reason } Alles andere funktioniert wie gewohnt.
model-gpu-lost = Das Grafikgerät ging verloren. Starte P4KX neu, um wieder Modelle zu sehen.
model-gpu-failed = Das Grafikgerät konnte das Modell nicht zeichnen. Wähle ein anderes Modell oder starte P4KX neu.
model-gpu-rejected = { $path } ist zu groß für das Grafikgerät.
model-skeleton = Skelett
model-loop = Wiederholen
model-play = Abspielen
model-pause = Pause
model-position = Position in der Animation
model-time = { $position } / { $duration }
model-animations = Animationen
model-bind-pose = Ausgangspose


preview-tab-incoming = Referenziert von
scroll-incoming = Verweisende Records
incoming-head-field = Feld
incoming-head-record = Verweisender Record
incoming-loading = Die Records, die auf { $path } verweisen, werden gesucht …
incoming-failed = Die Records, die auf { $path } verweisen, lassen sich nicht finden: { $reason }
incoming-stopped = Die Suche nach den Records, die auf { $path } verweisen, brach ohne Ergebnis ab.
incoming-archive-changed = Das Archiv wurde auf der Festplatte geändert. Neu laden, um die Records zu finden, die auf einen Record verweisen.
incoming-none = Kein Record verweist auf diesen Record.
incoming-follow = Den Record { $path } öffnen, der in { $field } auf diesen Record verweist
incoming-summary = { $references } von { $records }
incoming-reference-count =
    { $count ->
        [one] { $number } Referenz
       *[other] { $number } Referenzen
    }
incoming-record-count =
    { $count ->
        [one] { $number } Record
       *[other] { $number } Records
    }
incoming-truncated = Die ersten { $shown } von { $total } Referenzen sind aufgelistet.
incoming-incomplete = Einige Records ließen sich nicht lesen oder der Index ist voll, daher kann die Liste Referenzen auslassen.

preview-key-required = { $path } ist verschlüsselt. Gib den Archiv-Schlüssel ein, um es zu lesen.
preview-key-rejected = Der Archiv-Schlüssel entschlüsselt { $path } nicht. Gib den richtigen Schlüssel ein, um es zu lesen.
key-menu = Archiv-Schlüssel…
key-title = Archiv-Schlüssel
key-needed = Ein verschlüsselter Eintrag braucht den Schlüssel dieses Archivs. Gib ihn einmal ein; er wird als Datei in deinem Benutzerprofil gespeichert.
key-rejected = Der verwendete Schlüssel entschlüsselt dieses Archiv nicht. Gib den richtigen Schlüssel ein.
key-intro = Verschlüsselte Einträge lassen sich nur mit dem Schlüssel des Archivs lesen. Gib ihn einmal ein; er wird als Datei in deinem Benutzerprofil gespeichert.
key-label = Schlüssel
key-hint = 32 Hexadezimalziffern
key-show = Schlüssel anzeigen
key-stored = Ein Schlüssel ist gespeichert: { $masked }
key-not-stored = Es ist kein Schlüssel gespeichert.
key-save = Speichern
key-cancel = Abbrechen
key-remove = Gespeicherten Schlüssel entfernen
key-error-length = Ein Schlüssel hat 32 Hexadezimalziffern; eingegeben wurden { $digits }.
key-error-character = Zeichen { $position } ist keine Hexadezimalziffer.
key-error-wrong = Dieser Schlüssel entschlüsselt das Archiv nicht.
key-error-check = Der Schlüssel konnte nicht geprüft werden: { $reason }
key-error-store = Der Schlüssel konnte nicht gespeichert werden: { $reason }
key-error-remove = Der gespeicherte Schlüssel konnte nicht entfernt werden: { $reason }
key-checking = Der Schlüssel wird am Archiv geprüft…
key-note-variable = Die Umgebungsvariable P4KX_ARCHIVE_KEY enthält keinen gültigen Schlüssel und wird ignoriert.
key-note-store = Der gespeicherte Schlüssel kann nicht verwendet werden. Gib den Schlüssel erneut ein, um ihn zu ersetzen. ({ $reason })
about-title = Über P4KX
about-version = Version { $version }
about-copyright = Copyright (c) 2026 the P4KX contributors
about-licence = Lizenziert unter MIT OR Apache-2.0. Quellcode: https://github.com/greluc/P4KX
about-fan-project = P4KX ist ein inoffizielles Star-Citizen-Fanprojekt. Es steht in keiner Verbindung zur Cloud-Imperium-Gruppe und wird nicht von ihr unterstützt oder gebilligt.
about-no-game-data = Es liefert keine Spieldaten mit und liest nur deine eigene Star-Citizen-Installation.
about-responsibility = Du bist dafür verantwortlich, bei der Nutzung die Endbenutzer-Lizenzvereinbarung (EULA) und die Nutzungsbedingungen (Terms of Service) von Star Citizen einzuhalten.
about-owners = Alle Inhalte, die nicht von den Mitwirkenden von P4KX oder seinen Nutzern stammen, sind Eigentum ihrer jeweiligen Inhaber.
about-trademark = Star Citizen®, Roberts Space Industries® and Cloud Imperium® are registered trademarks of Cloud Imperium Rights LLC.
preview-tab-model-info = Info
model-info-geometry = Geometriedatei
model-info-geometry-in-file = In der Modelldatei
model-info-nodes = Knoten
model-info-meshes = Meshes
model-info-lods = Detailstufen
model-info-materials = Materialbibliothek
model-info-material-ids = Material-IDs
model-info-material-ids-none = Keine
model-info-bones = Knochen
model-info-animations = Animationen
model-info-animations-unread = Nicht gelesen
model-info-chunks = Chunks
model-info-chunks-value = { $total } ({ $read } gelesen, { $skipped } übersprungen, { $unknown } unbekannt)
model-info-materials-unnamed = Keine genannt
model-info-materials-loaded = { $library }, { $count ->
    [one] ein Teilmaterial
   *[other] { $number } Teilmaterialien
  }
model-info-materials-single = { $library }, ein Material für jede ID
model-info-materials-missing = { $library } (existiert nicht)
model-info-materials-unreadable = { $library } (nicht lesbar: { $reason })
model-info-lods-heading = Detailstufen
model-info-column-lod = LOD
model-info-column-vertices = Vertices
model-info-column-triangles = Dreiecke
model-info-column-meshes = Meshes
model-info-chunks-heading = Nicht gelesene Chunks ({ $number })
model-info-column-file = Datei
model-info-column-type = Typ
model-info-column-type-id = Typ-ID
model-info-column-usage = Verwendung
model-info-usage-skipped = Bekannt, nicht gelesen
model-info-usage-unknown = Unbekannt, übersprungen
model-info-file-model = Modelldatei
model-info-file-geometry = Geometriedatei
model-info-file-lod = Geometrie von LOD { $level }
model-info-type-unknown = Unbekannt
model-export-action = Als GLB exportieren …
model-export-title = Als GLB exportieren
model-export-model = Modell: { $name }
model-export-lod-heading = Detailstufe
model-export-lod = LOD { $level }, { $triangles ->
    [one] ein Dreieck
   *[other] { $number } Dreiecke
  }
model-export-materials-heading = Materialien
model-export-materials-colours = Einfache Farben, eine kleine und schnelle Datei
model-export-materials-textures = Mit Texturen, eine größere Datei
model-export-texture-mip-heading = Texturgröße
model-export-texture-mip = { $level ->
    [0] Volle Größe
   *[other] 1/{ $divisor } der Kantenlänge, Mip-Stufe { $level }
  }
model-export-animations-heading = Animationen
model-export-animations = { $count ->
    [one] Die Animation des Modells schreiben
   *[other] Die { $number } Animationen des Modells schreiben
  }
model-export-vertex-colours = Vertex-Farben schreiben
model-export-file-heading = Datei
model-export-choose-file = Datei wählen …
model-export-file-none = Keine Datei gewählt
model-export-file-filter = glTF-Binärdateien
model-export-start = Exportieren
model-export-running-title = Modell wird exportiert
model-export-reading = { $name } wird gelesen …
model-export-progress = { $done } von { $total } geschrieben, { $percent } %
model-export-summary-title = Modell exportiert
model-export-summary-cancelled-title = Export abgebrochen
model-export-summary-cancelled = Der Export wurde abgebrochen. Es wurde keine Datei geschrieben.
model-export-summary-file = Geschrieben nach { $path }
model-export-summary-counts = Knoten { $nodes }, Meshes { $meshes }, Materialien { $materials }, Vertices { $vertices }, Dreiecke { $triangles }, Größe { $size }
model-export-summary-bones = Knochen { $bones }, Meshes mit Skinning { $skinned }
model-export-summary-library-missing = Die Materialbibliothek { $library } existiert nicht; es wird das Standardmaterial verwendet.
model-export-summary-library-unreadable = Die Materialbibliothek { $library } ist nicht lesbar ({ $reason }); es wird das Standardmaterial verwendet.
model-export-summary-default-materials = { $count ->
    [one] Eine Material-ID hat kein Teilmaterial in der Bibliothek; dafür wird das Standardmaterial verwendet.
   *[other] { $number } Material-IDs haben kein Teilmaterial in der Bibliothek; dafür wird das Standardmaterial verwendet.
  }
model-export-summary-unnamed-materials = { $count ->
    [one] Ein Eintrag der Materialbibliothek ist kein Material; dafür wird das Standardmaterial verwendet.
   *[other] { $number } Einträge der Materialbibliothek sind kein Material; dafür wird das Standardmaterial verwendet.
  }
model-export-summary-hidden = { $count ->
    [one] Ein Submesh wurde ausgelassen, weil sein Material nichts zeichnet.
   *[other] { $number } Submeshes wurden ausgelassen, weil ihr Material nichts zeichnet.
  }
model-export-summary-chunks = Nicht exportierte Chunks ({ $number })
model-export-summary-chunk-unknown = Unbekannter Chunk { $index } { $type_id } übersprungen ({ $file })
model-export-summary-chunk-skipped = Chunk { $index } { $type } ({ $type_id }) wird nicht gelesen ({ $file })
model-export-summary-textures = Fehlende Texturen ({ $number })
model-export-summary-texture = { $material }: { $role ->
    [diffuse] Diffuse
    [normal] Normal
    [specular] Specular
   *[other] { $role }
  } { $file } ({ $reason })
model-export-summary-animations = Animationen { $written }, Kanäle { $channels }
model-export-summary-animations-unread = Nicht geschriebene Animationen ({ $number })
model-export-summary-animation-unread = { $name } ({ $source }): { $reason }
model-export-summary-animation-file-unread = { $source }: { $reason }
model-export-failed-title = Export fehlgeschlagen
model-export-failed = { $name } konnte nicht exportiert werden: { $reason }
model-export-stopped = Der Export von { $name } wurde ohne Ergebnis beendet.
model-export-key-required = { $name } ist verschlüsselt und das Archiv hat keinen Schlüssel. Gib den Archiv-Schlüssel ein und exportiere erneut.
model-export-key-rejected = Der Archiv-Schlüssel entschlüsselt { $name } nicht. Gib den richtigen Schlüssel ein und exportiere erneut.
model-export-archive-changed = Das Archiv hat sich auf dem Datenträger geändert, deshalb wurde { $name } nicht exportiert. Lade das Archiv neu und exportiere erneut.
scroll-hits = Suchtreffer
content-search-tab = Suche
content-search-label = Suche
content-search-hint = Gesuchter Text, z. B. Gladi
content-search-run = Suchen
content-search-cancel = Abbrechen
content-search-sources = Suchen in
content-search-source-records = Records
content-search-source-localization = Lokalisierung
content-search-source-entries = Einträge
content-search-empty-title = Kein Archiv geöffnet
content-search-empty-hint = Öffnen Sie ein Archiv, um die Inhalte seiner Records, Lokalisierungstexte und Einträge zu durchsuchen.
content-search-prompt = Geben Sie einen Text ein und wählen Sie Suchen. Er wird auch als Teil eines längeren Textes gefunden, unabhängig von der Groß- und Kleinschreibung.
content-search-searching = Suche läuft …
content-search-cancelled = Die Suche wurde abgebrochen.
content-search-failed = Die Suche ist fehlgeschlagen: { $reason }
content-search-stopped = Die Suche wurde ohne Ergebnis beendet.
content-search-archive-changed = Das Archiv hat sich auf dem Datenträger geändert. Laden Sie es neu, um es zu durchsuchen.
content-search-no-match = Nichts passt zu „{ $text }“.
content-search-none-here = Diese Quelle hat keinen Treffer.
content-search-segment = { $number }
content-search-segment-more = { $number }+
content-search-hits =
    { $count ->
        [one] { $number } Treffer in { $seconds } s
       *[other] { $number } Treffer in { $seconds } s
    }
content-search-more = Es gibt weitere Treffer. Pro Quelle bleiben nur die ersten { $number }; ein längerer Text grenzt die Suche ein.
content-search-skipped =
    { $count ->
        [one] { $number } Eintrag wurde nicht durchsucht: { $binary } binär, { $large } zu groß, { $unreadable } nicht lesbar.
       *[other] { $number } Einträge wurden nicht durchsucht: { $binary } binär, { $large } zu groß, { $unreadable } nicht lesbar.
    }
content-search-key-required =
    { $count ->
        [one] { $number } Eintrag ist verschlüsselt und wurde nicht durchsucht: Das Archiv hat keinen Archivschlüssel oder der Schlüssel passt nicht.
       *[other] { $number } Einträge sind verschlüsselt und wurden nicht durchsucht: Das Archiv hat keinen Archivschlüssel oder der Schlüssel passt nicht.
    }
content-search-key-button = Archivschlüssel eingeben
content-search-no-dataforge = Das Archiv enthält keine DataForge, deshalb wurde kein Record durchsucht.
content-search-column-record = Record-Pfad
content-search-column-field = Feld
content-search-column-text = Text
content-search-column-language = Sprache
content-search-column-key = Schlüssel
content-search-column-entry = Eintrag
content-search-column-line = Zeile
content-search-row = { $place }: { $text }
