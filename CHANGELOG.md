# Changelog

All notable public changes to TFM2 Editor are documented here.

## v1.2.1 — 2026-10-25

### Staff movement and release

- Added supported **Staff Movement** between teams and Free Agency through the existing Transfers interface.
- Added **Staff Release to Free Agency** through the existing Community confirmation workflow.
- Reused the verified Staff Release game-write engine and authoritative server readback; success is reported only after verification.
- Preserved the existing public confirmation, inline statuses and generic failure presentation.

### Bulk editing and existing features

- Added **Bulk Age and Salary** editing for selected Players and Staff, using one familiar Bulk Edit window and Apply flow without manual Preview Changes.
- Preserved supported selection across search filters and team views.
- Retained safeguards for verified write outcomes and uncertain operations without blind repeat writes.
- Preserved Player Release, Player Movement, Roster Tags, Ability Rating, contract editing and the LEAN/asynchronous performance improvements from v1.2.0.

### Compatibility

- Updated Community Editor to **v1.2.1**, Shared Bridge to **v1.0.24**, and game target to **Teamfight Manager 2 v0.6.5** (Stable ABI 10).
- Introduced **Protocol 26**; previous Editor/Bridge Protocol 25 generations are intentionally incompatible. Update both the desktop Editor and Workshop Bridge.
- English and Simplified Chinese (简体中文) remain supported.

### Validation and limitations

- Final release checks reported **19/19 PASS**, with **170 Community** and **256 tests in each Bridge variant** passing. Missing external fixtures remain **NOT TESTED**, not passed.
- Final Community and Development executables and public/private Bridge DLLs passed the reported active Kaspersky acceptance checks.
- The project maintainer confirmed successful native Community Staff Release.
- Transfer Always Success, Instant Retry, manual Free Agent contract signing outside supported movement flows, contract incentive editing and Training XP multipliers remain unavailable.
- Supported active-contract restrictions and read-only facility, stadium and gaming-house information remain unchanged.
- Back up career saves before using the Editor.

---

## v1.2.0 — 2026-10-16

### Major performance update

- Promoted major LEAN architecture and responsiveness improvements to Community.
- Optimized Match History delivery using the guarded 192 KiB transport, complete ordered reassembly, and the newest 10,000 completed matches.
- Added Refresh Spam Guard, request coalescing, asynchronous reads and stale-response rejection.
- Restored Search selection autoload into Player Editor and own-team-first ordering in supported workflows.

### Roster Tags

- Introduced individual and bulk **Roster Tags** in Player Editor and Team Management.
- Users can **Apply**, **Remove**, edit or replace Follow Team tags, including repeated **Apply → Remove → Apply** with different team tag values.
- **Follow Team** tracks only explicitly tagged players. Tracked players update their team tags on transfers, including supported in-game transfers when the Editor is closed.
- Follow Team's existing Free Agent behavior removes the visible team tag while retaining tracking for later signing.
- Never-tagged players remain untagged when joining a tagged team; they are not automatically enrolled.
- Protected **Custom Tags** from bulk and automatic updates.
- Removed persistent **Excluded** side effects from manual Remove; retained team tag overrides, guarded Name writes, verified prefix ownership and readback safeguards.

### Ability Rating, contracts and transfers

- Added read-only **Ability Rating** based on eight fundamental abilities to supported Player Editor, Search, filtering, sorting, Saved Lists and export views.
- Preserved the original 12-attribute **Actual Rating** calculation.
- Improved supported Player and Staff active-contract editing, including incentive-free contract handling and guarded readback.
- Improved Free Agent signing and supported Player transfers through the official Stable transfer system. Signing no longer automatically opens a contract dialog; manual supported Edit Contract remains available.
- Preserved Player/Staff Age editing, guarded Player Release and existing Community functionality.

### Compatibility

- Updated Community Editor to **v1.2.0**, ONE Shared Bridge to **v1.0.20** and game target to **TFM2 v0.6.4** (official Stable SDK v0.6.4 / ABI 10).
- Introduced **Protocol 25** with an intentional **hardblock** between the new generation and Community v1.1.0 / Bridge v1.0.9 / Protocol 24 in both directions.
- Unsupported generations are refused before game-data modifications. Update both the desktop Editor and Workshop Bridge.
- English and Simplified Chinese (简体中文) remain supported.

### Validation and limitations

- Final candidate quality checks reported **19/19 PASS**; **162 Community**, **319 Development** and **185 Bridge** automated tests passed with no failures. Each suite had one ignored missing-fixture test, not counted as passed.
- Maintainer reported the Windows candidate looked OK; this does not establish that every game or career-transition scenario has been tested.
- Staff recruitment/signing/movement and Staff Release, Transfer Always Success, Instant Retry, manual free-agent contract signing, incentive editing and Training XP multipliers remain unavailable.
- Full contract edits remain restricted to supported active contracts. Facility, stadium and gaming-house information remains read-only.
- Back up career saves before using the Editor.

---

## v1.1.0 — 2026-10-10

### Compatibility

- Updated Community Editor to v1.1.0 and TFM2 Editor Bridge to v1.0.9.
- Added support for Teamfight Manager 2 v0.6.3 through the official Stable API.
- Preserved the existing compatibility safeguards. Bridge v1.0.9 is the current recommended release; the supported minimum remains v1.0.7, subject to compatibility checks.

### Player and Staff editing

- Added Player Age editing.
- Added Staff Age editing.
- Restored releasing supported contracted players directly to Free Agency.
- Player Release removes the active roster relationship and changes the player's contract status without deleting or retiring the player.
- Added a clear release confirmation and an "already a Free Agent" message. Already-free players are not sent through a redundant release action.
- Preserved the established safety checks and readback behavior for supported game changes.

### Localization

- Expanded English and Simplified Chinese (简体中文) coverage for the Community interface.
- Updated new feature labels, dialogs, warnings, status messages and offline Bridge notices.
- Retained built-in English fallback, external Chinese locale support and persisted language settings.

### Validation

- Community release quality gates passed: 97 tests passed, 0 failed, 1 external-fixture test ignored.
- Shared Bridge quality gate passed: 108 tests passed, 0 failed, 1 external-fixture test ignored.
- The ignored older-save fixture-dependent tests were not counted as passed.
- Final release binaries passed the reported offline Kaspersky scan and launch checks.
- Player Age, Staff Age and Player Release were verified in-game by the project maintainer.

### Still unavailable

Staff recruitment/signing/movement, Staff Release to Free Agency, Transfer Always Success, Instant Retry, manual free-agent contract signing, contract incentive editing and Training XP multipliers remain unavailable in the public Community release.

---

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