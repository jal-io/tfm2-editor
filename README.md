# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through **TFM2 Editor Bridge**, allowing you to inspect and modify supported data in your active career while the game is running.

Created and maintained by **jal-io**.

> **Current release:** v0.5.4  
> **Required Bridge:** v0.2.79  
> **Supported game version:** Teamfight Manager 2 v0.5.8  
> **Platform:** Windows

[Download the latest release](https://github.com/jal-io/tfm2-editor/releases/latest) ·
[Steam Workshop Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) ·
[Documentation](https://jal-io.github.io/tfm2-editor/)

## Important installation note

**The Steam Workshop subscription installs TFM2 Editor Bridge only.**

The desktop **TFM2 Editor application is not included** with the Workshop subscription and must be downloaded separately from GitHub Releases.

Both the Editor and Bridge should be updated together.

The optional Simplified Chinese language pack is available as a separate GitHub release asset:

```text
lang.zh-cn.zip
```

English is built into the Editor and does not require a language pack.

## What is a live editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to inspect and modify supported career data through the Bridge.

## New in v0.5.4

### Teamfight Manager 2 v0.5.8 compatibility

TFM2 Editor v0.5.4 is a compatibility-focused update for **Teamfight Manager 2 v0.5.8**.

- Updated the desktop Editor to v0.5.4
- Updated TFM2 Editor Bridge to v0.2.79
- Updated compatibility protection for the new game version
- Preserved the existing v0.5.3 Community feature set
- No new user-facing features were added in this release

## Features

### Search

- Full Player database
- Full Staff database
- Full Teams database
- Quick Filters
- Advanced Search
- Actual Potential Min/Max filtering
- Saved Filters
- Shared Player and Staff Saved Lists
- Import and export
- Sortable and resizable columns
- Full-row multi-selection
- Click, Shift-click and Shift-drag selection
- Direct Player and Staff Editor navigation

### Player Editor

- Player name editing
- Compact identity section showing Name, ID, Position, Age and Team
- Editing for all 12 player attributes
- Primary, Secondary and Tertiary positions
- Position proficiency editing
- Hidden Actual Potential editing
- Automatic Potential Grade updates
- Champion Mastery editing
- Individual and bulk Champion Mastery changes
- Player Communication Level editing
- Read-only Pending Training XP information
- Active player contract and salary editing
- Compact two-column desktop layout
- Single-column fallback for narrow windows

### Staff Editor

- Staff name editing
- Compact identity section showing Name, ID, Role, Age and Team
- Staff search and selection
- Editing for all ten staff attributes
- Staff Communication Level editing
- Active staff contract and salary editing

### Team

The Community release includes the expanded **Team** workspace.

- Team overview and roster information
- Team Condition
- **Training XP multipliers for the player team**
- Contract Center
- Finance Center
- Recruitment Center
- Competition Center
- League Standings
- Team Schedule
- Match History
- Team Stats
- Strategy Presets
- Strategy editing
- Historical Performance
- Historical Synergy
- Full Historical Synergy Explorer
- Gaming House information
- Fan and facility information
- Team Summary

Historical analysis is based on completed career evidence and is intended to show what has worked historically. It does not guarantee an optimal future strategy.

### Player and Staff Management

- Move contracted players between teams
- Move contracted staff between teams
- Set contracted players to Free Agent
- Set contracted staff to Free Agent
- Create a contract and move a free-agent player to a selected team
- Create a contract and move a free-agent staff member to a selected team

Free-agent contract forms are filled with valid starting values and can be reviewed before the move is applied.

### Transfers

- Transfer Always Success
- Instant Retry
- Separate Player Management and Staff Management tools

### Economy

- Money editing
- Recruitment Budget editing
- Salary Budget editing
- Compact TFM2-style money input
- Live value previews

### Language and appearance

- Built-in English
- Optional Simplified Chinese (`简体中文`) language pack
- Persisted Language selection
- CJK-capable system font fallback
- Theme selector with System / Light / Dark
- Persisted Theme selection

### Compatibility safety

The Editor permanently displays the detected Bridge and compatibility state:

- **Compatibility: OK**
- **Compatibility: Warning**
- **Compatibility: Not Supported**

Known unsupported combinations disconnect active game-data access and block reads and writes until a compatible Bridge is active.

Older release generations are not compatible with the v0.5.4 release combination. Update both the Editor and Bridge together.

## Installation

### Recommended: Steam Workshop Bridge

1. Subscribe to [TFM2 Editor Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) on Steam Workshop.
2. Download the latest desktop Editor from [GitHub Releases](https://github.com/jal-io/tfm2-editor/releases/latest).
3. Extract `TFM2.Editor.v0.5.4.Release.0.5.8.zip`.
4. Start Teamfight Manager 2 v0.5.8.
5. Enable **TFM2 Editor Bridge** in the Mods menu.
6. Restart the game.
7. Load your career.
8. Start `tfm2_editor_0.5.4.exe`.
9. Confirm the Editor header shows:

```text
Connected · Bridge v0.2.79 · Compatibility: OK
```

If the Editor shows **Disconnected**, click **Reconnect**.

### Optional Simplified Chinese language pack

Download `lang.zh-cn.zip` from the same GitHub release and extract it into the same folder as `tfm2_editor_0.5.4.exe`.

The result should contain:

```text
TFM2 Editor/
├── tfm2_editor_0.5.4.exe
└── locales/
    └── zh-CN.json
```

### Manual Bridge installation

The GitHub release archive also contains the `tfm2_modifier_bridge` folder.

Move the entire folder to the Teamfight Manager 2 mods directory.

Example:

```text
C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
```

Your Steam installation path may be different.

## Mod compatibility

TFM2 Editor has been validated successfully with:

- **Real World Database '26**
- **League of Legends Champions by Silverbear**
  - 113 champions detected dynamically
- **Item Scroller**
- **Riot Games Item Expansion Pack**

Player, staff, team and champion data are loaded dynamically from the active database. Mods that add unrelated in-game functionality are outside the scope of Editor compatibility testing.

## Important

**Recommended: Save your career before making changes with the editor.**

TFM2 Editor modifies data in the active running career. Unexpected game behavior, data loss or save corruption may still be possible.

Use TFM2 Editor at your own risk.

## Documentation

- [TFM2 Editor Documentation](https://jal-io.github.io/tfm2-editor/)
- [Installation Guide](https://jal-io.github.io/tfm2-editor/installation.html)
- [Feature Guide and Screenshots](https://jal-io.github.io/tfm2-editor/features.html)
- [Changelog](CHANGELOG.md)

## Issues

Report bugs through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).

Please include:

- TFM2 Editor version
- TFM2 Editor Bridge version
- Teamfight Manager 2 version
- What you attempted to do
- What you expected
- What happened instead

## Support

TFM2 Editor is free to use.

[Support TFM2 Editor on Ko-fi](https://ko-fi.com/jalio)

## Source and attribution

TFM2 Editor and TFM2 Editor Bridge were created and are maintained by **jal-io**.

Copyright © 2026 jal-io.

The project is **source-available** under the repository's custom attribution license. You may view, study, fork and privately modify the source. Public redistribution and derivative releases must retain the required attribution and license terms.

See [LICENSE](LICENSE) for the full terms.

## AI disclosure

TFM2 Editor was created with AI-assisted coding and uses AI-assisted artwork. The project is developed, tested, packaged, maintained and released by **jal-io**.

## Disclaimer

TFM2 Editor is an unofficial community project and is not affiliated with or endorsed by Team Samoyed.

Teamfight Manager 2 and related game content belong to their respective rights holders.
