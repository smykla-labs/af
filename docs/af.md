# Command-Line Help for `af`

This document contains the help content for the `af` command-line program.

**Command Overview:**

* [`af`↴](#af)
* [`af completions`↴](#af-completions)
* [`af dot`↴](#af-dot)
* [`af dot ide`↴](#af-dot-ide)
* [`af git`↴](#af-git)
* [`af git clone-project`↴](#af-git-clone-project)
* [`af pgc`↴](#af-pgc)
* [`af shortcuts`↴](#af-shortcuts)
* [`af shortcuts abbreviations`↴](#af-shortcuts-abbreviations)
* [`af shortcuts abbreviations gcmff`↴](#af-shortcuts-abbreviations-gcmff)
* [`af shortcuts abbreviations gp`↴](#af-shortcuts-abbreviations-gp)
* [`af shortcuts abbreviations gd`↴](#af-shortcuts-abbreviations-gd)
* [`af browser`↴](#af-browser)
* [`af browser open`↴](#af-browser-open)
* [`af browser find`↴](#af-browser-find)

## `af`

**Usage:** `af <COMMAND>`

###### **Subcommands:**

* `completions` — Generate shell completion scripts
* `dot` — Helper commands related to dotfiles (defaults to `dot ide` if no subcommand is used)
* `git` — Git-related helper commands
* `pgc` — Shortcut for `af git clone-project`
* `shortcuts` — Short aliases for common command combinations (e.g. gcmff)
* `browser` — Open and inspect installed browsers



## `af completions`

Generate shell completion scripts

**Usage:** `af completions <SHELL>`

###### **Arguments:**

* `<SHELL>` — Target shell to generate completions for

  Possible values: `bash`, `elvish`, `fish`, `nushell`, `powershell`, `zsh`




## `af dot`

Helper commands related to dotfiles (defaults to `dot ide` if no subcommand is used)

**Usage:** `af dot [OPTIONS]
       dot ide [OPTIONS]`

**Command Alias:** `.`

###### **Subcommands:**

* `ide` — Open the dotfiles directory in an IDE

###### **Options:**

* `--path <PATH>` — Path to the dotfiles directory (overrides $DOTFILES_PATH)
* `-v`, `--verbose` — Increase logging verbosity
* `-q`, `--quiet` — Decrease logging verbosity



## `af dot ide`

Open the dotfiles directory in an IDE

If inside a JetBrains IDE, it will use that IDE to open the path. Otherwise, it prefers Zed, then GoLand, then VS Code.

**Usage:** `af dot ide [OPTIONS]`

###### **Options:**

* `--path <PATH>` — Path to the dotfiles directory (overrides $DOTFILES_PATH)



## `af git`

Git-related helper commands

**Usage:** `af git [OPTIONS] <COMMAND>`

**Command Alias:** `g`

###### **Subcommands:**

* `clone-project` — Clone a project repository and optionally open it in an IDE

###### **Options:**

* `-v`, `--verbose` — Increase logging verbosity
* `-q`, `--quiet` — Decrease logging verbosity



## `af git clone-project`

Clone a project repository and optionally open it in an IDE

**Usage:** `af git clone-project [OPTIONS] [REPOSITORY_URL]`

**Command Alias:** `cp`

###### **Arguments:**

* `<REPOSITORY_URL>` — The repository URL to clone (e.g. git@github.com:org/project.git)

###### **Options:**

* `--open-ide <OPEN_IDE>` — Open the cloned repository in an installed IDE picker, preselecting the best match

  Default value: `true`

  Possible values: `true`, `false`

* `-f`, `--force` — Force re-cloning even if the destination exists
* `--root-directory <ROOT_DIRECTORY>` — Root directory for placing the cloned project (uses $PROJECTS_PATH if set)
* `--directory <DIRECTORY>` — Exact directory path to clone into (overrides root-directory)
* `--rename-origin <RENAME_ORIGIN>` — Rename remote "origin" to "upstream" after cloning

  Default value: `true`

  Possible values: `true`, `false`

* `--convert-to-ssh <CONVERT_TO_SSH>` — If used URL is in HTTP(S) format, convert it to SSH format before cloning

  Default value: `true`

  Possible values: `true`, `false`




## `af pgc`

Shortcut for `af git clone-project`

**Usage:** `af pgc [OPTIONS] [REPOSITORY_URL]`

###### **Arguments:**

* `<REPOSITORY_URL>` — The repository URL to clone (e.g. git@github.com:org/project.git)

###### **Options:**

* `--open-ide <OPEN_IDE>` — Open the cloned repository in an installed IDE picker, preselecting the best match

  Default value: `true`

  Possible values: `true`, `false`

* `-f`, `--force` — Force re-cloning even if the destination exists
* `--root-directory <ROOT_DIRECTORY>` — Root directory for placing the cloned project (uses $PROJECTS_PATH if set)
* `--directory <DIRECTORY>` — Exact directory path to clone into (overrides root-directory)
* `--rename-origin <RENAME_ORIGIN>` — Rename remote "origin" to "upstream" after cloning

  Default value: `true`

  Possible values: `true`, `false`

* `--convert-to-ssh <CONVERT_TO_SSH>` — If used URL is in HTTP(S) format, convert it to SSH format before cloning

  Default value: `true`

  Possible values: `true`, `false`

* `-v`, `--verbose` — Increase logging verbosity
* `-q`, `--quiet` — Decrease logging verbosity



## `af shortcuts`

Short aliases for common command combinations (e.g. gcmff)

**Usage:** `af shortcuts [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `abbreviations` — Group of abbreviation subcommands (alias: a, abbr, abbreviation)

###### **Options:**

* `-v`, `--verbose` — Increase logging verbosity
* `-q`, `--quiet` — Decrease logging verbosity



## `af shortcuts abbreviations`

Group of abbreviation subcommands (alias: a, abbr, abbreviation)

**Usage:** `af shortcuts abbreviations <COMMAND>`

**Command Aliases:** `a`, `abbr`, `abbreviation`

###### **Subcommands:**

* `gcmff` — Expands to: git checkout <default-branch> && git fetch <remote> && git merge --ff-only <remote>/<default-branch>
* `gp` — Expands to: git push <remote> <branch> [optional flags]
* `gd` — Expands to: git diff <reference> -- <files> [optionally copy to clipboard]



## `af shortcuts abbreviations gcmff`

Expands to: git checkout <default-branch> && git fetch <remote> && git merge --ff-only <remote>/<default-branch>

**Usage:** `af shortcuts abbreviations gcmff`



## `af shortcuts abbreviations gp`

Expands to: git push <remote> <branch> [optional flags]

**Usage:** `af shortcuts abbreviations gp [OPTIONS]`

###### **Options:**

* `-r`, `--remote <REMOTE_PRIORITY>` — Remote priority strategy when pushing (e.g., upstream first, origin first)

  Default value: `origin`

  Possible values: `origin`, `origin-first`, `upstream`, `upstream-first`

* `-n`, `--no-verify` — Push with --no-verify flag (skip pre-push hooks)
* `-f`, `--force-with-lease` — Push with --force-with-lease flag (safe force push)
* `-F`, `--force` — Push with --force flag (unsafe force push, overrides remote)



## `af shortcuts abbreviations gd`

Expands to: git diff <reference> -- <files> [optionally copy to clipboard]

**Usage:** `af shortcuts abbreviations gd [OPTIONS]`

###### **Options:**

* `-f`, `--files <FILES>` — Specific files to diff (if omitted, diffs all changes)
* `-r`, `--reference <REFERENCE>` — Git reference to diff against (e.g. HEAD)
* `-p`, `--pbcopy` — Copy the diff output to clipboard using pbcopy



## `af browser`

Open and inspect installed browsers

**Usage:** `af browser [OPTIONS] <COMMAND>`

###### **Subcommands:**

* `open` — Group of browser subcommands (alias: b)
* `find` — Find installed browsers (alias: f)

###### **Options:**

* `-v`, `--verbose` — Increase logging verbosity
* `-q`, `--quiet` — Decrease logging verbosity



## `af browser open`

Group of browser subcommands (alias: b)

**Usage:** `af browser open [OPTIONS] <URL>`

**Command Alias:** `o`

###### **Arguments:**

* `<URL>` — The URL to open (must be a valid absolute URL)

###### **Options:**

* `--new-tab <NEW_TAB>` — Open each URL in a new tab (default: true)

  Default value: `true`

  Possible values: `true`, `false`

* `-b`, `--browser <BROWSER>` — Browser to use for opening the URL

  Default value: `firefox`

  Possible values: `brave`, `chrome`, `edge`, `firefox`, `opera`




## `af browser find`

Find installed browsers (alias: f)

**Usage:** `af browser find [OPTIONS]`

**Command Alias:** `f`

###### **Options:**

* `-k`, `--kind <BROWSER>` — Filter results by browser kinds (comma-separated)

  Default values: `brave`, `chrome`, `edge`, `firefox`, `opera`

  Possible values: `brave`, `chrome`, `edge`, `firefox`, `opera`

* `-a`, `--all` — Show all matching results (not just the first one per kind)
* `-p`, `--path` — Display only the full paths to executables



