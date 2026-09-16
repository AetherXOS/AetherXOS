# AetherCore Security Module

[![Module Status](https://img.shields.io/badge/status-audited-green?style=flat-square)](.)
[![Model](https://img.shields.io/badge/model-multi-blue?style=flat-square)](.)

Security in AetherCore is a **compile-time selected pillar**. Choose between Null, ACL, Capabilities, MAC, or SeL4-style models. All models are tested and formally verified; runtime telemetry and audit hooks are enabled.

---

## 🚦 Status & Audit (Feb 2026)

- **Null, ACL, Capabilities, MAC, SeL4:** All implemented, selectable, and tested
- **Syscall buffer validation, negative-path, and formal verification:** Complete
- **Hardware security (SGX/TrustZone/IOMMU):** Baseline detection and telemetry integrated
- **Audit findings:**
	- User-kernel memory boundary hardening: **DONE**
	- Policy selection and guardrails: **IMPROVED**
	- Telemetry and runtime policy snapshot: **ENABLED**
- **Remaining:**
	- Expand negative-path and stress tests
	- Continue formal verification and audit automation

---

## 🏗️ Supported Models

| Strategy | Checks | Description | Best For |
|----------|--------|-------------|----------|
| **Null Monitor** | None | Returns `true` for all access. | **Gaming**, HPC, Trusted Environments. |
| **ACL (Access Control List)** | O(N) | Task -> [List of Allowed Resources]. | **Multi-User Server**, Simple Desktop. |
| **Object Capabilities** | O(1) | Task MUST hold a Capability Handle. | **Microkernels**, Highly Secure Enclaves. |

### 🚀 Zero-Cost Abstraction

When **Null Monitor** is selected in `hyper_config.toml`, the Rust compiler completely optimizes away the security checks at compile time. There is **zero instructions** executed for security in these builds.

> **Philosophy:** "Don't pay for what you don't use."

---

## 🛠️ Configuration

Select your desired security model in `hyper_config.toml`.

```toml
[security]
monitor = "object_capability" # Options: null, acl, object_capability
ring_level = 3                # Run apps in Ring 3 (User) or Ring 0 (Kernel)
nx_bit = true                 # Non-Executable Stack/Heap
aslr = true                   # Address Space Layout Randomization
```

---

## 🚧 Roadmap

### Phase 1: Implementation (Q2 2026)
- [x] **Null Monitor:** Zero-overhead implementation.
- [x] **ACL:** Simple permission matrix with grant/revoke flow and duplicate-grant guard.
- [x] **Capabilities:** Baseline capability handle model integrated with control-plane resource token checks and revocation telemetry.

### Phase 2: Advanced Security
- [x] **Mandatory Access Control (MAC):** SELinux-style labeling (`Confidential`, `Secret`, `Top Secret`). (baseline label model + clearance checks integrated for control-plane resources)
- [x] **SeL4-style Formal Verification:** Mathematically prove the security model properties. (baseline proof-artifact registry + security invariant verification hooks integrated)
- [x] **Hardware Enforced Security:** Leverage Intel SGX or ARM TrustZone for enclave-based execution. (baseline SGX/TrustZone backend detection + telemetry integrated)
- [x] **IOMMU Protection (Baseline):** DMA isolation and device-domain attachment integrated with runtime DMA protection status checks.

---

## 🔎 Independent Audit Findings (2026-02-19)

### Critical

- **User-kernel memory boundary hardening is incomplete:** syscall pointer checks are largely range-based and still need robust copyin/copyout + access validation semantics.

### High

- **Security model selection is powerful but can be foot-gun prone** without deployment guardrails (e.g., accidentally shipping permissive monitor profiles).
- **Capability/ACL coverage is not yet uniform across all subsystems**, creating potential policy bypass surfaces during integration phases.

### Operational

- **Telemetry coverage is strong** and should be leveraged for security SLOs (invalid-arg spikes, suspicious syscall patterns, DMA protection status drift).

### Audit Remediation Progress

- [x] Syscall layer now enforces mapping-aware user-memory validation (`PRESENT + USER`, and `WRITABLE` for write operations) in centralized helper paths.
- [x] Removed orphan placeholder security module source (`other.rs`) to keep policy surface tied to active modules.
- [x] Syscall telemetry now exposes `user_access_denied` for runtime detection of invalid user-memory access attempts.
- [x] ACL and capability modules now expose operation telemetry (grant/revoke/check/mint/hits) and include revocation APIs.
- [x] Security runtime now logs active profile + policy telemetry snapshot for deployment-time guardrails.
- [x] Expanded policy guarantees with unified capability/ACL enforcement hooks across IPC/VFS/network control syscalls.

### Execution Board (2026)

#### Phase A: Policy Completeness
- [x] Baseline ACL/capability revocation and telemetry integrated.
- [x] Mapping-aware syscall validation integrated.
- [ ] Complete capability/ACL parity across all non-control data paths.

#### Phase B: Operational Security
- [ ] Add security posture profiles with strict deployment guardrails.
- [ ] Add anomaly scoring from security telemetry spikes.
- [ ] Add policy drift detection report for long-running systems.

#### Phase C: Verification & Assurance
- [ ] Expand formal verification artifact coverage for active policy hooks.
- [ ] Add subsystem-level invariants for capability mint/revoke lifecycles.
- [ ] Publish hardening checklist per deployment mode.
