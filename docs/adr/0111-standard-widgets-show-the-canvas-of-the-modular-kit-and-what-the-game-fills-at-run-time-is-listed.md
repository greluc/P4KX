# Standard widgets show the canvas of the ModularKit, and what the game fills at run time is listed

Decided for issue #171 (the radar view), on top of ADR-0089 (screens), ADR-0105 (images) and
ADR-0106 (layout). Nothing here was observed in the game; every rule is read from the Records
and from what their names say, and the open ones are listed at the end.

## What the Records hold, on LIVE with `p4kx dcb export` and `ui screens`

- Every ship that has a radar screen names the same two canvases: `components/map/mapdisplaymaster`
  (1920 x 1080, coordinate method `auto`) and `components/mobiglas/bb_radar_c_app_starmap`
  (`bb_screenradar_c_app_starmap` on the screens of crew stations and turrets). Ships differ only
  by the brand style: the Avenger Titan, Gladius, Sabre, Hornet, Carrack, Starfighter, Cutlass,
  Corsair, Freelancer, 300i and Constellation all list them. The Cutlass dashboard's small radar
  screens (`Screen_Small_Radar1`, `Screen_Small_Radar2`) name `m_physical_screen`, which holds one
  canvas widget and no content of its own.
- `mapdisplaymaster` is a stack of canvas widgets: the star map, the interior map, the galaxy
  map, the velocity effect, the readouts and three render-to-texture copies. The 3D content lives in
  `WidgetHoloVolume` and `WidgetWindow` widgets whose root, `3D Root`, is 1 x 1 pixel in a raw
  canvas; the planes, rings and spokes inside it (`rc_radarmapscreen_hostplane_visuals_large`:
  five `WidgetCircle` with stroke widths of 40 to 100 and eight spokes as display widgets of
  0.2 % x 30 % to 40 %) are percentages of that pixel, and the circles shade with a
  `primitiveSettings.primitiveMaterialPath` such as `ui_r_radarmapscreen_radial_grid.mtl`. The
  scale of the plane, its tilt and its material are set by the game at run time; the Records hold
  no default for them.
- The widget types of the radar screens that had no renderer: `WidgetRuntimeImage` (the maps,
  `imageSource` names a picture the game renders), `WidgetWindow` (a window onto a 3D scene),
  `WidgetDustVolume`, `GeometryEntityWidget` and `VehicleEntityWidget` (3D models),
  `WidgetHoloVolume`, `WidgetLight` and `WidgetLighting` (lights), `WidgetLine` (`targetWidget`
  names the circle it ends at), `WidgetEditBox` (`textBox` names the text widget that shows what
  is typed), `WidgetSeparator`, and the `Component*` and control widgets (buttons, scroll bars,
  drop-down menus, list items, check boxes, toggles).
- A separator names `direction` (`Horizontal`, `Vertical`) and `style` (`Primary`, `Secondary`,
  `Tertiary`) and has no look of its own. The Records of `ui/buildingblocks/modularkit/standard/widgets`
  hold one canvas per variant, named `<direction>separator<style>widgetstandard`; its root is a
  custom shape that a brand style gives a background, a height of 2 or 4 and borders. The entries
  select on the tags of the root and on the tags of its parent: `Widgets.separator.horizontal`
  and `Widgets.separator.secondary` of the tag tree. The separators of the screens carry only
  `Widgets.separator`; the game adds the two tags when it shows the canvas, and nothing else in
  the Records refers to the canvases. The same holds for the components
  (`modularkit/standard/components`) and the other standard widgets.

## Decisions

- A separator draws the canvas of its variant in its box, as a canvas widget draws the canvas it
  names. The canvas is found by its record path; the parent of its root is the separator, which
  carries the tags of its direction and its style for the selectors of that canvas, looked up in
  the tag tree by tag path. A variant the Archive has no canvas for is listed (`the Archive has no
  canvas for this standard widget`). The other standard widgets and the components are not
  resolved this way yet: they need the labels, icons and states the components get from their
  `labelProperties`, `iconProperties` and fill style, and the cascade of ADR-0089 has to evaluate
  the modifiers those canvases use; they stay listed without a renderer.
- What the game fills at run time keeps its box (background and border, as for every widget) and is
  listed with its reason instead of `no renderer for this element type`:
  `the image is rendered by the game at run time` (`WidgetRuntimeImage`), `3D content is drawn by
  the game at run time` (`WidgetWindow`, `WidgetDustVolume`, `GeometryEntityWidget`,
  `VehicleEntityWidget`, `WidgetHoloVolume`) and `the line ends at a widget the game places at run
  time` (`WidgetLine`). The first two count in `images_left_out`. Lights and an edit box draw
  nothing of their own (the box of an edit box is its own, its text is the widget it names), so
  they are not listed. This is the whole of the radar's dynamic content: contacts, stalks, the
  star field, the planes and the markers belong to 3D scenes and to lists with rows at run time,
  which a still does not show.
- A custom shape or icon that names no vector graphic is the box its style gives it when that box
  has a background or a border, and says nothing; it is listed (`names no vector graphic`) only
  when the box draws nothing either. The separators of the standard canvases are such shapes.
- No picture of the host plane is drawn: its scale, tilt and material are not in the Records, and
  a still of a made-up plane would be an invented graphic.

## Not drawn, and the questions that stay open

- The rings, spokes and range marks of the radar plane, the star field and the map pictures (see
  above); the contacts and their stalks.
- Buttons, scroll bars, drop-down menus, list items, check boxes and toggles
  (`Component*`, `WidgetCheckboxControl`, `WidgetToggleControl`, `WidgetTextEntry`), and the other
  standard widgets such as the progress meters.
- Whether the engine adds more tags than the direction and the style when it shows a standard
  canvas is not known; the two tags are what the selectors of the separator canvases ask for.

## Considered Options

- Drawing a separator as a plain line from its own fields: its colour and thickness are in the
  canvas of the variant, and a made-up line would not follow the brand.
- Looking the tags up once per screen: the tag tree is small enough to scan for the few separators
  of a screen, and a cache would hold a map that only separators need.
- Drawing the host plane tilted as an ellipse with the radial-gradient texture: the angle and the
  scale are not in the Records.
