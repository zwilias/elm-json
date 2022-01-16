[![CI Workflow](https://github.com/zwilias/elm-json/actions/workflows/ci.yaml/badge.svg)](https://github.com/zwilias/elm-json/actions/workflows/ci.yaml) [![npm version](https://badge.fury.io/js/elm-json.svg)](https://badge.fury.io/js/elm-json)

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

<!--ts-->
   * [Installation](#installation)
   * [Usage](#usage)
      * [elm-json help](#elm-json-help)
      * [Adding dependencies: elm-json install](#adding-dependencies-elm-json-install)
         * [Example: Installing the latest available version of a package](#example-installing-the-latest-available-version-of-a-package)
         * [Example: Installing the latest available 2.x.x version of a package](#example-installing-the-latest-available-2xx-version-of-a-package)
         * [Example: Installing as a test-dependency](#example-installing-as-a-test-dependency)
         * [Example: Installing multiple dependencies to a specified elm.json file](#example-installing-multiple-dependencies-to-a-specified-elmjson-file)
      * [Removing dependencies: elm-json uninstall](#removing-dependencies-elm-json-uninstall)
         * [Example: Uninstalling a package](#example-uninstalling-a-package)
      * [Upgrading dependencies: elm-json upgrade](#upgrading-dependencies-elm-json-upgrade)
         * [Example: Safely updating all dependencies](#example-safely-updating-all-dependencies)
         * [Example: Major version upgrades for your dependencies](#example-major-version-upgrades-for-your-dependencies)
      * [Initializing applications/packages: elm-json new](#initializing-applicationspackages-elm-json-new)
      * [Deeply listing all dependencies: elm-json tree](#deeply-listing-all-dependencies-elm-json-tree)
      * [For tooling: elm-json solve](#for-tooling-elm-json-solve)
      * [Generating shell completions: elm-json completions](#generating-shell-completions-elm-json-completions)

<!-- Added by: ilias, at: Fri Jun  5 19:07:47 CEST 2020 -->

<!--te-->

# Installation

Binaries are attached to the github releases and distributed for Windows, OS X
and Linux (statically linked with musl).

For ease of installation, an npm installer also exists:

```
npm install --global elm-json
```

# Usage

`elm-json` offers a bunch of subcommands to make life a little easier.

## `elm-json help`

```
USAGE:
    elm-json [FLAGS] <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
        --offline    Enable offline mode, which means no HTTP traffic will
                     happen
    -V, --version    Prints version information
    -v, --verbose    Sets the level of verbosity

SUBCOMMANDS:
    help         Prints this message or the help of the given subcommand(s)
    install      Install a package
    new          Create a new elm.json file
    tree         List entire dependency graph as a tree
    uninstall    Uninstall a package
    upgrade      Bring your dependencies up to date
```

Gives a quick overview of the more common subcommands. This can also be used for
finding documentation about specific subcommands.

## Adding dependencies: `elm-json install`

```
USAGE:
    elm-json install [FLAGS] <PACKAGE>... [-- <INPUT>]

FLAGS:
    -h, --help       Prints help information
        --test       Install as a test-dependency
    -V, --version    Prints version information
        --yes        Answer "yes" to all questions

ARGS:
    <PACKAGE>...    Package to install, e.g. elm/core or elm/core@1.0.2
    <INPUT>         The elm.json file to upgrade [default: elm.json]
```

`elm-json install` allows installing dependencies, at the latest version that
works given your existing dependencies, or a particular version if you so
choose. By adding the `--test` flag, the chosen package(s) will be added to your
`test-dependencies` rather than your regular `dependencies`.

### Example: Installing the latest available version of a package

```
elm-json install elm/http
```

Adds the latest version of `elm/http` to your dependencies.

For packages, it will use the latest possible version as the lowerbound, and the
next major as the exclusive upper bound. This mirrors the behaviour of `elm
install`.

For applications, this will pick the latest available version, adding all
indirect dependencies as well.

### Example: Installing the latest available 2.x.x version of a package

```
elm-json install elm/http@2
```

Adds the latest version of `elm/http` with `2` as its major version number to
your dependencies.

### Example: Installing as a test-dependency

```
elm-json install --test elm/http@2.0.0
```

Adds version 2.0.0 of `elm/http` to your test-dependencies.

For packages, the provided version is used as the lower bound, with the next
major being used as the exclusive upper bound.

For applications, this will install exactly the specified version.

### Example: Installing multiple dependencies to a specified `elm.json` file

```
elm-json install elm/http elm/json -- elm/elm.json
```

Add the latest possible versions of `elm/http` and `elm/json` to
`./elm/elm.json`.

## Removing dependencies: `elm-json uninstall`

```
USAGE:
    elm-json uninstall [FLAGS] <PACKAGE>... [-- <INPUT>]

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
        --yes        Answer "yes" to all questions

ARGS:
    <PACKAGE>...    Package to uninstall, e.g. elm/html
    <INPUT>         The elm.json file to upgrade [default: elm.json]
```

Uninstall dependencies. This is the inverse of `elm-json install` and its API is
similar but slightly simpler.

Version bounds may not be specified and `--test` is not an allowed flag for this
command.

### Example: Uninstalling a package

```
elm-json uninstall elm/html
```

Removes the `elm/html` package from your dependencies.

## Upgrading dependencies: `elm-json upgrade`

```
USAGE:
    elm-json upgrade [FLAGS] [INPUT]

FLAGS:
    -h, --help       Prints help information
        --unsafe     Allow major versions bumps
    -V, --version    Prints version information
        --yes        Answer "yes" to all questions

ARGS:
    <INPUT>    The elm.json file to upgrade [default: elm.json]
```

By default, this will only allow patch and minor changes for direct (test)
dependencies.

When the `--unsafe` flag is provided, major version bumps are also allowed. Note
that this may very well break your application. Use with care!

> **NOTE**: This subcommand does not yet support `elm.json` files with type
> `package`.

### Example: Safely updating all dependencies

```
elm-json upgrade
```

This command will check if any updates can safely be applied. In practice this
means that for your direct dependencies and direct test-dependencies, we'll look
for newer versions with the same major version number. Your indirect
dependencies and indirect test-dependencies may be modified in more ways,
depending on the constraints set by your direct dependencies.

### Example: Major version upgrades for your dependencies

```
elm-json upgrade --unsafe
```

If major version changes are available, this will attempt to apply them. Note
that this may still not update all dependencies to their latest release, if you
have another dependency preventing to do so.

If you want to upgrade a specific package to a specific version, try running
`elm-json install author/project@version`, which will tell you what package(s)
are preventing this from happening.

## Initializing applications/packages: `elm-json new`

```
USAGE:
    elm-json new

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information
```

Create a new `elm.json` file, for applications or packages.

This is very rudimentary right now.

## Deeply listing all dependencies: `elm-json tree`

```
USAGE:
    elm-json tree [FLAGS] [PACKAGE] [-- <INPUT>]

FLAGS:
    -h, --help       Prints help information
        --test       Promote test-dependencies to top-level dependencies
    -V, --version    Prints version information

ARGS:
    <PACKAGE>    Limit output to show path to some (indirect) dependency
    <INPUT>      The elm.json file to solve [default: elm.json]
```

Lists the entire dependency graph (with test-dependencies included when `--test`
is passed) as a tree.

Example output:

```
project
├── elm/core @ 1.0.2
├── elm/http @ 1.0.0
│   ├── elm/core @ 1.0.2 *
│   └── elm/json @ 1.1.3
│       └── elm/core @ 1.0.2 *
├── elm-community/json-extra @ 4.0.0
│   ├── elm/core @ 1.0.2 *
│   ├── elm/json @ 1.1.3 *
│   ├── elm/time @ 1.0.0
│   │   └── elm/core @ 1.0.2 *
│   └── rtfeldman/elm-iso8601-date-strings @ 1.1.3
│       ├── elm/core @ 1.0.2 *
│       ├── elm/json @ 1.1.3 *
│       ├── elm/parser @ 1.1.0
│       │   └── elm/core @ 1.0.2 *
│       └── elm/time @ 1.0.0 *
└── lukewestby/elm-http-builder @ 6.0.0
    ├── elm/core @ 1.0.2 *
    ├── elm/http @ 1.0.0 *
    ├── elm/json @ 1.1.3 *
    ├── elm/time @ 1.0.0 *
    └── elm/url @ 1.0.0
        └── elm/core @ 1.0.2 *

Items marked with * have their dependencies ommitted since they've already
appeared in the output.
```

Specifying a package-name will filter the tree so only paths leading to the
specified package, in direct and indirect dependencies, will be shown.

## For tooling: `elm-json solve`

```
USAGE:
    elm-json solve [FLAGS] [OPTIONS] [--] [INPUT]

FLAGS:
    -h, --help        Prints help information
    -m, --minimize    Choose lowest available versions rather than highest
        --test        Promote test-dependencies to top-level dependencies
    -V, --version     Prints version information

OPTIONS:
    -e, --extra <PACKAGE>...    Specify extra dependencies, e.g. elm/core or
                                elm/core@1.0.2

ARGS:
    <INPUT>    The elm.json file to solve [default: elm.json]
```

Documentation TBD. Intended for other tooling to use, not meant for human
consumption.

## Generating shell completions: `elm-json completions`

```
USAGE:
    elm-json completions <SHELL>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

ARGS:
    <SHELL>    The shell to generate the script for [possible values: bash,
               fish, zsh]
```

Create completion scripts for `elm-json` for `bash`/`fish`/`zsh`.
