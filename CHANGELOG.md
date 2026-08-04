# Changelog

All notable public changes to TFM2 Editor are documented here.

## v0.4.0 — 2026-08-04

### Compatibility

- Updated TFM2 Editor to v0.4.0.
- Updated TFM2 Editor Bridge to v0.2.43.
- Added compatibility protocol 4.
- Supports Teamfight Manager 2 v0.5.3.
- Added permanent Editor / Bridge compatibility status.
- Added warning-level compatibility handling.
- Added hard blocking for known unsupported combinations.
- Unsupported states disconnect active game-data access and block reads and writes.
- Added safe Reconnect recovery after installing a compatible Bridge.

### Player Search

- Added Actual Potential `Min ~ Max` to Advanced Player Search.
- Added inclusive minimum, maximum and exact-value filtering.
- Saved Filters preserve the Actual Potential enabled state, minimum and maximum.
- Older `.tfm2filter` files continue to load without migration.
- Added direct double-click navigation to Player Editor.
- Added full-row right-click actions.
- Preserved filters and sorting when opening a player.

### Staff Search

- Added a complete database-style Staff Search page.
- Added sortable and resizable columns.
- Added identity, role, team, age, salary and contract information.
- Added all ten staff attributes.
- Added Communication data.
- Added Quick Filters and Advanced Search.
- Added a separate Staff Saved Filters library.
- Added save, load, update, delete, import and export.
- Added direct navigation to Staff Editor.

### Teams Search

- Added a complete Teams Search database.
- Added search by team name, manager, league ID and team ID.
- Added League and My Team filters.
- Added roster-size and staff-count filters.
- Added sortable and resizable columns.
- Added active roster and staff counts.
- Added Roster Rating from contracted players' 12 attributes.
- Added Money, Recruitment Budget and Salary Budget.
- Added Merchandise Facility Grade.
- Added Home Stadium Grade.
- Added Training Facility Grade.
- Added the same selection behavior used by Player and Staff Search.

Teams Search is included as a database and selection tool. A dedicated Team workspace is deferred until it contains real Team editing features.

### Shared Player and Staff Saved Lists

- Added create, rename and delete.
- Added one or multiple selected players or staff members.
- Added shared `.tfm2list` format v2.
- Added import and export.
- Added automatic loading of legacy player-only v1 lists.
- Added Player Search and Staff Search filtering by list.
- Added direct opening of lists in the matching Search page.
- Missing or retired IDs remain visible and removable.

### Search selection

- Added the same blue full-row selection across Player, Staff and Teams Search.
- Added click-anywhere selection and deselection.
- Added selection checkboxes.
- Added `Select All Visible` and `Clear Selection`.
- Added Shift-click range selection and deselection.
- Added deterministic Shift-drag selection and deselection.
- Added matching selection behavior in Saved Lists.

### Player Editor

- Added the approved compact two-column desktop layout.
- Kept Attributes in the left column.
- Moved Positions and Potential into the right column.
- Kept Contract and Communication full-width below.
- Added a single-column fallback for narrow windows.
- Moved `Open Champion Mastery` beside the main attribute actions.
- Removed the unnecessary standalone Champion Mastery section.
- No Player Editor backend or write behavior was changed.

### Communication Level

#### Player

- Simplified the Community presentation.
- Added clear `Actual Communication` terminology.
- Added `Learned Regions`.
- Added `Apply Actual Communication`.
- Added `Set Actual to 100`.
- Shows Pending Training XP as read-only information for the selected region.
- Keeps Pending Training XP separate from the current 0–100 Communication Level.
- Retained existing add, update and Apply-0 removal behavior.

#### Staff

- Renamed the section to `Communication Level`.
- Renamed the editable field to `Actual Communication`.
- Renamed stored values to `Learned Regions`.
- Added `Apply Actual Communication`.
- Added `Set Actual to 100`.
- Simplified the Community presentation.
- Retained existing add, update and Apply-0 removal behavior.

### Currency display and input

- Replaced raw internal money values with compact TFM2-style values.
- Added input such as `$400K`, `2.5M`, `1B` and `1T`.
- Added live previews and validation before Apply.
- Applied compact formatting to Economy, contracts, salaries, transfer fees, bonuses, free-agent defaults and Search.

### Localization foundation

- Added a dedicated localization system with stable keys.
- Added embedded English fallback.
- Added external UTF-8 JSON locale support.
- Added safe fallback for missing keys.
- Added safe startup for invalid locale files.
- Added persisted language selection.
- Kept localization diagnostics Development-only.

Community v0.4.0 ships with embedded English only.

### Navigation and presentation

The main tabs are now:

```text
Search · Player Editor · Staff Editor · Recruitment · Economy
```

- Removed the empty Team main tab.
- Renamed `Contract & Finance` to `Contract`.
- Reduced tab text size, padding and spacing.
- Shortened Community Search instructions.
- Simplified Actual Rating, Actual Potential and Economy explanations.
- Highlighted `Recommended:` in the Player Editor save reminder.
- Simplified Player and Staff Contract tooltips.
- Renamed the Community Economy action from `Apply Economy` to `Apply`.

### Fixed

- Fixed Search retaining stale Player Editor, Staff Editor or unrelated load messages.
- Player Search now restores the total loaded Player Search dataset count.
- Staff Search now restores the total loaded Staff Search dataset count.
- Teams Search now restores the total loaded Teams Search dataset count.
- Fixed Player Management search mixing team names into player results.
- Player Management now searches player name or player ID only.
- Fixed inconsistent Player, Staff and Team selection colors.
- Fixed full-row selection and highlighting in Teams Search.
- Fixed Shift-click reusing an old range anchor.
- Fixed Shift-drag leaving incorrect rows selected when dragging back.
- Fixed empty salary input temporarily hiding the Contract section.
- Removed obsolete Rust warnings.
- Fixed future-incompatible float-literal warnings.

### Mod compatibility

Validated successfully with:

- **Real World Database '26**
- **League of Legends Champions by Silverbear**
  - 113 champions detected dynamically
- **Item Scroller**
- **Riot Games Item Expansion Pack**

### Known limitations

- Champion Mastery can reopen too large after high mastery values.
- Champion Mastery can auto-maximize near the right edge of the main window.
- Team workspace / Team Overview is not included.
- History / Stats Over Time is not included.
- Accepted Renewal is not included; contract editing affects the active contract.
- CJK font fallback is not ready.
- No additional Community translation is included in v0.4.0.

---

## v0.3.1 — 2026-08-02

### Compatibility

- Updated TFM2 Editor to v0.3.1.
- Updated TFM2 Editor Bridge to v0.2.39.
- Supports Teamfight Manager 2 v0.5.3.

### Fixed

- Enabled **Prospect** as a selectable Squad Status in the Player Contract Editor.
- Prospect can now be used when editing an existing active player contract.
- Prospect can now be used when creating a contract for a free-agent player.

---

## v0.3.0 — 2026-08-01

### Compatibility

- Updated TFM2 Editor to v0.3.0.
- Updated TFM2 Editor Bridge to v0.2.38.
- Supports Teamfight Manager 2 v0.5.3.

### Staff Editor

- Added staff search and selection.
- Added editing for supported staff attributes.
- Added staff salary editing.
- Added Staff Communication editing.
- Added active staff contract information and editing.

### Player Contract and Finance

- Added complete active player contract information.
- Added contract start and end dates.
- Added annual and weekly salary.
- Added transfer fee.
- Added Squad Status.
- Added POG Award Bonus.
- Added League Rank Bonus and required rank.
- Added Match Appearance Bonus.
- Added Match Win Bonus.

### Contract Editing

- Added active player contract editing.
- Added active staff contract editing.
- Added Apply Contract, Reset and Cancel controls.
- Added Squad Status editing through a dropdown.
- Added player contract bonus editing.
- Added automatically filled contract defaults for free agents.

### Player and Staff Management

- Added Move Contracted Staff.
- Added Set Player to Free Agent.
- Added Set Staff to Free Agent.
- Added Create Contract & Move Player.
- Added Create Contract & Move Staff.
- Added Apply Contract & Move Player.
- Added Apply Contract & Move Staff.
- Added support for moving free-agent players and staff to any selected team after creating a contract.

### Communication

- Added Player Communication editing.
- Added Staff Communication editing.
- Communication changes persist through Proceed and Save/Load.

### Actual Potential

- Improved the Actual Potential interface.
- Potential Grade now updates automatically when the numeric value changes.

### Community Build Improvements

- Renamed Contract Builder to **Edit Contract**.
- Separated Player Management and Staff Management.
- Added clearer mode-specific contract actions.
- Hidden Development-only Contract Flow tools.
- Added additional UI and wording improvements.

### Known issue

- The Champion Mastery window may occasionally open or expand too large.
- Manually resizing the window restores the normal layout.

---

## v0.2.31 — 2026-07-31

### Compatibility

- Updated TFM2 Editor for Teamfight Manager 2 v0.5.3.
- Updated TFM2 Editor Bridge for the v0.5.3 mod API.

### Features

- Champion Mastery editing.
- Individual and bulk Champion Mastery changes.
- Primary / Secondary / Tertiary position editor with proficiency controls.

### Existing features

- Economy editing.
- All 12 player attributes.
- Actual Potential and salary editing.
- Recruitment tools.
- Player Search, Quick Filters, Advanced Search and Saved Filters.

### Known issue

- The Champion Mastery window may occasionally open or expand too large.
- Manually resizing the window restores the normal layout.
