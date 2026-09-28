# Cross-platform directory definitions `xdd`

`xdd` registers and resolves cross-platform directory definitions.

## Configuration

On Linux, create `$XDG_CONFIG_HOME/xdd/config.toml`, or
`~/.config/xdd/config.toml` when `XDG_CONFIG_HOME` is not set. On Windows,
run `xdd config path` to find the configuration file (normally under
`%APPDATA%\xdd\config.toml`):

```toml
[roots]
docs = "~/Documents"
projects = "~/Code/contrib"
```

Root paths must be absolute, optionally using `~` or `~/...`. Root names may
contain any characters except `:`.

## URLs

A URL uses the following form:

```text
xdd://root:relative/path
```

For example:

```markdown
[Repo Path](xdd://projects:code/repo)
```

The path is resolved below the selected root. Percent encoding is supported,
including for spaces and path components. `..` is allowed only when the
normalised path remains below the root. Query parameters and fragments are not
supported yet.

## Usage

Open a URL explicitly with:

```bash
xdd open 'xdd://projects:code/repo'
```

Resolve a URL to its absolute path without opening it:

```bash
xdd resolve 'xdd://projects:code/repo'
```

Inspect and edit the configuration with:

```bash
xdd config path
xdd config edit
```

List configured roots:

```bash
xdd roots list
```

Create a link from an existing directory. The deepest matching root is chosen
automatically:

```bash
xdd link ~/Atelier/prj/xdd
# xdd://projects:xdd
```

Use `--root` to select an alias explicitly, and `--format` with `plain`,
`markdown`, `typst`, `org`, or `latex` to render a document link. Formatted
links use the directory name as their label by default; `--label` overrides it.



## Build

### Install on Linux

Build and run the program with:

```bash
cargo build
xdd register
```

`xdd register` installs a user-level desktop entry and makes `xdd` the default
handler for `xdd://` URLs. It delegates the MIME association to `xdg-mime`.
On systems where the relevant `mimeapps.list` is on a read-only filesystem,
`xdg-mime` cannot write the association and does not return status 0, so
`xdd register` cannot complete the registration. Use another method to write
the desktop association in that environment, such as managing a writable
`mimeapps.list` through the desktop environment or system configuration.

### Install on Windows

Download and extract `xdd-<version>-x86_64-pc-windows-msvc.zip`, or build with
`cargo build --release`. From the extracted directory, register the URL scheme
for the current user with PowerShell:

```powershell
powershell -ExecutionPolicy Bypass -File .\register.ps1 -ExePath .\xdd.exe
```

To register for all users, install the executable in a location accessible to
all users and run this from an elevated PowerShell session:

```powershell
powershell -ExecutionPolicy Bypass -File .\register.ps1 -ExePath .\xdd.exe -Global
```

Per-user registrations take precedence over the machine-wide one.

The registration stores the executable's absolute path; rerun the script if you
move it. For a source build, use
`packaging/windows/register.ps1 -ExePath target/release/xdd.exe` instead.
Windows paths in TOML can be written as literal strings, for example
`docs = 'C:\Users\me\Documents'`. `xdd register` currently works only on Linux.

### Build with Nix

```bash
nix build github:js0ny/xdd
```

Adding the package to your `environment.systemPackages` will handle the URI scheme registration automatically.

## Roadmap Before 1.0.0

- [x] Linux: Basic implementation on Linux
- [ ] Windows: Implement on Windows, define a denied list of symbols, should be cross-platform
- [ ] Linux: Configuration `linux.handler`, default to `xdg-open`
- [ ] macOS: Implement, URL Scheme with a swift wrapper
- [x] CLI: subcommands `config`, `roots`, and `link`

## License

[GPL-3.0-or-later](https://spdx.org/licenses/GPL-3.0-or-later.html)
