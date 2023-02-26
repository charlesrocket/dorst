# dørst
[![Crates.io](https://img.shields.io/crates/v/dorst)](https://crates.io/crates/dorst)
[![Tests](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml/badge.svg?branch=trunk)](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml)
[![codecov](https://codecov.io/gh/charlesrocket/dorst/branch/trunk/graph/badge.svg)](https://codecov.io/gh/charlesrocket/dorst)
### Intro

Backup codebases with Dørst.

### Compilation

```
cargo install dorst
```

### Usage

To begin, run `dorst` to create a configuration file in `$HOME/.config/dorst` and set backup targets (the current directory is the default destination). Dørts will use `gitconfig`'s credential helper for authentication if needed.

`dorst ~/backups/src`

Example:

```yaml
---
targets:
  - https://github.com/charlesrocket/dotfiles
  - https://github.com/charlesrocket/freebsd-station
```

The `-c`/`--config` flag allows the usage of an alterantive configuration file.
