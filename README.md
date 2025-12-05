# Pre Commit Submodule Hook

It looks like this (YMMV) if you configure it as your pre-commit hook:

```
dummy_repo_with_submodule on  main [!+] took 31s 
❯ git status
On branch main
Changes to be committed:
  (use "git restore --staged <file>..." to unstage)
        new file:   README.md

Changes not staged for commit:
  (use "git add <file>..." to update what will be committed)
  (use "git restore <file>..." to discard changes in working directory)
        modified:   sub (new commits)
```

```
dummy_repo_with_submodule on  main [!+] took 30s 
❯ ../submodule-hook/target/debug/submodule-hook
? The following submodules are modified but not staged for commit:
* `sub` is modified and not staged, (`git add sub` to add submodule to staging)
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