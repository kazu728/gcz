# gcz

![Crates.io](https://img.shields.io/crates/v/gcz)
[![test](https://github.com/kazu728/gcz/actions/workflows/test.yml/badge.svg)](https://github.com/kazu728/gcz/actions/workflows/test.yml)
![License](https://img.shields.io/crates/l/gcz)

**gcz** is a command-line tool that simplifies Git commit processes by providing an interactive interface for selecting commit types and composing commit messages following the conventional commits standard.

## Features

![screen.gif](./assets/screen.gif)

- **Interactive Commit Type Selection**: Choose from predefined commit types like `feat`, `fix`, `docs`, `style`, `refactor`, `perf`, `test`, `ci`, and `chore`.
- **Real-time Filtering**: Filter commit types by typing keywords.

## Installation

### Homebrew

```bash
brew install kazu728/tap/gcz
```

### Cargo

```bash
cargo install gcz
```

## Usage

Navigate to your Git repository and run:

```bash
gcz
```

### Command-line Options

- `-e`, `--emoji`: _(Work in Progress)_ Add emojis to the commit template.

## Testing

Run the following command to execute tests:

```bash
cargo test
```
