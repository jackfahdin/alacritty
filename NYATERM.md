# NyaTerm fork notes

This branch carries [NyaTerm](https://github.com/nyakang/nyaterm)'s local changes
to `alacritty_terminal` on top of an unmodified upstream base.

- Fork: <https://github.com/nyakang/alacritty>
- Upstream: <https://github.com/alacritty/alacritty>
- Base revision: `ede2ac144da4dec4c075bfa803aacf3b3739bce6` ("Fix unbounded
  zerowidth character limit", upstream `master`, `alacritty_terminal` 0.26.1-dev)
- Branch: `nyaterm`
- Crate: `alacritty_terminal` only. Nothing outside `alacritty_terminal/` is
  touched.

NyaTerm consumes this crate through `[patch.crates-io]`, and a patch entry only
applies if its version still satisfies the requirement, so the workspace asks for
`alacritty_terminal = "0.26.1-dev"` to match this base. The previous base was the
`0.17.0` release commit, which is not an ancestor of `master`: upstream released
0.17.0 on its own line and moved `master` straight to 0.18.0-dev.

## Patches

1. `feat(grid): track a wrapping scroll epoch per grid` — `Grid::scroll_epoch`
   counts physical rows rotated out by full-screen upward scrolling.
   `Grid::history_size()` cannot serve this purpose: once scrollback reaches
   `max_scroll_limit` the ring buffer keeps rotating while the reported history
   size stays constant.
2. `feat(term): expose stable per-screen epochs and generations` — read-only
   `primary_grid_scroll_epoch`, `alternate_grid_scroll_epoch`,
   `primary_screen_generation`, `alternate_screen_generation` and
   `reset_generation`. Entering the alternate screen advances the alternate
   generation because Alacritty clears that screen before swapping it into use.

All counters wrap on overflow. No ANSI, grid rotation, event, or parsing
behaviour is changed.

## Not carried here

- The four upstream release commits this series used to sit on top of
  (`Alacritty version 0.17.0`, its two release candidates, and `Fix release CI`).
  They are not on `master`, which went to 0.18.0-dev instead, and `master` has its
  own `Fix release CI`, so rebasing dropped all four rather than replaying them.
- The NyaTerm snapshot omits the 46 MiB reference-terminal fixtures under
  `alacritty_terminal/tests/` and consequently drops the `[[test]] name = "ref"`
  entry from the packaged manifest. Both are vendoring artifacts: this branch
  keeps the fixtures and the manifest entry, and `cargo test -p
  alacritty_terminal` runs the 45 upstream ref tests here.

## Validation

On Windows 11:

```sh
cargo test -p alacritty_terminal            # 137 lib + 45 ref + 1 doc, all passing
cargo clippy -p alacritty_terminal --all-targets   # warning-free
```

`cargo fmt --check` is not meaningful here: upstream's `rustfmt.toml` uses
nightly-only options, so a stable rustfmt reformats files this branch never
touches. The patched files are unchanged in shape from the previous base.
