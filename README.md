# 🐴 RusTOTPony

[![Build Status](https://travis-ci.org/Zebradil/rustotpony.svg?branch=master)](https://travis-ci.org/Zebradil/rustotpony)
[![Build status](https://ci.appveyor.com/api/projects/status/rx68dv1kjepslelh/branch/master?svg=true)](https://ci.appveyor.com/project/Zebradil/rustotpony/branch/master)


CLI manager of one-time password generators aka Google Authenticator.
Actually, it's simple in-file database which stores TOTP secret keys
without any encryption (will be added in the future).

## Installation

### From source

1. Clone this repo
1. Run `cargo install` from the inside of the repo directory
1. Keep calm and wait for compilation

Probably, you need `gcc` (Linux) or `clang` (Mac OS) to compile dependencies.

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

Steps:

1. Retrieve your secret key from TOTP provider (it must be base32 encoded)
1. Add new generator with `totp add GENNAME` (you will be asked for your secret)
1. Check new generator by `totp list` or just display dashboard with one-time passwords with `totp dash`

#### WARNING: Currently there is no encryption of the application database. Be careful with `~/.rustotpony/db.json` and keep it safe.

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