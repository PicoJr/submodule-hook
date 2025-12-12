[![Crates.io](https://img.shields.io/crates/v/submodule-hook.svg)](https://crates.io/crates/submodule-hook)

# Pre Commit Submodule Hook

This pre-commit hook asks you to confirm when submodules are either:
* modified and not staged for commit
* modified and staged for commit

It looks like this (YMMV) if you configure it as your pre-commit hook:

```
? The following submodules are modified but not staged for commit:
* sub2 (`git add sub2` to add submodule to staging)
The following submodules are modified and staged for commit:
* sub (`git restore --staged sub` to remove submodule from staging)
Do you wish to continue anyway? (y/n) › no
```

## Install and configure it as my `pre-commit` hook

### From crates.io (recommended)

1. install it: `cargo install submodule-hook`
2. install it as your `pre-commit` hook: `cp $(which submodule-hook) .git/hooks/pre-commit`

### From source

1. compile it: `cargo build --target=x86_64-unknown-linux-musl --release`
2. install it as your `pre-commit` hook: `cp target/x86_64-unknown-linux-musl/release/submodule-hook .git/hooks/pre-commit`

### Try it without setting it as a `pre-commit` hook

```
submodule-hook --repo <path-to-your-repo>
```

## Uninstall

Remove the hook using: `rm .git/hooks/pre-commit`

If you installed it from crates.io, you can remove the binary from your `~/.cargo/bin` directory using: `cargo uninstall submodule-hook`

## Configuration

Configuration is evaluated in this order:

1. global `~/.gitconfig`
2. local `.git/config`
3. CLI parameters cf `cargo run -- --help`
4. if no configuration is found it assumes `strict = false`, `staging = true`, `notstaging = true`

It means the CLI prioritizes the CLI parameters, then local config, then global config.

Edit local `.git/config` or global `~/.gitconfig`

```toml
[submodulehook]
    # if true the hook will fail when opening repository or submodule fails
    strict = false
    # if true also ask for confirmation before commit when a submodule is modified and staged
    staging = true
    # if true also ask for confirmation before commit when a submodule is modified and not staged
    notstaging = true
```

> if both `staging` and `notstaging` are set to `false` then the hook will be disabled

Or use `git config`:

```
git config submodulehook.strict false
git config submodulehook.staging true
git config submodulehook.notstaging true
```

## Debug

debug logs can be enabled using `RUST_LOG=debug`:

```
RUST_LOG=debug submodule-hook --repo <path-to-your-repo>
```

## Exit Code

* `0` if the hook ran without errors and the user chose to continue when prompted for confirmation
* `1` if the user chose not to continue when prompted for confirmation
* `130` if the user `ctrl-c` the hook

## CHANGELOG

Please see the [CHANGELOG](CHANGELOG.md) for a release history.
