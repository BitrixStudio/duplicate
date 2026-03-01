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
cargo install --path .
```
### With pre-built binaries (cross-platform)
```bash
curl -fsSL https://raw.githubusercontent.com/BitrixStudio/duplicate/master/install.sh | bash
```

### With PowerShell (Windows)
```powershell
irm https://raw.githubusercontent.com/BitrixStudio/duplicate/master/install.ps1 | iex
```

### License
MIT License (https://opensource.org/licenses/MIT)