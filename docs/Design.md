
# Overview

This project is a Gameboy emulator built using the Bevy game engine.

The project's primary aims include:
- Migrating one of my oldest coding projects to modern software tech
- Showcasing my MIDI Graph project's Bevy integration
- Integrating with ROMs from the homebrew community
- Being cross-platform, supporting at least Linux, Windows, and Android
- Using tests to validate against various GameBoy emulator test ROMs
- Support ROM hacks fairly well
- Support Game Genie and GameShark codes

# Local Storage

## Location

Data is all stored in a common app-specific location:
- Inside the home directory on Linux (using XDG conventions for settings files)
- Inside the appropriate home directory location on Windows
- Inside an app-specific directory on Android such that it will be backed
  up to users' Google Drive (however downloaded ROM files should not be backed
  up)

## Contents

Types of data stored include:
- General settings, including settings for the GameBoy key UI overlay,
  picture upscaling mode, UI theme, and more
- ROM provider configurations in a JSON file
- ROM metadata listing, including known ROMs in a JSON file, including ROM
  identifier strings or other metadata
- A last-played timestamp mapping
- ROM files, if they've come from an external provider and have been played
- Manually-created save states stored in a subdirectory named by a ROM
  identifier string
- Automatic save states created when the app is closed or sent to the
  background, or if the ROM is quit to return to the home screen, stored in
  the same ROM subdirectory as manual save states
- Data saved by the game itself, stored in the ROM identifier directory,
  to emulate cartridge storage (SRAM chip and/or oscillator state)
- Input device mappings for keyboard and controllers
- Audio graph configuration files

Input device mappings should be added automatically when a new device
is plugged in (keyboard will be added straight away, and new controllers
whenever a new one is plugged in). These will be initialised to some
sensible default configuration.

### ROM Identifier

The identifier string for a given ROM is the SHA-1 hash of the entire
ROM file but as if the ROM header bytes were all set to zeroes (this
modification ensures two ROMs with only header content differing are
treated as the same game). The header bytes should start at 0x0104 for
this purpose.

### Provider Identifier

ROM providers are assigned a new UUID upon creation which is used to
refer to them in other places internally.

### General Settings

The settings will be stored in a JSON file called `settings.json` holding
an object of key-value pairs where the keys are "forceButtonOverlay",
"upscalingMode", "emulationModel", "sgbOverlayEnable", "fontSize",
"uiTheme", and "audioPreset". All values are integers.

### ROM Providers

This is stored in `providers.json`, holding an array where each item is an
object containing:
- uuid: a UUID assigned to this provider
- friendlyName: a friendly name given to the provider
- priority: an integer representing the order in which results from this
  provider will appear in the ROM list (a number from 1 through 5)
- lastFetched, an optional timestamp (Unix epoch time) when it was last
  successfully fetched
- absoluteLocalDirPath, an optional string for local directories
- remoteFileUrl, an optional string for a single file's HTTP URL
- remoteApi, an optional object containing information to query a remote API

For remote APIs, the `remoteApi` object will contain these fields:
- getUrl: the URL for fetching ROMs from
- pagination: an optional object containing information about how pagination
  is used
- responseItems: an object describing the location and structure of the
  array of ROMs
- downloadUrl: the URL with placeholders which, when the placeholders are
  replaced by either an ID or a filename (the placeholders must each be `{id}`
  or `{filename}`), can be used to download the ROM file

The object in the responseItems contains:
- itemIdJsonPath: the path to the field in the items list which holds a unique
  identifier for a ROM within the provider
- itemNameJsonPath: optionally the path to the field in the items list which
  indicates the ROM's name
- itemAuthorJsonPath: optionally the path to the field in the items list which
  indicates the ROM's author
- itemLicenseJsonPath: optionally the path to the field in the items list
  which indicates the ROM's license
- itemFilenameJsonPath: the path to the field in the items list which
  indicates the filename of the item

For remote API pagination, the object containing its configuration contains:
- pageCountJsonPath: the path to the field in query results indicating the
  page count
- queryPage: the name of the query parameter used in requests to indicate page
  number
- maxPages: optionally the maximum number of pages to fetch

### ROM Metadata

This is stored in `roms.json`, holding an array where each item is an
object containing:
- "id" (optional), the ROM identifier string, if it is known
- "providerId" (required), the provider identifier where the ROM came from
- "fileName" (required), the name of the file
- "friendlyName" (optional), a friendly name for the game if known
- "author" (optional), a string describing the author
- "license" (optional), a string describing the license
- "remoteProviderId" (optional), a string containing an ID known to the remote
  provider, if applicable

### Last-played Timestamps

Stored in `timestamps.json` (a separate file since it's frequently updated),
this holds a single array called `lastPlayed`, where items contain:
- "id", the ROM identifier string
- "timestamp", the Unix timestamp of when it was last either opened or had a
  save state stored

### ROM Files

When the user plays a ROM which comes from an external provider, it needs to
be downloaded first. It will be stored as `roms/{id}/{filename}`, where `{id}`
is the ROM identifier string, and `{filename}` is the filename which probably
has a `.gb` or `.gbc` extension.

### Automatic Save States

Stored in `roms/{id}/auto.gsv` where `{id}` is the ROM identifier. The
format of the file contents is to be based on the legacy C++ version of this
project which will be provided later.

When the user opens a ROM from the home screen, and if there is an auto-save
stored for that ROM, they'll have the options to either resume from that save
or to start with a default state.

### Manual Save States

Stored in `roms/{id}/{n}.sav` where `{id}` is the ROM identifier and `{n}` is
the slot number. The format of the file contents will be the same as for
automatic save states.

Up to ten manual save states can be stored for each ROM, using "slot numbers"
from 0 through 9.

### ROM Data

Stored in `roms/{id}/sram.dat` or `roms/{id}/oscillator.dat`. Saves a dump of
emulated memory when any save state is stored (noting the save state file
itself also includes the same data). This gets loaded only on reset of the
emulator or starting without resuming an auto-save.

### Input Mappings

Stored in  `input.json`. Saves a list of input mappings, where each list item
is for a single input device.

The items will each contain:
- `type`: an enumeration value, from "keyboard" or "controller"
- `controllerModelId`: optionally a representation of the controller model
- `map`: the list of key mappings for the device

The items in the `map` key contain:
- `keyId`: the keyboard keycode or controller button code which this map
  applies to
- `mapTo`: action which the key maps to; either this is an event from "quitApp",
  "quitROM", "resetROM", "saveState0", "loadState0", "saveStateModifier",
  "loadStateModifier", "speedUp", "speedDown", or "pauseAndResume", or it maps
  to the emulated GameBoy input state which is activated by this key, from
  "dleft", "dright", "dup", "ddown", "a", "b", "start", or "select"

### Audio Graph Configurations

Stored in `audio/preset{N}.json`. The initial default file `audio/preset0.json`
is automatically created, but can be modified and reset to defaults. Additional
preset files can be created as well, for up to 10 in total.

Each of these files is on its own compatible with MIDI Graph's Bevy plugin.

# Input

Input devices which are supported are:
- Keyboard
- Mouse
- Controller
- Touchscreen

Internally, a representation of the GameBoy keys is maintained at all times.
Most of the physical input devices supported act to manipulate this internal
state and do most of their actions indirectly by manipulating that state.
Some devices have other effects.

Other actions which can be mapped to are:
- Exit the app
- Quit the ROM (return to the home screen)
- Reset the ROM
- Save state in a particular slot
- Load state from a particular slot
- Increase emulation speed
- Decrease emulation speed
- Pause/resume emulation

## Keyboard

Basic key mappings are preconfigured, and can be changed from settings. These
mappings control which keyboard key maps to which GameBoy key.

In addition to mapping to GameBoy keys, other events can be mapped to
keyboard keys as listed above.

Whichever keys are mapped to the emulated A and B buttons are also used within
menu screens. Focused UI elements get "activated" by pressing the A button, and
the B button is used as a generic "back" button, dismissing modal UI elements or
navigating back to a previous screen if there isn't a modal element shown.

## Mouse

The mouse is the only input device type which does not manipulate the internal
GameBoy key state. The mouse is instead used to click on UI buttons, and the
scroll wheel can scroll list views and scroll views in the UI.

If a modal UI elements is shown in a menu screen, clicking anywhere outside
of it will dismiss it.

## Controller

Basic button mappings are for a controller the first time that controller is
detected, and can be changed from settings. These mappings control which
controller button maps to which GameBoy key.

In addition to mapping to GameBoy keys, other events can be mapped to
controller buttons as listed above.

## Touchscreen

The overlay is not shown on the initial loading or splash screens, but on
all other screens it may be shown depending on general settings. Tapping on
the overlay and releasing will activate and deactivate the buttons in the
internal GameBoy key state.

Like the mouse, the touchscreen can also interact with the UI in additional
ways, including tapping on a UI button to activate it, and swiping inside
a list view or scroll view to scroll it. Tapping on an item inside a list
view will focus it, though tapping an already-selected item will activate
it (the same action as clicking it with the mouse or pressing a key which
maps to the A key). Tapping outside a modal element will dismiss it if
there is one shown.

# Design

## Theme

When the game starts up, a theme should be chosen at random from 16
available themes. Each theme has a palette of three colours and an image
asset name. In addition to these full themes, there is a minimal theme
which doesn't include a background image or music to play in menu screens.

For most themes, there is a persistent background which first appears when
the home screen does, using the asset defined by the randomly-selected theme,
and it remains there during all screens but fades away when the emulation
screen starts running, fading back in if emulation ends.

The theme properties defined by each of the non-minimal themes are:
- Background image asset
- Primary colour
- Secondary colour
- Tertiary colour

| Name          | Primary    | Secondary  | Tertiary   | Image/audio? |
| ------------- | ---------- | ---------- | ---------- | ------------ |
| Minimal       | #bc31ff    | #e4bda3    | #8cb9ca    | No           |
| Forest        | #45cc44    | #938d2f    | #894900    | Yes          |
| Jungle        | #b25b26    | #979d6e    | #059747    | Yes          |
| Temple        | #a5b585    | #2a9338    | #90afac    | Yes          |
| Cyber         | #4489b2    | #b6d74c    | #a64cd7    | Yes          |
| Engine room   | #e2e6d2    | #f04711    | #04ed07    | Yes          |
| Deep sea      | #2665e9    | #18ab52    | #6fbdba    | Yes          |
| Starry night  | #bceeec    | #e6ec94    | #df94ec    | Yes          |
| Alien space   | #df94ec    | #3260aa    | #5aaa32    | Yes          |
| Black hole    | #876ccc    | #8219b1    | #4040de    | Yes          |
| Loneliness    | #6b9fd8    | #2a6db6    | #6d5c99    | Yes          |
| Cathedral     | #6d5c99    | #f22b58    | #f2dd2b    | Yes          |
| Runway        | #969696    | #b0b240    | #4077b2    | Yes          |
| Swamp         | #29b782    | #367fac    | #806d50    | Yes          |
| Fire cavern   | #9c7237    | #fd5151    | #fab915    | Yes          |
| Twilight city | #ec66ab    | #a717e0    | #3083be    | Yes          |
| In the clouds | #a1c2d9    | #d9d4a1    | #ffffff    | Yes          |

Besides the variable theme colours, another colour can be used despite
the theme currently in use:

| Name          | Value      |
| ------------- | ---------- |
| Error colour  | #9d213b    |

## Background

The visuals shown in non-emulation screens consists of various layers,
including the UI itself along with visual embellishments.

The rendered layers shown during all of the non-emulation screens will be
something like this:
- Background image
- Circuit board design wrapping the UI viewport
- Binary text effects
- User interface elements
- Particle effects
- Heads-up display (GameBoy key mapping overlay)

### Background Image and Particle Effects

An image is shown in the background (the image is chosen from the 16
options based on the randomly-selected theme chose at startup), and some
particle effects looking like coloured fireflies floating around. The
colour of these particles is the tertiary colour from the theme. The
background image has the ability to render at any opacity, which is used
to fade out when emulation starts and back in when emulation ends, and
will also have a random stuttering blink effect while it is shown. At
most, the background will be rendered at 30% opacity (this number should
be defined in a reusable constant somewhere in the code).

### Circuit Board Design

An additional layer is drawn in front of the background with a circuit
board drawing. The design of this is provided in a design image, but has
additional behaviours which can't easily be drawn:
- Rounded rectangles each represent a particular screen in the app's
  site map
- Rectangles animate to expand and fill the screen (with an outer margin)
  when the corresponding screen becomes shown, and the rectangles animate
  to shrink when the corresponding screen is exited
- The rectangle furthest to the right side represents the emulation screen,
  and when this one becomes active, the entire overlay fades away along with
  the other background layers (using the same animation properties defined
  in shared constants)

The behaviour of this design necessitates that it is rendered procedurally
rather than by rendering images from files. There will be a lot of maths
involved in translating and scaling individual pieces of this overlay.

### Binary Text Effects

When a circuit board rectangle has animated to its full size, another
layer will appear in front. This layer will animate a grid of text showing
zeroes and ones in monospaced font. They are by default invisible, but
animate in groups to appear. There may be any number of groups being
animated, where each currently-animating group is a random selection
of consecute digits across a row in the grid, and an animation runs
along that group passing over digits from left to right (with multiple
digits influenced at any given time). Digits animate by fading in until
30% opacity (this must be defined as a code constants, but not reused for
anything else) and then fading out. The fade duration and group size should
also be defined as constants. Once the group finishes animating, the group
is deleted.

## Sound

Audio will play from a MIDI file during non-emulation screens, and will
start playing on the splash screen. The audio graph will receive messages
to manage playback of the MIDI file, but will cut playback (remaining
loaded and ready for instruction) when the emulation starts running.
When the app moves from the splash screen to the home screen, a signal
will be sent to the audio context instructing where to skip to in
playback of a MIDI file.

When the audio graph configuration is switched from the Audio Settings
screen, it takes immediate effect, modifying the way sound is produced
while still in the settings menus. It naturally also affects how audio
sounds while a game is later being emulated, given that the same audio
graph instance persists between menus and gameplay.

While in the main Settings screen, changing the theme must immediately
change the audio playback as well, sending a signal to skip to the
corresponding playback position in the MIDI file, just the same as when
the Home screen first appears with that theme active.

## UI

The user interfaces should use Bevy UI components, avoiding third-party
plugins unless it's necessary to cleanly implement a component, and the
interface needs to adapt well to mobile and desktop screens in landscape
orientation.

User interfaces should be designed for a smooth experience using any
combination of mouse, keyboard, a controller, or touchscreen, such that
navigation is easy with arrow keys or a mouse, and actions can be taken
quickly using controller buttons or keyboard keys which are hinted in
the UI. Clicking action buttons with the mouse or tapping them on a
touchscreen should also work. For the keyboard and controller to be
used for navigation, UI elements needs to be focusable, and the focused
element can be changed using arrow buttons.

Each UI element will have a Bevy Component which defines focus properties
(except text labels, which cannot be focused). It will have an ID number
which is unique on that screen, and mappings for four directions indicating
which ID (if any) should become the next focus element if that direction is
pressed while the element is focused. A marker Component should indicatr
which element has focus, and when a new screen is presented, there should
be a particular element within that UI which has focus initially.

User interface elements are typically drawn using the theme's primary
colour, and accents such as highlighting focus or mouse hover will
typicaly use the secondary theme colour.

Touchscreen controls during gameplay are supported using zones on the
screen. The layout of these controls mimics the buttons from a real
GameBoy device, and shows highlights of buttons which are currently
considered active. These highlights reflect the internal emulation state.

UI elements typically have rounded rectangle outlines, and the radius of
the corners for all elements on all devices will use a single constant
device-independent pixel value.

Many UI elements use translucent fill areas. The opacity of these
areas must also be a single constant value applied to all relevant
elements, and should be a fairly low opacity of 20%.

Constants are also used for dimensions, including a common element
height (used for buttons, text inputs, and list items) and various
width constants. There should also be a common inner padding used to
separate things from their outlines such as the text inside a text input
or inside a list item. All of these are defined as device-independent
pixel values.

### Icons

Icons shown about the app are taken from a texture asset. They are shown
in places such as:
- The controller overlay during gameplay
- The action hints shown in the lower-right corner of the menu screens
- An error icon shown next to the error message if emulation failed

The layout of the icons texture is an image which could be considered a
16-by-16 grid, containing:
- The d-pad texture in the top-left (8-by-4 grid unit)
- The B icon underneath it aligned to the left (4-by-4)
- The A icon to the right of the A icon (4-by-4)
- The error icon underneath the B icon (4-by-4)
- The Select icon to the right of the d-pad icon and at the top (4-by-2)
- The Start icon underneath Select (4-by-2)
- A generic circle button icon in the top-right corner (4-by-4)
- A generic trigger button icon underneath the Start icon (4-by-4)
- A generic shoulder button icon underneath the generic circle (4-by-4)
- Two rows of 4 coloured Xbox- and PlayStation-specific icons underneath the
  trigger and shoulder icons, each icon being 1-by-1 grid unit, containing:
  - Xbox X, Xbox Y, PS square, PS triangle in the first row
  - Xbox A, Xbox B, PS cross, PS circle in the second row

### Hero images

Some larger images are arranged in another texture for use in some menu
screens. The texture is a 2-by-2 grid where:
- The top-left quadrant is a controller image for the input mapping menu
- The top-right quadrant is a cloud sync image for the ROM provider menu
- The lower-right quadrant is a disk storage image for the ROM data menu

### Loading Indicators

A grid of 2 by 2 squares which animate, rotating the colours clockwise every
quarter of a second. The size of the grid squares should be define as a constant
in terms of device-independent pixels.

The loading indicator should be used anywhere appropriate, which is on screens
which appear initially before all the content is loaded for display on that
screen. Menu navigation should be such that screen changes occur immediately
and quickly, to keep the interface feeling responsive to user input, which will
necessitate loading indicators since content loading won't always be so
responsive.

The grid colours are black along with the three theme colours.

Loading indicators cannot receive input focus and have no behaviours besides
the automatic animation.

### Scroll Views

A parent container for child elements which allows the inner content to
scroll vertically if its total height exceeds the height of the container.
The scroll bar appears along the right edge of the container, consisting
of a track and a thumb. The scroll bar should only be visible at all if
the inner content can be scrolled.

The track uses the theme's primary colour (but the same lower opacity used
by button backgrounds), and the thumb uses the primary colour.

The scrollbar can be focused, but only if it's visible. If not visible,
focus changes skip over it and redirect to its own directional
next-focus IDs. Even though it will usually have all four directional
next-focus IDs set, inputting to move up or down while it has focus
will never move focus, but rather scroll the view up or down.

### Text Labels

Text left-aligned within its bounds. There are two versions of this
stylistically: heading text (for the top of the UI) and description
text (typically much longer). Vertical alignment will typically be
top-aligned but in rare cases will be centered (such as button labels
shown to the left of the button where the number of lines of text being
shown will depend at runtime on the screen size of the device).

The colour of the text is the theme's primary colour.

This element cannot be focused, and mouse hover has no effect.

### Buttons

A rounded rectangle outline with text centered inside. The background
is translucent.

The height is a common dimension, and the width is one of several dimension
values used for smaller UI elements.

The outline and text use the theme's primary colour. The fill area also
uses the primary colour.

When the button has focus, the outline and text change to the
secondary colour.

On mouse hover, the background changes to the secondary colour.

It's also possible for buttons to be disabled. In this case, their
outline becomes hidden.

### List Views

A list of items along with column headings. The element as a whole has a
rounded outline and no background fill. The width and height are independent
of how many items are inside, such that there's empty space inside if there
aren't many list items inside, while if there too many items inside to show
all at once, the list is scrollable. The heading row is not scrollable,
staying fixed to the top while content scrolls. There's padding around the
area where the heading and item rows are drawn, such that the scroll view's
outline is spaced away from the inner content. This padding should use a
common dimension value.

The outer element has a small padding along the right side where a scroll
bar can be shown. The scroll bar is only shown when the view is
scrollable.

This element maintains a kind of meta-focus, where one of the list items
(if any) is remembered as the focus item even if the scroll view itself
doesn't have focus. When the scroll view receives focus, it redirects
focus to that remembered item if there is one.

The heading row appears as text labels spaced apart and an underline
underneath the entire row. The height of this row is the same as the
height dimension used for buttons.

The list items appear as text labels which line up with the heading row of
the scroll view. The height is the same as the heading row. Text in these
label elements will truncate to an ellipsis if they cannot be shown in full.

The sizing of columns for the heading row and item rows is defined by
percentage values in the code which instantiates the scroll view.

The focus direction key mappings for the list items must be such that
pressing up and down changes the focus up and down the list, while pressing
left or right changes focus to the outer scroll view, and then the scroll
view's focus mappings will be in effect for the next directional key press.

The scroll view element uses the primary theme colour for the outline,
and has no background fill. The heading row uses the primary theme colour
for its text labels and for the underline. The scroll bar, if shown, uses
the primary colour for its track and thumb, with the track having the same
low opacity used for other elements (such as button backgrounds).

When the scroll view or any of its list items has focus, the scroll view's
outline changes to the secondary colour, the heading row text and
underline also change to use the secondary colour, and the scrollbar track
and thumb change to the secondary colour as well.

The list items use the primary theme colour for their text labels, and
by default have no background fill.

List items do have a background fill of the theme's primary colour with
common translucency when they're the remembered meta-focus item and the
scroll view doesn't have focus. When the scroll view has focus, the
remembered meta-focus item changes to use the secondary colour for both
its text and its translucent background, and when an item itself has focus
these same colour changes will take effect.

On mouse hover, list items will show the secondary colour as a translucent
background fill.

### Text Inputs

A rounded rectangle outline with text left-aligned inside. The text displays
user input (or pre-filled values), and if there is no such text to show, a
placeholder string is shown instead. The background is translucent.

Text inputs can be disabled using a marker Component, which prevents them
from being able to receive focus or be activated in any way. Mouse hover
effects will also not occur. Focus changes (using direction buttons) which
would normally land on a disabled text input will act like a redirect,
following the text input's mapping for the pressed direction.

The height is a common dimension, and the width is a dimension larger
than the one used for buttons.

The outline and content text use the theme's primary colour, as does the
background fill. The placeholder text uses the theme's tertiary colour.

When the element has focus, the outline and the content text change to
use the secondary colour.

On mouse hover, the background changes to the secondary colour.

Text inputs can be disabled. When this is the case, the outline becomes
hidden.

When a text element has focus, there should be a flashing cursor indicating
the text entry position. Using the keys mapped to the D-pad left and right
will move the cursor position, while D-pad up and down will change the focus
element.

On mobile devices, the software keyboard will need to be shown when a text
input has focus. The entire UI should pan upwards so that the focused
element is shown halfway up the area of the screen above this keyboard.

### Multi-select Inputs

A drop-down selection of items. The main visual element has a rounded
outline and centered text plus a downward-pointing chevron aligned
on the right. The background is translucent. The text shown inside is
the currently-selected option from the available choices.

The dimensions of the main element are guided by the same principles as
for buttons.

When the element is activated, a secondary element appears as a
floating list of the options available. The focused element immediately
changes to the item in this list which is the existing selection. The
list appears as a rounded outline with some inner padding and inside
of that the list items are rectangles.

This floating list must disappear (despawn) whenever none of its items
has focus any more. The focus direction key mappings for these items
must be such that pressing up and down changes the focus up and down the
list, while pressing left or right dismisses the list view without
changing the selection (activating an item in the list will dismiss
the floating list view while also changing the selection).

The items in the floating list view share the same dimensions as buttons,
and the outer outline needs to accomodate that width as well as the full
list height while maintaining a padding as well.

Usually, the floating list should appear just underneath the main
element. If the main element is, however, shown near the bottom of the
screen, the floating list must be sure to never be off-screen, but
rather clamp its position so that none of its edges are ever off the
screen's edges.

The main visual element uses the primary theme colour for the outline,
the text, the chevron, and the background fill.

When this element has focus, the outline, inner text, and chevron all
change to use the secondary colour.

On mouse hover, this main element uses the secondary colour for its
background fill.

The hovering list uses the secondary colour for its outline, and black
background fill. The items themselves by default use the primary colour
for text and have no background fill, while the text changes to the
secondary colour on focus, and the background fills (in a non-rounded
rectangle shape) with a translucent secondary colour on mouse hover.

The floating list can be repurposed for use without the parent select
element, such as on the Home screen where choosing a ROM which has an
auto-save stored will open a submenu. In this scenario, the submenu will
appear alongside the item which was selected.

### File Pickers

File pickers are implemented as a button and disabled text input alongside
each other. This is not a separate custom-defined element itself.

# App Layout

The screens include:
- An initial loading screen
- Splash screen with app name and developer branding
- An interface demo screen
- Home screen with a ROM list and a button to open settings
- Settings screen with general settings, a list of ROM providers, and more
- Controller mapping setup screen
- ROM provider setup screen
- Per-ROM data management screen
- Audio settings screen
- In-game screen, including a message area for things like ROM load failure

The designs for these screens are based around a landscape orientation,
however portrait orientation is also supported. In the case of running
in portrait, the typical two-panel layouts of all of the menu and
settings screens will adapt to instead flow in a single column inside a big
scroll area. A limit on the container width applies to layout panels so that
they are not spaced too ridiculously on ultrawide screens. and be nested inside
one big scroll area.

## Loading screen

This should show an animated indicator in the lower-right corner, and only
show for a short time while assets for the remaining screens (besides ROM
assets) are all loaded. The indicator will not require any assets to be
loaded, but rather use code-generated components so it's not reliant on the
asset-loading operations.

## Splash screen

This includes:
- The Shining Emulator app logo prominently in the centre
- The Shining Grimace developer logo and the text "Shining Grimace" in the
  lower-right corner

## Interface Demo Screen

This is a screen only shown during development for the purpose of testing
appearance and behaviours of all of the styled UI elements.

## Home screen

The list of ROM files is shown here, taking all of the ROMs from the configured
providers, and showing them according to user-configurable sorting rules. The
list includes the ROM filename (or instead a friendly name if known), the name
of the provider which it came from, optionally the ROM's license, optionally
the author, and optionally the time it was last played.

The column for when the ROM was last played should read:
- "Today, HH:MM MM" if it was today
- "Yesterday, HH:MM MM" if it was yesterday
- The date in all other cases

By default, the order is most-recently played first, and for ROMs which haven't
been played before, in order of ROM provider (using the `priority` field in ROM
provider data, mixing results from multiple providers if they share the same
priority value), and then sorting by alphabetic order if needed.

The ordering can also be changed by configuring primary and secondary sorting
fields, each offering these choices:
- Most-recently played (default for primary field)
- Provider priority (default for secondary field)
- Alphabetical (also used as a default fallback sorting after the primary
  and secondary fields have been applied)

Since the ROM list needs to be populated from remote providers, a loading
state is needed for this screen, and an information feed should be shown
at the bottom as necessary to show messages around load failures or anything
else relevant. If some providers succeeded but others failed, once the loading
state resolves the content from the succeeding providers will be displayed.
Whatever gets loaded from providers has metadata stored locally so it need
not be tried again later (unless a manual re-sync is requested in settings
menus).

This list should be scrollable within its bounds using the mouse scroll wheel
or using keyboard or controler direction buttons.

The list view should have a focus state which changes the outline colour, as
well as a focused-item state, where the first item is focused by default. The
keyboard/controller can be used to change the focused item if the list view has
focus, and clicking the mouse on an item will focus that item (and focus the
list view itself if it didn't already have focus). There should be a highlight
background for the focused item which is more opaque if the list view has
focus, and mouse hover over a non-focused item will temporarily show a focus
state for that item.

When a ROM in the list is activated, it will either open straight away
(downloading if needed) in the emulation screen. If the file has been
played before and has an auto-save stored, a popup will appear with options
to "Resume Auto-save", "Cold Boot", or "Cancel".

This list might take some time to load, so there should be a loading state
shown in the list view's place (show the text "Loading..." inside the list
view's bounds) while the list is loading. If some ROM providers fail to load,
these should be listed as error messages underneath the list view.

If crucial settings files fail to be loaded, this should be shown as an
error message on the Home screen. The app should not panic and crash in
such cases, which would ceave the user mystified.

The background for the non-emulation screens first appears here, and is taken
at random from 16 possible assets (a theme is chosen at random once on app
startup and never changed while the app is running; this theme determines
which background image is shown as well as determining the three palette
colours used to draw user interfaces).

The error and warning messages shown will be displayed in the theme's
secondary colour. They're shown for a brief period (defined in a constant
in code) and fade away afterwards until they "despawn". When there are multiple
messages, they stack in a column.

An instruction is sent to the audio graph to seek to a specific marker anchor
when the home screen starts up. The marker anchor number is theme-dependent.
This concept is defined in the MIDI Graph library.

On the home screen as well as all settings screens, an overlay is shown in the
lower-right of the screen with icons indicating which buttons of the main
active input device are being used for activating UI elements or for navigating
back. When a controller is connected, the icons will represent controller
buttons, else they'll represent keyboard keys. This is the same device
selection logic as the "primary input" shown in the Settings screen.

On the home screen, the overlay will read "Quit" for the back-navigation
button, while on settings screens the overlay will read "Back".

The overlay hint for the current action (next to the back-navigation hint)
should be context-sensitive. For example, it will show "Select" when some
input elements have focus, such as buttons or text inputs.

## Settings screen

View some general settings, the list of ROM providers, the primary input device,
and a list of ROMs which have any data stored in the local storage.

Under General Settings, these options are available:
- A drop-down selection for whether the GameBoy button state overlay is shown at
  all times, including a description text underneath it which explains that if
  this is not enabled, the overlay is only shown on touch devices with no
  non-touch input devices available
- A drop-down selection for the emulated GameBoy model, choosing between
  "Best for ROM", "GameBoy Mono", and "Super GameBoy". "GameBoy Mono" disables
  color and Super GameBoy features; "Super GameBoy" disables color.
- A drop-down selection for whether the Super GameBoy border is shown
- A selection for the upscaling mode, choosing between "None", "2x", "3x", and
  "4x", where "None" is the default option
- A selection for UI scaling, choosing between five percentage-based sizes
- A selection for the UI theme, choosing between "Random" (the default),
  "Minimal" (an option with no background image or menu music), or any of the
  16 themes (to force a theme to always be used rather than one being chosen at
  random on every launch)
- A drop-down selection of a "primary input device", which is not actually a
  saved setting but rather allows choosing a device to edit using the Edit
  Mappings button underneath it

Like the ROM list on the home screen, the two data views on the settings
screen need to be scrollable lists.

The primary input device shows only the device name in a drop-down selection,
which will be "Keyboard" for the keyboard, and a best attempt at a human-friendly
name for controllers. This selection is paired with a button to edit a mapping.
The "primary" input device is the one which dictates the styling of the button
action hints shown in the bottom-right corner of menus, but otherwise all
connected devices will provide input to the system according to their respective
input mappings that are stored.

The ROM data list will list the ROM name, the most-recently-played date, and
how much storage space is consumed for the particular game (not including
the ROM file itself if it came from a local ROM provider).

Next to each of those list views, buttons are there to manage list entries,
such as to delete an item, edit one, or to create a new item. Some items (such
as the keyboard mapping and the Homebrew Hub integration) cannot be deleted.
In the case of Homebrew Hub, it cannot be edited either. Buttons will be
disabled to reflect this.

As well as the lists and management buttons for those lists, other elements
exist for opening the audio settings screen.

## Input device mapping setup screen

Configure the mappings for an input device. Mappings can be edited, or reset
to default settings.

Mappings can be edited for the keyboard (though this mapping set cannot be
deleted) or for a controller (auto-added whenever a new controller device is
detected). Mappings can be unset to have no button mapped to a given action,
though unsetting the D-pad, A, or B buttons will auto-select instead a
sensible default (so as to not leave the user unable to navigate UIs any more).

For the most part the way they are configured is the same, but in the case of
loading and saving save states, keyboard is configured differently to
controllers. For the keyboard, a modifier key is assigned (such as "leftCtrl"
which is the default) that must be held while pressing a number key to load
or save the state in that slot, whereas for controllers a button can be
assigned to load or save specifically the state in slot 0.

## ROM provider setup screen

Configure, test, and enable or disable ROM providers. Some providers are built
in by default, and additional ones can be added. Local directories can be
configured as providers, and external sources can be configured if they're
either a single file from a URL, or can be queried using an API like what
gbdev.io has.

An integration with the gbdev.io API is preconfigured by default (initialised
when the app data store is created) and cannot be removed from settings but
can be disabled so that its ROMs are not shown in the ROM list. The integration
is formed from two ROM providers in settings, one for the GB platform and one
for the GBC platform (and the `order` key will be the same for these two
providers so that the results are mixed together in the ROM list). The `order`
key for the built-in integrations is 3, and the default value pre-filled for
a new provider is 1.

If a ROM provider is disabled, its ROMs will not be listed on the Home screen,
however any previously-stored data for that provider or for its ROMs will
not be deleted. Re-enabling the provider will restore access to the data that
had been downloaded previously.

Integrations which load test ROMs (for validating emulator correctness) are
also preconfigured by default, and cannot be removed, but disabled by default.

## Data Management screen

View detailed information about the data stored for the chosen ROM, including
a list of all of the files stored for it. If the game has SRAM or oscillator
storage files, they can be deleted from here.

## Audio Settings screen

Presets can be created or edited using this screen which features many drop-down
selections and some other options to control the audio sound for each of the
four channels of the emulated system.

Channels 1, 2, and 4 can have their "oscillator" changed, supporting square wave
(the default), triangle wave, sawtooth wave, LFSR noise, a sampler for built-in
samples (which makes a drop-down selection visible to choose which sample to use)
or a sampler for user-selected samples (which makes visible a file picker
underneath allowing selection of a `.wav` audio file).

Channels 1 and 2 support choosing an effect to modulate the sound, supporting
duty cycle (the default), low-pass cutoff, high-pass cutoff, notch filter
frequency, or vibrato.

## Gameplay

When this screen starts, the background will fade away, and the emulation
render viewport will be shown in front immediately. Loading a ROM could take
some time, since at least a local file is being read (and if the ROM comes from
an external provider and hasn't been downloaded yet, it will need to be
downloaded to somewhere local first). A loading indicator will be shown in
front of the emulation viewport

Heads-up display elements can appear over the emulation render. This may
include the GameBoy controller overlay (depending on settings and what kind
of input devices are connected), as well as brief messages such as "state saved
in slot 0", and an input for Game Genie and GameShark codes. The messages which
appear on this screen behave the same as the ones on the Home Screen.

When the input for Game Genie and GameShark codes is opened, emulation
automatically pauses, and the on-screen keyboard appears on mobile devices.
The user types a code and presses Enter, and a message pops up stating
whether the code was successfully applied or not. If the code is submitted,
emulation resumes automatically, regardless of whether the code was valid or
not. A button is shown alongside this input as a submit button which acts the
same as the Enter key when the input os focused. Cancelling this popup also
resumes emulation.

Whether a code is meant for GameShark or Game Genie should be inferred from its
format, where GameShark codes have three groups of three-digit hexadecimal
numbers separated by hyphens, and Game Genie codes consist of eight
hexadecimal digits.

When a ROM cannot be successfully loaded and started, or if a ROM crashes while
running, the render image will not be shown, and an error message will appear
in the middle of the view. The error message has information about what
happened. The possible messages are these:
- The file could not be opened for reading (with more details)
- The file is not a GameBoy ROM
- The ROM header could not be read
- The ROM header contains invalid vaues (with more details)
- Emulation stopped on an invalid instruction (with more details)
- An unknown error occurred (with more details)

While emulation is running, the button hints for the primary input device shown
in the bottom-right corner of menu screens is not shown by default. When there
was any kind of error, triggering a message to be shown, the overlay appears.
During gameplay, the overlay can also be made to appear if the user presses a
key on the primary input device which is not mapped to any action, and in that
case, the hint for "Back" will instead show the hint for "Quit" if there is
one assigned to that action, and the other hint will present the key for the
"Pause" action.

# Emulation Architecture

The emulation must be decoupled from the app's renderer. It should write to
a representation of the GameBoy's screen, writing RBG pixel values to a 160x144
pixel frame buffer taken from a ring buffer, and further logic (which might
apply effects such as upscaling) should be decoupled from the emulation.

# Audio

Audio will be handled using the MIDI Graph tool created by Shining Grimace,
specifically the Bevy integration for it, available on GitHub.

This library allows creating an "audio context", which is a Bevy resource
for managing playback. It plays audio by loading an "audio graph" from a JSON
file which defines what notes will sound like, and either playing a MIDI file
or playing notes sent as individual events.

The playback graph will be loaded from a JSON file which is built into the app,
and this will configure a graph that supports all GameBoy sound capabilities
in an authentic way.

Custom audio graph configurations are also possible by configuring these
in the Audio Settings screen. This screen manages JSON files which are
compatible with the MIDI Graph plugin.
