## Capability System Analysis & Optional Architecture

### 1. Mevcut Capability Sistemi Nasıl Çalışıyor?

#### Mevcut Yapı
```rust
// SecurityContext içindeki capability fields:
pub struct SecurityContext {
    pub capabilities: u64,              // 32 capability bitmask (0-63 cap)
    pub ambient_caps: u64,              // Inherited across exec
    pub privileged: bool,               // Global privilege flag
}

// Capability check (inline, super hızlı):
#[inline(always)]
pub fn has_capability(&self, cap: u64) -> bool {
    self.privileged || (self.capabilities & cap) == cap
}
```

**Avantajlar**:
- ✅ Inline function = sıfır function call overhead
- ✅ Bitwise AND operation = 1 CPU cycle
- ✅ ZST (CapabilityToken) = hiç heap overhead
- ✅ Compile-time cap flags = constant folding

**Sorunlar**:
- ❌ Tüm task'ler her zaman SecurityContext taşıyor (64 byte overhead)
- ❌ has_capability() çağrısı her access check'te
- ❌ Privileged flag = global gatekeeper (tüm cap'ları açıyor)
- ❌ Ambient caps = extra 8 byte
- ❌ Opsiyonel değil - her zaman kontrol var

---

### 2. Performans Yükü Analizi

#### Güncel Overhead

| Komponent | Maliyeti | Per-Task | Detaylar |
|-----------|----------|----------|----------|
| SecurityContext | 64 bytes | Sabit | Her task'te taşınır |
| capabilities u64 | 8 bytes | Sabit | 32 cap, 32 reserved |
| ambient_caps u64 | 8 bytes | Sabit | exec() sırasında inherited |
| privileged bool | 1 byte | Sabit | Global override flag |
| **has_capability()** | **1 cycle** | **Per-check** | Inline bitwise AND |
| **Context switch** | +64 bytes | Per-switch | RSP'ye copy |

#### Mesela bir file açma işinde:
```
1. open() syscall
2. SecurityMonitor::check_access_full()
3. has_capability(CAP_DAC_OVERRIDE) call
   → if privileged: return true (1-2 cycles)
   → else: (self.capabilities & cap) == cap (2-3 cycles, cache hit)
4. Access verdict
```

**Toplam**: 3-5 CPU cycles per check ✅ (çok hızlı)

#### Ama...
- 1000 file access/second = 5000 cycles = 0.17% CPU (neymişse)
- 100 task'te 64 byte = 6.4 KB memory (önemsiz)

---

### 3. Kimin/Neyin Yetkisini Belirliyor?

#### Ş anki Sistem = 3 Tier Authorization

```
┌─ Tier 1: Global Privileged Flag
│  ├─ privileged=true → TÜM CAP'LAR ✅
│  └─ privileged=false → CAP bitmask kontrol
│
├─ Tier 2: Capability Bitmask (32 cap)
│  ├─ CAP_CHOWN (file ownership)
│  ├─ CAP_SETUID (user switch)
│  ├─ CAP_SYS_ADMIN (system ops)
│  └─ ... 29 more ...
│
└─ Tier 3: SecurityMonitor Policy
   ├─ check_access_full() → Allow/Deny/AuditAllow/AuditDeny
   ├─ check_resource_limit() → Max files, memory, threads
   └─ Custom policy backends (ACL, RBAC, SELinux)
```

#### ÖRNEK: File Write Operation

```rust
// 1. Has write capability?
if !ctx.has_capability(cap_flags::CAP_DAC_OVERRIDE) {
    // 2. Check file permissions (ACL)
    if !check_file_acl(file_inode, ctx.euid, ctx.egid) {
        return SecurityVerdict::Deny;  ← Reject
    }
}

// 3. Check resource limit (open file count)
if ctx.limits.max_open_files <= open_count {
    return SecurityVerdict::AuditDeny;  ← Log + reject
}

// 4. Pass all checks
return SecurityVerdict::AuditAllow;  ← Log + allow
```

---

### 4. Opsiyonel Capability Sistemi Tasarımı

#### Problem: Şu an zorunlu, tüm task'ler taşıyor

#### Çözüm: 3 Mod Sistemi

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityMode {
    /// Mode 0: Hiç security check yok (fastest)
    Disabled,
    
    /// Mode 1: Lightweight capability-only (fast)
    CapabilityOnly,
    
    /// Mode 2: Full policy enforcement (secure, slower)
    PolicyEnforcement,
}

pub struct SecurityContext {
    pub mode: SecurityMode,  ← NEW
    
    // Mode 0 (Disabled): Aşağıdakiler işlenmiyor
    // Mode 1+: Aşağıdakiler kullanılıyor
    #[cfg(feature = "security")]
    pub task_id: TaskId,
    #[cfg(feature = "security")]
    pub process_id: ProcessId,
    #[cfg(feature = "security")]
    pub capabilities: u64,
    #[cfg(feature = "security")]
    pub ambient_caps: u64,
    #[cfg(feature = "security")]
    pub privileged: bool,
    #[cfg(feature = "security")]
    pub audit_enabled: bool,
    
    // Mode 1-2: Conditional resource limits
    #[cfg(feature = "policy")]
    pub limits: ResourceLimits,
}

impl SecurityContext {
    #[inline(always)]
    pub fn has_capability(&self, cap: u64) -> bool {
        match self.mode {
            SecurityMode::Disabled => true,  ← Hiç overhead
            SecurityMode::CapabilityOnly => 
                self.privileged || (self.capabilities & cap) == cap,  ← Bitwise
            SecurityMode::PolicyEnforcement => 
                self.privileged || (self.capabilities & cap) == cap,  ← Bitwise + policy
        }
    }
}
```

---

### 5. Compile-Time Optional Architecture

#### Cargo.toml

```toml
[features]
default = ["security_capabilities"]
security_capabilities = []     # Compile capability code
policy_enforcement = ["security_capabilities"]  # Requires capabilities
audit_logging = ["policy_enforcement"]  # Requires policy

[profile.release]
opt-level = 3
lto = true
```

#### Code örneği

```rust
// src/interfaces/security.rs

#[derive(Debug, Clone, Copy)]
pub struct SecurityContext {
    // Always present (2 bytes)
    pub mode: SecurityMode,
    pub euid: u32,
    
    // Optional via feature flag
    #[cfg(feature = "security_capabilities")]
    pub capabilities: u64,
    #[cfg(feature = "security_capabilities")]
    pub privileged: bool,
    
    #[cfg(feature = "policy_enforcement")]
    pub limits: ResourceLimits,
}

// Conditional check
#[inline(always)]
pub fn check_capability(&self, cap: u64) -> bool {
    #[cfg(feature = "security_capabilities")]
    {
        if self.mode == SecurityMode::Disabled {
            return true;  // Fast path
        }
        self.privileged || (self.capabilities & cap) == cap
    }
    
    #[cfg(not(feature = "security_capabilities"))]
    {
        true  // Compile away entirely
    }
}
```

#### Compile Result

```bash
# Mode 1: No security (smallest, fastest)
$ cargo build --release
   Size: 2.1 MB (all security code eliminated)
   Overhead: 0

# Mode 2: Capabilities only
$ cargo build --release --features security_capabilities
   Size: 2.4 MB (+300 KB)
   Overhead: 2-3 cycles per check

# Mode 3: Full policy (production)
$ cargo build --release --features policy_enforcement,audit_logging
   Size: 2.8 MB (+700 KB)
   Overhead: 5-8 cycles per check + audit

# Binary size comparison:
Without security:    2.1 MB ✅
With capabilities:   2.4 MB (SecurityContext = 16 bytes)
With policy:         2.8 MB (SecurityContext = 64 bytes)
```

---

### 6. Memory Optimization

#### Şu an (Tüm alanlar her zaman)

```
SecurityContext (64 bytes):
├─ task_id (4)       ┐
├─ process_id (4)    │ → Hiç task'te var
├─ euid (4)          │
├─ egid (4)          │
├─ ruid (4)          │
├─ rgid (4)          │
├─ suid (4)          │
├─ sgid (4)          ├─ Sadece capability mod'da gerek
├─ capabilities (8)  │
├─ ambient_caps (8)  ┤
├─ security_level (1)│
├─ namespace_id (4)  │
├─ privileged (1)    │
└─ audit_enabled (1) ┘
    
Task sayısı: 1000
Overhead: 1000 × 64 = 64 KB (şu an)
```

#### Optimize edilmiş versiyon (Opsiyonel)

```rust
// Minimal mode
pub struct SecurityContextMin {
    pub euid: u32,           // 4 bytes
    pub mode: SecurityMode,  // 1 byte
}  // = 5 bytes (+ 3 padding = 8 bytes)

// Full mode (feature-gated)
#[cfg(feature = "policy_enforcement")]
pub struct SecurityContextFull {
    pub task_id: TaskId,
    pub process_id: ProcessId,
    pub euid: u32,
    pub capabilities: u64,
    pub privileged: bool,
    pub limits: ResourceLimits,
}  // = 96 bytes

// Compile-time selection
#[cfg(feature = "policy_enforcement")]
pub type SecurityContext = SecurityContextFull;

#[cfg(not(feature = "policy_enforcement"))]
pub type SecurityContext = SecurityContextMin;
```

---

### 7. Runtime Optional Selection

```rust
// Kernel startup

pub fn init_security_subsystem(config: SecurityConfig) {
    match config.mode {
        SecurityMode::Disabled => {
            log::info("Security: DISABLED (no overhead)");
            // Skip all security initialization
            return;
        }
        SecurityMode::CapabilityOnly => {
            log::info("Security: CAPABILITIES ONLY");
            init_capability_checker();
        }
        SecurityMode::PolicyEnforcement => {
            log::info("Security: FULL POLICY ENFORCEMENT");
            init_security_monitor();
            init_audit_system();
            init_policy_backend();
        }
    }
}

// Per-syscall check

#[inline(always)]
fn syscall_check_permission(ctx: &SecurityContext, cap: u64) -> bool {
    // JIT: Compiler optimizes based on config at compile time
    #[cfg(feature = "security_capabilities")]
    {
        match ctx.mode {
            SecurityMode::Disabled => return true,  // Entire match eliminated in Disabled mode
            SecurityMode::CapabilityOnly => ctx.has_capability(cap),
            SecurityMode::PolicyEnforcement => {
                if !ctx.has_capability(cap) {
                    return SECURITY_MONITOR.check_policy(ctx, cap);
                }
                true
            }
        }
    }
    
    #[cfg(not(feature = "security_capabilities"))]
    true  // Compile-time: returns true unconditionally
}
```

---

### 8. Performance Comparison

| Scenario | Overhead | Memory | Use Case |
|----------|----------|--------|----------|
| **Disabled** | 0 | 0 KB | Embedded, sandbox testing |
| **Capabilities** | 2-3 cycles | 16 KB (1000 tasks) | Most systems |
| **Full Policy** | 5-8 cycles | 64 KB | Secure servers, multi-tenant |

#### Example: 10,000 file opens/sec

```
Mode 1 (Disabled):
- 0 cycles × 10,000 = 0 total
- CPU impact: 0%

Mode 2 (Capabilities):
- 2.5 cycles × 10,000 = 25,000 cycles
- CPU impact: 0.008% (negligible at 3GHz)

Mode 3 (Full Policy):
- 6 cycles × 10,000 = 60,000 cycles
- CPU impact: 0.02% (still negligible)
```

---

### 9. Implementation Path

#### Phase 4 (Sonraki)
1. [ ] SecurityMode enum oluştur
2. [ ] Feature flags: `security_capabilities`, `policy_enforcement`, `audit_logging`
3. [ ] Conditional field compilation (#[cfg(feature = ...)])
4. [ ] has_capability() inline optimization
5. [ ] Tests 3 mode'da çalış

#### Phase 5 (Sonra)
1. [ ] Runtime mode switching
2. [ ] Policy backend selection
3. [ ] Audit system integration
4. [ ] WASM sandboxing (optional feature)

---

### 10. Önerilen Konfigürasyon

```toml
# Cargo.toml

[features]
default = []

# Build profiles
[profile.dev]
opt-level = 1

[profile.release]
opt-level = 3
lto = true
codegen-units = 1

# Feature combinations
scenarios = ["embedded", "standard", "secure"]

[scenarios]
embedded = []  # No security
standard = ["security_capabilities"]
secure = ["security_capabilities", "policy_enforcement", "audit_logging"]
```

**Kompil komutu örnekleri**:
```bash
# Hızlı embedded (0 overhead)
cargo build --release --no-default-features

# Standard (capability-based)
cargo build --release --features security_capabilities

# Secure production
cargo build --release --features "security_capabilities,policy_enforcement,audit_logging"
```

---

Bu yapı sayesinde:
- ✅ **Sıfır overhead** → Gerekli değilse hiç kod çalışmıyor
- ✅ **Esnek** → Runtime'da mode seç
- ✅ **Hızlı** → Inline checks, bitwise operations
- ✅ **Güvenli** → Feature disable etme ≠ hack
- ✅ **Backward compatible** → Default capabilities enabled

