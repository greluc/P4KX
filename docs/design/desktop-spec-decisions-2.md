# P4K-Extractor · Desktop-Vorgaben, Entscheidungen D.22 bis D.29

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
ihren Teilen; ein Teil in einem anderen Ordner als sein Kopf gehört zur Ansicht des Kopfes. Keine
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
der Editor den Fokus bekommt, einmal je Archiv; bis dahin, und wo kein Name passt, bleibt die
Liste zu, statt einen leeren Zustand zu zeigen. „Abfrage speichern …" ist Ghost neben der CTA
„Abfrage ausführen" (D.7, D.15). Die gespeicherten Abfragen stehen darunter als aufgeklappter
`.panel-header` mit 36-px-Zeilen: Name fett, nach 240 px mit Auslassung, dahinter die Abfrage
einzeilig in `color-gray-2-text`. Die Liste ist eine fokussierbare Rollfläche („Gespeicherte
Abfragen", D.27), ihre Zeilen bleiben eigene Tabulatorziele; die Vervollständigungsliste nimmt nie
den Fokus, der gewählte Eintrag rollt mit ↑ und ↓ in Sicht. Ohne gespeicherte Abfrage steht unter dem Kopf ein Satz in
`color-gray-2-text`, der sagt, wie man eine speichert, kein `.empty-state`: der Abschnitt ist
klein und der Editor darüber die Aktion. Ein Klick auf die Zeile lädt und startet sie;
„Umbenennen" (Ghost) und „Löschen" (Quiet-Danger) stehen als `btn-xs` rechts, weil das Löschen
die einzige zerstörende Aktion ist. Speichern und Umbenennen fragen den Namen in einem
`.krt-modal` ab; ein vergebener Name ersetzt beim Speichern die Abfrage (Hinweis in
`color-gray-2-text`) und sperrt das Umbenennen (Hinweis in `color-danger-text`). Löschen fragt
in einem `.krt-modal--danger` nach (D.3), dessen Fuß nach dem Modal-Specimen Ghost „Abbrechen"
und das rot gefüllte `.btn.btn-danger` „Löschen" trägt.
