
# Stage 1 - Base App

Create an app with asset-loading functionality and only some static screens.

Assets:
- assets/images/shining-grimace-logo.png
- assets/images/shining-emulator-logo.png
- assets/fonts/UbuntuMono-Regular.ttf
- assets/images/theme-*.png
- assets/images/icons.png

Steps:
a. Install Bevy 0.19 (using the first release candidate, version 0.19.0-rc.1)
   with Cargo audio features disabled, and set up the App with a 2D camera and
   showing the loading UI
b. Include the flake.nix file from Bevy's documentation on Linux dependencies
c. Define app states and a resource to hold all asset handles
d. Initialise on app startup a resource which chooses at random one of the
   16 themes and stores its 3 palette colours and background asset name in a
   Bevy resource (background asset is "theme-*.png" where * is 1-16)
e. Implement the loading functionality, and the splash screen, and progress to
   the splash screen once all assets are loaded (use the UbuntuMono font)

# Stage 2 - Local Storage, User Input, User Interfaces

Implement local storage, ROM providers, and settings screens.

Assets:
- assets/images/controller.png
- assets/images/storage.png
- assets/images/sync.png

Steps:
a. Create functionality (using modular coding patterns, and a Bevy plugin if
   appropriate) to initialise, fetch, or save, of the local storage data
   types
b. Create a plugin for collecting raw inputs from keyboards and controllers
c. Create a plugin for managing the rendering of the background image, including
   fading it in and out as needed, and rendering the particle effects
d. Create a plugin for managing the circuit board overlay, including code to
   manage all of the animations
e. Define common reusable UI element styles, and build the interface demo screen
f. Create a plugin for managing the binary text effects background layer, including code
   to manage all of the animations
g. Use the UI components to build the Home screen (without the ROM list working
   yet), making this now the screen shown after the splash screen
h. Use the UI components to build the Settings screen (without the ROM provider
   list or the ROM list working yet), and make sure the button hints in the
   lower-right corner of menu screens is functional, is based on the inferred
   primary input device, and adapts to the current UI context
i. Add UI Scaling option in the settings screen, with a drop-down offering five options
   to choose from
j. Use the UI components to build the ROM Provider screen, and make the ROM lists
   on the Home screen and Settings screen work, as well as the providers list in
   Settings
k. Use the UI components to build the input device mapping screen
l. Use the UI components to build the ROM data storage screen
m. Use the UI components to build the Audio Settings screen (but without the
   audio playback being implemented yet)
n. Remove the Interface Demo screen's code

# Stage 3 - Android App

Create an Android version of the app.

Steps:
a. Use the conventional Android app structure for a native activity that's
   compatible with Bevy and sets the minimum Android version to Oreo
b. Build the Rust crate using a Gradle task
c. Ensure screen rotation works well and that the software keyboard works
   as needed
d. Ensure local storage is functional for all supported Android versions for
   local settings storage and ROM download files

# Stage 4 - Rendering Setup

Without the emulation yet, implement a rendering structure.

Steps:
a. Implement the design features of the emulation screen: fade background
   elements away, fade audio away, show a placeholder error message "Not
   Implemented", and allow the user to navigate back to the Home screen
b. Create a plugin for writing to GameBoy frames borrowed from a ring buffer,
   drawing random greyscale pixels every frame, scheduled to produce frames
   as close as possible to the refresh rate of a real GameBoy
c. Create a plugin for rendering the last-written GameBoy frame buffer

# Stage 5 - Audio Setup

Still without emulation, integrate the MIDI Graph Bevy plugin and the
music playback for menus.

Assets:
- assets/audio/music.mid
- assets/audio/sample-*.wav

Steps:
a. Add the plugin to the project
b. Load the MIDI file and defeult audio configuration during the loading
   screen
c. Implement the track control as per the current theme on the transition from
   the Splash screen of the app
d. Ensure that playback stops when a game is launched from the Home screen, and
   starts again when navigating back
e. Ensure everything in the Audio Settings screen works as expected
f. Ensure that changing the theme in the Settings screen immediately seeks to
   the corresponding audio playback position

# Stage 6 - Emulation

Implement the whole emulation module and hook it into the existing
rendering and device input infrastructure.

To be written.

# Stage 7 - Emulation Validation

Test the emulation using GameBoy validation ROMs.

Steps:
a. Add a ROM provider, disabled by default, which loads publicly-available
   ROMs that validate emulation correctness
b. Add integration tests to run emulation on validation ROMs

# Stage 8 - Fixing all known issues

Resolve all known issues to to get the product to a production-grade point.

Steps: 
a. Test everything - all possible things - and keep the issues list below updated
b. Fix everything on the list below
c. Finalise all assets (app icons, hero images, audio samples, menu music)

Known issues:
- Think animations in menus: corrode away layouts on transition?
- "rhythm-land" reports that this is an inaccurate emulator
- Audio keeps running, holding the current notes, when the game is paused
- Audio needs to be tweaked to match a good reference video
- On Android, the menu music shows audio channels that are not a well-volume-balanced mix
- On Android, emulation seems to keep running when the app gets backgrounded
- On Android, the app goes blank on screen rotation
- Controllers haven't been tested on Android
- Windows hasn't been tested

# Stage 9 - Polish & Embellishments

Polish everything, and add extra unnecessary things for charm and replayability.

NOTE: Move some of these up?

Steps:
a. Consider accessibility
b. Consider multi language support
c. Consider compatibility with libretro cores
d. Revise the Homebrew Hub integration: are we using it within its terms, and can the ROM list use the most highly-regarded ROMs only?
e. Check is vsync enabled, or any other possible cause of high CPU usage in settings
f. Make sure all dead-code annotations are removed
g. Consider animations: UI trees fading in/out on screen change, popup UIs scaling
   in/out and fading in/out on appear/disappear, row focus highlights in list views
   translating from one to another, scrolling being ease-out rather than instant
h. Easter eggs?
