# 🚀 AetherCore Guest Support - Setup & Maintenance

Quick reference for setting up and maintaining the guest OS support system.

---

## Initial Setup (First Time)

### 1. Ensure Registry File Exists

```bash
# Check that distro-registry.json exists in xtask directory
ls -la xtask/distro-registry.json

# Should show:
# -rw-r--r-- ... distro-registry.json
```

### 2. Verify Registry Loads

```bash
# Build and test registry loading
cargo check -p xtask

# Run guest registry tests
cargo test -p xtask -- commands::ops::guest

# Should see:
# test_registry_loads ... ok
# test_distro_resolution_basic ... ok
# etc.
```

### 3. Test a Download

```bash
# Try downloading Ubuntu (requires internet)
cargo run -p xtask -- run guest --distro ubuntu-24.04 --download

# Should download, cache, and launch QEMU
```

---

## Adding a New Distribution

### Quick Start (2 minutes)

**1. Open `xtask/distro-registry.json`**

**2. Find the right section** (alphabetically organized by distro name)

**3. Add your distro:**

```json
"my-custom-distro": {
  "name": "My Custom Linux",
  "description": "A lightweight custom distro",
  "website": "https://example.com",
  "versions": {
    "1.0": {
      "released": "2024-05",
      "variants": {
        "minimal": {
          "x86_64": [
            "https://example.com/distro-1.0-x86_64.tar.gz"
          ]
        }
      }
    }
  },
  "aliases": ["custom", "my-distro"]
}
```

**4. Save and test:**

```bash
cargo run -p xtask -- run guest --distro my-custom-distro --download
# Or using alias
cargo run -p xtask -- run guest --distro custom --download
```

---

## Distro Registry File Structure

```
xtask/
├── distro-registry.json          ← Main registry (JSON format)
└── src/commands/ops/guest.rs     ← Loader/parser (Rust)
```

**Key point:** Edit JSON, not Rust code!

---

## Registry Format Cheat Sheet

### Minimal Entry

```json
"distro-name": {
  "name": "Display Name",
  "description": "Brief description",
  "versions": {
    "1.0": {
      "variants": {
        "minimal": {
          "x86_64": ["https://url.tar.gz"]
        }
      }
    }
  },
  "aliases": ["short-name"]
}
```

### Full Entry (with metadata)

```json
"distro-name": {
  "name": "Distribution Name",
  "description": "Longer description",
  "website": "https://official-site.com",
  "versions": {
    "2.0": {
      "codename": "Release Name",
      "type": "lts",
      "released": "2024-04",
      "support_until": "2026-04",
      "variants": {
        "minimal": {
          "x86_64": [
            "https://primary-mirror.com/distro-2.0-x86_64.tar.gz",
            "https://backup-mirror.com/distro-2.0-x86_64.tar.gz"
          ],
          "arm64": ["https://mirror.com/distro-2.0-arm64.tar.gz"]
        },
        "server": {
          "x86_64": ["https://mirror.com/distro-2.0-server.tar.gz"]
        }
      }
    }
  },
  "aliases": ["short", "alternative-name"]
}
```

---

## Common Tasks

### Task: Add Ubuntu 25.04 (New Release)

```json
// In distro-registry.json, under "ubuntu" → "versions", add:
"25.04": {
  "codename": "Noble Numbat Plus",
  "type": "regular",
  "released": "2025-04",
  "support_until": "2026-10",
  "variants": {
    "minimal": {
      "x86_64": [
        "https://cloud-images.ubuntu.com/minimal/releases/25.04/release/ubuntu-25.04-minimal-cloudimg-amd64-root.tar.gz"
      ]
    }
  }
}
```

Then test:
```bash
cargo run -p xtask -- run guest --distro ubuntu-25.04 --download
```

### Task: Add Fedora Server Variant

```json
// In distro-registry.json, under "fedora" → "versions" → "40" → "variants", add:
"server": {
  "x86_64": [
    "https://download.fedoraproject.org/pub/fedora/linux/releases/40/Server/x86_64/images/Fedora-Server-40-1.14-x86_64-dvd.iso"
  ]
}
```

Then use:
```bash
cargo run -p xtask -- run guest --distro fedora-40-server --download
```

### Task: Add Mirror for Slow Downloads

```json
// Find existing distro entry, modify variant URLs to add backup:
"x86_64": [
  "https://primary.mirror.org/ubuntu-24.04-x86_64.tar.gz",
  "https://backup1.mirror.org/ubuntu-24.04-x86_64.tar.gz",
  "https://backup2.mirror.org/ubuntu-24.04-x86_64.tar.gz"
]
```

xtask will try each URL in order until one succeeds.

### Task: Deprecate an Old Version

Simply remove or comment out the version:

```json
// Before
"20.04": { ... },

// After (commented out)
// "20.04": { ... },  // Deprecated, no longer maintained
```

### Task: Change Default/Alias

```json
// Modify aliases to change which version users get by default
"aliases": ["ubuntu-lts", "ubuntu"]  // "ubuntu" now points to latest

// Before you update: update ubuntu-lts alias
// After: users typing "ubuntu" get the new default
```

---

## Validation & Testing

### Validate JSON Syntax

```bash
# Using Python (if available)
python -m json.tool xtask/distro-registry.json > /dev/null && echo "✓ Valid JSON"

# Using jq (if installed)
jq . xtask/distro-registry.json > /dev/null && echo "✓ Valid JSON"

# Using Node.js
node -e "require('./xtask/distro-registry.json')" && echo "✓ Valid JSON"
```

### Test Registry Loads

```bash
cargo test -p xtask -- commands::ops::guest::tests::test_registry_loads
```

### Test Individual Distro

```bash
cargo run -p xtask -- run guest --distro ubuntu-24.04

# If successful, you'll see:
# "📥 Resolving distro URLs for: ubuntu-24.04"
# "Trying URL [1/1]: https://..."
```

### Run All Guest Tests

```bash
cargo test -p xtask -- commands::ops::guest

# Should pass all tests:
# test_registry_loads ... ok
# test_distro_resolution_basic ... ok
# test_distro_resolution_by_alias ... ok
# test_direct_url ... ok
# test_unknown_distro_returns_empty ... ok
```

---

## Troubleshooting

### Problem: "Distro registry not found"

**Cause:** `distro-registry.json` is missing or in wrong location

**Fix:**
```bash
# Check file exists
ls xtask/distro-registry.json

# Check working directory
pwd
# Should be: /path/to/OS (repo root)

# Run from repo root
cd ~/Desktop/OS
cargo run -p xtask -- run guest --distro ubuntu-24.04
```

### Problem: "Unknown distro 'xyz'"

**Cause:** Distro not in registry

**Fix:**
```bash
# List available distros (check distro-registry.json)
cat xtask/distro-registry.json | grep '".*":'

# Or add your own distro to registry
```

### Problem: JSON Syntax Error

**Cause:** Malformed JSON when editing registry

**Fix:**
```bash
# Validate JSON
python -m json.tool xtask/distro-registry.json

# Look for errors like:
# - Missing commas between entries
# - Unclosed braces/brackets
# - Trailing commas in arrays

# Use online validator: https://jsonlint.com/
```

### Problem: URLs Work but Download Fails

**Cause:** Mirror is down or slow

**Fix:**
```bash
# Add backup mirrors to the variant:
"x86_64": [
  "https://fast-mirror.com/distro.tar.gz",
  "https://slow-mirror.com/distro.tar.gz",
  "https://backup-mirror.com/distro.tar.gz"
]

# Then xtask will try each until one works
```

---

## Best Practices

### ✅ DO

- ✅ Use HTTPS URLs only
- ✅ Provide multiple mirrors per variant
- ✅ Test URLs before committing
- ✅ Keep version strings consistent
- ✅ Document release dates when available
- ✅ Use clear variant names (minimal, server, cloud, desktop)
- ✅ Validate JSON before committing
- ✅ Add comments for custom entries
- ✅ Test with: `cargo run -p xtask -- run guest --distro X --download`

### ❌ DON'T

- ❌ Use HTTP (insecure)
- ❌ Hardcode URLs in Rust code (edit JSON instead)
- ❌ Add invalid JSON (will break loading)
- ❌ Leave broken mirror URLs
- ❌ Forget to test after editing
- ❌ Add entries without documentation
- ❌ Commit without validation
- ❌ Use non-existent mirror URLs

---

## Workflow: Contributing New Distro

### 1. Research

```bash
# Find official cloud images or minimal rootfs
# Example for custom distro:
# https://example.com/releases/distro-1.0-x86_64.tar.gz
```

### 2. Edit Registry

```bash
# Open xtask/distro-registry.json
# Add new distro entry in alphabetical order
# Provide multiple mirrors if possible
```

### 3. Validate

```bash
# Check JSON syntax
python -m json.tool xtask/distro-registry.json > /dev/null

# Check registry loads
cargo test -p xtask -- commands::ops::guest::tests::test_registry_loads
```

### 4. Test

```bash
# Try actual download (if internet available)
cargo run -p xtask -- run guest --distro your-distro --download

# Or test without download:
cargo run -p xtask -- run guest --distro your-distro
```

### 5. Commit

```bash
git add xtask/distro-registry.json
git commit -m "Add support for <distro> distribution"
git push
```

---

## Registry Statistics

To see how many distros are supported:

```bash
# Count distros (rough estimate)
grep -o '"name":' xtask/distro-registry.json | wc -l

# List all distro keys
grep -o '"[a-z-]*": {' xtask/distro-registry.json | head -20
```

---

## Updates & Maintenance Schedule

| Task | Frequency | Owner |
|------|-----------|-------|
| Check for new Ubuntu LTS | Yearly | Maintainer |
| Update security mirrors | Quarterly | Maintainer |
| Test all distro URLs | Monthly | CI/CD |
| Deprecate EOL versions | Yearly | Maintainer |
| Community contributions | As needed | Reviewer |

---

## Questions?

- **How to add custom distro?** → See "Adding a New Distribution" section above
- **How to add mirror?** → Edit variant's URL array, xtask tries each
- **How to test changes?** → `cargo run -p xtask -- run guest --distro X --download`
- **How to validate?** → `python -m json.tool xtask/distro-registry.json`
- **Need help?** → Check [DISTRO_REGISTRY_GUIDE.md](DISTRO_REGISTRY_GUIDE.md) for full documentation

Happy distributing! 🐧
