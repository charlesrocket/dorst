# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2023-06-29

### Bug Fixes

- Transparent link labels
- Add `win.close` action
- Do not update repos if `window` lists are empty
- Handle color scheme setting

### CI/CD

- Add changelog
- Move `rustfmt`
- Fix `cache`

### Features

- Add `repo_box` controller
- Add `remove_button`
- Indicate removed repo
- Add row popover (#11)

### Miscellaneous tasks

- Version bump

### Refactor

- Move `repos()`
- Drop channels
- Move `text_prompt()`

### Testing

- Move `files`/`helper`

## [0.7.2] - 2023-06-26

### Bug Fixes

- Handle trailing slashes
- Indicate invalid links
- Fix scaling

### Miscellaneous tasks

- Version bump

### Refactor

- Edit `update_repos()`

### Testing

- Fix `config_empty()`
- Add `config_invalid_url()`
- Update `init()`

## [0.7.1] - 2023-06-23

### Bug Fixes

- Improve progress bar scale
- Update progress bar animation
- Handle text clipping
- Drop `RevealerTransitionType`
- Improve row spacing
- Set `valign`
- Increase `repo_box` height

### Miscellaneous tasks

- Version bump

### Styling

- Fix formatting

## [0.7.0] - 2023-06-22

### Bug Fixes

- Update `AdwClamp` spacing
- Improve input widgets
- Correct `mirror_repo` arguments
- Set `pb` size
- Improve `link`

### Features

- Handle window settings
- Improve status output
- Add SSH filter
- Add `destination` setting
- Improve `load_settings`
- Add progress bars
- Add progress bars (#10)

### Miscellaneous tasks

- Version bump
- Update serde_json

### Refactor

- Improve `Message::MirrorRepo` readability
- Merge git functions
- Improve `Window` strings
- Fix `args`
- `progress_bar` -> `pb`

### Styling

- Fix `set_directory`
- Fix `set_property`

## [0.6.0] - 2023-06-15

### Bug Fixes

- Process targets concurrently
- Update accelerators
- Set `default_width`
- Update menu

### CI/CD

- Switch `lint` to macos
- Update threshold

### Documentation

- Update usage

### Features

- Add `gui`
- Add `name_label`
- Add progress bar
- Add `ToastOverlay`
- Handle errors
- Add banners
- Hide inactive progress bar
- Indicate succesful rows

### Miscellaneous tasks

- Version bump
- Update dependencies

### Refactor

- Move `repo_data` iterator
- Change `mirror_repo() signature`
- Rename channel variables

### Testing

- Fix `config_empty()`

### Build

- Add custom script
- Add features

## [0.5.2] - 2023-05-11

### Miscellaneous tasks

- Bump git2 to 0.17
- Bump clap to 4.2
- Update dependencies

