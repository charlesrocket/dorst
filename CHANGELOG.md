# Changelog

All notable changes to this project will be documented in this file.

## [0.20.0] - 2026-04-13

### Bug Fixes

- Add missing lifetime syntax
- Hide header bar icon
- Improve credential loop
- Consistent errors
- Drop `updated` icon

### Documentation

- Quote cargo commands
- Move packaging status

### Features

- Add OS string

### Operations

- Bump actions/checkout from 4 to 5
- Bump actions/labeler from 5 to 6
- Bump actions/checkout from 5 to 6
- Bump actions/cache from 4 to 5
- Bump codecov/codecov-action from 5 to 6

### Refactor

- Drop redundant `if`

### Styling

- Fix `set_callbacks()` formatting

### Testing

- Use explicit lifetime with `Commit`
- Move helper

### Build

- Bump slab from 0.4.10 to 0.4.11
- Update gtk
- Bump `toml` to 1.1
- Drop `vendored-openssl`

## [0.19.5] - 2025-08-04

### Bug Fixes

- Update wrappers

### Documentation

- Add packaging status
- Fix `Backups`

### Build

- Update GTK dependencies
- Bump `indicatif` to 0.18
- Bump `toml` to 0.9

## [0.19.4] - 2025-06-10

### Bug Fixes

- `MessageDialog` -> `AlertDialog`
- Edit version string

### Documentation

- Update README

### Testing

- Emit dialog signals
- Fix dialogs

### Build

- Bump adw features to 1.7

## [0.19.3] - 2025-05-11

### Bug Fixes

- Update desktop entry

### Miscellaneous tasks

- Move desktop files

### Build

- Add lto profile
- Update `exclude`
- Bump gtk features to 4.18

## [0.19.2] - 2025-05-09

### Bug Fixes

- Fix `repo_data.link` clone
- Drop redundant allocations
- Drop redundant clones
- Use explicit match with `process_repo`
- Improve `tx` optional
- Set missing feature
- Set `clone_repo` feature
- Set `fetch_repo` feature

### Miscellaneous tasks

- Remove makefile

### Styling

- Remove extra semicolons

### Build

- Bump `built` to 0.8

## [0.19.1] - 2025-04-17

### Bug Fixes

- Prevent data loss on crash
- Use explicit result list checks

### Refactor

- Prepare for rust 2024

### Styling

- Fix `current_branch()` formatting
- Fix project formatting

### Build

- Set `async-channel` version
- Bump `git2` to 0.20
- Bump rust edition to 2024

## [0.19.0] - 2025-01-12

### Bug Fixes

- Drop redundant `target` references
- Import `property::PropertySet`

### Features

- Add concurrent processing

### Miscellaneous tasks

- Update LICENSE

### Operations

- Bump codecov/codecov-action from 4 to 5
- Fix build job
- Update target list

## [0.18.1] - 2024-08-13

### Bug Fixes

- Rename `task-limiter` menu item

## [0.18.0] - 2024-08-07

### Bug Fixes

- Normalize `toml` output

### Features

- Migrate to `AboutDialog`
- Migrate to `PreferencesDialog`

### Operations

- Configure dependabot
- Update dependabot settings

### Build

- Bump clap from 4.5.9 to 4.5.11
- Bump predicates from 3.1.0 to 3.1.2
- Bump tokio from 1.38.1 to 1.39.2
- Bump toml from 0.8.15 to 0.8.16
- Bump assert_cmd from 2.0.14 to 2.0.15
- Bump clap from 4.5.11 to 4.5.13
- Bump toml from 0.8.16 to 0.8.19
- Bump tempfile from 3.10.1 to 3.11.0

## [0.17.1] - 2024-07-17

### Bug Fixes

- `set_from_icon_name()` -> `set_icon_name()`
- Update `remote.update_tips()`
- Update `clone!`

### Miscellaneous tasks

- Update dependencies
- Bump `git2` to 0.19

### Operations

- Bump `cache` to v4

## [0.17.0] - 2024-05-15

### Bug Fixes

- Fix task queue

### Features

- Handle directories via preferences window
- Allow directory path selection

### Miscellaneous tasks

- Update `clap` to 4.5

### Refactor

- Use `Config::default()`
- Drop `DorstPreferences` directory properties

## [0.16.0] - 2024-04-09

### Bug Fixes

- Update `load_css()`
- Handle missing fields
- Change `edit_button` confirmation

### Documentation

- Update example config
- Fix example line

### Features

- [**breaking**] Switch to TOML
- Show config alert

### Miscellaneous tasks

- Update dependencies
- Add funding info
- Add PR template

### Operations

- Bump `codecov-action` to v4
- Bump `gtk4` to 4.12.5
- Use `gtk4-rs` container
- Bump `labeler` to v5
- Update labels

### Refactor

- Fix `restore_data()` redundancies

### Testing

- Update `cli::files`

### Build

- Fix `tracing` dependencies
- Bump gtk features to 4.12

## [0.15.3] - 2024-02-16

### Build

- Make glib-build-tools optional

## [0.15.2] - 2024-02-15

### Bug Fixes

- Update `ObjectExt` import

### Miscellaneous tasks

- Disallow deprecated functions
- Update LICENSE
- Add `async-channel`
- Update GTK dependencies

### Refactor

- Switch to `async-channel`

## [0.15.1] - 2024-02-12

### Miscellaneous tasks

- Update dependencies
- Allow deprecated functions

## [0.15.0] - 2023-11-14

### Bug Fixes

- Update target prompt

### Documentation

- Describe backups
- Update `Usage`

### Features

- Change default behavior

### Testing

- Update tests

## [0.14.0] - 2023-10-25

### Bug Fixes

- Add default height to `DorstPreferences`

### Features

- Add `adw::PreferencesWindow`
- Add `logs_switch`
- Add `preferences` (#33)

### Refactor

- Convert `thread_pool` into property
- Rearrange menu
- `set_settings()` -> `setup_settings()`

### Testing

- Add `pool_limit()`
- Add `task_limiter()`

## [0.13.1] - 2023-09-23

### Bug Fixes

- Use static gresource

### Documentation

- Do not mention `sudo`
- Add GTK4

### Miscellaneous tasks

- Ignore gresource
- Add freedesktop files
- Resize app icon
- Set desktop entry flag

### Operations

- Update package lists
- Drop nightly builds

### Refactor

- Drop `Command` import

### Testing

- Fix `window()`

### Build

- Move custom script
- Update glib command
- Exclude data directory
- Add Makefile
- Improve compatability

## [0.13.0] - 2023-09-14

### Bug Fixes

- Change error icon
- Fix error wrapping
- Change error formatting
- Move error tooltip
- Unlink popover buttons
- Improve `GtkScrolledWindow` layout
- Fix `INVALID` links
- Improve handling of invalid links (#32)
- Edit `empty` message
- Adjust entry margin

### Features

- Drop `banner`
- Add error heading
- Add popover tooltips
- Show errors in row popover (#30)

### Miscellaneous tasks

- Bump `git2` to 0.18
- Bump `built` to 0.7

### Operations

- Drop `cache-apt-pkgs-action`
- Update `publish` job

### Refactor

- Add `repo_box` template

### Testing

- Fix popover tests
- Update `backup_error()`
- Update `invalid_url()` config
- Check invalid labels

## [0.12.1] - 2023-09-07

### Bug Fixes

- Fix `edit` dialog
- Fix `remove` dialog
- Drop `glib::idle_add_local()`
- Merge error strings
- Update `obj.connect_completed_notify()`
- Fix `RepoMessage::Reset`
- Fix `update_rows()`
- Set `repo_name` attribute

### Documentation

- Add `Features`

### Operations

- Update `checkout`
- Disable `fail-fast`

### Refactor

- Drop redundant `to_string()`
- Move `process_repo()`
- Drop `RepoMessage::Start`
- Reuse `updated` status
- Use `RepoObject` to process targets (#29)
- Move `completed_notify` callback
- `Message` -> `RowMessage`
- Drop `pending` status
- Use `repo_data` methods

### Testing

- Add `about_window()`
- Update `backup()`
- Add `backup_error()`
- Update `ssh_filter()`

### Build

- Set `git2` features

## [0.12.0] - 2023-09-01

### Bug Fixes

- Status check behind `tx.is_some()`
- Fix `status_revealer` margin

### Features

- Show updated targets
- Show the number of updated targets
- Indicate updated repositories (#24)
- Add `current_branch()`
- Indicate current branch
- Add status icon
- Indicate branches (#25)
- Add loggers
- Add `logs` option
- Add logging facility (#26)
- Add row menu
- Add `edit_button`
- Add `repos_list_count`
- Allow `link` edits (#27)
- Deactivate rows with `controls_disabled()`
- Add row dialogs
- Use `MessageDialog` (#28)

### Miscellaneous tasks

- Fix description

### Operations

- Add `cli` feature
- Set codecov layout
- Update features
- Fix codecov conditions
- Rename test job
- Fix test conditions

### Refactor

- Add `toggle_backups` template callback
- Add directory selection functions
- Use `updated_list`
- Reconnect `branch_revealer`
- Fix `edit_button.connect_clicked()` formatting
- Update `create_repo_row()`

### Testing

- Drop `features()`
- Add `toggle_backups()`
- Do not push to `dev` branch
- Update `backup()`
- Update `invalid_url()`
- Update `mirror()`
- Fix `mirror()`
- Fix `remove_target()`
- Add `edit_target()`
- Fix `edit_target()`
- Fix `remove_target()`

### Build

- Add `logs` feature

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

### Operations

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

### Operations

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

### Miscellaneous tasks

- Update description

### Operations

- Rename jobs
- Add `features`

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

### Operations

- Calculate checksums
- Use `dtolnay/rust-toolchain`
- Add `cross` builds
- Update `release` requirements
- Update `cross` setup
- Add `build` (#18)

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

### Features

- Track `fetch_repo()` progress
- Add `CssProvider`
- Set progress bar colors
- Set progress bar transition
- Track `mirror_repo()` progress (#12)
- Set `progressbar.main-progress`

### Miscellaneous tasks

- Update categories

### Operations

- Add pr labeler
- Add codecov
- Setup pr labeler (#13)

### Refactor

- Update `gtk` declarations
- Rename progress bar classes

### Styling

- Fix formatting
- Fix `pb_box` formatting

## [0.8.1] - 2023-07-01

### Bug Fixes

- Update `constructed()`
- Add `StyleManager` to `imp`

### Refactor

- Drop `setup_debug()`
- Add `Window::new()`

## [0.8.0] - 2023-06-29

### Bug Fixes

- Transparent link labels
- Add `win.close` action
- Do not update repos if `window` lists are empty
- Handle color scheme setting

### Features

- Add `repo_box` controller
- Add `remove_button`
- Indicate removed repo
- Add row popover (#11)

### Operations

- Add changelog
- Move `rustfmt`
- Fix `cache`

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

### Operations

- Switch `lint` to macos
- Update threshold

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

## [0.1.0] - 2023-02-03


