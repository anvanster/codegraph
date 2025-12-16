# C Parser Layered Architecture Implementation Plan

## Overview

This document outlines a plan to enhance the `codegraph-c` parser with a layered processing architecture inspired by the `kernel-analyzer` crate. The goal is to achieve better parsing success rates on real-world kernel/driver code across multiple operating systems.

## Current State

### What We Have
- tree-sitter based C parser with tolerant mode
- Basic preprocessor with:
  - 45+ type macros (u8, u32, size_t, etc.)
  - 50+ attribute macros to strip (__init, __user, etc.)
  - container_of and offsetof expansion
- Call graph extraction
- Cyclomatic complexity metrics

### Current Limitations
- No platform detection (assumes Linux)
- Type macros recognized but not defined (tree-sitter still sees unknown types)
- Limited GCC extension handling
- No callback/ops struct resolution
- No conditional evaluation (#if 0 blocks parsed as errors)
- ~30% of errors from preprocessor conditionals

### Test Results (Linux ICE Driver)
| Mode | Success | Functions | Errors |
|------|---------|-----------|--------|
| Strict | 27% | 322 | fails |
| Tolerant | 100% | 3,679 | 1,394 |
| Kernel | 100% | 3,683 | 1,398 |

## Proposed Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      Source Code                            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 1: Platform Detection                                │
│  ─────────────────────────────────────────────────────────  │
│  • Pattern matching (includes, macros, types)               │
│  • Weighted confidence scoring                              │
│  • Returns: PlatformId + confidence                         │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 2: Header Stub Injection                             │
│  ─────────────────────────────────────────────────────────  │
│  • Parse #include directives                                │
│  • Inject stub definitions for known headers                │
│  • Platform-specific type definitions                       │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 3: Conditional Evaluation                            │
│  ─────────────────────────────────────────────────────────  │
│  • Strip #if 0 ... #endif blocks                            │
│  • Optionally evaluate simple conditions                    │
│  • Preserve line numbers with empty lines                   │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 4: GCC Extension Neutralization                      │
│  ─────────────────────────────────────────────────────────  │
│  • __attribute__((...)) removal                             │
│  • __extension__, __asm__ handling                          │
│  • Statement expressions ({ }) → (0)                        │
│  • typeof → int replacement                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 5: Attribute Stripping                               │
│  ─────────────────────────────────────────────────────────  │
│  • Platform-specific attributes                             │
│  • __init, __exit, __user, __iomem, etc.                    │
│  • Function-like attribute removal                          │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 6: tree-sitter Parsing                               │
│  ─────────────────────────────────────────────────────────  │
│  • Fault-tolerant AST generation                            │
│  • ERROR node handling                                      │
│  • Entity extraction (functions, structs, calls)            │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│  Layer 7: Semantic Analysis                                 │
│  ─────────────────────────────────────────────────────────  │
│  • Callback resolution (ops struct assignments)             │
│  • Call normalization (kmalloc → MemAlloc)                  │
│  • Call type classification                                 │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    CodeIR / CallGraph                       │
└─────────────────────────────────────────────────────────────┘
```

## Implementation Plan

### Phase 1: Platform Detection System

**New module**: `src/platform/mod.rs`

```rust
pub trait PlatformModule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn detection_patterns(&self) -> Vec<DetectionPattern>;
    fn header_stubs(&self) -> &HeaderStubs;
    fn attributes_to_strip(&self) -> &[&'static str];
    fn ops_structs(&self) -> &[OpsStructDef];
}

pub struct PlatformRegistry {
    platforms: Vec<Box<dyn PlatformModule>>,
}
```

**Detection patterns with weights**:
```rust
pub struct DetectionPattern {
    pub kind: DetectionKind,  // Include, Macro, FunctionCall, TypeName
    pub pattern: String,
    pub weight: f32,
}
```

**Supported platforms** (initial):
| Platform | Key Patterns |
|----------|--------------|
| Linux | `linux/`, `MODULE_LICENSE`, `__init` |
| FreeBSD | `sys/param.h`, `DRIVER_MODULE`, `__FreeBSD__` |
| Darwin/XNU | `mach/`, `IOKit/`, `kext` |

**Files to create**:
- `src/platform/mod.rs` - PlatformModule trait, registry
- `src/platform/linux.rs` - Linux kernel module
- `src/platform/freebsd.rs` - FreeBSD module
- `src/platform/darwin.rs` - Darwin/XNU module
- `src/platform/generic.rs` - Fallback for unknown

**Estimated complexity**: Medium

---

### Phase 2: Header Stub System

**New module**: `src/stubs.rs`

```rust
pub struct HeaderStubs {
    headers: HashMap<String, String>,
}

impl HeaderStubs {
    pub fn get_for_includes(&self, source: &str) -> String;
    pub fn add(&mut self, path: &str, content: &str);
}
```

**Key insight**: Instead of just recognizing `u32` as a type macro, actually define it:

```rust
stubs.add("linux/types.h", r#"
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long long u64;
typedef u16 __le16;
typedef u32 __le32;
// ... etc
"#);
```

**Initial stub headers for Linux**:
| Header | Key Definitions |
|--------|-----------------|
| `linux/types.h` | u8, u16, u32, u64, __le*, __be*, atomic_t |
| `linux/kernel.h` | size_t, bool, NULL, printk |
| `linux/module.h` | MODULE_*, EXPORT_SYMBOL, THIS_MODULE |
| `linux/fs.h` | file_operations, inode_operations |
| `linux/pci.h` | pci_driver, pci_device_id |
| `linux/netdevice.h` | net_device_ops, ethtool_ops |
| `linux/slab.h` | kmalloc, kfree, GFP_* |
| `linux/mutex.h` | mutex, DEFINE_MUTEX |
| `linux/spinlock.h` | spinlock_t, DEFINE_SPINLOCK |

**Estimated complexity**: Medium

---

### Phase 3: Conditional Evaluation

**Enhance**: `src/preprocessor.rs`

```rust
pub enum ConditionalStrategy {
    KeepAll,        // Don't modify conditionals
    StripAll,       // Remove all # lines except #include
    EvaluateSimple, // Strip #if 0, keep #if 1
}

pub fn evaluate_conditionals(source: &str, strategy: ConditionalStrategy) -> String;
```

**EvaluateSimple logic**:
1. Track nesting depth
2. When `#if 0` encountered, skip until matching `#endif`
3. Handle `#else` - if in `#if 0` block, start including
4. Preserve line count with empty lines for position mapping

**Example**:
```c
int a;          // kept
#if 0           // removed
int b;          // removed
#else           // removed
int c;          // kept
#endif          // removed
int d;          // kept
```

**Estimated complexity**: Low-Medium

---

### Phase 4: GCC Extension Neutralizer

**New module**: `src/gcc.rs`

```rust
pub struct GccNeutralizer {
    patterns: Vec<(Regex, Replacement)>,
}

pub struct NeutralizedSource {
    pub code: String,
    pub transformations: Vec<Transformation>,
}
```

**Patterns to handle**:
| Pattern | Action | Example |
|---------|--------|---------|
| `__attribute__((...))` | Strip | `__attribute__((packed))` → `` |
| `__extension__` | Strip | `__extension__ union` → `union` |
| `__typeof(x)` | Replace with `int` | `__typeof(foo)` → `int` |
| `({ expr; })` | Replace with `(0)` | `({ x + 1; })` → `(0)` |
| `__asm__ volatile(...)` | Strip | entire statement |
| `__restrict` | Strip | `int * __restrict p` → `int * p` |
| `__inline__` | Strip | |

**Transformation tracking** (for position mapping):
```rust
pub struct Transformation {
    pub original_range: Range<usize>,
    pub kind: TransformKind,
    pub original_text: String,
}
```

**Estimated complexity**: Medium

---

### Phase 5: Enhanced Attribute Stripping

**Enhance**: `src/preprocessor.rs`

Move attribute stripping to be platform-configurable:

```rust
impl PlatformModule for LinuxModule {
    fn attributes_to_strip(&self) -> &[&'static str] {
        &[
            "__init", "__exit", "__user", "__kernel", "__iomem",
            "__percpu", "__rcu", "__cold", "__hot", "__pure",
            "__must_check", "__deprecated", "noinline",
            "__always_inline", "asmlinkage", "__visible",
            "__cacheline_aligned", "__read_mostly",
        ]
    }
}
```

**Estimated complexity**: Low

---

### Phase 6: Callback Resolution

**New module**: `src/callbacks.rs`

```rust
pub struct OpsStructDef {
    pub struct_name: String,
    pub fields: Vec<OpsFieldDef>,
}

pub struct OpsFieldDef {
    pub name: String,
    pub category: CallbackCategory,
}

pub enum CallbackCategory {
    Init, Cleanup, Open, Close, Read, Write,
    Ioctl, Mmap, Poll, Probe, Remove, Interrupt,
}

pub struct ResolvedCallback {
    pub struct_type: String,
    pub field: String,
    pub function: String,
    pub category: CallbackCategory,
    pub line: usize,
}
```

**Resolution algorithm**:
1. For each known ops struct type
2. Search for designated initializers: `.field = function_name`
3. Match against ops struct field definitions
4. Record callback assignment

**Example**:
```c
static const struct file_operations my_fops = {
    .open = my_open,      // → ResolvedCallback { field: "open", function: "my_open", category: Open }
    .release = my_close,  // → ResolvedCallback { field: "release", function: "my_close", category: Close }
};
```

**Estimated complexity**: Medium

---

### Phase 7: Call Normalization

**New module**: `src/unified.rs`

```rust
pub enum UnifiedCall {
    // Memory
    MemAlloc, MemFree, MemRealloc, MemCopy, MemSet,
    // User/Kernel boundary
    CopyFromUser, CopyToUser,
    // Synchronization
    LockAcquire, LockRelease, WaitEvent, SignalEvent,
    // I/O
    IoRemap, IoUnmap, IoRead, IoWrite,
    // DMA
    DmaAlloc, DmaFree, DmaMap, DmaUnmap,
    // Interrupts
    InterruptRegister, InterruptUnregister,
    // Device
    DeviceRegister, DeviceUnregister,
    // Logging
    Log,
    // Other
    Other(String),
}

pub trait CallNormalizer {
    fn normalize(&self, function_name: &str, args: &[String]) -> UnifiedCall;
}
```

**Linux normalization examples**:
| Function | Category |
|----------|----------|
| kmalloc, kzalloc, kcalloc | MemAlloc |
| kfree, kvfree, vfree | MemFree |
| copy_from_user, get_user | CopyFromUser |
| mutex_lock, spin_lock | LockAcquire |
| ioremap, pci_ioremap_bar | IoRemap |
| request_irq | InterruptRegister |
| printk, pr_info, dev_err | Log |

**Estimated complexity**: Low-Medium

---

### Phase 8: Enhanced Call Target Classification

**Enhance**: `src/visitor.rs`

```rust
pub enum CallTarget {
    /// Direct function call: foo()
    Direct(String),
    /// Function pointer: (*fn_ptr)()
    FunctionPointer { variable: String },
    /// Ops struct callback: ops->method()
    OpsCallback { object: String, field: String },
    /// Macro invocation: MACRO()
    Macro(String),
    /// Unknown target
    Unknown,
}

pub struct CallEdge {
    pub caller: String,
    pub callee: CallTarget,
    pub call_type: CallType,
    pub unified: Option<UnifiedCall>,
    pub line: usize,
    pub column: usize,
}
```

**Estimated complexity**: Low

---

## File Structure After Implementation

```
crates/codegraph-c/
├── src/
│   ├── lib.rs
│   ├── parser_impl.rs       # CodeParser trait impl
│   ├── extractor.rs         # Main extraction logic
│   ├── visitor.rs           # AST visitor
│   ├── preprocessor.rs      # Legacy (to be refactored)
│   ├── mapper.rs            # IR mapping
│   │
│   ├── pipeline/            # NEW: Layered processing
│   │   ├── mod.rs           # Pipeline orchestration
│   │   ├── conditionals.rs  # #if evaluation
│   │   └── gcc.rs           # GCC extension neutralizer
│   │
│   ├── platform/            # NEW: Platform abstraction
│   │   ├── mod.rs           # PlatformModule trait, registry
│   │   ├── linux.rs         # Linux kernel
│   │   ├── freebsd.rs       # FreeBSD
│   │   ├── darwin.rs        # Darwin/XNU
│   │   └── generic.rs       # Fallback
│   │
│   ├── stubs/               # NEW: Header stubs
│   │   ├── mod.rs           # HeaderStubs struct
│   │   └── linux/           # Linux header stubs
│   │       ├── types.rs
│   │       ├── kernel.rs
│   │       ├── fs.rs
│   │       └── ...
│   │
│   ├── callbacks.rs         # NEW: Ops struct resolution
│   └── unified.rs           # NEW: Call normalization
│
└── examples/
    ├── analyze_errors.rs
    ├── test_tolerant_kernel.rs
    └── test_platform_detection.rs  # NEW
```

---

## Implementation Priority

### High Priority (Phase 1-3)
These provide the biggest error reduction:

1. **Header Stubs** - Defines types that currently cause "unknown type" errors
2. **Conditional Evaluation** - Eliminates 30% of current errors
3. **GCC Neutralizer** - Handles __attribute__ and extensions cleanly

### Medium Priority (Phase 4-5)
Platform abstraction for multi-OS support:

4. **Platform Detection** - Auto-detect Linux vs FreeBSD vs Darwin
5. **Platform-specific Attributes** - Configurable per-OS

### Lower Priority (Phase 6-8)
Enhanced semantic analysis:

6. **Callback Resolution** - Track function pointer assignments
7. **Call Normalization** - Unified semantic categories
8. **Enhanced Call Classification** - Better call target types

---

## Expected Outcomes

### Error Reduction Targets
| Current Errors | After Phase 1-3 | Target |
|----------------|-----------------|--------|
| 1,394 (tolerant) | ~400-600 | 50-70% reduction |

### Key Metrics
- **Syntax errors**: From ~1,400 to <500
- **Clean parses**: From 27% to >60%
- **Function extraction**: Maintain 3,600+ functions
- **Platform detection**: >90% accuracy for Linux/FreeBSD/Darwin

---

## Testing Strategy

### Unit Tests
- Each layer tested in isolation
- Pattern matching tests for platform detection
- Stub generation tests
- Conditional evaluation tests
- GCC neutralization tests

### Integration Tests
- Full pipeline on Linux kernel files
- FreeBSD driver samples
- Darwin/XNU kext samples

### Regression Tests
- Ensure tolerant mode still extracts same entities
- No reduction in function/struct count
- Complexity metrics unchanged

---

## Migration Path

1. Implement new modules alongside existing preprocessor
2. Create `LayeredPipeline` that orchestrates all layers
3. Add `ExtractionOptions::layered()` configuration
4. Keep `ExtractionOptions::for_kernel_code()` using old approach
5. Compare results, tune thresholds
6. Deprecate old approach when new is proven

---

## References

- `kernel-analyzer` crate architecture
- tree-sitter error recovery documentation
- GCC extension documentation
- Linux kernel coding style
- FreeBSD driver development guide
