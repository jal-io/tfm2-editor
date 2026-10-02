# Changelog

All notable public changes to TFM2 Editor are documented here.

## v1.0.0 — 2026-10-02

### Major Stable API migration

- Updated TFM2 Editor to v1.0.0.
- Updated TFM2 Editor Bridge to v1.0.1.
- Added support for Teamfight Manager 2 v0.6.2.
- Migrated the public Editor and shared Bridge from the previous legacy SDK generation to the new official Stable API.
- Rebuilt the supported Community workflows on the new Stable data and action paths.
- The previous v0.5.x Editor / Bridge generation is not compatible with the new v1.0 generation.

### Restored Community functionality

- Restored the six public Community areas: Search, Player Editor, Staff Editor, Team, Recruitment and Economy.
- Restored Player, Staff and Team databases, sorting, Quick Filters, Advanced Search, Saved Filters and Saved Lists.
- Restored Player name, attributes, positions, proficiency, Actual Potential, Champion Mastery, Communication, condition and stamina editing.
- Restored Staff name, attributes, Communication, salary and supported active-contract editing.
- Restored Team finance, Recruitment Budget, Salary Budget, fans, existing merchandise and Strategy editing.
- Restored Strategy Presets.
- Restored Competition Center, League Standings, Team Schedule, Match History, finished series/set results and MVP information where available.
- Restored Team Stats and Performance Center.
- Restored Historical Performance, Historical Synergy and Full Historical Synergy Explorer using supported career evidence.
- Missing match, replay or scouting evidence is not fabricated.

### Player transfers

- Ported supported Player Quick Move to the game's Stable transfer system.
- Supported contracted-player moves use the official Stable transfer action.
- Supported free-agent Player moves use the game's Stable transfer flow and game-assigned contract behavior where applicable.
- Removed reliance on legacy raw team-relation mutation.
- Uncertain transfer actions are not automatically retried.

### Contracts

- Restored standalone active salary and contract end-date editing for supported Player and Staff contracts.
- Supported full-contract editing is restricted to validated active same-team cases.
- Existing unsupported fields and incentives are preserved instead of being overwritten.
- Contract incentive construction/editing is currently unavailable.
- Manual free-agent contract signing through the old contract builder is currently unavailable.

### Temporarily unavailable

The following previous Community actions do not currently have a validated safe Stable API replacement and are disabled in v1.0.0:

- Staff recruitment, signing and movement between teams.
- Releasing Players or Staff directly to Free Agency.
- Transfer Always Success.
- Instant Retry.
- Manual free-agent contract signing/default generation.
- Contract incentive editing.
- Training XP multipliers.

Unsupported actions are blocked rather than emulated through unsafe legacy behavior.

### Stable data limitations

- Scouted / club-known Potential estimates are unavailable when no reliable populated Stable evidence exists.
- Actual Potential is not substituted for missing scouting estimates.
- Nationality is not exposed as an independent writable field; supported region/language information is used where available.
- Facility, stadium and gaming-house information remains read-only.
- Missing scouting reports, replay evidence or historical records are shown as unavailable rather than invented.

### Startup and compatibility

- Added the v1.0.0 compatibility notice explaining the TFM2 v0.6.2 Stable API migration and temporarily unavailable functionality.
- Added optional Ko-fi support access from the notice.
- Ko-fi support is completely optional and does not unlock any Editor functionality.
- Hardened the compatibility boundary between the previous v0.5.x generation and the new v1.0 generation.
- Unsupported Editor / Bridge combinations are blocked from active game-data access.
- There is no legacy fallback or mixed old/new runtime path.

### Language and appearance

- English remains built into the Editor.
- Simplified Chinese (简体中文) remains supported.
- Language selection persists between restarts.
- System / Light / Dark theme selection remains available and persists between restarts.

### Release validation

- Community formatting passed.
- Strict Community Clippy with `-D warnings` passed.
- Community tests passed: 67 distinct tests including the supplied-save parser fixture.
- Shared Bridge formatting and strict Clippy passed.
- Shared Bridge tests passed: 26 / 26.
- Combined executed Community + Bridge tests: 93 distinct passed.
- Community / Development separation validation passed.
- Stable Bridge entrypoint validation passed with no legacy entrypoint.
- Final Community v1.0.0 runtime smoke testing passed on Teamfight Manager 2 v0.6.2.
- Proceed and save/reload behavior was included in the release smoke test.

---

## v0.5.4 — 2026-09-03

### Compatibility

- Updated TFM2 Editor to v0.5.4.
- Updated TFM2 Editor Bridge to v0.2.79.
- Added support for Teamfight Manager 2 v0.5.8.
- Updated the Editor / Bridge compatibility generation for the new game version.
- Older release generations are blocked from active game-data access when the matching v0.5.4 Bridge is required.

### Release scope

- Clean compatibility migration from the existing v0.5.3 Community release.
- Preserved the complete v0.5.3 Community feature set.
- No new user-facing Community features were added in this release.
- Unreleased Development and research features remain excluded from the Community release.

---

## v0.5.3 — 2026-08-29

### Compatibility

- Updated TFM2 Editor to v0.5.3.
- Updated TFM2 Editor Bridge to v0.2.78.
- Added support for Teamfight Manager 2 v0.5.7.
- Updated the Editor / Bridge compatibility generation for the new game version.
- Older release generations are blocked from active game-data access when the matching v0.5.3 Bridge is required.

### Release scope

- Clean compatibility migration from the existing v0.5.2 Community release.
- Preserved the complete v0.5.2 Community feature set.
- No new user-facing Community features were added in this release.
- Training XP, Simplified Chinese, CJK font support, Theme selection and Team Strategy preset improvements remain included unchanged.

---

## v0.5.2 — 2026-08-21

### Compatibility

- Updated TFM2 Editor to v0.5.2.
- Updated TFM2 Editor Bridge to v0.2.75.
- Added support for Teamfight Manager 2 v0.5.6.
- Updated the Editor / Bridge compatibility rules for the v0.5.2 release combination.
- Older release generations are blocked from active game-data access when the matching v0.5.2 Bridge is required.

### Training XP multipliers

- Added Training XP multipliers for the player-controlled team.
- Added individual per-player multipliers for the current roster.
- Added a Full Roster Multiplier action.
- Added Reset All to x1.0.
- Added one-decimal multiplier support from x1.0 upward.
- Added per-save persistence so different careers can keep different Training settings.
- Training modifies only the supported core Training XP fields; normal game training timing, eligibility, thresholds and progression remain in control.

### Simplified Chinese and CJK support

- Added Simplified Chinese (`简体中文`) as the first additional Community language.
- Added `lang.zh-cn.zip` as a separate optional GitHub release asset.
- English remains embedded in the Editor and requires no external locale file.
- Added CJK-capable system font fallback.
- CJK font fallback is initialized independently of the active locale so `简体中文` renders correctly from an English startup.
- Language selection persists between Editor restarts.

### Theme selection

- Added a Theme selector in the application header directly after Language.
- Added **System**, **Light** and **Dark** options.
- System follows the current Windows theme.
- Light and Dark force the selected appearance.
- Theme selection persists between Editor restarts.
- Changing Language preserves Theme, and changing Theme preserves Language.

### Team Strategy presets

- Fixed new Strategy presets so Save stores the exact current Strategy editor values.
- Replaced the old separate Edit workflow with one Save workflow.
- Existing presets can now be overwritten without creating duplicates.
- Rename + Overwrite updates the same preset under the new name.
- Save as New preserves the original preset and creates a second preset.
- Duplicate names are automatically suffixed when required.
- Split Strategy values such as 1-3-1 and 1-4 are preserved correctly.
- Applying a saved preset to the team remains separate from preset storage.

### Release validation

- Completed the final Windows App quality gate with zero Community and Development warnings.
- Clippy with `-D warnings` passed.
- Development App tests passed: 134 / 134.
- Bridge tests passed: 48 / 48.
- Final Community runtime smoke testing passed on Teamfight Manager 2 v0.5.6.

---

## v0.4.2 — 2026-08-06

### Compatibility

- Updated TFM2 Editor to v0.4.2.
- Updated TFM2 Editor Bridge to v0.2.49.
- Added support for Teamfight Manager 2 v0.5.4.
- Rebuilt the Bridge against the Teamfight Manager 2 v0.5.4 classic Mod SDK.
- Updated Editor / Bridge compatibility rules for the new release combination.
- Older Bridge builds that do not support name editing are blocked from active game-data access.
- Community and Development compatibility handling remain separated so a valid Development combination is not blocked by an older Community rule.

### Player name editing

- Added Player Name Editing to Player Editor.
- Added an editable name field and Apply Name action.
- Added validation for empty names, control characters and names over 100 characters.
- Leading and trailing spaces are removed before applying a name.
- UTF-8 names and separator characters are transferred safely between the Editor and Bridge.
- Player Search and Player Editor selection data update after a successful name change.
- Name changes were validated through Refresh, Proceed and Save/Load.

### Staff name editing

- Added Staff Name Editing to Staff Editor.
- Added an editable name field and Apply Name action.
- Applied the same validation and safe text handling used by Player Name Editing.
- Staff Search and Staff Editor selection data update after a successful name change.
- Name changes were validated for contracted and free-agent staff.

### Player Editor identity section

- Added the same compact identity presentation used by Staff Editor.
- The first row now shows Name, Apply Name and ID.
- The second row shows Position and Age.
- The third row shows Team.
- Position uses the existing localized position formatter.
- Identity information refreshes when the selected player or loaded player data changes.

### Safety recommendation

- Added the existing Player Editor save recommendation to Staff Editor.
- Added the same recommendation to Recruitment.
- Player Editor, Staff Editor and Recruitment now use one shared presentation:

```text
Recommended: Save your career before making changes with the editor.
```

### Community build cleanup

- Kept Development-only Team workspace data and research tools outside the public Community payload.
- Marked intentionally unused Development-only fields correctly in the Community build.
- Removed release-gate compiler and Clippy warnings without changing runtime behavior.
- Preserved all existing Player, Staff, Search, Recruitment, Economy, Lists and compatibility behavior.
- No additional Community language pack is included; English remains embedded in the executable.

### Known limitations

- Champion Mastery can reopen too large after high mastery values.
- Champion Mastery can auto-maximize near the right edge of the main window.
- Development-only Team workspace and Team research tools are not included in Community.
- History / Stats Over Time is not included.
- Accepted Renewal is not included; contract editing affects the active contract.
- CJK font fallback is not ready.
- Community v0.4.2 ships with embedded English only.

---

## v0.4.0 — 2026-08-04

### Compatibility

- Updated TFM2 Editor to v0.4.0.
- Updated TFM2 Editor Bridge to v0.2.43.
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