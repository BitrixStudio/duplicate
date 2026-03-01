# duplicate

Run **N copies** of the same command and view their outputs side-by-side in one terminal window.

Useful for:
- comparing outputs (e.g. two directories)
- running multiple environments in parallel
- watching multiple logs / pings side-by-side

> Note: This version of the tool currently captures `stdout/stderr` via pipes. It is just for MVP phase at this stage. Fully interactive TUI apps (vim, htop) require PTY support and may be added later.

---

## Installation

### With Cargo (cross-platform)
```bash
cargo install duplicate-cli
```
### With pre-built binaries (cross-platform)
```bash
curl -fsSL https://raw.githubusercontent.com/BitrixStudio/duplicate/master/install.sh | bash
```

### With PowerShell (Windows)
```powershell
irm https://raw.githubusercontent.com/BitrixStudio/duplicate/master/install.ps1 | iex
```

---


## Usage
```bash
duplicate [OPTIONS] <CMD> [ARGS]...
```
-n, --n <N> number of panes/instances (default: 2)

Instance arguments can be provided in two ways:

### Shorthand mode

If ::: is not present, the last N arguments are treated as per-instance arguments (one arg per instance).

```bash
duplicate -n 3 cmd /C dir . src target
```

runs:
```bash
cmd /C dir .
cmd /C dir src
cmd /C dir target
```

### Grouped mode

If ::: is present, the arguments before ::: are treated as common arguments, and the arguments after ::: are N instance arguments.

```bash
duplicate -n 3 curl -sS ::: https://example.com ::: https://httpbin.org/get ::: https://example.org
```
###### ⚠️ This functionality is still experimental and may not work as expected on all operating systems.


#### More examples
Windows:
```bash
duplicate cmd /C dir . src
```
```bash
duplicate -n 4 cmd /C ping 127.0.0.1 localhost 8.8.8.8 1.1.1.1
```
```bash
duplicate -n 3 cmd /C dir . src target
```

Linux/macOS:
```bash
duplicate ls -lha /etc /var
```
```bash
duplicate -n 3 ls -lha /etc /var /tmp
```
```bash
duplicate -n 2 curl -sS ::: https://example.com ::: https://example.org
```
---

## License
MIT License (https://opensource.org/licenses/MIT)
