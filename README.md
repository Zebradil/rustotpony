# 🐴 RusTOTPony

![crates.io](https://img.shields.io/crates/v/rustotpony.svg)
![build](https://github.com/zebradil/rustotpony/workflows/build/badge.svg)

CLI manager of [time-based one-time password](https://en.wikipedia.org/wiki/Time-based_One-time_Password_algorithm) generators.
It is a desktop alternative to Google Authenticator.

## Installation

### Arch Linux

Packages, available in [AUR](https://aur.archlinux.org/):

- [rustotpony](https://aur.archlinux.org/packages/rustotpony/)
- [rustotpony-bin](https://aur.archlinux.org/packages/rustotpony-bin/)

### Other

Grab an appropriate binary from [the latest release](https://github.com/Zebradil/rustotpony/releases/latest) and put it
in a place of your choice. If you're on the *nix system, don't forget to set proper permissions: `chmod +x totp`.

### Build manually

#### From crates.io

Make sure you have `$HOME/.cargo/bin` in your `$PATH`.

```shell
cargo install rustotpony
```

#### From source

1. Clone this repo
1. Run `cargo install` from the inside of the repo directory
1. Keep calm and wait for the compilation

Probably, you will need `gcc` (Linux) or `clang` (Mac OS) to compile dependencies.

## Usage

```text
$ totp help
🐴 RusTOTPony 0.3.2

Manager of one-time password generators

Usage: totp [COMMAND]
Commands:
  dash       Show realtime dashboard with all generators
  list       List all generators
  add        Add a new generator
  delete     Delete a generator
  rename     Rename a generator
  eradicate  Delete all generators
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

### Choose your password wisely

At the very first run, `totp` asks for a password for a new database.
It's located at `$HOME/.rustotpony/db.json` (don't be confused by `json` extension it's a binary file in fact).
If you forget the password or want to change it, you have to remove `$HOME/.rustotpony` directory.
It's not convenient, but I'm going to improve usability and add a command for changing the password.

### Basic scenario

1. Retrieve a secret key from your TOTP provider (it must be encoded with base32, for example, `GEZDGMZSGE2TKCQ=`)

    ```shell
    $ # Creating a fake secret key for demo purposes
    $ echo 123321555 | base32
    GEZDGMZSGE2TKNIK
    ```

1. Add new generator with `totp add <NAME>` (you will be asked for a secret and a password)

    ```shell
    $ # Adding a new TOTP generator
    $ totp add demo
    Enter your secret code:
    Enter your database pass:
    New application created: demo
    ```

    If it's not the first run, you'll be asked for a password twice: for opening the database and for saving it.

1. Use `totp list` to check your secrets

    ```shell
    $ # Listing all secrets in the database
    $ totp list
    Enter your database pass:
    +------+------------------+----------+
    | name | key              | username |
    +------+------------------+----------+
    | demo | GEZDGMZSGE2TKNIK |          |
    +------+------------------+----------+
    ```

1. Use `totp dash` or just `totp` for real-time dashboard

    ```shell
    $ # Display real-time dashboard with all generators
    $ totp
    Enter your database pass:
    Welcome to RusTOTPony realtime dashboard! Press ^C to quit.
    [=============================================               ]
    009216 demo
    ```

1. After hitting ^C it'll clean up the dashboard

    ```shell
    $ totp
    Enter your database pass:
    I won't tell anyone about this 🤫
    ```

## TODO

- command completion
- database password caching
- tests
- refactor `show` and `show-all` commands

## License

Licensed under [the MIT License][MIT License].

[MIT License]: https://github.com/zebradil/rustotpony/blob/master/LICENSE
