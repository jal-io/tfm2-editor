---
layout: default
title: Installation
---

# Installation

This guide applies to:

- **TFM2 Editor v0.5.2**
- **TFM2 Editor Bridge v0.2.75**
- **Teamfight Manager 2 v0.5.6**
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

For v0.5.2, download:

```text
TFM2.Editor.v0.5.2.Release.0.5.6.zip
```

Extract the archive.

The main release contains the Windows Editor and the files needed for manual Bridge installation.

If you want Simplified Chinese, also download the optional language pack:

```text
lang.zh-cn.zip
```

English is built into the Editor and does not require an external language pack.

### 3. Enable the Bridge in Teamfight Manager 2

1. Start **Teamfight Manager 2 v0.5.6**.
2. Open the **Mods** menu.
3. Enable **TFM2 Editor Bridge**.
4. Restart the game.

Restart the game after enabling or updating the Bridge.

### 4. Start the Editor

1. Start Teamfight Manager 2.
2. Load your career.
3. Start `tfm2_editor_0.5.2.exe`.
4. If needed, click **Reconnect**.

When the installation is correct, the Editor header should show:

```text
Connected · Bridge v0.2.75 · Compatibility: OK
```

You can now use the supported Search, Player Editor, Staff Editor, Team, Transfers and Economy tools.

## Simplified Chinese language pack

Simplified Chinese is distributed separately so the main release remains a clean English installation.

To install it:

1. Download `lang.zh-cn.zip` from the same GitHub release.
2. Extract it into the same folder as `tfm2_editor_0.5.2.exe`.

The result should look like:

```text
TFM2 Editor/
├── tfm2_editor_0.5.2.exe
└── locales/
    └── zh-CN.json
```

Do not move `zh-CN.json` into the game or Bridge mod folder.

After installation, use the **Language** selector in the Editor header to switch between English and Simplified Chinese (`简体中文`).

The selected language is remembered between restarts.

## Theme selection

The Editor header includes a **Theme** selector directly after Language.

Available options:

- **System** — follows the current Windows theme
- **Light** — forces Light mode
- **Dark** — forces Dark mode

The selected Theme is remembered between restarts.

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
5. Start `tfm2_editor_0.5.2.exe`.

## Updating from an older release

TFM2 Editor v0.5.2 uses the release combination for Teamfight Manager 2 v0.5.6.

Do not mix older Editor or Bridge generations with the v0.5.2 release.

Update:

- the desktop Editor to **v0.5.2**
- TFM2 Editor Bridge to **v0.2.75**
- Teamfight Manager 2 to **v0.5.6**

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

- Teamfight Manager 2 v0.5.6 is running
- a career is loaded
- TFM2 Editor Bridge v0.2.75 is installed
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
