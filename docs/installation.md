---
layout: default
title: Installation
---

# Installation

This guide applies to **TFM2 Editor v0.2.31** with **Teamfight Manager 2 v0.5.3**.

## 1. Download TFM2 Editor

Download the latest release from the repository's [Releases](https://github.com/jal-io/tfm2-editor/releases) page.

Extract the downloaded ZIP file.

The release package contains:

```text
TFM2 Editor v0.2.31 (Release)/
├── README.txt
├── tfm2_editor.exe
└── tfm2_modifier_bridge/
    ├── mod.mod_info
    └── tfm2_modifier_bridge.dll
```

## 2. Install TFM2 Modifier Bridge

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

## 3. Enable the Bridge

1. Start **Teamfight Manager 2**.
2. Enable **TFM2 Modifier Bridge** in the game's mod menu.
3. Restart the game.

The restart is required after enabling the bridge.

## 4. Start TFM2 Editor

1. Start **TFM2 Editor**.
2. Load your save in Teamfight Manager 2.
3. Use the editor while the game and bridge are running.

## Important

> **Back up your save files before using TFM2 Editor.**

TFM2 Editor is still under development. Editing game data may cause unexpected behavior or save corruption.

Use TFM2 Editor at your own risk.

## Updating

When a new version of TFM2 Editor is released:

1. Download and extract the new release.
2. Replace the old `tfm2_modifier_bridge` folder if the release includes an updated bridge.
3. Start the new TFM2 Editor executable.

Check the release notes for compatibility information before updating.

## Problems?

If the editor cannot connect to the game, first check that:

- TFM2 Modifier Bridge is installed in the correct `mods` folder.
- The bridge is enabled in Teamfight Manager 2.
- The game was restarted after enabling the bridge.
- Your TFM2 version is supported by the editor.

If the problem continues, report it through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).
