# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through TFM2 Editor Bridge, allowing you to inspect and modify your active career while the game is running.

Created and maintained by **jal-io**.

**Current release:** v0.3.1
**Supported game version:** Teamfight Manager 2 v0.5.3
**Bridge version:** v0.2.39

## What is a Live Editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to view and modify supported game data through the bridge.

## Features

### Economy

- Money editing
- Transfer Budget editing
- Salary Budget editing
- Scout Budget editing

### Player Editor

- All 12 player attributes
- Primary / Secondary / Tertiary position editing and proficiency
- Actual Potential editing with automatic Potential Grade updates
- Player Communication editing
- Salary editing
- Champion Mastery editing
- Individual and bulk Champion Mastery changes

### Player Contract & Finance

View complete active player contract information, including:

- Current team
- Contract start and end dates
- Annual and weekly salary
- Transfer fee
- Squad Status
- POG Award Bonus
- League Rank Bonus and required rank
- Match Appearance Bonus
- Match Win Bonus

Active player contracts can be edited directly through the Player Contract Editor.

Supported Squad Status options include:

- Core Player
- Key Player
- Starter
- Substitute
- Prospect

### Staff Editor

- Staff search and selection
- Staff attribute editing
- Staff salary
- Staff Communication editing
- Staff contract information
- Active staff contract editing

### Player and Staff Management

- Move contracted players between teams
- Move contracted staff between teams
- Set contracted players to free agency
- Set contracted staff to free agency
- Create a contract and move a free-agent player to any selected team
- Create a contract and move a free-agent staff member to any selected team

Free-agent contract forms are automatically filled with valid starting values and can be reviewed or changed before the move is applied.

### Recruitment

- Transfer Always Success
- Instant Retry
- Separate Player Management and Staff Management tools

### Player Database

- Player database search
- Quick Filters
- Advanced Search
- Saved Filters

Some features are still under development.

## Contract Editing Note

TFM2 Editor supports editing existing active player and staff contracts.

Full contract renewal, where the game creates a separate replacement contract, is not included yet.

## Installation

### Recommended: Steam Workshop

1. Subscribe to [TFM2 Editor Bridge](https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765) on Steam Workshop.

2. Download the latest desktop editor from the [Releases](https://github.com/jal-io/tfm2-editor/releases) page and extract the archive.

3. Start Teamfight Manager 2, enable **TFM2 Editor Bridge**, then restart the game.

4. Load your career, then start **TFM2 Editor**.

### Manual Bridge Installation

The release archive also contains the `tfm2_modifier_bridge` folder.

Move the entire folder to your Teamfight Manager 2 mods directory.

Example:

```text
C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
```

Your Steam installation path may be different.

## Important

TFM2 Editor is still under development.

**Always back up your save files before using the editor.**

Editing game data may cause unexpected behavior or save corruption.

Use TFM2 Editor at your own risk.

## Known Issues

- The Champion Mastery window may occasionally open at a very large size.
- It may also expand when positioned close to the right edge of the main TFM2 Editor window.
- If this happens, manually resizing the Champion Mastery window restores the normal layout.

These are currently UI issues and do not affect Champion Mastery editing itself.

## Documentation

User guides, installation instructions, and feature screenshots are available through the project documentation:

- [TFM2 Editor Documentation](https://jal-io.github.io/tfm2-editor/)
- [Installation Guide](https://jal-io.github.io/tfm2-editor/installation.html)
- [Feature Guide](https://jal-io.github.io/tfm2-editor/features.html)

## Source & Attribution

TFM2 Editor and TFM2 Editor Bridge were created by **jal-io**.

Copyright © 2026 jal-io.

The source code is publicly available for learning, contribution, forking, and private modification.

If you publicly redistribute or release a modified or derivative version, clear credit to **TFM2 Editor** and **jal-io** must be retained.

Do not present modified or derivative versions as entirely original work, and do not remove existing copyright or attribution notices.

See the repository `LICENSE` file for the full terms.

## Issues

If you find a bug, please report it through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).

When possible, include:

- TFM2 Editor version
- Teamfight Manager 2 version
- What you were trying to do
- What happened instead

## Support

TFM2 Editor is free to use.

If you enjoy the project and want to support continued development, you can support me on Ko-fi:

[Support me on Ko-fi](https://ko-fi.com/jalio)

## Disclaimer

TFM2 Editor is an unofficial community project and is not affiliated with or endorsed by Team Samoyed.

Teamfight Manager 2 and related game content belong to their respective rights holders.
