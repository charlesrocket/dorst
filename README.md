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

* `gui` _GTK4_

## CLI
### Compilation

```shell
make
make install # as root
```

### Usage

To begin, run `dorst` to create a configuration file in `$HOME/.config/dorst` and set targets (the current directory is the default backup destination). Dørts supports `ssh-agent` and can use `gitconfig`'s credential helper for authentication.

`dorst -b ~/backups/src`

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

```shell
make features=gui
make install features=gui # as root
```

### Usage

`dorst --gui`

## Backups

Dorst's backups are (git) mirrors: `git clone example.dorst`
