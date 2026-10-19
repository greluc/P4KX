app-title = P4K-Extractor
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
extraction-folder-none = Kein Ordner gewählt
extraction-choose-folder = Ordner wählen …
extraction-policy-heading = Bereits vorhandene Dateien
extraction-policy-skip = Überspringen
extraction-policy-overwrite = Überschreiben
extraction-policy-if-changed = Nur bei Änderung überschreiben
extraction-conversion = CryXml und DataForge in XML und Texturen umwandeln
extraction-texture-merge = Geteilte Texturen zu einer DDS zusammenführen
extraction-texture-png = Texturen als PNG exportieren (HDR als OpenEXR)
extraction-texture-mip-largest = Größte Mip-Stufe
extraction-texture-mip-level = Mip-Stufe
extraction-texture-raw = Rohe Texturteile behalten
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

dataforge-node = DataForge
dataforge-node-loading = Wird geladen …
preview-tab-dataforge = DataForge
dataforge-loading = DataForge wird aus { $path } geladen …
dataforge-unsupported = DataForge-Version { $version } wird von dieser Version des P4K-Extractors nicht unterstützt.
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
catalog-file-filter = Basetool-Katalog
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
catalog-near-limit = Der Katalog liegt nahe am Upload-Limit des Basetools von { $limit }.
catalog-over-limit = Der Katalog liegt über dem Upload-Limit des Basetools von { $limit }; das Basetool lehnt den Upload ab.
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
