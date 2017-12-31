# RusTOTPony

## Usage

Base commads (simple CRUD):

- show dashboard
- show all apps
  - with codes
  - without codes
- show code for app
- add app (title, subtitle, key)
- rename app
- delete app
- clear database

```sh
$ totp dash
$ totp list
$ totp show-all
$ totp show APPNAME
$ totp add APPNAME KEY # ask for key interactively?
$ totp delete APPNAME # with confirmation
$ totp rename APPNAME NEWNAME
$ totp eradicate # with non-trivial confirmation
```

## TODO

- completion
- encription with (manual or with keychain)
- password caching