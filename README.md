# 🐴 RusTOTPony

CLI manager of one-time password generators aka Google Authenticator.
Actually, it's simple in-file database which stores TOTP secret keys
without any encryption (will be added in the future).

## Installation

### From source

```sh
$ cargo install
```

## Usage

```text
USAGE:
    totp [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    add          Add a new generator
    dash         Show realtime dashboard with all generators
    delete       Delete generator
    eradicate    Delete all generators
    help         Prints this message or the help of the given subcommand(s)
    list         List all generators
    rename       Rename generator

Try `totp help [SUBCOMMAND]` to see help for the given subcommand
```

## TODO

- command completion
- database encryption
- database password caching
- tests
- binaries for main platforms
- refactor `show` and `show-all` commands

## License

Licensed under [the MIT License][MIT License].

[MIT License]: https://github.com/zebradil/rustotpony/blob/master/LICENSE