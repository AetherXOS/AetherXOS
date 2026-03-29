#!/usr/bin/env python3
"""
Analyze which syscall stubs are blocking glibc compatibility.
Categorizes stubs by priority for Linux application support.
"""

import json
from pathlib import Path
from collections import defaultdict

# Load the gap inventory
gap_report_path = Path("reports/abi_gap_inventory/summary.json")
if not gap_report_path.exists():
    print(f"Gap report not found at {gap_report_path}")
    print("Please run: python scripts/linux_abi_gap_inventory.py")
    exit(1)

gap_data = json.loads(gap_report_path.read_text())

# Categorize syscalls by priority
categories = {
    "CRITICAL_FILE_IO": {
        "description": "Essential file I/O operations - absolutely required",
        "syscalls": ["read", "write", "open", "openat", "close", "lseek", "getdents64", "readdir", "readv", "writev", "preadv", "pwritev"],
    },
    "CRITICAL_PROCESS": {
        "description": "Essential process operations",
        "syscalls": ["fork", "clone", "clone3", "execve", "execveat", "wait4", "waitpid", "exit", "exit_group"],
    },
    "CRITICAL_MEMORY": {
        "description": "Essential memory management",
        "syscalls": ["mmap", "mmap2", "munmap", "brk", "mprotect", "mremap"],
    },
    "CRITICAL_SIGNALS": {
        "description": "Essential signal handling",
        "syscalls": ["rt_sigaction", "rt_sigprocmask", "rt_sigpending", "rt_sigtimedwait", "sigaltstack"],
    },
    "IMPORTANT_THREADING": {
        "description": "Important for multi-threaded glibc",
        "syscalls": ["futex", "futex2", "clone3", "set_tid_address", "set_robust_list", "get_robust_list"],
    },
    "IMPORTANT_FD_OPS": {
        "description": "Important file descriptor operations",
        "syscalls": ["dup", "dup2", "dup3", "pipe", "pipe2", "poll", "epoll_create", "epoll_wait", "select"],
    },
    "IMPORTANT_FS": {
        "description": "Important filesystem operations",
        "syscalls": ["stat", "lstat", "fstat", "statx", "access", "faccessat", "mkdir", "rmdir", "rename", "unlink"],
    },
    "SUPPORT": {
        "description": "Support functions glibc may use",
        "syscalls": ["time", "clock_gettime", "gettimeofday", "utime", "utimens"],
    }
}

# Build a map of syscall name -> priority
syscall_priority = {}
for priority, info in categories.items():
    for syscall in info["syscalls"]:
        syscall_priority[syscall] = priority

# Analyze stub entries
stub_analysis = defaultdict(list)

for entry in gap_data.get("entries", []):
    if entry["category"] != "stub":
        continue
    
    fn_name = entry["function"]
    file_path = entry["file"]
    
    # Extract syscall name from function name (e.g., sys_linux_read -> read)
    if fn_name.startswith("sys_linux_"):
        syscall_name = fn_name[10:]  # remove "sys_linux_" prefix
    else:
        syscall_name = fn_name
    
    priority = syscall_priority.get(syscall_name, "OTHER")
    stub_analysis[priority].append({
        "syscall": syscall_name,
        "function": fn_name,
        "file": file_path,
    })

# Print analysis
print("=" * 100)
print("GLIBC COMPATIBILITY BLOCKER ANALYSIS".center(100))
print("=" * 100)
print()

for category in ["CRITICAL_FILE_IO", "CRITICAL_PROCESS", "CRITICAL_MEMORY", "CRITICAL_SIGNALS", 
                 "IMPORTANT_THREADING", "IMPORTANT_FD_OPS", "IMPORTANT_FS", "SUPPORT", "OTHER"]:
    if category not in stub_analysis:
        continue
    
    stubs = stub_analysis[category]
    if not stubs:
        continue
    
    desc = categories.get(category, {}).get("description", category)
    print(f"\n{category}: {desc}")
    print("-" * 100)
    print(f"Stub Count: {len(stubs)}\n")
    
    for stub in sorted(stubs, key=lambda x: x["syscall"]):
        print(f"  • {stub['syscall']:30} in {stub['file'].split('/')[-1]:40}")

print("\n" + "=" * 100)
total_critical = len(stub_analysis.get("CRITICAL_FILE_IO", [])) + \
                len(stub_analysis.get("CRITICAL_PROCESS", [])) + \
                len(stub_analysis.get("CRITICAL_MEMORY", [])) + \
                len(stub_analysis.get("CRITICAL_SIGNALS", []))
print(f"TOTAL CRITICAL STUBS BLOCKING GLIBC: {total_critical}")
print("=" * 100)

# Recommendation
print("\nRECOMMENDED IMPLEMENTATION ORDER:")
print("1. CRITICAL_FILE_IO  - read, write, open, close, lseek (5 syscalls)")
print("2. CRITICAL_PROCESS  - fork, clone, exec, wait (4 syscalls)")
print("3. CRITICAL_MEMORY   - mmap, brk, mprotect (3 syscalls)")
print("4. CRITICAL_SIGNALS  - rt_sigaction, rt_sigprocmask (2 syscalls)")
print("\nEstimated coverage: With these ~14 syscalls, most basic glibc programs can run.")
