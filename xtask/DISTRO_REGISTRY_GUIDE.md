# 📋 Distro Registry - Extensible Distribution Configuration

The distro registry (`distro-registry.json`) is the **single source of truth** for all supported Linux distributions in AetherCore xtask. Users and maintainers can easily add new distributions, versions, and variants without touching Rust code.

---

## Overview

Instead of hardcoding distro URLs in the Rust code, we maintain a structured JSON registry that defines:

- **Distributions** - Base Linux distros (Ubuntu, Debian, Fedora, etc.)
- **Versions** - Specific releases (24.04, 22.04, etc.)
- **Variants** - Build types (minimal, server, cloud, etc.)
- **Architectures** - CPU targets (x86_64, arm64, etc.)
- **URLs** - Download mirrors with fallbacks

### Benefits

✅ **Easy to update** - No code recompilation needed  
✅ **User-extensible** - Add your own distros in minutes  
✅ **Maintainable** - Clear structure, self-documenting  
✅ **Flexible** - Support multiple mirrors and fallbacks  
✅ **Versioned** - Track metadata like release dates, support dates  

---

## Structure

```json
{
  "version": "1.0",
  "distros": {
    "ubuntu": {
      "name": "Ubuntu Linux",
      "description": "Popular Debian-based distribution...",
      "website": "https://ubuntu.com",
      "versions": {
        "24.04": {
          "codename": "Noble Numbat",
          "type": "lts",
          "released": "2024-04",
          "support_until": "2034-04",
          "variants": {
            "minimal": {
              "x86_64": [
                "https://cloud-images.ubuntu.com/minimal/releases/24.04/release/ubuntu-24.04-minimal-cloudimg-amd64-root.tar.gz"
              ]
            }
          }
        }
      },
      "aliases": ["ubuntu-lts", "ubuntu"]
    }
  }
}
```

### Key Fields

| Field | Type | Purpose |
|-------|------|---------|
| `name` | string | Human-readable name (e.g., "Ubuntu Linux") |
| `description` | string | Brief overview |
| `website` | string | Official project URL |
| `versions` | object | Map of version → VersionEntry |
| `aliases` | array | Short names users can type (e.g., "ubuntu", "ubuntu-lts") |
| `codename` | string | Release codename (optional, e.g., "Noble Numbat") |
| `type` | string | Release type: "lts", "stable", "rolling", "testing" |
| `variants` | object | Map of variant → arch → URLs |

---

## Adding a New Distribution

### Step 1: Choose a Key

Pick a unique ID for your distro (lowercase, no spaces):

```json
"my-distro": { ... }
```

### Step 2: Add Metadata

```json
"my-distro": {
  "name": "My Custom Distro",
  "description": "A custom lightweight Linux distribution",
  "website": "https://my-distro.example.com",
  "versions": { ... },
  "aliases": ["my-distro", "mycustom"]
}
```

### Step 3: Add Versions

Each distro can have multiple versions:

```json
"versions": {
  "1.0": {
    "codename": "FirstRelease",
    "type": "stable",
    "released": "2024-01",
    "support_until": "2026-01",
    "variants": { ... }
  }
}
```

### Step 4: Add Variants and URLs

Variants represent different builds (minimal, server, desktop, etc.):

```json
"variants": {
  "minimal": {
    "x86_64": [
      "https://download.my-distro.org/1.0/minimal-x86_64.tar.gz",
      "https://mirror.backup.org/my-distro/1.0/minimal-x86_64.tar.gz"
    ],
    "arm64": [
      "https://download.my-distro.org/1.0/minimal-arm64.tar.gz"
    ]
  },
  "server": {
    "x86_64": [
      "https://download.my-distro.org/1.0/server-x86_64.tar.gz"
    ]
  }
}
```

### Step 5: Test

```bash
# Run xtask with your new distro
cargo run -p xtask -- run guest --distro my-distro --download

# Or use an alias
cargo run -p xtask -- run guest --distro mycustom --download
```

---

## Complete Example: Adding Alpine 3.20

```json
{
  "alpine": {
    "name": "Alpine Linux",
    "description": "Lightweight, security-oriented Linux distribution",
    "website": "https://alpinelinux.org",
    "versions": {
      "3.20": {
        "released": "2024-05",
        "support_until": "2026-05",
        "variants": {
          "virt": {
            "x86_64": [
              "https://dl-cdn.alpinelinux.org/alpine/v3.20/cloud/alpine-virt-3.20.0-x86_64.iso",
              "https://mirror.backup.org/alpine/v3.20/cloud/alpine-virt-3.20.0-x86_64.iso"
            ],
            "arm64": [
              "https://dl-cdn.alpinelinux.org/alpine/v3.20/cloud/alpine-virt-3.20.0-aarch64.iso"
            ]
          }
        }
      }
    },
    "aliases": ["alpine"]
  }
}
```

Then use:

```bash
cargo run -p xtask -- run guest --distro alpine-3.20 --download
```

---

## Best Practices

### 1. Provide Multiple Mirrors

Always provide fallback mirrors:

```json
"x86_64": [
  "https://official.mirror.org/distro.tar.gz",
  "https://backup1.mirror.org/distro.tar.gz",
  "https://backup2.mirror.org/distro.tar.gz"
]
```

### 2. Use HTTPS

Always use HTTPS URLs for security:

```json
"x86_64": ["https://example.com/distro.tar.gz"]  // ✅ Good
```

### 3. Test URLs

Verify URLs work before committing:

```bash
curl -I https://your-mirror.org/distro.tar.gz
# Should return 200 OK
```

### 4. Set Metadata

Include support dates if available:

```json
"released": "2024-04",
"support_until": "2026-04"
```

### 5. Use Clear Variant Names

- `minimal` - Barebones, no GUI
- `server` - Server-oriented, CLI tools
- `desktop` - Includes GUI, development tools
- `cloud` - Cloud image format (QCOW2, etc.)
- `virt` - Virtualization-ready

### 6. Document Your Additions

Add a comment or update this file when adding distros:

```json
{
  "metadata": {
    "last_updated": "2024-05-15",
    "notes": "...",
    "contributions": [
      "ubuntu/debian - official sources",
      "fedora - official project",
      "custom-distro - user contribution"
    ]
  }
}
```

---

## File Format Reference

### Full Schema Example

```json
{
  "version": "1.0",
  "description": "AetherCore Distro Registry",
  "distros": {
    "distro-key": {
      "name": "Full Name",
      "description": "Short description",
      "website": "https://url",
      "versions": {
        "1.0": {
          "codename": "Release Codename",
          "type": "lts|stable|rolling|testing",
          "released": "YYYY-MM",
          "support_until": "YYYY-MM",
          "variants": {
            "variant-name": {
              "x86_64": ["https://url1", "https://url2"],
              "arm64": ["https://url"],
              "riscv64": ["https://url"]
            }
          }
        }
      },
      "aliases": ["short", "alternative"]
    }
  },
  "metadata": {
    "last_updated": "YYYY-MM-DD",
    "maintained_by": "Your Name",
    "notes": "Any notes"
  }
}
```

---

## Searching & Resolution

When a user runs:

```bash
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download
```

xtask resolves the distro using this **priority order**:

1. **Exact key match** - Look for `"ubuntu-24.04"` in distros
2. **Alias match** - Check if `ubuntu-24.04` is in any distro's aliases
3. **Version pattern** - Split by `-` and find distro + version combo
4. **Direct URL** - If starts with `https://`, use as-is
5. **Fallback** - Return empty (graceful degradation)

---

## Programmatic Access

The registry is loaded and parsed by Rust code in `xtask/src/commands/ops/guest.rs`:

```rust
pub fn resolve_distro_urls(distro: &str) -> Vec<String>
```

This function:
- Loads `distro-registry.json`
- Parses JSON structure
- Resolves distro identifier to URLs
- Returns list of download mirrors (with fallbacks)

Users don't need to touch this code; just update the JSON!

---

## Maintenance

### Updating Official Distros

When Ubuntu releases a new version:

```bash
# Edit distro-registry.json
# Add new version under "ubuntu" → "versions"
# Test with:
cargo run -p xtask -- run guest --distro ubuntu-25.04 --download
```

### Checking Coverage

See which distros are currently supported:

```bash
# Print registry
cargo run -p xtask -- run guest --distro invalid 2>&1 | grep "Failed"
# Or check distro-registry.json directly
```

### Validating JSON

```bash
# Using Python
python -m json.tool distro-registry.json > /dev/null && echo "Valid"

# Using online tools
# https://jsonlint.com/
```

---

## FAQ

**Q: Can I add a distro without modifying source code?**  
A: Yes! Just edit `distro-registry.json` and run xtask.

**Q: What if a mirror goes down?**  
A: xtask tries each URL in the list until one succeeds. Provide multiple mirrors.

**Q: Can I override the registry at runtime?**  
A: Currently no, but could add `--registry-path` flag if needed.

**Q: How do I contribute new distros?**  
A: Edit `distro-registry.json` and submit a PR to the AetherCore repo.

**Q: Can users provide their own registry?**  
A: Possible with future enhancement: `--distro-registry /path/to/custom.json`

---

## File Location

```
c:\Users\oyunm\Desktop\OS\
├── xtask/
│   ├── distro-registry.json    ← You are here
│   ├── src/
│   │   └── commands/ops/guest.rs
│   └── Cargo.toml
```

---

## Quick Reference: Distros Currently Supported

Run `cargo test -p xtask -- commands::ops::guest` to see all registered distros.

Or check [the registry file](distro-registry.json) directly.

---

**Happy distro adding!** 🐧
