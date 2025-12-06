# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

## 0.1.0 - 2025-12-06

Initial release of submodule-hook pre-commit tool

### Added
- Interactive confirmation prompts for submodule changes before commit
- Detection of modified but not staged submodules
- Detection of modified and staged submodules
- Configuration support via git config (global and local)
- CLI parameters for runtime configuration (`--strict`, `--confirm-staging`, `--confirm-not-staging`, `--repo`)
- Strict mode option for failing on repository/submodule errors
- Configurable confirmation for staged submodule changes
- Configurable confirmation for unstaged submodule changes
- Static binary support via musl compilation
- Comprehensive unit tests for core functionality