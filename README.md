# TFM2 Editor

**TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.**

It connects to the running game through TFM2 Modifier Bridge, allowing you to inspect and modify your active career while the game is running.

Created and maintained by **jal-io**.

**Current release:** v0.2.19  
**Supported game version:** Teamfight Manager 2 v0.5.2

## What is a Live Editor?

Unlike a traditional save or database editor, TFM2 Editor works with the currently running game.

Start Teamfight Manager 2, load your career, then launch TFM2 Editor to view and modify supported game data through the bridge.

## Features

- Economy editing
- Player Editor
- All 12 player attributes
- Player positions and proficiency
- Actual Potential editing
- Salary editing
- Recruitment tools
- Transfer Always Success
- Instant Retry
- Move contracted players between teams
- Player database search
- Quick Filters
- Advanced Search
- Saved Filters

Some features are still under development.

## In Development

The next release is already in development.

Planned for the next release:

- Champion Mastery editing
- Individual and bulk Champion Mastery changes
- Redesigned Primary / Secondary / Tertiary position UI

## Installation

1. Download the latest release from the [Releases](https://github.com/jal-io/tfm2-editor/releases) page and extract the archive.

2. Move the entire `tfm2_modifier_bridge` folder to your Teamfight Manager 2 mods folder.

   Example:

   ```text
   C:\Program Files (x86)\Steam\steamapps\common\Teamfight Manager2\mods
   ```

   Your Steam installation path may be different.

3. Start Teamfight Manager 2, enable **TFM2 Modifier Bridge**, then restart the game.

4. Start **TFM2 Editor** and load your save.

## Important

TFM2 Editor is still under development.

**Always back up your save files before using the editor.**

Editing game data may cause unexpected behavior or save corruption.

Use TFM2 Editor at your own risk.

## Known Issues

The current development version contains a few UI issues related to the Champion Mastery window:

- The Champion Mastery window may occasionally open at a very large size, especially when viewing a player whose mastery values have already been set to 100.
- The Champion Mastery window may also expand when positioned close to the right edge of the main TFM2 Editor window.
- If this happens, manually resizing the Champion Mastery window restores the normal layout.

These are currently UI issues and do not affect Champion Mastery editing itself.

## Documentation

User guides, installation instructions, and feature screenshots are available through the project documentation:

- [TFM2 Editor Documentation](https://jal-io.github.io/tfm2-editor/)
- [Installation Guide](https://jal-io.github.io/tfm2-editor/installation.html)
- [Feature Guide](https://jal-io.github.io/tfm2-editor/features.html)

## Source & Attribution

TFM2 Editor and TFM2 Modifier Bridge were created by **jal-io**.

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
