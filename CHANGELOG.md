# Changelog

All notable changes to this project will be documented in this file.

## [0.11.3] - 2023-08-09

### Bug Fixes

- Fix deltas
- Merge objects/deltas feedback
- Do not reset row feedback (#23)

### Refactor

- Add `window::Status`

### Styling

- Fix `set_row_channel()` formatting

### Testing

- Add `start()`

## [0.11.2] - 2023-08-06

### Bug Fixes

- Fix `button.controls` transition
- Drop `directory_dialog`
- Fix `progressbar` colors
- Set `progressbar.row-progress` transitions

### Refactor

- Drop `AtomicUsize`

### Testing

- Fix window builder

## [0.11.1] - 2023-08-03

### Bug Fixes

- Fix empty `ListBox`
- Set `row-progress` radius
- Replace `set_placeholder()` with `stack_list`
- Animate controls

### Refactor

- Use property methods
- Update `empty` header bar
- Do not connect to `revealer`

### Styling

- Fix css formatting

### Testing

- Fix `xdg_path()`
- Increase `backup()` breaks
- Add `config()`

## [0.11.0] - 2023-08-02

### Bug Fixes

- Scroll `GtkListBox` only
- Fix `GtkScrolledWindow` animation
- Fix `task-limiter` action
- `thread::spawn()` -> `gio::spawn_blocking()`
- Update `Variant`s
- Fix `ControlFlow`
- Update `close_request()`
- Update `setup_repos()`

### Features

- Add thread pool
- Add `task_limiter`
- Add `set_task_limiter()`
- Implement thread pooling (#21)
- Enable task limiter
- Handle task limiter setting
- Improve color scheme handling
- Add `HTTPS` filter
- Convert `task-limiter` to `PropertyAction`
- Update shortcuts
- Move main progress bar
- Attach `main-progress` to linked widgets
- Update `main-menu`

### Miscellaneous tasks

- Update dependencies
- Update `gui` dependencies (#22)

### Refactor

- Update `gtk`/`std` imports
- Use `#[glib::derived_properties]`
- Update `RepoObject`

### Styling

- Fix `task_limiter()` formatting
- Move `action_task_limiter`

### Testing

- Update `ssh_filter()`
- More `ssh_filter()` errors
- Add `task_limiter()`
- Update `wait_ui()`
- Add task limiter data
- Update `settings()` config
- Fix `backup()` breaks

## [0.10.3] - 2023-07-26

### Bug Fixes

- Fix target filter

### CI/CD

- Fix code coverage conditions
- Add `flag_management`

### Refactor

- `mirror_all` -> `process_targets`
- Add `controls_disabled()`

### Testing

- Improve `backup()`
- Add `ssh_filter()`
- Add `remove_target()`
- Fix `test_path()`
- Fix `backup()`

## [0.10.2] - 2023-07-25

### Bug Fixes

- Properly indicate failures

### CI/CD

- Update `codecov`
- Update coverage settings
- Publish after `create-release`
- Test features
- Rename `test` job
- Set `codecov-action` requirements
- Include features in `test` job (#20)
- Add `codecov` flags
- Switch to `setup-xvfb`

### Styling

- Fix `repo_entry_empty()` formatting

### Testing

- Add `tests` module
- Add `color_scheme()`
- Add `backup()`
- Add `settings()`
- Add `repo_entry_empty()`
- Add gui tests (#19)
- Call `toggle-color-scheme` action
- Update `color_scheme()`
- Update `settings()`
- Add `entries()`

## [0.10.1] - 2023-07-19

### Bug Fixes

- Edit `config` argument
- Use `CARGO_PKG_DESCRIPTION`

### CI/CD

- Rename jobs
- Add `features`

### Miscellaneous tasks

- Update description

### Testing

- Add `cli` module

### Build

- Fix feature scopes

## [0.10.0] - 2023-07-18

### Bug Fixes

- Adjust progress bar transitions
- Edit `source_prompt`
- Handle `button_source_dest` css class
- Fix `show_about_dialog()`
- Detailed dialog toasts
- Fix `silent`
- Update layout
- Change status page icon

### CI/CD

- Calculate checksums
- Use `dtolnay/rust-toolchain`
- Add `cross` builds
- Update `release` requirements
- Update `cross` setup
- Add `build` (#18)

### Documentation

- Update example
- Update description
- Fix `Usage`

### Features

- Add commit hash to version string
- Include commit hash in version strings (#14)
- Add `clone_target()`
- Add `bootstrap`
- Add target clones
- Add `backups_enabled`
- Add `clone` (#15)
- Set application links
- Add changelog link
- Change GUI flag
- Hide `button_backup_dest`
- Add placeholder
- Set status page (#17)

### Refactor

- Move channel logic
- `clone_target` -> `process_target`
- Use `repo` in `process_target()`
- Update `format!` strings
- Fix `fetch_repo()`
- Update `show_message()`
- Drop `default_branch`
- Add `repos_list_activated`

### Styling

- Fix `process_target()` formatting
- Fix `GtkEntry`

### Testing

- Fix configs
- Fix `init()`
- Add `test_path()`
- Add `bootstrap()`
- Fix helper
- Add `features()`

### Build

- Add build-time information

## [0.9.2] - 2023-07-08

### Bug Fixes

- Improve progress bar animations

### Refactor

- Drop `success_item`

## [0.9.1] - 2023-07-07

### Bug Fixes

- Use `SwingRight`
- Drop `Message::Spin`
- Update `Message::Progress`

## [0.9.0] - 2023-07-04

### Bug Fixes

- Check `tx`
- Set `clone_repo()` deltas
- Improve main progress bar
- Change progress bar width
- Do not clamp input widgets
- Drop `width_request()`
- Drop `height_request()`
- Align main progress bar
- Update `progress_bar` container

### CI/CD

- Add pr labeler
- Add codecov
- Setup pr labeler (#13)

### Features

- Add `CssProvider`
- Set progress bar colors
- Set progress bar transition
- Track `mirror_repo()` progress (#12)
- Set `progressbar.main-progress`

### Miscellaneous tasks

- Update categories

### Refactor

- Update `gtk` declarations
- Rename progress bar classes

### Styling

- Fix `pb_box` formatting

## [0.8.1] - 2023-07-01

### Bug Fixes

- Update `constructed()`
- Add `StyleManager` to `imp`

### Features

- Track `fetch_repo()` progress

### Refactor

- Drop `setup_debug()`
- Add `Window::new()`

### Styling

- Fix formatting

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

