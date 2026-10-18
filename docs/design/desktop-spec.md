# P4K-Extractor · Desktop-Vorgaben aus dem KRT Design System

Quelle: `krt-profit/design-system` `main`, Version `1792333362-0b15`. Maschinenlesbar:
[`p4kx-desktop-spec.json`](p4kx-desktop-spec.json). Artboards: Design-Canvas „P4K-Extractor Desktop".
Logo: [`p4kx-logo.svg`](p4kx-logo.svg) (ab 64 px), [`p4kx-glyph.svg`](p4kx-glyph.svg) (bis 32 px).

Alle Maße in px bei 1 rem = 16 px. Farben, Glows und Gewichte als Token-Namen aus `tokens.json`.
Die wenigen `rgba(…)`-Werte stehen so in `krt-components.css` und haben dort kein Token; sie
sind keine neuen Farben.

## Grundregeln

- **Zeilendichte 36 px** für Tabelle, Baum, Channels und Dropdown-Einträge (D.1).
- **Bedienelemente bleiben 44 px** (Buttons, Suchfeld, Tabs, Menü-Links). Kompakte
  DS-Ausnahmen: `.krt-segmented--sm` 34 px, `.btn.btn-xs` 32 px.
- **Schriftgewicht nach Größe:** Text unter 16 px in Regular 400, Light 300 nur für Fließtext ab
  16 px, Bold 700 für Labels, Buttons, Überschriften. Black 900 nur im Logo-Lockup, das als Bild
  ausgeliefert wird; die App braucht also drei Fontdateien.
- **Fokusring:** 2 px `color-primary`, Abstand 2 px bei Bedienelementen, −2 px (innen) bei Zeilen
  und Tabs, −3 px bei Segmenten. Auf der CTA weiß.
- **Deaktiviert:** Deckkraft 0,45, kein Hover.
- **Gedrückt:** Das DS kennt keinen Gedrückt-Zustand. Vorgabe hier: CTA `color-accent-dark`,
  Ghost/Outline `control-selected-bg`, Quiet-Danger `rgba(163,0,10,0.2)`, Zeilen wie Hover.
- **Übergänge:** nur Farbe/Deckkraft, 200 ms.
- **Daten-Schreibweise:** Dateinamen, Pfade, Globs und Schnellfilter bleiben unverändert (wie
  `.chip--data`); Versalien nur für Labels, Buttons, Überschriften und Tabellenköpfe.

## Komponententabelle

| Element | DS-Klasse | Höhe | Innenabstand | Abstand | Schrift (px / Gewicht / Tracking) | Versalien | Farben | Rahmen | Glow | Fokus |
| :-- | :-- | --: | :-- | :-- | :-- | :-- | :-- | :-- | :-- | :-- |
| Fenster-Titelleiste | nativ (DWM) | 32 | – | – | System | nein | bg `color-gray-4`, Text `color-gray-1` | unten 1 `color-gray-3` | – | System |
| Kopfzeile | App-Header | 46 | x 16 | Marke→Menü 24 | Wortmarke 14,4 / 700 / 0,05 em | ja | bg `surface-panel`, Marke `color-primary` | unten 2 `color-primary` | – | – |
| Menü-Link | – (nav) | 44 | x 14 | 0 | 12,8 / 700 / 0,05 em | ja | `control-fg`; Hover `color-primary`; offen `control-selected-bg` + `control-selected-fg` | offen: Unterstrich 3 `color-primary` | – | innen −2 |
| Dropdown | `.krt-combobox__listbox` | – | y 4 | Untermenü −4 überlappend | – | – | bg `color-gray-4` | 1 `color-primary` | `shadow-dropdown` | – |
| Dropdown-Eintrag | `.krt-combobox__option` | 36 | x 12 | Spalten 12 | 14,4 / 400 / 0; Kürzel 12,8 `color-gray-2-text` | nein | `color-gray-1`; Hover bg `color-gray-3` + `color-primary`; Tastatur bg `color-primary` + `color-bg-black` | Trenner 1 `color-gray-3`, y 4 | – | Tastatur = Füllung |
| Seitenleisten-Titel | `.page-head__eyebrow` | ~32 | 14 / 12 / 6 | – | 11,2 / 700 / 0,15 em | ja | `color-gray-2-text` | – | – | – |
| Channel-Zeile | `.master-row` | 36 | x 12 | Name↔Build 8 | Name 12,8 / 700 / 0,05 em; Build 12,8 / 400 | Namen sind Konstanten | `color-gray-1`, Build `color-gray-2-text`; Hover `color-gray-3`; aktiv bg `color-surface-input`, Text `color-white` | unten 1 `color-gray-3`; aktiv links 3 `color-primary` (innen) | – | innen −2 |
| Baumzeile | `.tree-row` / `.tree-name` | 36 | links 8 + 12 je Ebene, rechts 8 | Chevron↔Name 6 | 13,76 / 400; aktiv 700 | nein | wie Channel-Zeile; Chevron `color-primary` 12 px, Strich 3 | Ebenen-Schiene 1 `color-gray-3` | – | innen −2 |
| Toolbar | `.list-panel__toolbar` | 68 | 12 | 12, bricht um | – | – | bg `surface-panel` | unten 1 `color-gray-3` | – | – |
| Suchfeld | `.list-panel__search input` | 44 | links 38,4, rechts 12 | Basis 260, min. 200 | 16 / 400; Platzhalter kursiv | nein | bg `control-bg`, Text `color-gray-1`, Platzhalter `color-gray-2-text` | 1 `control-border`; Fokus `color-primary` | Fokus `glow-primary` | Rahmen + Glow |
| Schnellfilter | `.krt-segmented--sm` / `__item[aria-pressed]` | 34 | x 12 | Label↔Gruppe 12 | 12,8 / 400 (gewählt 700) / 0 | **nein** | `control-fg` auf `control-bg`; Hover `control-bg-hover` + `color-white`; gewählt `control-selected-bg` + `control-selected-fg` | 1 `control-border`, Trenner 1; gewählt innen 1 + unten 3 `control-selected-border` | – | innen −3 |
| Button-Grundform | `.btn` | 44 (xs 32) | x 16, y 8 (xs 8,8 / 4) | Icon↔Text 8 | 12,8 / 700 / 0,03 em (xs 10,88) | ja | siehe Leiter | Radius 0 | – | 2 / +2 |
| CTA | `.btn.btn--cta` | 44 | wie oben | – | wie oben | ja | bg `color-primary`, Text `color-bg-black`; Hover `color-accent-light`; gedrückt `color-accent-dark` | – | `glow-primary`, Hover `glow-primary-lg` | weiß, +2 |
| Outline | `.btn-outline` | 44 | wie oben | – | wie oben | ja | `color-primary` auf transparent; Hover `color-accent-light` | 1 `color-primary` | – | 2 / +2 |
| Ghost | `.btn-ghost` | 44 | wie oben | – | wie oben | ja | `control-fg` auf `control-bg`; Hover `color-primary` | 1 `control-border`; Hover `color-primary` | – | 2 / +2 |
| Quiet-Danger | `.btn-quiet-danger` | 44 | wie oben | – | wie oben | ja | `color-gray-2-text` auf `control-bg`; Hover `color-danger-text` | 1 `control-border`; Hover `color-danger` | – | 2 / +2 |
| Tabellenkopf | `th` / `.tree-head .tree-cell` | 36 | x 12,8 | – | 11,84 / 700 / 0,07 em | ja | bg `color-surface-input`, `color-gray-1`; sortiert `color-white` | unten 2 `color-primary` | – | innen −2 |
| Sortieranzeige | Chevron (Icon-Set) | 12 | – | Label↔Icon 6 | – | – | aktiv `color-primary`; Hover-Vorschau `color-gray-2-text` | – | – | über Kopfzelle |
| Tabellenzeile | `tbody tr` / `td` | 36 | x 12,8; Checkbox-Spalte 44 | – | 13,76 / 400; aktiv 700 | nein | `color-gray-1`; Zebra `rgba(255,255,255,0.02)`; Hover `color-gray-3`; ausgewählt `control-selected-bg`; aktiv bg `color-surface-input`, Text `color-white` | unten 1 `color-gray-3`; aktiv links 3 `color-primary` (innen) | – | innen −2 |
| Checkbox | `input[type=checkbox]` | 19,2 | – | Trefferfläche = Zelle | Haken | – | bg `color-surface-input`; an `color-primary`, Haken `color-bg-black`; gemischt: Balken 10 × 2 | 1 `control-border`; Hover `color-primary` | – | 2 / +2 |
| Radio | `input[type=radio]` | 19,2 (Zeile 36) | – | Label 12 | Label 14,4 / 400 | nein | wie Checkbox, Punkt 8 `color-bg-black` | 1 `control-border`, rund | – | 2 / +2 |
| Tab-Leiste | `.tab-nav` / `.tab` | 44 | Tab x 14, Leiste links 12 | – | 11,52 / 700 / 0,06 em | ja | `color-gray-2-text`; Hover `color-gray-1`; aktiv `color-white` | unten 1 `color-gray-3`; aktiv 3 `color-primary` | – | innen −2 |
| Vorschau-Panel | Fläche + `.tab-nav` | Start 236, min. 120 | Pfadzeile 32, x 16; Inhalt x 16 y 8 | – | Pfad 12,8 / 400 | nein | bg `surface-panel`; Pfad `color-gray-1`, Größe `color-gray-2-text` | 1 `color-gray-3` | – | – |
| Hex-Zeile | – | 20 | – | Offset 88; Byte-Zelle 22 + 2; nach 8 Bytes 10; ASCII-Abstand 16, Zelle 10 | 12,8 / 400, Ziffern tabellarisch | Hex in Großbuchstaben | Offset und `00` `color-gray-2-text`; Bytes/ASCII `color-gray-1`; nicht druckbar „." gedämpft | – | – | – |
| Statusleiste | `.krt-footer` (Desktop-Dichte) | 32 | x 16; Einträge x 12 | Trenner 1 `color-gray-3` | 12,8 / 400 | nein | bg `surface-panel`, `color-gray-2-text`; Pfad `color-gray-1`; „Suche …" `color-primary` | oben 1 `color-gray-3` | – | – |
| Leerzustand | `.empty-state` | füllt | 24 | 12 | Titel 13,6 / 700 / 0,06 em; Text 12,8 | Titel ja | bg `color-bg-dark-gray`, Titel `color-gray-1`, Text `color-gray-2-text`; Ziehen: `rgba(231,126,35,0.10)` | 1 gestrichelt `color-gray-3`; Ziehen `color-primary` | – | Aktion |
| Hinweisleiste | `.alert.alert-warning` | ~44 | x 16, y 10 | 12 | 14,4 / 400 | nein | bg `rgba(255,210,63,0.15)`, Text `color-gray-1`, Icon `color-warning` | 1 `color-warning`, links 4 | – | Aktion |
| Modal | `.krt-modal` (`--wide` 600) | – | Kopf 16 / 12, Körper 16, Fuß 16 / 12 | Gruppen 20, Fuß 8 | Titel 13,6 / 700 / 0,06 em; Körper 14,4 / 400 | Titel ja | bg `color-bg-dark-gray`, Titel `color-white`; Scrim `rgba(0,0,0,0.8)` flach | 1 `color-gray-3`, oben 3 `color-primary`, Ecken 13 × 2 | `glow-primary-lg` | Fokusfalle, Esc |
| Modal Danger | `.krt-modal--danger` | – | wie Modal | – | Titel `color-danger-text` | ja | – | oben 3 + Ecken `color-danger` | `glow-danger-lg` | wie Modal |
| Aufklappbarer Abschnitt | `.panel-header` | 44 | x 16, y 12,8 | Liste max. 180 hoch | 13,6 / 700 / 1 px | ja | bg `color-surface-input`, Titel `color-white`, Chevron `color-primary`; Hover `color-gray-3` | 1 `color-gray-3`, links 4 `color-primary` | – | 2 / +2 |
| Fortschrittsbalken | `.attendance-meter` | 8 | – | Text→Balken 12 | Text 14,4 / 400, tabellarisch | nein | Spur `color-surface-input`; Füllung `color-primary`; beim Abbrechen `color-gray-2` | 1 `color-gray-3` | – | – |
| Spinner | `.krt-spinner` | 20 (inline 14) | – | Text 8 | – | – | Spur `rgba(231,126,35,0.25)`, Bogen `color-primary` | Ring 2 | – | – |
| Detailliste | `.kv-list.kv-ruled` | – | unten 8,8 | Zeilen 8,8, Spalten 20; Schlüsselspalte min. 140 | Schlüssel 14,4 / 700 / 0,04 em; Wert 16,8 / 300 | Schlüssel ja | Schlüssel `color-gray-2-text`, Wert `data-fg` | unten 1 `color-gray-3` | – | – |
| Abschnittstitel | `.section-title` | – | unten 8 | oben 32 (als Erstes 0), unten 16 | 16 / 700 / 0,05 em | ja | `color-gray-1` | unten 1 `color-gray-3` | – | – |
| Link | `a` | Text | – | – | 12,8 / 700 | nein | `color-primary`; Hover `color-accent-light` + Unterstrich | – | – | 2 / +2 |
| Tooltip | `.scu-hint__bubble` | – | x 9,6, y 8; max. 272 breit | 500 ms Verzögerung | 12,48 / 400, Zeilenhöhe 1,4 | nein | bg `rgba(0,0,0,0.95)`, `color-gray-1` | 1 `color-primary` | 0 0 10 0 `rgba(231,126,35,0.3)` | – |
| Scrollbar | `::-webkit-scrollbar` | 12 (Seitenleiste 8) | Griff ohne Einzug | Griff min. 32 | – | – | Spur `color-gray-4`; Griff `color-gray-2`, Hover `color-accent-light`, Ziehen `color-primary` | keine Haarlinie | – | – |
| Größenänderungs-Griff | – | 6 | Griff 2 × 24 mittig | – | – | – | Feld `surface-canvas`; Griff `color-gray-2`; Hover volle Länge `color-accent-light`; Ziehen `color-primary` | – | – | 2 / +2, Pfeiltasten |

## Entscheidungen

**D.1 Zeilendichte: 36 px (Tree-Table-Dichte), keine 44-px-Pflicht für Datenzeilen.**
36 px entsprechen der vorhandenen DS-Dichte `.tree-cell` (0,5 rem Abstand, 0,86 rem Schrift). Die
wörtliche `td`-Höhe von ~56 px zeigt auf 1440 × 900 nur etwa sieben Zeilen. 34 px wären möglich,
aber Hover, aktive Zeile und Auswahl brauchen etwas Luft, um bei 1 Mio. Zeilen auseinanderzuhalten
zu sein. Die 44-px-Regel ist eine Touch-Regel; P4K-Extractor ist ein reines Maus- und
Tastatur-Werkzeug. Datenzeilen sind keine einzelnen Ziele: die ganze Zeile ist klickbar, und die
Checkbox bekommt die volle Zelle (44 × 36) als Trefferfläche. Eigenständige Bedienelemente bleiben
44 px.

**D.2 Einrückung: 12 px je Ebene (`space-3`) plus 1-px-Ebenenschienen.** Zehn Ebenen kosten
120 px, bei 300 px Breite bleiben rund 150 px für den Namen. Mit 24 px wären es 240 px und kaum
noch Platz. Die Schienen (`color-gray-3`) zeigen die Tiefe, damit 12 px reichen. Volle Namen
stehen im Tooltip, die Seitenleiste lässt sich bis 480 px verbreitern.

**D.3 Modal nur, wo eine Entscheidung fällt.** „Einträge extrahieren", die Bestätigung und die
Schriftlizenz sind modal, mit flachem Scrim `rgba(0,0,0,0.8)`. „Extraktion läuft" und
„Extraktion beendet" sind nicht modal und haben keinen Scrim. Während der Extraktion kann man
weiter browsen und die Vorschau nutzen, das Archiv wird nur gelesen. Die drei
Extraktionsaktionen und „Neu laden" sind gesperrt, bis die Extraktion endet.

**D.4 Marke: ein neues P4K-Extractor-Zeichen plus Wortmarke, in Orange.** Das Zeichen gehört zur
Basetool-Familie (Ring, Keil, Vierpunkt-Stern) und erzählt das Werkzeug: Der Keil liest durch den
Ring (das Archiv), aus der Öffnung kommen Datenblöcke heraus. `basetool-extractor-logo` gehört
dem SC-Extractor, `krt.webp` ist die Organisationsmarke; beides würde verwechselt. In der
Kopfzeile: Zeichen 24 px plus „P4K-EXTRACTOR" Bold 700, 14,4 px, `color-primary`. Fenster- und
Taskleisten-Icon: vereinfachtes Zeichen in Orange auf schwarzer Kachel, damit es auf heller und
dunkler Taskleiste lesbar bleibt. Das Logo kommt nur in Orange, Weiß oder Schwarz vor.

**D.5 Fortschritt: Orange.** Fortschritt ist laufende Aktivität, wie Spinner und
Lade-Indikator, die das DS orange zeichnet. Grün bedeutet im DS einen erreichten Zustand
(eingecheckt). Neutral wäre auf `color-surface-input` zu schwach. Beim Abbrechen wird die
Füllung `color-gray-2`.

**D.6 Daten bleiben unverändert.** Schnellfilter, Dateinamen, Pfade, Build-Nummern und
Hex-Werte stehen ohne Versalien und ohne Tracking da, wie bei `.chip--data`. Versalien würden
`Game2.dcb` zu einem anderen Suchbegriff machen und Groß- und Kleinschreibung in Pfaden
verstecken.

**D.7 Keine CTA im Hauptfenster.** „Ausgewählte extrahieren …" ist Outline (betonte
Zweitaktion, gesperrt ohne Auswahl), „Angezeigte …" und „Alle …" sind Ghost. Die gefüllte CTA
gehört in den Dialog: „Extrahieren" bzw. „Ganzes Archiv extrahieren". In „Extraktion beendet" ist
„Schließen" die CTA, weil es die einzige Aktion ist und Enter sie auslöst.

**D.8 „Archiv geändert": Warning.** Es ist nichts kaputt, es gibt einen neuen Stand (ein
Launcher-Update). Danger bleibt für echte Fehler („Extraktion fehlgeschlagen").

**D.9 Leerzustand mit Aktion „Archiv öffnen …" als Outline.** Ziehen und Ablegen ist weder per
Tastatur noch per Screenreader erreichbar, deshalb braucht es eine Schaltfläche. Outline statt
CTA, weil der übliche Weg die erkannten Channels links sind.

## Bewusste Abweichungen von den Web-Specimens

| Abweichung | Grund |
| :-- | :-- |
| Datenzeilen, Baum, Channels, Dropdown-Einträge 36 statt 44 px | Maus/Tastatur-Desktop, Dichte bei 1 Mio. Zeilen (D.1) |
| Baum 12 statt 24 px je Ebene | zehn Ebenen in 300 px (D.2) |
| Text unter 16 px in Regular 400 statt Light 300 | Lesbarkeit kleiner Schrift auf Schwarz |
| Statusleiste 32 statt ~51 px (`.krt-footer` hat 16/32 Innenabstand) | eine Zeile Statusinfo, kein Web-Footer mit Links |
| Kein `text-transform` | egui setzt Versalien beim Zeichnen; der Barrierefreiheits-Name bleibt normal geschrieben |
| Scrim flach statt `backdrop-filter: blur` | egui kann nicht weichzeichnen |
| Scrollbar-Griff volle Breite, keine Haarlinie an der Spur | egui-Scrollbar ohne Einzug; Haarlinie entfällt, weil sie eigene Zeichnung bräuchte |
| Tooltip-Schatten als ein Blur-Schatten 0/0/10/0 | genau ein Schatten je Rechteck |
| Modal ohne Slide, nur 200 ms Deckkraft | nur Farb- und Deckkraft-Übergänge |
| Chevron als gezeichneter Pfad statt `▶` | Lato hat kein `▶`; egui bräuchte eine Fallback-Schrift |
| Hex in festen Zellen statt Monospace | einzige Schrift Lato; die Zellen halten die Spalten, auch für A–F |
| Checkbox „gemischt" (Balken) ergänzt | DS hat keinen Teilauswahl-Zustand, die Tabelle braucht ihn |
| DataForge-Übersicht: drei gleich breite Tabellenspalten statt `table-layout: auto` | egui legt Tabellen nicht nach Inhalt aus; gleiche Spalten erfinden keine Breite, lange Werte enden mit Auslassung, Name, Typ und Record-Pfad stehen vollständig im Tooltip der Zeile |
| Gedrückt-Zustand ergänzt | DS definiert keinen; Werte stammen aus vorhandenen Tokens bzw. Hover-Werten |
| `.panel-header`-Titel 13,6 statt 17,6 px im Dialog | darf nicht größer sein als der Dialogtitel |
| Logo-Lockup mit Black 900 als Bild | die App lädt nur Light, Regular und Bold |

## Texte

UI-Texte aus `crates/p4kx-gui/locales`. Neu für die Entwürfe: „Kein Archiv geöffnet"
(Leerzustand-Titel), „Loslassen zum Öffnen" (Ziehen), „Schnellfilter", „Neu laden"-Menüeintrag mit
F5, „Ausgewählte extrahieren …" im Archiv-Menü mit Strg+E, Seitenleisten-Separator „Seitenleiste
verbreitern" / „Vorschau vergrößern" als Barrierefreiheits-Namen. Keine festen Label-Breiten:
Toolbar, Modal-Fuß und Statusleiste brechen um oder kürzen Pfade mit Auslassung.
