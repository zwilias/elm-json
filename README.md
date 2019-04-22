# elm-json
> Deal with your elm.json

> **NOTE:** This is very much a work in progress. May mess up your files
> completely. Use with care.

`elm-json` provides a bunch of tools to make common tasks involving your
`elm.json` files a little easier: upgrading your dependencies, installing
specific versions of packages, removing dependencies or initializing new
packages, to name a few.

`elm-json` never writes to files used by the official toolchain, except
`elm.json` itself. It may, however, read some files from your `ELM_HOME` when
possible, to prevent downloading things you may already have on your filesystem.

## Installation

Binaries are attached to the github releases and distributed for Windows, OS X
and Linux (statically linked with musl).

For ease of installation, an npm installer also exists:

```
npm install --global elm-json
```

## Usage

`elm-json` offers a bunch of subcommands to make life a little easier.

### `elm-json help`

Gives a quick overview of the more common subcommands. This can also be used for
finding documentation about specific subcommands.

### `elm-json install`

> More info: `elm-json help install`

`elm-json install` allows installing dependencies, at the latest version that
works given your existing dependencies, or a particular version if you so
choose. By adding the `--test` flag, the chosen package(s) will be added to your
`test-dependencies` rather than your regular `dependencies`.

#### Examples

```
elm-json install elm/http
```

Adds the latest version of `elm/http` to your dependencies.

For packages, it will use the latest possible version as the lowerbound, and the
next major as the exclusive upper bound. This mirrors the behaviour of `elm
install`.

For applications, this will pick the latest available version, adding all
indirect dependencies as well.


```
elm-json install --test elm/http@2.0.0
```

Adds version 2.0.0 of `elm/http` to your test-dependencies.

For packages, the provided version is used as the lower bound, with the next
major being used as the exclusive upper bound.

For applications, this will install exactly the specified version.

```
elm-json install elm/http elm/json -- elm/elm.json
```

Add the latest possible versions of `elm/http` and `elm/json` to
`./elm/elm.json`.

### `elm-json uninstall`

> More info: `elm-json help uninstall`

Uninstall dependencies. This is the inverse of `elm-json install` and its API is
similar but slightly simpler.

Version bounds may not be specified and `--test` is not an allowed flag for this
command.

#### Examples

```
elm-json uninstall elm/html
```

Removes the `elm/html` package from your dependencies.

> **NOTE**: This subcommand does not yet support `elm.json` files with type
> `package`.

### `elm-json upgrade`

> More info: `elm-json help upgrade`

Upgrade your dependencies.

By default, this will only allow patch and minor changes for direct (test) dependencies.

When the `--unsafe` flag is provided, major version bumps are also allowed. Note
that this may very well break your application. Use with care!

> **NOTE**: This subcommand does not yet support `elm.json` files with type
> `package`.

### `elm-json new`

> More info: `elm-json new`

Create a new `elm.json` file, for applications or packages.

This is very rudimentary right now.

### `elm-json solve`

> More info: `elm-json help solve`

Documentation TBD. Intended for other tooling to use, not meant for human consumption.

### `elm-json completions`

> More info: `elm-json help completions`

Create completion scripts for `elm-json` for `bash`/`fish`/`zsh`.
