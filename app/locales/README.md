# TFM2 Editor locales

TFM2 Editor uses a hybrid localization model.

- English (`en-US`) is compiled into the executable and is always available.
- Additional languages are loaded from the `locales` folder beside the executable.
- Missing translation keys fall back to the embedded English text.
- An English-only Community release does not need a `locales` folder.

## Development and Community packaging

Development builds copy every JSON locale, including `en-US.json`. The external
English file is a hot-reloadable override used with **Reload locales** and the
localization diagnostics.

Community builds do not copy `en-US.json`, because English is already embedded.
Only additional languages such as `zh-CN.json` are distributed beside the EXE.

## File format

```json
{
  "locale": "xx-YY",
  "name": "Language name",
  "strings": {
    "tabs.economy": "Translated text"
  }
}
```

- `locale` must be a unique locale code such as `en-US` or `zh-CN`.
- `name` is the language name shown in the language selector.
- `strings` maps stable translation keys to translated UI text.
- Keep placeholders unchanged, for example `{author}`, `{version}`, `{status}`,
  `{language}`, `{error}` and `{count}`.
- Translate complete sentences instead of combining small translated fragments.
- Invalid JSON files are ignored so they cannot prevent the editor from starting.
- Development builds report invalid files and missing keys in the header.

The selected language is stored in `tfm2_editor_settings.json` beside the
executable.

## Current migration status

The first full Community UI pass covers Economy, Player Editor, Staff Editor,
Communication, Champion Mastery, Edit Contract, Recruitment, Player Search,
Advanced Search and Saved Filters. Dynamic status/error messages, domain enums
and some generated table labels are migrated in the next pass.

For a quick Development test, edit a visible value in `dist/locales/en-US.json`
and click **Reload locales**. Restore the English file after testing.
