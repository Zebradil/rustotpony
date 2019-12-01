# 🐴 RusTOTPony

![](https://github.com/zebradil/rustotpony/workflows/build/badge.svg)


CLI manager of [time-based one-time password](https://en.wikipedia.org/wiki/Time-based_One-time_Password_algorithm) generators.
It is a desktop alternative for Google Authenticator.

## Installation

Grab an appropriate binary from [the latest release](https://github.com/Zebradil/rustotpony/releases/latest).

### Build manually

#### From crates.io

Make sure you have `$HOME/.cargo/bin` in your `$PATH`.

```sh
$ cargo install rustotpony
```

#### From source

1. Clone this repo
1. Run `cargo install` from the inside of the repo directory
1. Keep calm and wait for compilation

Probably, you will need `gcc` (Linux) or `clang` (Mac OS) to compile dependencies.

## Usage

```text
$ totp help
🐴  RusTOTPony 0.2.3
German Lashevich <german.lashevich@gmail.com>
CLI manager of one-time password generators aka Google Authenticator

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

### Choose your password wisely

At the very first run `totp` asks for a password for a new database. It's located at `$HOME/.rustotpony/db.json` (don't be confused by `json` extension, actually, it's a binary file). If you forget the password or want to change it, you have to remove `$HOME/.rustotpony` directory. It's not convenient, but I'm going to improve usablity and an option for changing password.

### Basic scenario

1. Retrieve a secret key from your TOTP provider (it must be encoded with base32, for example: `GEZDGMZSGE2TKCQ=`)
    ```sh
    $ # Creating a fake secret key for demo purposes
    $ echo 123321555 | base32
    GEZDGMZSGE2TKNIK
    ```
    
1. Add new generator with `totp add <NAME>` (you will be asked for a secret and a password)
    ```sh
    $ # Adding a new TOTP generator
    $ totp add demo
    Enter your secret code: 
    Enter your database pass: 
    New application created: demo
    ```
    If it's not the first run, you'll be asked for password twice: for opening database and for saving it.

1. Use `totp list` to check your secrets
    ```sh
    $ # Listing all secrets in the database
    $ totp list
    Enter your database pass: 
    +------+------------------+----------+
    | name | key              | username |
    +------+------------------+----------+
    | demo | GEZDGMZSGE2TKNIK |          |
    +------+------------------+----------+
    ```
1. Use `totp dash` or just `totp` for realtime dashboard
    ```sh
    $ # Display real-time dashboard with all generators
    $ totp
    Enter your database pass: 
    Welcome to RusTOTPony realtime dashboard! Press ^C to quit.
    [=============================================               ]
    009216 demo
    ```
1. After hitting ^C it'll cleanup the dashboard
    ```sh
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
