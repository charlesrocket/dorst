# `dørst`
[![Crates.io](https://img.shields.io/crates/v/dorst)](https://crates.io/crates/dorst)
[![Tests](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml/badge.svg?branch=trunk)](https://github.com/charlesrocket/dorst/actions/workflows/tests.yml)
[![codecov](https://codecov.io/gh/charlesrocket/dorst/branch/trunk/graph/badge.svg)](https://codecov.io/gh/charlesrocket/dorst)
### Usage

It takes a YAML file with targets and an optional backup destination.

`dorst example.yml -p /tmp/src-backups`

`example.yml`:
```yaml
---
# Example list
targets:
  - https://github.com/charlesrocket/dotfiles
  - https://github.com/charlesrocket/freebsd-station
  - https://github.com/charlesrocket/freebsd-server
```
