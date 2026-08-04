---
layout: default
title: Installation
---

# Installation

This guide applies to **TFM2 Editor v0.4.0**, **TFM2 Editor Bridge v0.2.43**, and **Teamfight Manager 2 v0.5.3**.

## Recommended installation

### 1. Install the Bridge from Steam Workshop

Subscribe to **TFM2 Editor Bridge** on Steam Workshop.

> **Important:** the Workshop item installs the **Bridge only**.
> It does **not** install the desktop TFM2 Editor application.

### 2. Download the desktop Editor from GitHub

Download the latest release from the repository's [Releases](https://github.com/jal-io/tfm2-editor/releases) page.

Extract the downloaded ZIP file.

The release package contains:

```text
TFM2.Editor.v0.4.0.Release.0.5.3.zip
├── README.txt
├── tfm2_editor_0.4.0.exe
└── tfm2_modifier_bridge/
    ├── mod.mod_info
    └── tfm2_modifier_bridge.dll
```

### 3. Enable the Bridge in-game

1. Start **Teamfight Manager 2**
2. Open the **Mods** menu
3. Enable **TFM2 Editor Bridge**
4. Restart the game

The restart is required after enabling the Bridge.

### 4. Start the Editor

1. Load your career in Teamfight Manager 2
2. Start **TFM2 Editor**
3. If needed, click **Reconnect**

When everything is correct, the header should show:

```text
Connected · Bridge v0.2.43 · Compatibility: OK
```

## Manual Bridge installation

If you prefer not to use Steam Workshop, you can install the Bridge manually from the GitHub release package.

Move the entire `tfm2_modifier_bridge` folder into the Teamfight Manager 2 `mods` folder.

Default Steam location:

```text
C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
```

Your Steam installation path may be different.

After copying the folder, it should look similar to:

```text
Teamfight Manager2/
└── mods/
    └── tfm2_modifier_bridge/
        ├── mod.mod_info
        └── tfm2_modifier_bridge.dll
```

Then enable **TFM2 Editor Bridge** in the in-game Mods menu and restart the game.

## Compatibility warnings

TFM2 Editor includes a permanent compatibility check between the desktop Editor and the installed Bridge.

### Compatibility: OK
Your Editor and Bridge match and the Editor can work normally.

### Compatibility: Warning
The installed Bridge is older or newer than the version expected by this Editor release.

Some features may not work correctly. Update the Bridge or Editor as instructed by the warning window.

### Compatibility: Not Supported
This Editor / Bridge combination is blocked.

In this state, the Bridge connection is closed and active game-data reads and writes are disabled until a compatible version is installed.

## Updating

When a new release becomes available:

1. Download the new desktop Editor from GitHub
2. Update the Bridge through Steam Workshop **or** replace the manual `tfm2_modifier_bridge` folder
3. Start the new Editor
4. Confirm the header shows the expected Bridge version and **Compatibility: OK**

## Problems?

If the Editor cannot connect to the game, first check that:

- The correct **TFM2 Editor Bridge** version is installed
- The Bridge is enabled in Teamfight Manager 2
- The game was restarted after enabling the Bridge
- Your Teamfight Manager 2 version is supported
- The Editor header does not show **Not Supported**

If the problem continues, report it through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).

## Important

> **Back up your save files before using TFM2 Editor.**

TFM2 Editor modifies data in the active running career. Unexpected behavior, data loss, or save corruption may still be possible.

Use the editor at your own risk.
