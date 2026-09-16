# 🔍 AetherXOS: Linux Syscall Quality Audit & Upgrade Roadmap

## 🚩 Audit Findings: Simulation Violations
The following syscalls are currently implemented as **stubs** or **simulations**, violating the "Simulation is Forbidden" (Simülasyon Yasaktır) standard.

### 1. `io_uring` Subsystem
*   **Current State**: `io_uring_setup`, `enter`, and `register` only manage a fake FD counter. No async operation logic exists.
*   **Fix**: Implement a real `IoUring` engine with ring-buffer management and VFS/Network integration.

### 2. `openat2` (Security)
*   **Current State**: Wraps `openat` and ignores `RESOLVE_*` flags.
*   **Fix**: Update VFS path resolution to enforce `RESOLVE_BENEATH` and other security constraints.

### 3. `pidfd` Family
*   **Current State**: Minimal wrappers that often ignore the "file-descriptor-to-pid" safety guarantees.
*   **Fix**: Implement `pidfd` as a native kernel object that holds a strong reference to a `Task`.

### 4. `landlock` (Sandboxing)
*   **Current State**: Pure stub (always returns success but does nothing).
*   **Fix**: Integrate with the `security` module to enforce rulesets during path resolution.

### 5. `memfd_create` & `memfd_secret`
*   **Current State**: Redirects to a regular file in a ramfs-like structure. No sealing or secrecy.
*   **Fix**: Implement anonymous memory-backed files with sealing support.

---

## 📈 Performance & Stability Plan

### 1. High-Performance `io_uring` (Goal: < 50ns submission overhead)
*   **Technique**: Use `mmap` for ring buffers to avoid all syscall overhead after setup.
*   **Integration**: Hook into the `hal` interrupt handlers to complete I/O requests directly into the CQ (Completion Queue).

### 2. Lock-Free Syscall Dispatch
*   **Problem**: Current dispatcher uses various global locks (e.g., `LINUX_IO_URING_IDS`).
*   **Solution**: Use per-task or lock-free data structures (e.g., `DashMap`-like or simple arrays for FDs).

### 3. Comprehensive `statx`
*   **Improvement**: Fill all fields including `btime`, `attributes`, and `mount_id`.

---

## 🚀 Execution Strategy

1.  **Audit Complete**: (This document)
2.  **Implementation Wave 1 (Security & Core)**: `openat2`, `pidfd`, `memfd_create`.
3.  **Implementation Wave 2 (Performance)**: `io_uring` infrastructure.
4.  **Implementation Wave 3 (Refinement)**: `statx`, `landlock` basics, `seccomp` full validation.

---

## ⚖️ Comparison vs. Linux (Updated)

| Metric | Linux (Standard) | AetherXOS (Current) | AetherXOS (Target) |
|:---|:---|:---|:---|
| **Async I/O** | Native `io_uring` | **Stub** | **Native Zero-Copy** |
| **Path Security** | `openat2` | **Ignored** | **Strict Enforced** |
| **PID Management** | `pidfd` (Mature) | **Partial** | **Full Reference-Counted** |
| **Memory Files** | `memfd` (Sealing) | **Simulated** | **Native Anonymous** |
