---
layout: default
title: Guide
---

# TFM2 Editor Guide

Welcome to the user guide for **TFM2 Editor**, an unofficial live editor and companion tool for **Teamfight Manager 2**.

Created and maintained by **jal-io**.

> **Current release:** v0.4.0
> **Required Bridge:** v0.2.43
> **Supported game version:** Teamfight Manager 2 v0.5.3
> **Platform:** Windows

## Getting Started

- [Installation](installation.html)
- [Features](features.html)

## What is TFM2 Editor?

TFM2 Editor connects to the running game through **TFM2 Editor Bridge** and lets you inspect and modify supported data in your active career.

Unlike a traditional save editor or database editor, changes are made through the currently running game while your career is loaded.

## Important installation note

**Subscribing on Steam Workshop installs TFM2 Editor Bridge only.**

The desktop **TFM2 Editor application must be downloaded separately from GitHub Releases**.

- **Steam Workshop:** installs the Bridge mod used by the game
- **GitHub Releases:** provides the Windows desktop Editor application

## Main features in v0.4.0

### Search
- Full **Player Search**
- Full **Staff Search**
- Full **Teams Search**
- Shared **Saved Lists**
- Quick Filters
- Advanced Search
- Saved Filters
- Sorting and column resizing
- Shared blue multi-selection across Player, Staff, and Teams Search
- Double-click to open Player Editor or Staff Editor

### Player Editor
- Edit all 12 player attributes
- Edit Primary, Secondary, and Tertiary positions
- Edit position proficiency
- Edit hidden **Actual Potential**
- Automatic Potential Grade updates
- Open **Champion Mastery**
- Edit active player contracts
- Edit **Communication Level**
- View read-only **Pending Training XP**
- Compact two-column layout

### Staff Editor
- Edit all supported staff attributes
- Edit active staff contracts
- Edit **Communication Level**

### Recruitment
- Transfer Always Success
- Instant Retry
- Player management
- Staff management
- Contract creation for free agents

### Economy
- Edit Money
- Edit Recruitment Budget
- Edit Salary Budget
- Compact TFM2-style currency input with live preview

### Compatibility safety
TFM2 Editor permanently shows the current Bridge state in the header:

- **Compatibility: OK**
- **Compatibility: Warning**
- **Compatibility: Not Supported**

Unsupported combinations are blocked to protect the active career from unsafe reads or writes.

## Included / not included

### Included in Community v0.4.0
- Search
- Player Editor
- Staff Editor
- Recruitment
- Economy
- Compatibility safety system
- Localization foundation with embedded English fallback

### Not included in Community v0.4.0
- Dedicated Team editing workspace
- History / stats-over-time workspace
- Contract renewal / future replacement contracts
- Additional Community translation packs

## Important

**Always back up your save files before using TFM2 Editor.**

Editing game data may cause unexpected behavior or save corruption. Use the editor at your own risk.

## Bug reports

If you find a bug, please report it through [GitHub Issues](https://github.com/jal-io/tfm2-editor/issues).

Please include:

- TFM2 Editor version
- TFM2 Editor Bridge version
- Teamfight Manager 2 version
- What you tried to do
- What you expected
- What happened instead

## Source & attribution

TFM2 Editor and TFM2 Editor Bridge were created and are maintained by **jal-io**.

Copyright © 2026 jal-io.

See the repository `LICENSE` file for attribution, usage, and redistribution terms.

## Disclaimer

TFM2 Editor is an unofficial community project and is not affiliated with or endorsed by Team Samoyed.

Teamfight Manager 2 and related game content belong to their respective rights holders.
