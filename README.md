# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through **TFM2 Editor Bridge**, allowing you to inspect and modify supported data in your active career while the game is running.

Created and maintained by **jal-io**.

> **Current release:** v0.4.2
> **Required Bridge:** v0.2.49
> **Supported game version:** Teamfight Manager 2 v0.5.4
> **Platform:** Windows

[Download the latest release](https://github.com/jal-io/tfm2-editor/releases/latest) ·
[Steam Workshop Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) ·
[Documentation](https://jal-io.github.io/tfm2-editor/)

## Important installation note

**The Steam Workshop subscription installs TFM2 Editor Bridge only.**

The desktop **TFM2 Editor application is not included** with the Workshop subscription and must be downloaded separately from GitHub Releases.

## What is a live editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to inspect and modify supported career data through the Bridge.

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

Teams Search is included as a database and selection tool. Development-only Team editing and research tools are not included in the public Community release.

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

### Player and Staff Management

- Move contracted players between teams
- Move contracted staff between teams
- Set contracted players to Free Agent
- Set contracted staff to Free Agent
- Create a contract and move a free-agent player to any selected team
- Create a contract and move a free-agent staff member to any selected team

Free-agent contract forms are filled with valid starting values and can be reviewed before the move is applied.

### Recruitment

- Transfer Always Success
- Instant Retry
- Separate Player Management and Staff Management tools

### Economy

- Money editing
- Recruitment Budget editing
- Salary Budget editing
- Compact TFM2-style money input
- Live value previews

### Compatibility safety

The Editor permanently displays the detected Bridge and compatibility state:

- **Compatibility: OK**
- **Compatibility: Warning**
- **Compatibility: Not Supported**

Known unsupported combinations disconnect active game-data access and block reads and writes until a compatible Bridge is active.

## Name editing

Player and staff names can be edited directly in their respective editors.

- Leading and trailing spaces are removed.
- Empty names are rejected.
- Control characters are rejected.
- Names are limited to 100 characters.
- Search and editor selection data refresh after a successful change.

This is especially useful for free-agent players and staff whose names cannot always be changed through the normal in-game interface.

## Installation

### Recommended: Steam Workshop Bridge

1. Subscribe to [TFM2 Editor Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) on Steam Workshop.
2. Download the latest desktop Editor from [GitHub Releases](https://github.com/jal-io/tfm2-editor/releases/latest).
3. Extract the release archive.
4. Start Teamfight Manager 2.
5. Enable **TFM2 Editor Bridge** in the Mods menu.
6. Restart the game.
7. Load your career.
8. Start `tfm2_editor_0.4.2.exe`.
9. Confirm the Editor header shows:

```text
Connected · Bridge v0.2.49 · Compatibility: OK
```

If the Editor shows **Disconnected**, click **Reconnect**.

### Manual Bridge installation

The GitHub release archive also contains the `tfm2_modifier_bridge` folder.

Move the entire folder to the Teamfight Manager 2 mods directory.

Example:

```text
C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
```

Your Steam installation path may be different.

## Contract editing note

TFM2 Editor edits existing active player and staff contracts.

Full contract renewal, where the game creates a separate future replacement contract, is not included yet.

## Mod compatibility

TFM2 Editor has been validated successfully with:

- **Real World Database '26**
- **League of Legends Champions by Silverbear**
  - 113 champions detected dynamically
- **Item Scroller**
- **Riot Games Item Expansion Pack**

Player, staff, team and champion data are loaded dynamically from the active database. Mods that add unrelated in-game functionality are outside the scope of Editor compatibility testing.

## Known limitations

- Champion Mastery can reopen too large after high mastery values.
- Champion Mastery can auto-maximize near the right edge of the main window.
- Development-only Team workspace and Team research tools are not included in Community.
- History / Stats Over Time is not included.
- Accepted Renewal is not included; contract editing affects the active contract.
- CJK font fallback is not ready.
- Community v0.4.2 ships with embedded English only.

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
