# TFM2 Editor v0.3.0 – Contract & Staff Management Update

TFM2 Editor v0.3.0 is a major feature update focused on staff editing, contracts, communication, and complete player and staff movement between teams and free agency.

## Compatibility

- **TFM2 Editor:** v0.3.0
- **TFM2 Editor Bridge:** v0.2.38
- **Teamfight Manager 2:** v0.5.3
- **Platform:** Windows

## What’s New

### Staff Editor

Added a complete Staff Editor with support for:

- Staff search and selection
- Staff attribute editing
- Staff salary
- Staff Communication
- Staff contract information
- Active staff contract editing

### Player Contract & Finance

The Player Editor now displays the complete active player contract:

- Current team
- Contract start and end dates
- Annual and weekly salary
- Transfer fee
- Squad Status
- POG Award Bonus
- League Rank Bonus and required rank
- Match Appearance Bonus
- Match Win Bonus

### Contract Editing

Active player and staff contracts can now be edited directly.

Player contract editing includes:

- Contract start and end dates
- Annual salary
- Transfer fee
- Squad Status
- POG Award Bonus
- League Rank Bonus and required rank
- Match Appearance Bonus
- Match Win Bonus

Player and staff contract windows now include:

- Apply Contract
- Reset
- Cancel

Reset reloads the current contract values without applying changes.

### Player and Staff Management

Player and Staff Management now support the complete movement flow:

- Move contracted players between teams
- Move contracted staff between teams
- Set contracted players to free agency
- Set contracted staff to free agency
- Create a contract and move a free-agent player to any selected team
- Create a contract and move a free-agent staff member to any selected team

Free-agent actions use:

- **Create Contract & Move Player**
- **Create Contract & Move Staff**
- **Apply Contract & Move Player**
- **Apply Contract & Move Staff**

Free-agent contract forms are automatically filled with valid starting values and can be reviewed or changed before the move is applied.

### Player and Staff Communication

Communication editing is now supported for both players and staff.

Changes update the active career and persist through:

- Proceed
- Save/Load

### Actual Potential

Improved the Actual Potential interface:

- Enter an exact Actual Potential value
- Potential Grade updates automatically
- Apply the value directly to the active career

## Community Build Improvements

- Renamed Contract Builder to **Edit Contract**
- Added separate Player Management and Staff Management sections
- Added mode-specific free-agent contract actions
- Added Reset support to contract windows
- Hidden Development-only Contract Flow tools
- Removed Development-only text and controls from the Community build
- Additional UI and wording improvements

## Public Source Update

The public repository has been updated to match this release:

- TFM2 Editor Community source updated to v0.3.0
- TFM2 Editor Bridge source updated to v0.2.38
- Development-only probes and internal tools are not included
- README, feature documentation, screenshots, and roadmap updated

## Contract Editing Note

This release supports editing an existing active contract.

Full contract renewal, where the game creates a separate future replacement contract, is not included yet.

## Known Issue

- The Champion Mastery window may occasionally open or expand too large.
- Manually resizing the window restores the normal layout.
- This is a UI issue and does not affect Champion Mastery editing.

---

# Previous Release

## v0.2.31

### Compatibility

- Updated TFM2 Editor for Teamfight Manager 2 v0.5.3.
- Updated TFM2 Modifier Bridge for the v0.5.3 mod API.

### Features

- Champion Mastery editing.
- Individual and bulk Champion Mastery changes.
- Primary / Secondary / Tertiary position editor with proficiency controls.

### Existing Features

- Economy editing.
- All 12 player attributes.
- Actual Potential and salary editing.
- Recruitment tools.
- Player Search, Quick Filters, Advanced Search, and Saved Filters.

### Known Issue

- The Champion Mastery window may occasionally open or expand too large. Manually resizing the window restores the normal layout.

---

## Installation

The Steam Workshop item installs **TFM2 Editor Bridge**.

The desktop editor must be downloaded separately from this GitHub release.

Installation guide:

https://jal-io.github.io/tfm2-editor/installation.html

Steam Workshop:

https://steamcommunity.com/sharedfiles/filedetails/?id=3775240765

## Important

TFM2 Editor is an unofficial live editor and companion tool for Teamfight Manager 2.

Teamfight Manager 2 and your career must be running while using the editor.

Always back up your save files before making changes. Editing game data may cause unexpected behavior or save corruption.

## Feedback and Bug Reports

Please report problems through GitHub Issues:

https://github.com/jal-io/tfm2-editor/issues

When reporting a problem, please include:

- TFM2 Editor version
- Teamfight Manager 2 version
- What you attempted to do
- What you expected to happen
- What happened instead
