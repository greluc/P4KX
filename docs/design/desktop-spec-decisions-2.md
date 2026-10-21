# P4K-Extractor · Desktop-Vorgaben, Entscheidungen ab D.22

Fortsetzung von [desktop-spec.md](desktop-spec.md), das die Grundregeln, die Komponententabelle und die
Entscheidungen D.1 bis D.20 enthält; Maße und Farben stehen dort und in
[p4kx-desktop-spec.json](p4kx-desktop-spec.json).

**D.22 Record-Vorschau als Tab „Record" mit Referenzen als Links.** Ist im Record-Baum ein Record
gewählt (durch Klick, Record-Suche, Link oder Zurück/Vor), bietet die Vorschau hinter „DataForge"
den Tab „Record" an und öffnet ihn; „XML" zeigt dann das Record-XML des Records statt eines
CryXml-Eintrags, mit der Zeilenansicht aus D.19, und „Kopieren" kopiert es. Die Pfadzeile nennt
Record-Pfad und Record-Namen (`color-gray-2-text`), darunter eine `.facts-bar` ohne Richtung mit
Typ, GUID und Name; sie hält Typ und GUID auf einer Zeile, wo eine `.kv-list` in der 236 px hohen
Vorschau kaum Platz für Felder ließe. Es folgen der Tabellenkopf „Feld" | „Wert" und die Felder als
virtuelle 36-px-Zeilen mit Zebra und Hover wie die Feldänderungen (D.16), zwei gleich breite
Spalten. Eigene Kinder stehen eingerückt unter ihrem Feld, 12 px je Ebene mit den Ebenen-Schienen
des Baums (D.2); Array-Elemente heißen `[0]`, `[1]` …, ein Struct zeigt seinen Typ, ein Array die
Zahl der Elemente. Werte bleiben unverändert (D.6), `null` ist `color-gray-2-text`, ein nicht
lesbarer Wert „Fehler: Grund" in `color-danger-text` an seinem Feld; der Rest des Records steht
weiter da. Eine Reference ist ein Link nach der DS-Regel für `a` (12,8 / 700, `color-primary`,
Hover `color-accent-light` mit Unterstrich, Fokusring 2 / +2) mit dem Record-Pfad des Ziels, eine
Reference auf eine Stelle im Record als „Pfad · Stelle" wie in D.16; der Typ des Ziels folgt 8 px
dahinter in `color-gray-2-text`. Der Link wählt das Ziel im Record-Baum (Ordner klappen auf, der
Baum rollt hin) und in der Vorschau. Eine Reference auf einen fehlenden Record ist ein Fehlerwert,
bleibt aber als Link in `color-danger-text` anklickbar; statt zu navigieren erscheint über der
Faktenleiste ein `.alert.alert-danger` mit der GUID, bis ein anderer Record gewählt wird.
„Zurück" und „Vor" stehen als Ghost `btn-xs` links von „Kopieren", solange die Listenfläche die
Records zeigt, gesperrt ohne Schritt in ihrer Richtung; Alt+← und Alt+→ sowie die Maustasten
Zurück und Vor tun dasselbe, außer ein Textfeld hat den Fokus. Der Weg merkt sich jede Auswahl eines
Records (höchstens 256 Schritte je Richtung); eine neue Auswahl nach „Zurück" verwirft den Weg
nach vorn, und „Zurück" bei einem gewählten Ordner kehrt erst zum aktuellen Record zurück. Gelesen
wird der Record von einem Thread je Archiv, der DataForge aus dem einen Cache des Archivs offen hält
und nur die neueste Anfrage beantwortet; Record-Baum, Abfrage und Record-Export lesen DataForge aus
demselben Cache, also nur einmal.

**D.26 Texturgruppen in der Eintragstabelle.** Ein gesplittetes DDS (Kopf `.dds` mit Teilen
`.dds.N` und Alpha-Tail `.dds.a`/`.dds.Na`) ist eine Zeile mit der Gesamtgröße; der Name steht nach
dem Chevron-Slot, danach die Teilezahl in `color-gray-2-text` wie die Zusatzspalte der Gruppenzeile
im `tree-table`. Alle Zeilen der Namenszelle reservieren den Chevron-Slot (`CHEVRON` + `TREE_GAP`
nach `TREE_PADDING_LEFT`), damit die Namen untereinander stehen; nur Gruppenzeilen zeichnen den
Chevron (`color-primary`, Fokusring `FOCUS_OFFSET`). Aufgeklappt stehen die Teile darunter, eine
Ebene (`TREE_INDENT`) eingerückt mit Ebenenschiene wie im Baum, mit eigener Größe, Kompression und
Verschlüsselung. Das Häkchen einer Gruppen- oder Teilezeile setzt alle Einträge der Gruppe;
Suche, die einen Teil trifft, zeigt die ganze Gruppe; „Angezeigte" und „Ausgewählte" extrahieren
alle Teile. Sortieren nach Größe nutzt die Gesamtgröße. Teile ohne Kopf bleiben einzelne Zeilen.
Die Statuszeile („angezeigt", „ausgewählt") und „Angezeigte (N)" zählen Einträge, eine Textur mit
ihren Teilen; ein Teil in einem anderen Ordner als sein Kopf gehört zur Ansicht des Kopfes
(Ordner, die sich nur durch Groß-/Kleinschreibung oder `/` und `\` unterscheiden, sind ein Ordner,
ADR-0023). Keine
neuen Maße (`tableRow.textureGroup` im JSON der Spec nennt die benutzten).

**D.27 Scrollflächen der Vorschau und des Vergleichs rollen mit der Tastatur.** egui rollt eine
`ScrollArea` nur mit Maus und Rad; ohne Tastenbehandlung käme ein Tastaturnutzer über die erste
Bildschirmseite nicht hinaus. Deshalb ist jede lange Fläche eine fokussierbare Rollfläche
(`widgets::scroll_region`), wie die Zeilenansicht des XML (D.19): die Tabulatortaste erreicht sie,
der Fokusring sitzt innen (−2) um die Fläche, und ihr Barrierefreiheits-Name nennt den Inhalt
(„Textzeilen", „Hexadezimalzeilen", „Bildansicht", „Zeilenunterschiede", „Geänderte Felder",
„Record-Felder", „Umgewandeltes XML"). Pfeil auf/ab rollt um eine Zeile (beim Bild um 36 px),
Pfeil links/rechts ebenso zur Seite, Bild auf/ab um eine Seite weniger eine Zeile, Pos1 und Ende
zum Anfang und Ende. Solange die Fläche den Fokus hat, gehören ihr die Pfeiltasten; Tab verlässt
sie. Die Zeilen der Feldänderungen und der Record-Felder bleiben eigene Tabulatorziele.

**D.28 Lokalisierung als vierter Tab im Arbeitsbereich.** „Lokalisierung" folgt auf „Abfrage".
Sie braucht ein geöffnetes Archiv und liest die `global.ini` aller Sprachen im Hintergrund, sobald
der Tab erstmals erscheint; bis dahin steht ein Spinner mit „Die Lokalisierungsdateien werden
gelesen …" im Körper (D.5). Die Toolbar ist die der Listenfläche: Suchfeld mit Eyebrow „Suche",
rechts „Schlüssel kopieren" und „Text kopieren" als Ghost (D.7), gesperrt ohne Auswahl bzw. ohne
Text in der gewählten Zelle. Die Tabelle hat einen Kopf „Schlüssel" und je Sprache der Datei einen
Kopf mit dem Sprachnamen (Versalien, nicht sortierbar); die Zeilen sind die Schlüssel, ohne
Beachtung der Schreibweise sortiert, 36 px mit Zebra und Hover wie die Tabelle. Spalten: Schlüssel
300, Sprachen je 320 (min. 80), verschiebbar, die letzte füllt, mit horizontaler Scrollleiste
(`localizationTable` im JSON). Schlüssel und Texte stehen unverändert (D.6) auf einer Zeile; das
Zeilenende `\n` bleibt als geschriebener Text stehen, Steuer- und Richtungszeichen erscheinen als
Escapes in `color-gray-2-text` und lange Texte enden nach 1024 Zeichen mit Auslassung (D.19). Eine
fehlende, leere oder Platzhalter-Übersetzung ist ein `.chip--warning` „Fehlt" in der Zelle: Lücken
sind der Sinn der Tabelle und sollen auffallen, die Farbe ist nie der einzige Hinweis (D.11).
Ein Klick oder Fokus plus Enter wählt eine Zelle: ihre Zeile steht in `control-selected-bg`, die
Zelle selbst wie die aktive Zeile (Balken links, Weiß, Bold); Strg+C kopiert den Text der Zelle,
sonst den Schlüssel, solange kein Textfeld den Fokus hat und kein Labeltext markiert ist. Kopiert
wird der Text mit echten Zeilenumbrüchen. Die Tabelle rollt wie jede lange Fläche mit der
Tastatur (D.27): sie ist eine fokussierbare Rollfläche mit dem Namen „Lokalisierungszeilen", der
Fokusring sitzt innen (−2); Pfeil auf/ab rollt um eine Zeile (36 px), links/rechts ebenso zur
Seite, Bild auf/ab um eine Seite weniger eine Zeile, Pos1 und Ende zum Anfang und Ende. Die Suche
findet Schlüssel und die in den Zellen gezeigten Texte aller Sprachen (ein als „Fehlt" gezeigter
Text zählt nicht) ohne Beachtung der Groß-/Kleinschreibung (ein führendes `@` wird ignoriert),
läuft bei jedem Tastendruck in einem Hintergrund-Thread, bricht die vorige Suche wirklich ab und
lässt bis zum Ergebnis die vorherigen Zeilen stehen; verschwindet die gewählte Zeile aus dem
Ergebnis, ist nichts mehr gewählt, damit „Kopieren" nie eine verborgene Zeile kopiert; der
`.list-panel__footer` nennt „n von m Schlüsseln" und, solange sie läuft, „Suche läuft …" mit
Spinner. Wurde eine Datei mit ungültigen Bytes oder übersprungenen Zeilen gelesen, steht darüber
ein `.alert-warning` mit den Hinweisen. Gezeichnet und im Cache gehalten werden nur die sichtbaren
Zeilen; die ganze Tabelle wird nie je Frame gefiltert oder formatiert.

**D.29 Sprache der Record-Texte als Menü „Record-Texte".** Die Sprache der Texte neben den
Lokalisierungsschlüsseln gilt für den Tab „Record" und für das Record-XML; sie ist eine
Fensteroption wie die Oberflächensprache und steht deshalb als Menü-Link rechts von „Export" in der
Kopfzeile (Maße `menuLink`), nicht im Vorschau-Panel: das hat bei 236 px Starthöhe keinen Platz für
eine weitere Zeile, und ein 44-px-`select` mit Beschriftung verdrängte die Feldzeilen. Die Einträge
sind `dropdownItem` mit Radio wie im Sprachmenü: zuerst „Nur Schlüssel", dann die Sprachen, deren
`global.ini` das Archiv enthält, benannt in der Oberflächensprache. Ohne `global.ini` ist das Menü
gesperrt, auch für Hilfstechnik. Ohne Wahl folgt die Vorschau der Oberflächensprache, sofern das
Archiv sie hat. Ein Radio zeigt nur, was gewählt wurde: „Nur Schlüssel" ist nie der Ersatz für
eine Sprache, die das Archiv nicht hat; dann ist kein Radio gewählt, und unter den Einträgen
nennt eine Zeile in `color-gray-2-text` die fehlende Sprache. In der Zeile steht der Schlüssel in
`color-gray-2-text`, 8 danach der Text in `color-gray-1` (`tableRow`-Schrift, mit Auslassung
gekürzt; Zeilenumbrüche, Steuer- und Bidi-Zeichen als Escapes in `color-gray-2-text` wie in D.19
und D.28); fehlt der Text, steht dort ein `chip--warning` „Fehlt", nie nur Farbe, an der Kante der
Zelle abgeschnitten, wenn sie zu schmal ist. Platzhalter und leere Schlüssel bleiben allein. Lässt sich die `global.ini` nicht lesen, zeigt ein `alert-warning`
über der Faktenleiste den Grund, und die Vorschau zeigt nur Schlüssel. Ein Sprachwechsel lässt die
Vorschau stehen, bis die neue Fassung fertig ist. Keine neuen Maße (`recordPreview.keyText` im JSON
der Spec).

**D.30 Vervollständigung an der Schreibmarke, gespeicherte Abfragen unter dem Editor.** Beim Tippen
öffnet sich unter der Schreibmarke ein `.krt-combobox__listbox` mit den Namen, die dort passen:
Struct-Namen nach `from` und in `[…]` (der Typ der Elemente, seine Basen und alles, was davon
erbt), Feldnamen am Pfadanfang, nach `.` und nach `->`, jeweils mit dem Typ rechts in
`color-gray-2-text`. Die Liste steht an der Schreibmarke statt unter dem ganzen Feld, weil der
Editor mehrzeilig ist und ein Name mitten im Text ergänzt wird; reicht der Platz darunter nicht,
klappt sie über die Zeile. Weil sie nicht am Feld hängt, trägt sie den 1-px-Rand in
`color-primary` auf allen vier Seiten, nicht `border-top: none` der Combobox unter einem Feld;
sonst sind Farben, Einträge und Schatten die der Combobox. Der getippte Anfang ist fett wie
`<mark>`, der per Tastatur gewählte Eintrag orange gefüllt, Hover wie im Dropdown. Der Fokus
bleibt im Editor: ↑ und ↓ wählen, Enter, Tab oder Klick übernehmen das ganze Wort um die
Schreibmarke, Esc schließt, Strg+Leertaste öffnet wieder; steht der gewählte Name schon so da,
bricht Enter die Zeile wie ohne Liste. Höchstens 50 Namen stehen in der Liste, darunter „n weitere
Namen, weiter tippen" als `.krt-combobox__notice`. Das Schema liest ein Hintergrund-Thread, wenn
der Editor den Fokus bekommt, einmal je Archiv, und einmal je Cache-Stand noch einmal, wenn der
Editor den Fokus hat und kein Schema da ist (nach einem Fehler oder „Neu laden"); bis dahin, und
wo kein Name passt, bleibt die Liste zu, statt einen leeren Zustand zu zeigen. Sie öffnet sich
nicht von selbst, wenn das Schema ankommt, außer Tippen oder Strg+Leertaste fanden es fehlend;
bei geschlossener Liste bewegt Tab den Fokus weiter. „Abfrage speichern …" ist Ghost neben der CTA
„Abfrage ausführen" (D.7, D.15). Die gespeicherten Abfragen stehen darunter als aufgeklappter
`.panel-header` mit 36-px-Zeilen: Name fett, nach 240 px mit Auslassung, dahinter die Abfrage
einzeilig in `color-gray-2-text`. Die Liste ist eine fokussierbare Rollfläche („Gespeicherte
Abfragen", D.27), ihre Zeilen bleiben eigene Tabulatorziele; die Vervollständigungsliste nimmt nie
den Fokus, der gewählte Eintrag rollt mit ↑ und ↓ in Sicht. Ohne gespeicherte Abfrage steht
unter dem Kopf ein Satz in `color-gray-2-text`, der sagt, wie man eine speichert, kein `.empty-state`: der Abschnitt ist
klein und der Editor darüber die Aktion. Ein Klick auf die Zeile lädt und startet sie;
„Umbenennen" (Ghost) und „Löschen" (Quiet-Danger) stehen als `btn-xs` rechts, weil das Löschen
die einzige zerstörende Aktion ist. Speichern und Umbenennen fragen den Namen in einem
`.krt-modal` ab; ein vergebener Name ersetzt beim Speichern die Abfrage (Hinweis in
`color-gray-2-text`) und sperrt das Umbenennen (Hinweis in `color-danger-text`). Löschen fragt
in einem `.krt-modal--danger` nach (D.3), dessen Fuß nach dem Modal-Specimen Ghost „Abbrechen"
und das rot gefüllte `.btn.btn-danger` „Löschen" trägt.

**D.31 Container als Ordner im Pfadbaum.** Ein `.socpak` steht nach den Unterordnern seines
Ordners als gewöhnliche Baumzeile: Name unverändert (D.6), Chevron, Ebenen-Schienen. Ein eigenes
Symbol gibt es nicht, weil das Icon-Set des DS keines für Archive hat; die Endung unterscheidet
den Container schon. Während er geöffnet wird, stehen hinter dem Namen wie beim DataForge-Knoten
der Inline-Spinner (14) und „Wird geöffnet …" in `color-gray-2-text`. Lässt er sich nicht öffnen,
steht dort ein kurzer Hinweis: „Lässt sich nicht öffnen" in `color-danger-text`, weil der Container
selbst defekt ist; „Nicht geöffnet: Speichergrenze", „Archiv geändert" und „Öffnen abgebrochen"
in `color-warning` (D.8), weil sie wieder verschwinden, sobald Speicher frei ist oder das Archiv
neu geladen wird. Der volle Grund steht im Tooltip der Zeile, weil die Seitenleiste für ihn zu schmal ist. Ein Container, der
sich nicht lesen lässt, und einer nach einer Änderung des Archivs haben keinen Chevron; nach der
Speichergrenze oder einem abgebrochenen Öffnen bleibt der Chevron, ein erneutes Aufklappen versucht
es noch einmal. Schließt die Speichergrenze einen Container, klappen er und seine Ordner zu; die
Haken seiner Einträge bleiben erhalten. Wählt man einen Container oder einen seiner Ordner,
zeigt die Tabelle dessen Einträge. Suche, Auswahl, „Ausgewählte extrahieren …", „Angezeigte
extrahieren …" und Vorschau gelten dann diesem Container, „Alle extrahieren …" weiter dem ganzen
Archiv. Container liegen in den gefalteten Ordnern des Eintragsbaums (ADR-0023): Schreibweise und
Trennzeichen (`/` oder `\`) ändern nicht, in welchem Ordner ein Container steht; das gilt auch in
einem Container. Ein `.socpak`, dessen Ordnerpfad ein leeres Segment hat, steht im nächsten
vorhandenen Ordner darüber, sonst an der Wurzel. Die Statusleiste nennt Pfad und Build weiter des
Archivs; zeigt die Tabelle einen Container, steht an der Stelle der Eintragszahl des Archivs
„Container: n Einträge" mit der Zahl seiner Einträge, passend zu „n angezeigt".

**D.32 Tonformat der Extraktion als Radio-Zeile unter den Texturausgaben.** Ist die Umwandlung an,
steht im Dialog „Einträge extrahieren" eingerückt unter den Texturausgaben eine Zeile „Sounds als"
(14,4 / 400, `color-gray-1`, wie die Radio-Labels) mit den Radios „Ogg", „WAV" und „WEM,
unverändert" nebeneinander, 16 voneinander wie die Steuerelement-Zeile im Specimen FormControls
(`gap: 16px`), Radio↔Label 12 wie bei der Radio-Gruppe. Drei sich ausschließende
Werte sind Radios wie Konfliktregel und Mip-Stufe, kein `.krt-segmented--sm`: der Schnellfilter
gehört in Toolbars, im Modal bleiben Formularelemente. Eine Zeile statt einer Spalte, weil die
Werte kurz sind und der Dialog sonst bei 800 px Fensterhöhe scrollen müsste. Darunter steht in
12,8 / 400 `color-gray-2-text`, wie der Hinweis auf den neuen Ordnernamen, dass Ogg für Codecs ohne
Ogg-Form als WAV geschrieben wird und Soundbanks zusätzlich in einen Ordner je Bank entpackt werden.
Ist die Umwandlung aus, verschwindet die Zeile mit den Texturausgaben. Die Wahl gilt für die nächste
Extraktion weiter und steht als `extraction_audio` in der Einstellungsdatei.

**D.33 Soundbanks im Tab „Audio".** Wählt man eine Soundbank (`.bnk`), öffnet die Vorschau den Tab
„Audio" wie bei einer WEM (D.18); die Pfadzeile nennt hinter der Größe, durch „·" getrennt, wie
viele WEMs die Bank einbettet („400,4 KiB · 9 WEMs"). Darunter steht von oben nach unten die
Tonvorschau der gewählten WEM mit denselben Fakten und Bedienelementen wie bei einer losen WEM
(D.18), solange eine gewählt ist, und darunter die Liste der WEMs, die den Rest der Höhe füllt. Die
Liste ist eine Tabelle in der Dichte der Eintragstabelle (D.1): Kopf und Zeilen 36, Spalten ID 106,
Codec 112 und Dauer 100 (je ab 80 verstellbar; die Breite des breitesten Textes in Fett bei der Zeilenschrift plus Zellrand beidseitig, aufgerundet: zehnstellige ID 105,4, „Wwise Vorbis" 110,6, „9:59:59.999" 99,9), Größe füllt den Rest und steht rechtsbündig; der
Kopf ist der der Tabelle, aber nicht sortierbar, weil die Bank die WEMs nach ID ordnet. Ein Codec,
den die Kopfdaten nicht nennen, eine unbekannte Dauer und die Marken „Vorgeladen" und „Plug-in"
stehen in `color-gray-2-text`; die aktive Zeile ist wie in der Eintragstabelle fett mit dem orangen
Balken. Jede Zeile ist ein Tabulatorziel mit Fokusring innen −2 und heißt für Screenreader „WEM
123, PCM, 0:01.000"; Enter und Klick wählen sie. Die Liste ist eine fokussierbare Rollfläche
(„Eingebettete WEMs", D.27) und rollt die gewählte Zeile einmal in Sicht, weil die Tonvorschau
über ihr die Liste beim Wählen verkürzt. Weil die Liste Platz braucht, öffnet das Panel für
Soundbanks 373 hoch statt 236 (Tab-Leiste 44, Pfadzeile 32, Faktenleiste 32,
Bedienelemente 16 + 32, Kopf 36, vier Zeilen zu 36 und 37 Panelrand aus Abstand, Rahmen und Linien, gemessen; so bleiben bei gewählter WEM vier Zeilen sichtbar) und merkt sich seine Größe getrennt von der der anderen Vorschauen;
es füllt seine Höhe immer, damit sie schon beim Lesen der Bank feststeht. Nur eine WEM ist
gewählt; eine andere zu wählen beendet den Ton der ersten. Eine WEM, von der die Bank nur den
vorgeladenen Anfang hält, und Plug-in-Medien (Faltungshall-Impulse) haben keinen Ton: statt der
Bedienelemente steht ein Satz in `color-warning`, bei der vorgeladenen mit dem Hinweis auf den
losen WEM-Eintrag gleicher ID. Ein Codec ohne Decoder und ein Fehler beim Dekodieren stehen wie
in D.18 in `color-danger-text`, mit dem Namen der WEM („WEM 123 aus Data\Sounds\x.bnk") statt des
Hinweises auf Text und Hex, die hier die Bank zeigen. Eine unlesbare Bank steht als Satz in
`color-danger-text`, eine ohne WEMs als schlichter Satz; das übrige Fenster bleibt bedienbar. Die
Liste ist eine Tabelle und kein Baum, weil die Bank keine Ebenen kennt; Ereignisse und Auslöser
gehören zur Auflösung der Sounds (M11).

**D.34 Reiter „Referenziert von" listet die verweisenden Records.** Ist ein Record gewählt, steht
nach „Record" ein weiterer Reiter der Vorschau-Leiste (D.1 Tab-Leiste, 44 px, gleiche Beschriftung
wie die übrigen Reiter in Versalien). Der Reiter erfindet nichts Eigenes: Er zeigt unter der
Pfadzeile (32 px, Record-Pfad und Name wie im Record-Reiter) eine Zählzeile im Rahmen des
Vorschau-Körpers („3 Referenzen von 2 Records", 14,4 / 400 `color-gray-1`, wie die Zählzeile
der Abfrage und der `.page-status` der Pagination des Design Systems, das keine eigene
Zählzeile kennt) und darunter dieselbe virtuelle Liste wie der Record-Reiter: Tabellenkopf 36 px mit den Spalten „Feld" und
„Verweisender Record" zu gleichen Hälften, Zeilen 36 px mit Zebra, Hover `control-bg` und
Trennlinie. Links steht das Feld des verweisenden Records mit Array-Index (`Components[3].maker`,
`color-gray-1`, 13,76 / 400), rechts der Link auf den verweisenden Record (`color-primary`, 12,8 / 700, Hover
`color-accent-light` mit Unterstreichung, Fokusring wie beim Reference-Link des Record-Reiters)
und dahinter sein Typ als Trailer in `color-gray-2-text`. Der Link heißt für Screenreader „Den
Record … öffnen, der in … auf diesen Record verweist" und nennt so Record und Feld. Ein Klick
folgt dem Link wie die Referenz im Record-Reiter, bleibt aber im Reiter „Referenziert von", damit
man sich durch die Verweise bewegen kann; Zurück und Vor (Alt+←/Alt+→) gelten für beide Reiter.
Die Liste endet bei 10.000 Zeilen; darüber meldet eine zweite Zeile in `color-gray-2-text`, wie
viele von wie vielen Referenzen aufgelistet sind. Fehlt ein Teil des Index (Records, die sich nicht
lesen ließen, oder ein voller Index), steht unter der Zählzeile ein `.alert--warning` in der
vollen Breite des Körpers (x 16), auch wenn kein Record verweist. Der Index wird erst beim ersten
Öffnen des Reiters gebaut: bis dahin steht im Körper der Inline-Spinner (14) mit „Die Records, die
auf … verweisen, werden gesucht …" in `color-gray-1`, ein Fehler in `color-danger-text`, ein
geändertes Archiv in `color-warning` (D.8), ein Record ohne Verweise „Kein Record verweist auf
diesen Record." in `color-gray-2-text`. Der Reiter erscheint nur, solange ein Record gewählt ist;
wählt man einen Eintrag, kehrt die Vorschau zu „Text" zurück wie bei den anderen Reitern. Mit
dem Reiter sind es bis zu sechs Reiter neben Zurück, Vor und Kopieren; passen sie nicht in die
Leiste, bleibt rechts Platz für diese Schaltflächen und die Reiter scrollen waagerecht mit dem
Mausrad (`.tab-nav` hat `overflow-x: auto`), der gewählte Reiter wird sichtbar gehalten.

**D.35 Modellvorschau als Tab „Modell" mit 3D-Ansicht.** Wählt man ein Modell (`.cgf`, `.cga`,
`.skin`), öffnet die Vorschau den Tab „Modell" hinter „Audio"; andere Einträge kehren zu „Text"
zurück. Unter der Pfadzeile (D.14) steht eine `.facts-bar` ohne Richtung mit Vertices, Dreiecken
und Größe (Breite × Tiefe × Höhe in Metern, zwei Nachkommastellen), und im freien Platz rechts
daneben die Bedienhinweise in 12,8 / 400 `color-gray-2-text`, rechtsbündig und mit Auslassung
gekürzt; unter 120 px Breite entfallen sie, der volle Text bleibt im Zugänglichkeitsbaum. Darunter
liegt die 3D-Ansicht, mindestens 96 px hoch und so breit und hoch wie der Rest des Panels, auf
`surface-inset` mit einer Haarlinie 1 `color-gray-3` innen. Die Ecken sind quadratisch, weil sich
ein egui-Paint-Callback nur auf ein Rechteck beschneiden lässt (Abweichung, ADR-0031); alles um die
Ansicht folgt dem Designsystem. Das Modell ist Spielinhalt, kein Teil des Designs: es ist neutral
grau und mit einem Kopf- und einem Schräglicht beleuchtet, damit Flächen lesbar bleiben, und von
den Farbregeln ausgenommen. „Ansicht zurücksetzen" ist `.btn-ghost.btn-xs` (eine Routineaktion,
kein CTA), liegt 8 px vom Rand oben rechts über der Ansicht und ist deaktiviert, solange die
Ansicht das Modell eingepasst zeigt. Ziehen dreht, Rechts- oder Mittelklick-Ziehen oder
Umschalt-Ziehen verschiebt, Mausrad und Zwei-Finger-Zoom zoomen. Die Ansicht ist eine
fokussierbare Fläche („3D-Ansicht von …", Innenring −2 `color-primary` wie die Scrollflächen,
D.27): bei Fokus drehen die Pfeiltasten, Umschalt und Pfeile verschieben, `+` und `-` zoomen, Pos1
setzt zurück; die Pfeiltasten verlassen den Fokus nicht. Die Meldungen stehen anstelle der Ansicht
unter der Faktenleiste: Lesefehler, Grafikgerät fehlt, zu klein oder verloren und fehlgeschlagenes
Zeichnen in `color-danger-text`; zu groß, ohne Dreiecke und zu groß für das Grafikgerät in
`color-gray-1` (wie „zu groß" in der Vorschau); geändertes Archiv in `color-warning` (D.8). Das
Panel behält seine Höhe (236 px, frei verstellbar); wer mehr vom Modell sehen will, zieht den
Griff darüber.

**D.37 Fenster „Über P4K-Extractor" im Menü Hilfe.** Das Menü Hilfe führt als ersten Eintrag „Über
P4K-Extractor" (14,4 / 400 wie die Dropdown-Einträge, D.1) und öffnet ein Modal im Aufbau der
Schriftlizenz: Kopf mit Titel und Schließen-Icon, Fuß mit der CTA „Schließen". Das Modal hat die
normale Breite (`.krt-modal`), nicht die breite, die dem Formular-Modal vorbehalten ist. Der Körper
steht in 14,4 / 400 `color-gray-1`, also über der Mindestgröße von 10 pt der Fankit-Richtlinien,
Absätze 12 voneinander und nach dem letzten kein Abstand, damit der Körper über und unter dem Text
gleich weit von den Haarlinien von Kopf und Fuß steht: die Version (700), die Urheberzeile
(wörtlich die Copyright-Zeile aus `LICENSE-MIT`), die Lizenz mit der Quelltext-Adresse, der Hinweis
auf ein inoffizielles Fanprojekt ohne Verbindung zur und ohne Billigung durch die
Cloud-Imperium-Gruppe, dass keine Spieldaten mitgeliefert und nur die eigene Installation gelesen
wird, die Verantwortung für EULA und Terms of Service, der Eigentumshinweis und zuletzt die
Markenangabe. Die Urheberzeile und die Markenangabe sind Rechtstext und stehen in beiden Sprachen
im englischen Wortlaut. Nur Text, kein Fankit-Logo.

**D.39 Tab „Suche" durchsucht die Inhalte des Archivs.** Hinter „Lokalisierung" steht der Tab
„Suche" (Tab-Leiste wie D.1, Großbuchstaben, orange Unterstreichung). Er liegt neben dem
Pfad-Filter des Tabs „Durchsuchen": der Filter wählt Einträge nach Pfad, die Suche findet Text in
den Inhalten (ADR-0044). Kopf: das Suchfeld wie in Durchsuchen und Lokalisierung (Eyebrow „Suche",
Lupe, Hinweis in kursiv), rechts die CTA „Suchen" (die eine Hauptaktion; während der Suche die
Ghost-Schaltfläche „Abbrechen"); Enter im Feld startet ebenfalls. Darunter die Zeile „Suchen in"
mit drei Checkboxen (Records, Lokalisierung, Einträge, alle gewählt; ohne Quelle ist die CTA
gesperrt). Die Suche läuft nicht bei jedem Tastendruck, sondern auf Befehl, weil ein Lauf alle
Quellen liest; ein laufender Lauf zeigt den Spinner (`SPINNER_INLINE`) und „Suche läuft …" im
Rand der Vorschau (`preview_body`), wie die Meldungen vor und nach einem Lauf.

Das Ergebnis: eine einzige Warnleiste (`.alert.alert-warning`) mit einer Zeile je Hinweis, wenn
weitere Treffer existieren (je Quelle höchstens 1.000), wenn Records fehlen, weil das Archiv keine
DataForge hat, und wenn Einträge nicht durchsucht wurden (binär, zu groß, nicht lesbar, jeweils mit
Zahl). Verschlüsselte Einträge ohne passenden Archivschlüssel sind eine eigene Zeile derselben
Leiste mit der Ghost-Schaltfläche „Archivschlüssel eingeben", die den Schlüsseldialog öffnet
(`KeyReason::Menu`): Eine Suche öffnet den Dialog nie von selbst, sie zählt diese Einträge nur
(ADR-0044). Die Leiste hat den Rand des Panels wie die Segmentleiste (`SPACE_3`) und den
Außenabstand unten des `.alert` (`margin-bottom: 1.5rem`, `SPACE_6`). Darunter die Segmentleiste
(`.krt-segmented`, D.1) mit einer Quelle je Segment und der Trefferzahl als `.krt-segmented__count`:
eigener Text im Segment in 700, `color-gray-2-text` (im gewählten Segment `control-selected-fg`),
ohne Klammern, mit „+" hinter der Zahl bei weiteren Treffern, getrennt vom Namen durch 0,5 rem
(`SEGMENT_COUNT_GAP`); die Ziffern von Lato sind tabellarisch. Die Segmente tragen den Namen und
die Zahl auch als zugänglichen Namen, „Records 2". Die Segmentleiste wählt die Quelle, deren
Treffer die Liste zeigt, anfangs die erste mit Treffern; hat die gewählte Quelle keinen Treffer,
steht statt der Liste „Diese Quelle hat keinen Treffer."
Die Liste ist eine virtuelle Tabelle mit drei Spalten in Tabellenkopf und Zeilenhöhe von D.2
(Records: Record-Pfad, Feld, Text; Lokalisierung: Sprache, Schlüssel, Text; Einträge: Pfad, Zeile,
Text). Der Ausschnitt in der letzten Spalte steht in 13,8 / 400 `color-gray-1`, der Treffer darin
in 700 `color-white` (Daten sind weiß, Orange bleibt Aktion und Identität). Die gewählte Zeile
trägt den orangen Balken von D.2; die Liste ist eine fokussierbare Scrollfläche (D.24) mit dem
Namen „Suchtreffer". Der Fuß (`.list-panel__footer`) nennt Trefferzahl und Dauer. Texte aus dem
Archiv (Pfade, Schlüssel, Feldnamen, Ausschnitte) laufen durch denselben Filter wie die Ausgabe
der CLI: Steuer- und Bidi-Zeichen werden durch U+FFFD ersetzt. Lato hat für U+FFFD kein Glyph;
das Fenster zeigt das Ersatzzeichen wie jedes Zeichen ohne Glyph als Ersatzkästchen der
Schriftenkette und zeichnet kein eigenes Zeichen dafür. Die Zeilen der Liste werden einmal je
Liste, Sprache, Breite und Skalierung gesetzt und beim Scrollen wiederverwendet.

Ein Klick auf einen Treffer öffnet ihn: ein Record wird im Record-Baum gewählt (die Vorschau
wechselt auf „Record"), ein Eintrag in seinem Ordner mit aufgeklappten Ordnern darüber, ein
Lokalisierungstext im Tab „Lokalisierung" mit der Zelle der Sprache. Wartet das Ziel auf Daten
(DataForge, Lokalisierungsdateien), wird es geöffnet, sobald sie gelesen sind. Der Tab fügt
keine Farbe, Größe oder Form hinzu; alle Werte stammen aus `tokens.json`. Geprüft mit Renders in
en-GB und de-DE.

**D.40 Tab „Info" listet, was eine Modelldatei enthält.** Für ein Modell (`.cgf`, `.cga`, `.skin`,
`.chr`) steht hinter „Modell" der Tab „Info" (D.1 Tab-Leiste, 44 px). Er erfindet nichts Eigenes,
sondern folgt der Zusammenfassung des DataForge-Tabs: unter der Pfadzeile (D.14) in einer
Rollleiste die `.kv-list` mit Knoten, Meshes (der feinsten Stufe), LODs, Geometriedatei,
Materialbibliothek, Material-IDs, Knochen, Animationen und Chunks (Schlüssel 14,4 / 700 in
Versalien `color-gray-2-text`, Wert `detail-value-size` / 300 `color-data-fg`, Haarlinie unter
jeder Zeile), darunter je eine Überschrift im Stil der Panel-Titel (Versalien, 700, Haarlinie) und
eine Tabelle: „Detailstufen" mit LOD, Vertices, Dreiecken und Meshes, und, nur wenn das Modell
Chunks nicht gelesen hat, „Nicht gelesene Chunks (n)" mit Datei, Typ, Typ-ID und Verwendung
(„Bekannt, nicht gelesen" oder „Unbekannt, übersprungen"). Tabellenkopf 36 px mit `surface-input`
und 2 px `color-primary` darunter, Zeilen 36 px mit Zebra, Hover `control-bg` und Trennlinie, die
Spalten zu gleichen Teilen. Die Tabelle der Chunks nennt höchstens 200 Zeilen; die Zahl in der
Überschrift zählt alle. Die Materialbibliothek steht mit ihrem Zustand („objects/ship, 3
Teilmaterialien", „… (existiert nicht)", „… (nicht lesbar: …)"); die Animationen stehen als Zahl der
Animationen, die ein Export ohne Zusatz schreibt (additive nicht mitgezählt), und als „Nicht
gelesen", wenn sie nicht gelesen werden konnten. Alle Texte sind einmal je Modell,
Sprache, Breite und Skalierung gesetzt und liegen im Frame-Speicher von egui; ein Frame setzt
nichts neu. Meldungen (liest, nicht lesbar, Archiv geändert, zu groß) stehen anstelle der Liste
wie im Tab „Modell" (D.35); der Tab zeigt die Liste auch für ein Modell, das der Viewer nicht
zeichnen kann (Skelett, keine Dreiecke). Wählt man ein weiteres Modell, bleibt der Tab „Info"
gewählt.

**D.41 „Als GLB exportieren …" in der Vorschau-Leiste und im Menü Export.** Für ein Modell steht
links von „Kopieren" am rechten Ende der Leiste „Als GLB exportieren …" (`.btn-ghost.btn-xs`, eine
Routineaktion, kein CTA), deaktiviert, solange das Modell nicht gelesen ist oder ein Export läuft;
das Menü Export führt denselben Eintrag nach „Record-Export …". Beide öffnen das breite
Formular-Modal des Record-Exports (`.krt-modal`, D.17) mit Kopf „Als GLB exportieren" und Schließen-Icon und Fuß mit
„Schließen" und der CTA „Exportieren". Der Körper beginnt mit „Modell: …" in 12,8 / 400
`color-gray-2-text` und gliedert sich in Gruppen mit Gruppenüberschrift (D.17, Abstand
`modal-group-gap`): „Detailstufe" mit einem Radio je Stufe („LOD 0, 12 Dreiecke"), „Materialien"
mit einem Radio je Modus („Einfache Farben, eine kleine und schnelle Datei" und „Mit Texturen, eine
größere Datei"), bei „Mit Texturen" darunter die Gruppe „Texturgröße" mit einem Radio je Mip-Stufe
(„Volle Größe", „1/2 der Kantenlänge, Mip-Stufe 1" bis Stufe 3), und der Checkbox
„Vertex-Farben schreiben"; für ein Modell mit Animationen folgt die Gruppe „Animationen" mit der
Checkbox „Die n Animationen des Modells schreiben" (aus, bis man sie wählt); und „Datei" mit dem Pfadfeld (schreibgeschützt, Platzhalter „Keine Datei
gewählt") und der Schaltfläche „Datei wählen …"; sie öffnet den Speichern-Dialog des Systems mit
`.glb`-Filter, dem Namen des Modells und dem Ordner des letzten Exports. Liegt die Datei in einer
Star-Citizen-Installation, steht darunter dasselbe `.alert--danger` wie bei der Extraktion (ADR-0036)
und die CTA bleibt deaktiviert. Die CTA ist deaktiviert, bis eine Datei gewählt ist. Stufe,
Materialmodus, Mip-Stufe, Animationen, Vertex-Farben und Ordner werden gemerkt (Einstellungsdatei; ein Modus, den
diese Version nicht kennt, gilt als „Einfache Farben"); hat das nächste Modell die Stufe nicht,
steht die feinste gewählt. Die Werte der Info-Zeilen stehen einzeilig in der Wertspalte (Breite des
Fensters abzüglich Schlüsselspalte, Spaltenabstand und Rollleiste) und enden bei Überlänge mit „…";
ein Tooltip nennt den ganzen Wert. Namen und Gründe aus dem Archiv erscheinen ohne
Steuerzeichen.

**D.42 Fortschritt, Zusammenfassung und Fehler des GLB-Exports.** Der Export läuft im
Hintergrund; sein Fortschrittsfenster hat den Aufbau des Record-Exports (D.17): Titel „Modell wird
exportiert", beim Lesen den Inline-Spinner (14) mit „… wird gelesen …", beim Schreiben „x von y
geschrieben, n %" (in Byte-Größen) über dem Fortschrittsbalken, und im Fuß „Abbrechen" (während
des Abbruchs der Spinner mit „Breche ab …"). Danach steht ein nicht modales Fenster in der breiten
Form (`.krt-modal`) mit der CTA „Schließen": bei Erfolg „Modell exportiert" mit dem Zielpfad, den
Zählern (Knoten, Meshes, Materialien, Vertices, Dreiecke, Größe), bei Knochen deren Zahl, in
`color-warning` den Hinweisen, dass die Materialbibliothek fehlt oder nicht lesbar ist und das
Standardmaterial gilt, dass Material-IDs oder Bibliothekseinträge ohne Material darauf
zurückfallen, und in `color-gray-1`, wie viele Submeshes fehlen, weil ihr Material nichts zeichnet; wurden Animationen
gewählt, steht in `color-gray-1` „Animationen n, Kanäle m", und die aufklappbare Liste „Nicht
geschriebene Animationen (n)" nennt je Zeile eine additive Animation oder eine Animationsdatei, die
nicht lesbar war, mit Grund (höchstens 200 Zeilen);
darunter die aufklappbare Liste „Nicht exportierte Chunks (n)" mit einer Zeile je übersprungenem
oder unbekanntem Chunk. Die Zahl in der Überschrift zählt alle übersprungenen und unbekannten Chunks, auch wenn
die Liste nur 200 nennt. Mit Texturen steht davor die aufklappbare Liste „Fehlende Texturen (n)" mit
Material, Rolle (Diffuse, Normal, Specular), Datei und Grund je Zeile; das Material behält dort seine
Grundfarbe. Bei Abbruch steht „Export abgebrochen" mit einem Satz in `color-warning`, es wurde keine
Datei geschrieben und keine Teildatei liegt da. Ein Fehler steht in `color-danger-text` im Fenster
„Export fehlgeschlagen" und lässt das übrige Fenster bedienbar; fehlt der Archiv-Schlüssel, steht der
Hinweis in `color-warning` und der Schlüssel-Dialog (ADR-0027) öffnet sich von selbst, ein
geändertes Archiv ebenso in `color-warning`.
