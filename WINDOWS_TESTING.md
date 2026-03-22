# Windows Testing Guide for MacWinShare

## Prerequisites

### On Windows PC:

1. **Install Rust** (if not already installed):
   ```powershell
   # Download and run rustup-init.exe from https://rustup.rs
   # Or use winget:
   winget install Rustlang.Rustup
   ```

2. **Install Node.js** (for frontend build):
   ```powershell
   winget install OpenJS.NodeJS.LTS
   ```

3. **Install Visual Studio Build Tools** (required for Rust on Windows):
   ```powershell
   winget install Microsoft.VisualStudio.2022.BuildTools
   # Select "Desktop development with C++" workload during installation
   ```

## Transfer Project to Windows

### Option A: Git Clone (if you have a repo)
```powershell
git clone <your-repo-url>
cd macwinshare
```

### Option B: Copy via Network Share
1. On Mac, share the project folder or use AirDrop
2. Copy `macwinshare` folder to Windows

### Option C: USB Drive
1. Copy the entire `macwinshare` folder to USB
2. Transfer to Windows PC

## Build on Windows

```powershell
cd macwinshare

# Install npm dependencies
npm install

# Build frontend
npm run build

# Run tests (core library)
cargo test -p macwinshare-core

# Build release binary
cargo build --release

# The binary will be at: target\release\macwinshare.exe
```

## Testing Checklist

### 1. Unit Tests
```powershell
cargo test -p macwinshare-core
# Expected: 38+ tests pass
```

### 2. Windows Platform Tests
```powershell
cargo test -p macwinshare-platform-windows
```

### 3. Build Verification
```powershell
cargo build --release
# Check binary exists and runs:
.\target\release\macwinshare.exe --help
```

### 4. Tauri Dev Mode
```powershell
# Install Tauri CLI
cargo install tauri-cli

# Run in dev mode
cargo tauri dev
```

### 5. Cross-Machine Connection Test

**On Mac (Server):**
```bash
cd macwinshare
cargo run --release -- --server
# Note the IP address shown
```

**On Windows (Client):**
```powershell
cd macwinshare
cargo run --release -- --client <mac-ip-address>
```

## Expected Results

| Test | Expected Outcome |
|------|------------------|
| `cargo test -p macwinshare-core` | 38+ tests pass |
| `cargo build --release` | Successful, ~8MB binary |
| `cargo tauri dev` | Window opens with UI |
| Cross-machine connection | Handshake succeeds |

## Troubleshooting

### Missing MSVC Tools
```
error: linker `link.exe` not found
```
**Solution:** Install Visual Studio Build Tools with C++ workload

### Certificate Errors
```
error: could not find certificate
```
**Solution:** Delete `%LOCALAPPDATA%\MacWinShare\ssl\` and restart

### Firewall Blocking
- Allow `macwinshare.exe` through Windows Firewall
- Port 24800 (TCP) must be open for connections
- Port 24801 (UDP) for discovery

### Input Permissions
Windows requires no special permissions for input capture/injection (unlike macOS Accessibility).

## Reporting Results

Please report back:
1. Test output from `cargo test -p macwinshare-core`
2. Any build errors
3. Screenshot of Tauri window (if successful)
4. Connection test results between Mac and Windows
