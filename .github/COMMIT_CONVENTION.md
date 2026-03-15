# Commit Convention

This project follows [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/).

## Format

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

- **type**: one of the types below
- **scope**: optional, e.g. `config`, `exif`, `watcher`, `ci`, `spk`
- **subject**: imperative, lowercase, no trailing period, ≤72 characters

## Types

| Type       | Use for                                                   |
|------------|-----------------------------------------------------------|
| `feat`     | A new feature                                             |
| `fix`      | A bug fix                                                 |
| `chore`    | Maintenance tasks (deps bump, tooling, etc.)              |
| `docs`     | Documentation only changes                               |
| `style`    | Code style (formatting, no logic change)                  |
| `refactor` | Code change that neither fixes a bug nor adds a feature   |
| `perf`     | Performance improvements                                  |
| `test`     | Adding or fixing tests                                    |
| `build`    | Build system or scripts (SPK, cross-compile)              |
| `ci`       | CI/CD configuration changes                               |
| `revert`   | Reverts a previous commit                                 |

## Examples

```
feat(watcher): add debounced inotify support
fix(exif): handle missing DateTimeOriginal gracefully
chore(deps): bump notify to 6.1
docs(readme): add installation instructions for DSM 7
test(processor): add parametrized tests for conflict resolution
refactor(naming): extract pattern engine to separate module
perf(processor): use streaming instead of loading files in RAM
build(spk): add armv7 cross-compilation step
ci: add ARM artifact upload to CI workflow
```

## Breaking changes

Append `!` after the type/scope and add a `BREAKING CHANGE:` footer:

```
feat(config)!: rename move_file to move_files

BREAKING CHANGE: the config key `move_file` has been renamed to `move_files`.
Update your config.toml accordingly.
```
