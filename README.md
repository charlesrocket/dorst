# `dørst`
[![Crates.io](https://img.shields.io/crates/v/dorst)](https://crates.io/crates/dorst)
[![Tests](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml/badge.svg?branch=trunk)](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml)
[![codecov](https://codecov.io/gh/charlesrocket/dorst/branch/trunk/graph/badge.svg)](https://codecov.io/gh/charlesrocket/dorst)
### Usage

Run `dorst` to create a configuration file in `$HOME/.config/dorst` and set the first backup target.

`dorst -p /tmp/src-backups`

Example without authentication:

```yaml
---
targets:
  - https://github.com/charlesrocket/dotfiles
  - https://github.com/charlesrocket/freebsd-server
```

Add `ssh_key` path and `ssh_pass_protected` boolean to authenticate with SSH key:

```yaml
---
ssh_key: ~/private-ssh-key
ssh_pass_protected: true
targets:
  - https://github.com/charlesrocket/freebsd-station
  - git@github.com:charlesrocket/openbsd-station
```
