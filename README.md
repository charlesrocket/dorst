# dørst
[![Crates.io](https://img.shields.io/crates/v/dorst)](https://crates.io/crates/dorst)
[![Tests](https://github.com/charlesrocket/dorst/actions/workflows/ci.yml/badge.svg?branch=trunk)](https://github.com/charlesrocket/dorst/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/charlesrocket/dorst/branch/trunk/graph/badge.svg)](https://codecov.io/gh/charlesrocket/dorst)
### Intro

Bootstrap (and backup) codebases with Dørst.

### Features
##### Default

* `logs`
* `cli`

##### Optional

* `gui`

## CLI
### Compilation

```
cargo install dorst
```

### Usage

To begin, run `dorst` to create a configuration file in `$HOME/.config/dorst` and set targets (the current directory is the default backup destination). Dørts supports `ssh-agent` and can use `gitconfig`'s credential helper for authentication.

`dorst ~/backups/src`

Example:

```yaml
---
source_directory: ~/src
targets:
  - https://github.com/charlesrocket/dotfiles
  - https://github.com/charlesrocket/freebsd-station
  - git@gitlab.com:charlesrocket/openbsd-station.git
```

The `-c`/`--config` flag allows the usage of an alterantive configuration file.

## GUI
### Compilation

```
cargo install dorst --features gui
```

### Usage

`dorst --gui`
