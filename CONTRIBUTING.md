# Contributing

Thank you for your interest in improving razer-trinity-mapper.

## Bug reports

Open an issue describing what happened, what you expected, and your
environment (distribution, desktop, Wayland or X11). Include daemon logs
(`journalctl --user -u trinity-mapper` or stderr) if relevant.

## Scope

This project supports the **Razer Naga Trinity** only — it is purpose-built
for its 12-button side grid and deliberately not a general-purpose remapper.
For other mice, [input-remapper](https://github.com/sezanzeb/input-remapper)
is a better fit.

## Translations

One language = one TOML file in `crates/trinity-gui/locales/`. Copy
`en.toml`, translate the values, and add the locale to `LOCALES` and
`native_name()` in `crates/trinity-gui/src/settings.rs`. The test suite
verifies all files expose the same key set.

## Pull requests

- One feature per branch.
- `cargo test` and `cargo clippy --all-targets -- -D warnings` must pass.
- Code and comments in English only.

## Development

```bash
git clone https://github.com/4lador/razer-trinity-mapper
cd razer-trinity-mapper
cargo build
cargo test
just check        # full suite (fmt, clippy, tests, architecture)
```

## License

By contributing, you agree that your contributions will be licensed under
the GPL-3.0-or-later license.
