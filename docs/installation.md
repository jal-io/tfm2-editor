---
layout: default
title: Installation
---

# Installation

This guide applies to:

- **TFM2 Editor v0.5.0**
- **TFM2 Editor Bridge v0.2.59**
- **Teamfight Manager 2 v0.5.5**
- **Windows**

The Editor and Bridge should be updated together.

## Recommended installation

### 1. Install TFM2 Editor Bridge from Steam Workshop

Subscribe to **TFM2 Editor Bridge** on Steam Workshop:

[TFM2 Editor Bridge — Steam Workshop](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765)

> **Important:** the Workshop item installs the **Bridge only**.  
> It does **not** install the Windows desktop TFM2 Editor application.

Steam Workshop is the recommended way to install the Bridge because Steam can keep the Bridge files updated.

### 2. Download TFM2 Editor from GitHub

Download the latest release from:

[TFM2 Editor — GitHub Releases](https://github.com/jal-io/tfm2-editor/releases/latest)

For v0.5.0, download:

```text
TFM2.Editor.v0.5.0.Release.0.5.5.zip
```

Extract the archive.

The release package contains:

```text
TFM2.Editor.v0.5.0.Release.0.5.5/
├── README.txt
├── tfm2_editor_0.5.0.exe
└── tfm2_modifier_bridge/
    ├── mod.mod_info
    └── tfm2_modifier_bridge.dll
```

The included Bridge folder is provided for manual installation. If you use Steam Workshop, use the Workshop-managed Bridge instead of keeping a second manual copy active.

### 3. Enable the Bridge in Teamfight Manager 2

1. Start **Teamfight Manager 2 v0.5.5**.
2. Open the **Mods** menu.
3. Enable **TFM2 Editor Bridge**.
4. Restart the game.

Restart the game after enabling or updating the Bridge.

### 4. Start the Editor

1. Start Teamfight Manager 2.
2. Load your career.
3. Start `tfm2_editor_0.5.0.exe`.
4. If needed, click **Reconnect**.

When the installation is correct, the Editor header should show:

```text
Connected · Bridge v0.2.59 · Compatibility: OK
```

You can now use the supported Search, Player Editor, Staff Editor, Team, Transfers and Economy tools.

## Manual Bridge installation

If you do not want to use Steam Workshop, install the Bridge from the GitHub release archive.

Copy the complete:

```text
tfm2_modifier_bridge
```

folder into the Teamfight Manager 2 `mods` directory.

Default Steam location:

```text
C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
```

Your Steam installation path may be different.

The result should look similar to:

```text
Teamfight Manager2/
└── mods/
    └── tfm2_modifier_bridge/
        ├── mod.mod_info
        └── tfm2_modifier_bridge.dll
```

Then:

1. Start Teamfight Manager 2.
2. Enable **TFM2 Editor Bridge** in the Mods menu.
3. Restart the game.
4. Load your career.
5. Start `tfm2_editor_0.5.0.exe`.

## Updating from v0.4.2

TFM2 Editor v0.5.0 introduces a new compatibility boundary for Teamfight Manager 2 v0.5.5.

Do not mix the older v0.4.2 generation with the v0.5.0 release.

Update:

- the desktop Editor to **v0.5.0**
- TFM2 Editor Bridge to **v0.2.59**
- Teamfight Manager 2 to **v0.5.5**

If you use Steam Workshop, allow Steam to update the Bridge before starting the new Editor.

## Compatibility states

TFM2 Editor continuously checks the active Editor / Bridge combination.

### Compatibility: OK

The Editor and Bridge match and active game-data access is available.

### Compatibility: Warning

The detected environment requires attention. Follow the information shown by the Editor before making changes.

### Compatibility: Not Supported

The combination is blocked.

Active game-data reads and writes remain unavailable until a supported Editor / Bridge combination is installed.

## Connection problems

If the Editor cannot connect, check that:

- Teamfight Manager 2 v0.5.5 is running
- a career is loaded
- TFM2 Editor Bridge v0.2.59 is installed
- the Bridge is enabled in the Mods menu
- the game was restarted after enabling or updating the Bridge
- only the intended Bridge installation is active
- the Editor header does not report **Not Supported**

If the problem remains, click **Reconnect**.

For reproducible problems, report the issue through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).

## Important

> **Recommended: Save your career before making changes with the editor.**

TFM2 Editor modifies data in the active running career. Unexpected behavior, data loss, or save corruption may still be possible.

Use TFM2 Editor at your own risk.
