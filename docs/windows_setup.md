# Windows Setup Guide

This page complements [`CONTRIBUTING.md`](../CONTRIBUTING.md) with Windows-specific
instructions for installing and using `soroban-cost-linter`. The examples below are
**PowerShell** (not `cmd.exe`); if you are using Command Prompt, the equivalent
commands are noted inline.

## Choose your environment

You have two viable paths on Windows. Pick the one that matches your goal:

| Option | Pros | Cons |
| --- | --- | --- |
| **WSL2 with Ubuntu** (recommended) | Identical to the project's CI; fewest surprises; full Linux toolchain. | Extra OS feature to enable; some cross-filesystem operations are slower. |
| **Native Windows + PowerShell** | No VM overhead; works with Windows-native editors. | Some Rust nightly + Dylint dynamic-loading flows require the **MSVC** linker (Visual Studio Build Tools), and this path is not regularly tested upstream. |

**Recommendation:**

- If you only want to **use** the linter on existing Soroban contracts, **WSL2 is the
  path of least resistance** — the tool's CI only runs on Ubuntu, so you will see the
  same toolchain and linker behaviour locally.
- If you are **actively contributing** to `soroban-cost-linter`, **WSL2 is strongly
  recommended** as well. Native Windows development is not regularly tested upstream,
  so a CI-only discrepancy is more likely.

---

## Option A — WSL2 with Ubuntu (recommended)

### 1. Enable WSL2

From a **PowerShell (Administrator)** prompt:

```powershell
wsl --install
```

Restart when prompted, then launch "Ubuntu" from the Start menu and create a Linux
user when prompted.

### 2. Install the Rust toolchain inside Ubuntu

Open the Ubuntu shell from the Start menu (or run `wsl` from PowerShell):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source "$HOME/.cargo/env"
```

Verify:

```bash
rustc --version
cargo --version
```

### 3. Install Dylint and the linter

Follow the standard cross-platform commands — they work the same on Ubuntu:

```bash
cargo install cargo-dylint dylint-link --version "^6.0.1"

cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
```

### 4. Clone and build the linter (filesystem matters)

Open the Ubuntu shell **from inside WSL2**, not from `/mnt/c/`:

```bash
# ✅ Linux filesystem — fast, recommended
cd ~
git clone https://github.com/Tollcraft/soroban-cost-linter.git
cd soroban-cost-linter
cargo build
```

```bash
# ⚠️ Windows filesystem mounted at /mnt/c — works but is significantly slower
#     and trips up scripts that translate CRLF↔LF.
cd /mnt/c/Users/<you>/projects
git clone https://github.com/Tollcraft/soroban-cost-linter.git
cd soroban-cost-linter
cargo build
```

**Tip:** If you already cloned into `/mnt/c/...` and builds are slow, move the
checkout under `~/` and add a symlink from the original location. The Linux-side
`target/` directory will then live on the much faster Linux filesystem.

### 5. Run the linter

```bash
# From the root of your Soroban contract workspace (still inside WSL2)
cd ~/projects/<my-soroban-contract>
cargo cost-lint
```

Then proceed to [`docs/integration.md`](integration.md) to wire the linter into
your editor and CI. WSL2 + Ubuntu lifts the rest of the platform-specific
constraints; nothing else in this repo needs changing.

---

## Option B — Native Windows + PowerShell

> **Heads-up:** If `cargo cost-lint` ever errors with dynamic-loading messages on
> native Windows, fall back to **Option A** (WSL2). It is faster to set up than to
> diagnose a native-Windows dynamic-linker mismatch.

### 1. Prerequisites

#### 1.1. Rust toolchain (MSVC)

Install via [`rustup`](https://rustup.rs):

1. Download [`rustup-init.exe`](https://win.rustup.rs/x86_64).
2. Run it and accept the default **MSVC** toolchain (`stable-x86_64-pc-windows-msvc`).
   Pick the GNU toolchain only if you have a specific reason — the Soroban ecosystem
   defaults to MSVC on Windows.
3. Confirm rustup added `%USERPROFILE%\.cargo\bin` to your user `Path` environment
   variable (it normally does this automatically).

Verify in a **new** PowerShell window:

```powershell
rustc --version
cargo --version
```

#### 1.2. Visual Studio Build Tools (for the MSVC linker)

`rustc` on the MSVC toolchain needs `link.exe`. Install **Visual Studio Build Tools
2022** with the **"Desktop development with C++"** workload:

- Download: <https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022>
- In the Visual Studio Installer, tick **"Desktop development with C++"**.
- This installs `link.exe`, the Windows 10/11 SDK, and the MSVC C runtime.

Verify (from PowerShell):

```powershell
where.exe link.exe
# Expected: a path under "C:\Program Files\Microsoft Visual Studio\..."
```

> If `link.exe` is not found but Visual Studio Build Tools is installed, you can
> load the Visual Studio environment into your current PowerShell session instead
> of opening the "x64 Native Tools Command Prompt for VS 2022":
>
> ```powershell
> Import-Module "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\Common7\Tools\Microsoft.VisualStudio.DevShell.ps1"
> Enter-VsDevShell -VsInstallPath "C:\Program Files\Microsoft Visual Studio\2022\BuildTools" -SkipAutomaticLocation
> ```
>
> (Replace `BuildTools` with `Community` / `Professional` / `Enterprise` if you
> installed a full Visual Studio SKU instead of Build Tools.)

#### 1.3. Git for Windows

Install [Git for Windows](https://git-scm.com/download/win). Accept the option to
add `git` to your `Path`.

Then enable long-path support (many Rust crates generate deeply-nested paths):

```powershell
git config --global core.longpaths true
```

#### 1.4. (Optional) Disable CRLF autotranslation for Rust checkouts

Git for Windows defaults to converting line endings to CRLF on checkout. Rust
source files should stay LF. Configure per-checkout:

```powershell
cd path\to\soroban-cost-linter
git config core.autocrlf false
```

Or globally, before cloning:

```powershell
git config --global core.autocrlf input
```

### 2. Install Dylint and the linter

Open a **PowerShell** prompt (no admin needed):

```powershell
# Install Dylint (matching pin in CONTRIBUTING.md and README.md)
cargo install cargo-dylint dylint-link --version "^6.0.1"

# Install the linter wrapper
cargo install --git https://github.com/Tollcraft/soroban-cost-linter.git cargo-cost-lint
```

### 3. PATH setup

`cargo install` writes binaries to `%USERPROFILE%\.cargo\bin` (typically
`C:\Users\<you>\.cargo\bin`). Confirm it is on your `Path` for the current shell:

```powershell
where.exe cargo-dylint
where.exe cargo-cost-lint
```

If either command reports "INFO: Could not find files", add the directory to your
user `Path` permanently:

1. Press <kbd>Win</kbd>, type **"environment variables"**, and open **"Edit the
   system environment variables"**.
2. Click **Environment Variables…**.
3. Under **User variables**, select **Path** → **Edit…** → **New**.
4. Paste `%USERPROFILE%\.cargo\bin`.
5. Click OK on all three dialogs and **open a new PowerShell window** so the
   change takes effect.

For a single shell session, you can also add it temporarily:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

### 4. Clone and build the linter

```powershell
git clone https://github.com/Tollcraft/soroban-cost-linter.git
cd soroban-cost-linter
cargo build
```

### 5. Run the linter on a contract

```powershell
cd path\to\my-soroban-contract
cargo cost-lint
```

PowerShell preserves the standard cargo subcommand form (`cargo-<name>`), so
`cargo cost-lint` works without any further configuration.

When you invoke `cargo cost-lint --fix`, the tool writes updated source files
in-place — make sure your editor saves them with **LF** line endings, not CRLF,
or `git diff` will show spurious whitespace churn.

---

## Common Windows issues

| Symptom | Likely cause | Fix |
| --- | --- | --- |
| `error: linker 'link.exe' not found` | Visual Studio Build Tools missing or installed without the C++ workload. | Install Build Tools with the **Desktop development with C++** workload, restart PowerShell, and confirm `where.exe link.exe`. |
| `link.exe` not found even though Build Tools is installed | The Visual Studio environment variables are not active in your shell. | Open the **x64 Native Tools Command Prompt for VS 2022** from the Start menu and run `cargo` from there, or load VsDevCmd into PowerShell via `Enter-VsDevShell` (see Section 1.2). |
| `cargo install` prints `failed to fetch` / network errors | Corporate proxy or antivirus blocking outbound HTTPS. | Set `HTTPS_PROXY` per your policy; whitelist `crates.io` and `github.com` in your antimalware product. |
| `cargo-dylint` errors with `could not find libstd` | Host toolchain doesn't match the pinned nightly. | Keep your host `rustup default` and the linter's pinned nightly in sync — see [Toolchain upgrade guide](../CONTRIBUTING.md#5-upgrading-the-nightly-toolchain). |
| Crate dependency fails on `openssl-sys` or similar C-bindgen crates | Missing C library headers. | Prefer crates that already target `rustls` (no native OpenSSL dependency), or install OpenSSL headers via `vcpkg` (`vcpkg install openssl:x64-windows-static-md`). |
| Antivirus quarantines `target/` or `%USERPROFILE%\.cargo` | Defender heuristic on scripts / dynamic libraries. | Add Windows Defender exclusions for `%USERPROFILE%\.cargo` and your project's `target\` directory. |
| `crate `X`` doesn't build on Windows | Upstream crate without upstream Windows support. | This is an upstream issue — please open an issue against the crate's repo, then fall back to **Option A (WSL2)** on your machine. |
| `cargo cost-lint --fix` writes files with CRLF endings | Your editor or PowerShell redirect added CRLF. | Configure your editor to preserve LF. To normalise an existing checkout **without losing uncommitted work**, install `dos2unix` (e.g. via `choco install dos2unix` or scoop) and run it on the affected files. For a repo-wide guarantee, add a `.gitattributes` with `* text=auto eol=lf`, then re-clone once. |
| Slow compile times on spinning-disk machines | Many small files + pagefile thrash. | Move the project's `target\` directory to a faster drive; add a Windows Defender exclusion for the new location (see "Antivirus quarantines" row above). |

## When in doubt

1. **Try WSL2 first** — it removes ~90% of Windows-only friction and matches CI.
2. If you must stay on native Windows, install the **MSVC** toolchain plus
   **Visual Studio Build Tools**, use **PowerShell** (not `cmd.exe`), and keep
   `rustup default` aligned with the project's pinned nightly
   (`nightly-2026-04-16` at the time of writing).
3. File any Windows-specific issues at
   <https://github.com/Tollcraft/soroban-cost-linter/issues> with the
   `windows` label so this guide can be kept up to date.

## See also

- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — Linux/macOS contributor setup and code quality standards.
- [`docs/integration.md`](integration.md) — Editor and CI integration (largely OS-agnostic).
- [`docs/false_positives.md`](false_positives.md) — How to suppress, configure, or report false positives.
