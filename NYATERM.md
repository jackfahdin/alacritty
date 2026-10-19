# NyaTerm fork notes

This branch carries [NyaTerm](https://github.com/nyakang/nyaterm)'s local changes
to `alacritty_terminal` on top of an unmodified upstream base.

- Fork: <https://github.com/nyakang/alacritty>
- Upstream: <https://github.com/alacritty/alacritty>
- Base revision: `542147067dd016fc5f49120ef0ba605a98a4e3a7` (upstream `master`, 2026-10-20)
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

The 2026-09-22 merge to `d692748d3f` applied without conflicts. It removes
upstream's obsolete per-row column count limit while retaining NyaTerm's grid
scroll epochs and screen generations.

## 2026-10-13 upstream rebase

Upstream `master` is fixed at `29dc55375da817d53540f4caefbe708703c57b07`.
Previous published NyaTerm head: `e3d2ce97748039dedcd9316778d61d65c7b8ff2e`.
The previous branch is retained locally as `backup/nyaterm-20261013`.

Replay commits before this documentation update:

- `828c56e8e8fefeaac0e617184d3f7371ac05ef81` feat(grid): track a wrapping scroll epoch per grid
- `1c2bbd3b995323cd287409b4cdc2ce803260f2c2` feat(term): expose stable per-screen epochs and generations
- `f4cece9e14904b919d2665374fb63ad8e781c1f3` docs: record the NyaTerm fork baseline and patch set
- `adc66d0c9383229d6d78192e5cd4f0d3972e069d` docs: update the NyaTerm fork baseline

Both scroll epoch and per-screen/reset generation patches replayed unchanged.
Windows validation passed: `cargo test -p alacritty_terminal` (137 library,
45 reference and 1 documentation tests), and
`cargo clippy -p alacritty_terminal --all-targets`.
Stable whole-workspace formatting is excluded for the upstream nightly format policy.


Validation is on Windows; native macOS/Linux checks are delegated to the
branch workflow. Interactive GUI/file-drop and real-server authentication
checks were not performed by this update. Their outcomes must not be inferred
from scripted peers or compile checks.

## 2026-10-20 upstream rebase

Fixed upstream target: `542147067dd016fc5f49120ef0ba605a98a4e3a7`.
Previous published head: `4155cc1d57788444594d9d31cf233568ef113ee8`.
Original branch retained as local `backup/nyaterm-20261020`; the audit directory
contains the original patch, history and final range-diff.

Both grid scroll-epoch and per-screen/reset-generation commits replay unchanged.
Windows validation: cargo test -p alacritty_terminal passed 137 library, 45
reference and one documentation test; cargo clippy -p alacritty_terminal
--all-targets passed. Stable whole-workspace fmt remains excluded under upstream
nightly formatting policy. Native platform CI runs after publication.
