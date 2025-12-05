# Pre Commit Submodule Hook

This pre-commit hook reminds you of your submodules before you commit.

It looks like this (YMMV) if you configure it as your pre-commit hook:

```
? The following submodules are modified but not staged for commit:
* sub2 (`git add sub2` to add submodule to staging)
The following submodules are modified and staged for commit:
* sub (`git restore --staged sub` to remove submodule from staging)
Do you wish to continue anyway? (y/n) › no
```

## How to build it so that it runs with minimal dependencies (statically)

Build using musl

```
cargo build --target=x86_64-unknown-linux-musl
```

and then

```
❯ ldd target/x86_64-unknown-linux-musl/debug/submodule-hook
        statically linked
```

## How to configure it as my `pre-commit` hook ?

1. compile it: `cargo build --target=x86_64-unknown-linux-musl --release`
2. install it `cp target/x86_64-unknown-linux-musl/release/submodule-hook .git/hooks/pre-commit`

## Configuration

Configuration is evaluated in this order:

1. `~/.gitconfig`
2. `.git/config`
3. CLI parameters cf `cargo run -- --help`
4. if no configuration is found it assumes `strict = false`, `staging = true`, `notstaging = true`

Edit `.git/config` or `~/.gitconfig`

```toml
[submodulehook]
    # if true the hook will fail when opening repository or submodule fails
    strict = false
    # if true also ask for confirmation before commit when a submodule is modified and staged
    staging = true
    # if true also ask for confirmation before commit when a submodule is modified and not staged
    notstaging = true
```

Or use `git config`:

```
git config submodulehook.strict false
git config submodulehook.staging true
git config submodulehook.notstaging true
```