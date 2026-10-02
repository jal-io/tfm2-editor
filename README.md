# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through **TFM2 Editor Bridge**, allowing you to inspect and modify supported data in your active career while the game is running.

Created and maintained by **jal-io**.

> **Current release:** v1.0.0  
> **Required Bridge:** v1.0.1  
> **Supported game version:** Teamfight Manager 2 v0.6.2  
> **Platform:** Windows

[Download the latest release](https://github.com/jal-io/tfm2-editor/releases/latest) ·
[Steam Workshop Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) ·
[Documentation](https://jal-io.github.io/tfm2-editor/)

## Important installation note

**The Steam Workshop subscription installs TFM2 Editor Bridge only.**

The desktop **TFM2 Editor application is not included** with the Workshop subscription and must be downloaded separately from GitHub Releases.

Both the Editor and Bridge should be updated together.

English is built into the Editor. Simplified Chinese (简体中文) is also supported and is included with the v1.0.0 release. A separate `lang.zh-cn.zip` language package is available as a GitHub release asset.

## What is a live editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to inspect and modify supported career data through the Bridge.

## New in v1.0.0

### Teamfight Manager 2 v0.6.2 and Stable API migration

TFM2 Editor v1.0.0 is the first public release of the new Stable generation for **Teamfight Manager 2 v0.6.2**.

The game moved to a new Stable API, which required a major technical migration of both the desktop Editor and TFM2 Editor Bridge.

- Added support for **Teamfight Manager 2 v0.6.2**
- Migrated the Editor and Bridge to the new Stable API
- Updated **TFM2 Editor Bridge to v1.0.1**
- Restored the core Player, Staff, Team, Recruitment and Economy workflows
- Restored Player, Staff and Team database search and filtering
- Restored Champion Mastery, Actual Potential, Communication and supported contract editing
- Restored Team finance, budgets, fans, merchandise and Strategy editing
- Restored Competition Center, Match History, Team Stats and historical tactics analysis
- Player Quick Move now uses the game's supported Stable transfer flow
- Unsupported actions are disabled instead of using unsafe legacy behavior

The previous v0.5.x Editor/Bridge generation is not compatible with v1.0.0 / Bridge v1.0.1.

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
- Compact identity information
- Editing for all 12 player attributes
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
- Staff identity, role and age information
- Editing for all ten staff attributes
- Staff Communication editing
- Active salary and contract end-date editing
- Supported active same-team contract editing

Staff recruitment, signing and movement between teams are currently unavailable.

### Team

The Community release includes the expanded **Team** workspace.

- Team overview, roster and staff information
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

The following actions are currently unavailable in Community v1.0.0:

- Staff recruitment, signing and movement between teams
- Releasing Players or Staff directly to Free Agency
- Transfer Always Success
- Instant Retry
- Manual free-agent contract signing/default generation
- Contract incentive editing
- Training XP multipliers

Some full-contract operations are restricted to supported active same-team contracts. Existing unsupported contract fields and incentives are preserved rather than overwritten.

TFM2 Editor does not emulate unsupported features through unsafe legacy writes.

## Compatibility safety

The Editor displays the detected Bridge and compatibility state and blocks known unsupported combinations.

TFM2 Editor v1.0.0 requires:

- **TFM2 Editor Bridge v1.0.1**
- **Teamfight Manager 2 v0.6.2**

The previous TFM2 Editor v0.5.x generation is not compatible with the v1.0.1 Bridge generation.

There is no mixed legacy/Stable fallback. Update both the desktop Editor and Workshop Bridge when moving to v1.0.0.

## Installation

### Recommended: Steam Workshop Bridge

1. Subscribe to [TFM2 Editor Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) on Steam Workshop.
2. Download the latest desktop Editor from [GitHub Releases](https://github.com/jal-io/tfm2-editor/releases/latest).
3. Extract `TFM2.Editor.v1.0.0.Release.0.6.2.zip`.
4. Start Teamfight Manager 2 v0.6.2.
5. Enable **TFM2 Editor Bridge** in the Mods menu.
6. Restart the game.
7. Load your career.
8. Start `tfm2_editor_1.0.0.exe`.
9. Confirm the Editor is connected to **Bridge v1.0.1** with **Compatibility: OK**.

If the Editor shows **Disconnected**, click **Reconnect**.

### Simplified Chinese

Simplified Chinese is supported by the v1.0.0 desktop Editor.

The main release package includes the locale file. A separate `lang.zh-cn.zip` release asset is also available if you need to restore or install the language file separately.

The expected layout is:

```text
TFM2 Editor/
├── tfm2_editor_1.0.0.exe
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
