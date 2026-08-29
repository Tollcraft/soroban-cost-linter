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
2. Run it and install the repository-pinned nightly toolchain (`nightly-2026-04-16`) with required components:
   ```powershell
   rustup toolchain install nightly-2026-04-16 --component rustc-dev llvm-tools-preview rustfmt clippy
   rustup default nightly-2026-04-16
   ```
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

#### 1.4. Disable CRLF autotranslation for Rust checkouts

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

---

## Common Windows issues

- **`LINK : fatal error LNK1104: cannot open file 'kernel32.lib'`**: The MSVC linker
  cannot find the Windows SDK libraries. Ensure that the "Desktop development with C++"
  workload is fully installed via the Visual Studio Installer and that you are using
  the MSVC toolchain (`-msvc` target).
- **`error: unloaded library` or dynamic loading errors on native Windows**: This
  happens if `dylint-link` or the toolchain components are mismatched or if mixed
  GNU/MSVC toolchains are used. Ensure both rustup and Visual Studio Build Tools
  are consistently targeting `x86_64-pc-windows-msvc`. If issues persist, switch to
  **Option A (WSL2)**.
- **Line endings (`CRLF` vs `LF`)**: Ensure `core.autocrlf` is set to `false` or `input`
  before cloning the repository to prevent git from converting LF line endings in Rust
  source files to CRLF, which can cause compiler or macro parsing discrepancies.
