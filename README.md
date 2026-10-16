# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through **TFM2 Editor Bridge**, allowing you to inspect and modify supported data in your active career while the game is running.

Created and maintained by **jal-io**.

> **Current release:** v1.2.0  
> **Required Bridge:** v1.0.20 or a compatible newer Protocol 25 Bridge  
> **Protocol:** 25  
> **Supported game version:** Teamfight Manager 2 v0.6.4  
> **Platform:** Windows

[Download the latest release](https://github.com/jal-io/tfm2-editor/releases/latest) ·
[Steam Workshop Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) ·
[Documentation](https://jal-io.github.io/tfm2-editor/)

## Important installation note

**The Steam Workshop subscription installs TFM2 Editor Bridge only.**

The desktop **TFM2 Editor application is not included** with the Workshop subscription and must be downloaded separately from GitHub Releases.

**Community v1.2.0 and Bridge v1.0.20 establish a new Protocol 25 generation.** Update both together; old Editor/Bridge generations are intentionally hard-blocked.

English is built into the Editor. Simplified Chinese (简体中文) is also supported and is included with the v1.2.0 release. A separate `lang.zh-cn.zip` language package is available as a GitHub release asset.

## What is a live editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to inspect and modify supported career data through the Bridge.

## New in v1.2.0

### Major performance and feature update — TFM2 v0.6.4

Community v1.2.0 and **TFM2 Editor Bridge v1.0.20 / Protocol 25** deliver major internal LEAN optimizations, improved responsiveness and new player-management tools.

- **Faster Match History:** optimized History delivery through a guarded **192 KiB** transport, retaining supported career evidence and the newest 10,000 completed matches.
- **Improved responsiveness:** Refresh Spam Guard, request coalescing and asynchronous data loading reduce redundant work and stale results.
- **Roster Tags:** manually tag individual players or a whole roster. **Follow Team** updates a tracked player's tag on transfers; **Custom** tags are protected.
- **Ability Rating:** a read-only average of eight fundamental abilities alongside the unchanged 12-attribute **Actual Rating**, shown in supported Editor, Search, filter and list workflows.
- **Search navigation:** selected players automatically load into Player Editor; supported Team selection puts your own team first.
- **Contract and transfer reliability:** improved supported Player/Staff contract editing, Free Agent signing and Player transfers.
- Updated to **Teamfight Manager 2 v0.6.4** using the official Stable API.

**Roster Tags are opt-in per player.** Untagged players do not automatically start tracking when joining a roster. Bulk Apply/Remove can be repeated with a new tag value. Existing Follow Team Free Agent behavior is preserved.

**Compatibility:** Protocol 25 deliberately blocks mixing Community v1.2.0 / Bridge v1.0.20 with the previous Community v1.1.0 / Bridge v1.0.9 generation. Update both components together.

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
- Automatic loading of selected players in Player Editor
- Ability Rating display, sorting and filtering where supported
- Own-team-first ordering in applicable Team selection workflows

### Player Editor

- Player name editing
- Player age editing
- Release contracted players to Free Agency
- Compact identity information
- Editing for all 12 player attributes
- Original 12-attribute Actual Rating and new read-only eight-ability Ability Rating
- Individual Roster Tags: Follow Team, Custom, Edit and Remove
- Primary, Secondary and Tertiary positions
- Position proficiency editing
- Hidden Actual Potential editing
- Champion Mastery editing
- Individual and bulk Champion Mastery changes
- Player Communication editing
- Condition and stamina editing
- Active salary and contract end-date editing
- Supported active same-team contract editing

Scouted or club-known Potential estimates are currently unavailable when the Stable API does not provide reliable populated evidence. They are not substituted with Actual Potential.

### Staff Editor

- Staff name editing
- Staff age editing
- Staff identity, role and age information
- Editing for all ten staff attributes
- Staff Communication editing
- Active salary and contract end-date editing
- Supported active same-team contract editing

Staff recruitment, signing and movement between teams are currently unavailable.

### Team

The Community release includes the expanded **Team** workspace.

- Team overview, roster and staff information
- Bulk Roster Tags for the selected team: Apply and Remove, with Custom Tags protected
- Team Condition
- Contract Center
- Finance Center
- Recruitment Center
- Competition Center
- League Standings
- Team Schedule
- Match History
- Finished series and set results
- MVP information where available
- Pre-Match Analysis where evidence exists
- Team Stats and Performance Center
- Strategy Presets
- Strategy editing
- Historical Performance
- Historical Synergy
- Full Historical Synergy Explorer
- Fans and popularity
- Existing merchandise editing
- Gaming House, facility and stadium information
- Team Summary

Historical analysis is based on supported completed-career evidence. Missing match, replay or scouting evidence is not invented, and historical results do not guarantee an optimal future strategy.

Facility, stadium and gaming-house information remains read-only.

### Recruitment and Player Movement

- Move supported contracted players between teams
- Move supported free-agent players through the game's Stable transfer system
- Release supported contracted players to Free Agency
- Automatic destination-team tag updates for explicitly tracked Follow Team players
- Supported roster and recruitment views
- Generated contract and salary behavior is handled by the game where applicable

Uncertain transfer actions are not automatically repeated.

### Economy

- Money editing
- Recruitment Budget editing
- Salary Budget editing
- Compact TFM2-style money input
- Live value previews
- Player-team finance editing

### Language and appearance

- Built-in English
- Simplified Chinese (简体中文)
- Persisted Language selection
- CJK-capable system font fallback
- Theme selector with System / Light / Dark
- Persisted Theme selection

## Temporarily unavailable

The new Stable API does not currently expose a safe replacement for every feature from the previous TFM2 Editor generation.

The following actions are currently unavailable in Community v1.2.0:

- Staff recruitment, signing and movement between teams
- Releasing Staff directly to Free Agency
- Transfer Always Success
- Instant Retry
- Manual free-agent contract signing/default generation
- Contract incentive editing
- Training XP multipliers

Some full-contract operations are restricted to supported active same-team contracts. Existing unsupported contract fields and incentives are preserved rather than overwritten.

TFM2 Editor does not emulate unsupported features through unsafe legacy writes.

## Compatibility safety

**Community v1.2.0 requires Bridge v1.0.20 or a compatible newer Bridge using Protocol 25**, for Teamfight Manager 2 v0.6.4 (Stable SDK ABI 10).

- **Community v1.2.0 + Bridge v1.0.20 / Protocol 25:** supported.
- **Community v1.1.0 + new Bridge v1.0.20:** **hard-blocked**.
- **Community v1.2.0 + old Bridge v1.0.9 / Protocol 24:** **hard-blocked**.
- Other unsupported or cross-generation combinations are rejected before any game changes.

Incompatible generations appear as **RED** and cannot perform game-data writes. Compatible future Protocol 25 Bridge patch releases may be accepted under the existing compatibility rules.

**Update both the desktop Editor and Workshop Bridge.** Do not mix v1.1.0 and v1.2.0 components. The earlier v1.1.0 release remains a historical release, not a compatible counterpart to the new Bridge.

## Installation

### Recommended: Steam Workshop Bridge

1. Subscribe to [TFM2 Editor Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) on Steam Workshop.
2. Download the latest desktop Editor from [GitHub Releases](https://github.com/jal-io/tfm2-editor/releases/latest).
3. Extract `TFM2.Editor.v1.2.0.Release.0.6.4.zip`.
4. Start Teamfight Manager 2 v0.6.4.
5. Enable **TFM2 Editor Bridge** in the Mods menu.
6. Restart the game.
7. Load your career.
8. Start `tfm2_editor_1.2.0.exe`.
9. Confirm the Editor is connected to **Bridge v1.0.20** with **Compatibility: OK**.

If the Editor shows **Disconnected**, click **Reconnect**.

### Simplified Chinese

Simplified Chinese is supported by the v1.2.0 desktop Editor.

The main release package includes the locale file. A separate `lang.zh-cn.zip` release asset is also available if you need to restore or install the language file separately.

The expected layout is:

```text
TFM2 Editor/
├── tfm2_editor_1.2.0.exe
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

## Important

**Recommended: Save your career before making changes with the editor.**

Always keep backups of important career saves.

TFM2 Editor modifies supported data in the active running career. Some changes may only become fully visible in the game after normal progression / **Proceed**.

Unexpected game behavior, data loss or save corruption may still be possible.

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

Support is completely optional. No Editor functionality is locked behind donations.

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
