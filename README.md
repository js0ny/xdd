# Cross-platform directory definitions `xdd`

`xdd` registers and resolves cross-platform directory definitions.

## Configuration

Create `$XDG_CONFIG_HOME/xdd/config.toml`, or `~/.config/xdd/config.toml` when
`XDG_CONFIG_HOME` is not set:

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

## Build

### Build on Linux

Build and run the program with:

```bash
cargo build
xdd register
```

`xdd register` installs a user-level desktop entry and makes `xdd` the default
handler for `xdd://` URLs. Open a URL explicitly with:

```bash
xdd open 'xdd://projects:code/repo'
```

Targets are opened with `xdg-open`. The Linux handler is currently fixed and is
not configurable yet.

### Build with Nix

```bash
nix build github:js0ny/xdd
```
