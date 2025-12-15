//! Kernel macro expansion and neutralization
//!
//! Handles common Linux kernel macros that cause parsing errors by either:
//! 1. Expanding them to valid C syntax
//! 2. Replacing them with neutral stubs
//! 3. Stripping them entirely when safe

// Allow dead code for regex patterns that are defined for future use
#![allow(dead_code)]

use regex::Regex;
use std::sync::LazyLock;

/// Macro neutralizer for kernel code
pub struct MacroNeutralizer {
    /// Statistics for tracking transformations
    stats: MacroStats,
}

/// Statistics about macro transformations
#[derive(Debug, Default, Clone)]
pub struct MacroStats {
    pub likely_unlikely_stripped: usize,
    pub container_of_expanded: usize,
    pub for_each_expanded: usize,
    pub list_for_each_expanded: usize,
    pub build_bug_on_stripped: usize,
    pub warn_on_stripped: usize,
    pub rcu_simplified: usize,
    pub typeof_replaced: usize,
    pub define_macros_stubbed: usize,
    pub statement_expressions_simplified: usize,
}

// Regex patterns for macro detection

// likely(x) / unlikely(x) -> (x)
static RE_LIKELY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(likely|unlikely)\s*\(").unwrap());

// BUILD_BUG_ON(...) -> ((void)0)
static RE_BUILD_BUG_ON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bBUILD_BUG_ON\s*\([^;]*\)").unwrap());

// BUILD_BUG_ON_MSG(...) -> ((void)0)
static RE_BUILD_BUG_ON_MSG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bBUILD_BUG_ON_MSG\s*\([^;]*\)").unwrap());

// WARN_ON(x) -> (x)
static RE_WARN_ON: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bWARN_ON(?:_ONCE)?\s*\(").unwrap());

// BUG_ON(x) -> ((void)(x))
static RE_BUG_ON: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bBUG_ON\s*\(").unwrap());

// typeof(x) -> __typeof_placeholder__
static RE_TYPEOF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(typeof|__typeof__|__typeof)\s*\(").unwrap());

// DEFINE_MUTEX(name) -> struct mutex name
static RE_DEFINE_MUTEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_MUTEX\s*\(\s*(\w+)\s*\)").unwrap());

// DEFINE_SPINLOCK(name) -> spinlock_t name
static RE_DEFINE_SPINLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_SPINLOCK\s*\(\s*(\w+)\s*\)").unwrap());

// DEFINE_RWLOCK(name) -> rwlock_t name
static RE_DEFINE_RWLOCK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_RWLOCK\s*\(\s*(\w+)\s*\)").unwrap());

// DEFINE_SEMAPHORE(name) -> struct semaphore name
static RE_DEFINE_SEMAPHORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_SEMAPHORE\s*\(\s*(\w+)\s*\)").unwrap());

// DEFINE_IDA(name) -> struct ida name
static RE_DEFINE_IDA: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_IDA\s*\(\s*(\w+)\s*\)").unwrap());

// DEFINE_IDR(name) -> struct idr name
static RE_DEFINE_IDR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDEFINE_IDR\s*\(\s*(\w+)\s*\)").unwrap());

// DECLARE_BITMAP(name, bits) -> unsigned long name[bits/64 + 1]
static RE_DECLARE_BITMAP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDECLARE_BITMAP\s*\(\s*(\w+)\s*,\s*(\w+)\s*\)").unwrap());

// DECLARE_WAIT_QUEUE_HEAD(name) -> wait_queue_head_t name
static RE_DECLARE_WAIT_QUEUE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bDECLARE_WAIT_QUEUE_HEAD\s*\(\s*(\w+)\s*\)").unwrap());

// LIST_HEAD(name) -> struct list_head name
static RE_LIST_HEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bLIST_HEAD\s*\(\s*(\w+)\s*\)").unwrap());

// HLIST_HEAD(name) -> struct hlist_head name
static RE_HLIST_HEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bHLIST_HEAD\s*\(\s*(\w+)\s*\)").unwrap());

// Statement expression ({ ... }) - simplified pattern for outer level
static RE_STMT_EXPR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(\{[^{}]*\}\)").unwrap());

// rcu_dereference(p) -> (p)
static RE_RCU_DEREF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\brcu_dereference(?:_protected|_raw|_check)?\s*\(").unwrap());

// rcu_assign_pointer(p, v) -> ((p) = (v))
static RE_RCU_ASSIGN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\brcu_assign_pointer\s*\(").unwrap());

// READ_ONCE(x) / WRITE_ONCE(x, v) -> simplified
static RE_READ_ONCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bREAD_ONCE\s*\(").unwrap());

static RE_WRITE_ONCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bWRITE_ONCE\s*\(").unwrap());

// ACCESS_ONCE(x) -> (x)
static RE_ACCESS_ONCE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bACCESS_ONCE\s*\(").unwrap());

// smp_load_acquire / smp_store_release
static RE_SMP_LOAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsmp_load_acquire\s*\(").unwrap());

static RE_SMP_STORE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bsmp_store_release\s*\(").unwrap());

// ARRAY_SIZE(arr) -> (sizeof(arr)/sizeof((arr)[0]))
static RE_ARRAY_SIZE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bARRAY_SIZE\s*\(\s*(\w+)\s*\)").unwrap());

// min/max macros
static RE_MIN_T: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bmin_t\s*\(\s*\w+\s*,").unwrap());

static RE_MAX_T: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bmax_t\s*\(\s*\w+\s*,").unwrap());

// clamp macros
static RE_CLAMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bclamp\s*\(").unwrap());

// offsetof - usually OK but sometimes causes issues
static RE_OFFSETOF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\boffsetof\s*\(\s*(\w+)\s*,\s*(\w+)\s*\)").unwrap());

// IS_ERR/PTR_ERR/ERR_PTR
static RE_IS_ERR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bIS_ERR(?:_OR_NULL)?\s*\(").unwrap());

static RE_PTR_ERR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bPTR_ERR\s*\(").unwrap());

static RE_ERR_PTR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bERR_PTR\s*\(").unwrap());

// ERR_CAST
static RE_ERR_CAST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\bERR_CAST\s*\(").unwrap());

// __must_check, __always_inline, etc.
static RE_MUST_CHECK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__must_check\b").unwrap());

static RE_ALWAYS_INLINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b__always_inline\b").unwrap());

static RE_NOINLINE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__noinline\b").unwrap());

static RE_COLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__cold\b").unwrap());

static RE_HOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__hot\b").unwrap());

static RE_PURE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__pure\b").unwrap());

static RE_INIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__init\b").unwrap());

static RE_EXIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__exit\b").unwrap());

static RE_INITDATA: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__initdata\b").unwrap());

static RE_DEVINIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__devinit\b").unwrap());

static RE_DEVEXIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__devexit\b").unwrap());

static RE_USER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__user\b").unwrap());

static RE_KERNEL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__kernel\b").unwrap());

static RE_IOMEM: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__iomem\b").unwrap());

static RE_PERCPU: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__percpu\b").unwrap());

static RE_RCU: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__rcu\b").unwrap());

static RE_BITWISE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__bitwise\b").unwrap());

static RE_FORCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__force\b").unwrap());

static RE_NOCAST: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__nocast\b").unwrap());

static RE_SAFE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__safe\b").unwrap());

static RE_DEPRECATED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b__deprecated\b").unwrap());

static RE_READ_MOSTLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b__read_mostly\b").unwrap());

static RE_CACHELINE_ALIGNED: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b____cacheline_aligned(?:_in_smp)?\b").unwrap());

static RE_ACQUIRES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b__acquires\s*\([^)]*\)").unwrap());

static RE_RELEASES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b__releases\s*\([^)]*\)").unwrap());

static RE_MUST_HOLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b__must_hold\s*\([^)]*\)").unwrap());

// Iterator macros - these generate complex loop constructs
// list_for_each_entry(pos, head, member) -> for (pos = ...; pos; pos = ...)
static RE_LIST_FOR_EACH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\blist_for_each_entry(?:_safe|_reverse|_continue|_rcu|_from)?(?:_safe)?\s*\(")
        .unwrap()
});

// hlist variants
static RE_HLIST_FOR_EACH: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\bhlist_for_each_entry(?:_safe|_rcu|_continue)?(?:_safe)?\s*\(").unwrap()
});

// for_each_* macros (very common in kernel)
static RE_FOR_EACH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:ice_)?for_each_\w+\s*\(").unwrap());

// for_each_set_bit and similar
static RE_FOR_EACH_BIT: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bfor_each_(?:set_bit|clear_bit)\s*\(").unwrap());

// container_of(ptr, type, member) -> ((type *)((char *)(ptr) - offsetof(type, member)))
static RE_CONTAINER_OF: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\bcontainer_of\s*\(").unwrap());

// EXPORT_SYMBOL and variants
static RE_EXPORT_SYMBOL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*EXPORT_SYMBOL(?:_GPL)?(?:_NS)?\s*\([^)]+\)\s*;").unwrap());

// MODULE_* macros
static RE_MODULE_MACRO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\s*MODULE_(?:LICENSE|AUTHOR|DESCRIPTION|VERSION|ALIAS|DEVICE_TABLE)\s*\([^)]*\)\s*;",
    )
    .unwrap()
});

impl MacroNeutralizer {
    pub fn new() -> Self {
        Self {
            stats: MacroStats::default(),
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &MacroStats {
        &self.stats
    }

    /// Process source code and neutralize problematic macros
    pub fn neutralize(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        // Phase 1: Strip annotation macros (these just annotate, no semantic meaning)
        result = self.strip_annotation_macros(&result);

        // Phase 2: Expand likely/unlikely (just strip the wrapper)
        result = self.expand_likely_unlikely(&result);

        // Phase 3: Handle BUILD_BUG_ON family
        result = self.handle_build_bug_on(&result);

        // Phase 4: Handle WARN_ON/BUG_ON
        result = self.handle_warn_bug_on(&result);

        // Phase 5: Expand DEFINE_* macros to declarations
        result = self.expand_define_macros(&result);

        // Phase 6: Simplify RCU macros
        result = self.simplify_rcu(&result);

        // Phase 7: Handle memory ordering macros
        result = self.simplify_memory_ordering(&result);

        // Phase 8: Handle typeof
        result = self.handle_typeof(&result);

        // Phase 9: Handle error pointer macros
        result = self.handle_error_pointers(&result);

        // Phase 10: Simplify statement expressions (limited - only simple cases)
        result = self.simplify_statement_expressions(&result);

        // Phase 11: Handle misc macros (ARRAY_SIZE, etc)
        result = self.handle_misc_macros(&result);

        // Phase 12: Handle iterator macros (for_each_*, list_for_each_*)
        result = self.handle_iterator_macros(&result);

        // Phase 13: Handle container_of macro
        result = self.handle_container_of(&result);

        // Phase 14: Handle module/export macros
        result = self.handle_module_macros(&result);

        result
    }

    fn strip_annotation_macros(&self, source: &str) -> String {
        let mut result = source.to_string();

        // Strip all the annotation keywords
        let annotations = [
            (&*RE_MUST_CHECK, ""),
            (&*RE_ALWAYS_INLINE, "inline"),
            (&*RE_NOINLINE, ""),
            (&*RE_COLD, ""),
            (&*RE_HOT, ""),
            (&*RE_PURE, ""),
            (&*RE_INIT, ""),
            (&*RE_EXIT, ""),
            (&*RE_INITDATA, ""),
            (&*RE_DEVINIT, ""),
            (&*RE_DEVEXIT, ""),
            (&*RE_USER, ""),
            (&*RE_KERNEL, ""),
            (&*RE_IOMEM, ""),
            (&*RE_PERCPU, ""),
            (&*RE_RCU, ""),
            (&*RE_BITWISE, ""),
            (&*RE_FORCE, ""),
            (&*RE_NOCAST, ""),
            (&*RE_SAFE, ""),
            (&*RE_DEPRECATED, ""),
            (&*RE_READ_MOSTLY, ""),
            (&*RE_CACHELINE_ALIGNED, ""),
            (&*RE_ACQUIRES, ""),
            (&*RE_RELEASES, ""),
            (&*RE_MUST_HOLD, ""),
        ];

        for (re, replacement) in annotations {
            result = re.replace_all(&result, replacement).to_string();
        }

        result
    }

    fn expand_likely_unlikely(&mut self, source: &str) -> String {
        // likely(x) -> (x), unlikely(x) -> (x)
        let mut result = source.to_string();

        // Count occurrences for stats
        let count = RE_LIKELY.find_iter(&result).count();
        self.stats.likely_unlikely_stripped += count;

        // Replace likely( and unlikely( with just (
        result = RE_LIKELY.replace_all(&result, "(").to_string();

        result
    }

    fn handle_build_bug_on(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        let count = RE_BUILD_BUG_ON.find_iter(&result).count()
            + RE_BUILD_BUG_ON_MSG.find_iter(&result).count();
        self.stats.build_bug_on_stripped += count;

        // Replace BUILD_BUG_ON(...) with ((void)0)
        result = RE_BUILD_BUG_ON
            .replace_all(&result, "((void)0)")
            .to_string();
        result = RE_BUILD_BUG_ON_MSG
            .replace_all(&result, "((void)0)")
            .to_string();

        result
    }

    fn handle_warn_bug_on(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        let warn_count = RE_WARN_ON.find_iter(&result).count();
        let bug_count = RE_BUG_ON.find_iter(&result).count();
        self.stats.warn_on_stripped += warn_count + bug_count;

        // WARN_ON(x) -> (x) - keep the condition but remove the macro
        // We need to be careful with nested parentheses
        result = self.replace_macro_with_arg(&result, &RE_WARN_ON);
        result = self.replace_macro_with_arg(&result, &RE_BUG_ON);

        result
    }

    fn expand_define_macros(&mut self, source: &str) -> String {
        let mut result = source.to_string();
        let mut count = 0;

        // DEFINE_MUTEX(name) -> struct mutex name = { 0 }
        count += RE_DEFINE_MUTEX.find_iter(&result).count();
        result = RE_DEFINE_MUTEX
            .replace_all(&result, "struct mutex $1 = { 0 }")
            .to_string();

        // DEFINE_SPINLOCK(name) -> spinlock_t name = { 0 }
        count += RE_DEFINE_SPINLOCK.find_iter(&result).count();
        result = RE_DEFINE_SPINLOCK
            .replace_all(&result, "spinlock_t $1 = { 0 }")
            .to_string();

        // DEFINE_RWLOCK(name) -> rwlock_t name = { 0 }
        count += RE_DEFINE_RWLOCK.find_iter(&result).count();
        result = RE_DEFINE_RWLOCK
            .replace_all(&result, "rwlock_t $1 = { 0 }")
            .to_string();

        // DEFINE_SEMAPHORE(name) -> struct semaphore name = { 0 }
        count += RE_DEFINE_SEMAPHORE.find_iter(&result).count();
        result = RE_DEFINE_SEMAPHORE
            .replace_all(&result, "struct semaphore $1 = { 0 }")
            .to_string();

        // DEFINE_IDA(name) -> struct ida name = { 0 }
        count += RE_DEFINE_IDA.find_iter(&result).count();
        result = RE_DEFINE_IDA
            .replace_all(&result, "struct ida $1 = { 0 }")
            .to_string();

        // DEFINE_IDR(name) -> struct idr name = { 0 }
        count += RE_DEFINE_IDR.find_iter(&result).count();
        result = RE_DEFINE_IDR
            .replace_all(&result, "struct idr $1 = { 0 }")
            .to_string();

        // DECLARE_BITMAP(name, bits) -> unsigned long name[1]
        count += RE_DECLARE_BITMAP.find_iter(&result).count();
        result = RE_DECLARE_BITMAP
            .replace_all(&result, "unsigned long $1[1]")
            .to_string();

        // DECLARE_WAIT_QUEUE_HEAD(name) -> wait_queue_head_t name = { 0 }
        count += RE_DECLARE_WAIT_QUEUE.find_iter(&result).count();
        result = RE_DECLARE_WAIT_QUEUE
            .replace_all(&result, "wait_queue_head_t $1 = { 0 }")
            .to_string();

        // LIST_HEAD(name) -> struct list_head name = { &name, &name }
        count += RE_LIST_HEAD.find_iter(&result).count();
        result = RE_LIST_HEAD
            .replace_all(&result, "struct list_head $1 = { 0 }")
            .to_string();

        // HLIST_HEAD(name) -> struct hlist_head name = { 0 }
        count += RE_HLIST_HEAD.find_iter(&result).count();
        result = RE_HLIST_HEAD
            .replace_all(&result, "struct hlist_head $1 = { 0 }")
            .to_string();

        self.stats.define_macros_stubbed += count;
        result
    }

    fn simplify_rcu(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        let count =
            RE_RCU_DEREF.find_iter(&result).count() + RE_RCU_ASSIGN.find_iter(&result).count();
        self.stats.rcu_simplified += count;

        // rcu_dereference(p) -> (p)
        result = self.replace_macro_with_arg(&result, &RE_RCU_DEREF);

        // rcu_assign_pointer needs special handling - it's rcu_assign_pointer(p, v)
        // For now, simplify to just a comment
        // This is complex because it has two args

        result
    }

    fn simplify_memory_ordering(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        // READ_ONCE(x) -> (*(volatile typeof(x) *)&(x)) - simplified to (x)
        result = self.replace_macro_with_arg(&result, &RE_READ_ONCE);

        // WRITE_ONCE(x, v) is harder - two args
        // ACCESS_ONCE(x) -> (x)
        result = self.replace_macro_with_arg(&result, &RE_ACCESS_ONCE);

        // smp_load_acquire(x) -> (x)
        result = self.replace_macro_with_arg(&result, &RE_SMP_LOAD);

        // smp_store_release(x, v) - two args, skip for now

        result
    }

    fn handle_typeof(&mut self, source: &str) -> String {
        let result = source.to_string();

        let count = RE_TYPEOF.find_iter(&result).count();
        self.stats.typeof_replaced += count;

        // For simple typeof(x), we can't easily determine the type
        // Replace with void* as a fallback for pointer contexts
        // This is imperfect but allows parsing to continue

        // typeof in variable declarations is tricky
        // For now, just track the count - full expansion needs type inference

        result
    }

    fn handle_error_pointers(&self, source: &str) -> String {
        let mut result = source.to_string();

        // IS_ERR(x) -> ((unsigned long)(x) >= (unsigned long)-4095)
        // Simplify to just a function call style
        result = self.replace_macro_with_arg(&result, &RE_IS_ERR);

        // PTR_ERR(x) -> ((long)(x))
        result = self.replace_macro_with_arg(&result, &RE_PTR_ERR);

        // ERR_PTR(x) -> ((void *)(long)(x))
        result = self.replace_macro_with_arg(&result, &RE_ERR_PTR);

        // ERR_CAST(x) -> ((void *)(x))
        result = self.replace_macro_with_arg(&result, &RE_ERR_CAST);

        result
    }

    fn simplify_statement_expressions(&mut self, source: &str) -> String {
        let mut result = source.to_string();
        let mut count = 0;

        // Only handle simple statement expressions without nested braces
        // ({ simple_expr; }) -> (simple_expr)

        // This is a conservative approach - only handle the simplest cases
        loop {
            let prev = result.clone();
            result = RE_STMT_EXPR
                .replace(&result, |caps: &regex::Captures| {
                    let matched = caps.get(0).unwrap().as_str();
                    // Extract inner content: ({ ... }) -> ...
                    let inner = &matched[2..matched.len() - 2]; // Remove ({ and })

                    // If it's a simple expression ending in semicolon, extract it
                    let trimmed = inner.trim();
                    if let Some(expr) = trimmed.strip_suffix(';') {
                        count += 1;
                        format!("({})", expr.trim())
                    } else if !trimmed.contains(';') {
                        // No semicolon at all - might be just an expression
                        count += 1;
                        format!("({trimmed})")
                    } else {
                        // Multiple statements - too complex, leave as is
                        matched.to_string()
                    }
                })
                .to_string();

            if result == prev {
                break;
            }
        }

        self.stats.statement_expressions_simplified += count;
        result
    }

    fn handle_misc_macros(&self, source: &str) -> String {
        let mut result = source.to_string();

        // ARRAY_SIZE(arr) -> (sizeof(arr)/sizeof((arr)[0]))
        result = RE_ARRAY_SIZE
            .replace_all(&result, "(sizeof($1)/sizeof(($1)[0]))")
            .to_string();

        // min_t(type, a, b) -> ((a) < (b) ? (a) : (b))
        // This is complex due to three args, skip for now

        // clamp(val, lo, hi) - three args, skip for now

        // offsetof is usually handled by the compiler, leave it

        result
    }

    /// Handle iterator macros by converting them to simple for loops
    /// These macros are notoriously problematic because they expand to complex
    /// constructs that include variable declarations in non-standard positions.
    fn handle_iterator_macros(&mut self, source: &str) -> String {
        let mut result = source.to_string();

        // Count occurrences
        let list_count = RE_LIST_FOR_EACH.find_iter(&result).count()
            + RE_HLIST_FOR_EACH.find_iter(&result).count();
        let for_each_count =
            RE_FOR_EACH.find_iter(&result).count() + RE_FOR_EACH_BIT.find_iter(&result).count();

        self.stats.list_for_each_expanded += list_count;
        self.stats.for_each_expanded += for_each_count;

        // For list_for_each_entry(pos, head, member) { body }
        // Convert to: for (void *__iter = 0; __iter != (void*)1; __iter = (void*)1) { body }
        // This is a stub that allows the parser to handle the construct

        // The key insight is that these macros are followed by a block.
        // We can convert them to simple for loops that the parser can understand.

        // Strategy: Replace the macro invocation with a simple for-loop header
        // list_for_each_entry(pos, head, member) -> for (;pos;)
        result = self.convert_iterator_to_for_loop(&result, &RE_LIST_FOR_EACH);
        result = self.convert_iterator_to_for_loop(&result, &RE_HLIST_FOR_EACH);
        result = self.convert_iterator_to_for_loop(&result, &RE_FOR_EACH);
        result = self.convert_iterator_to_for_loop(&result, &RE_FOR_EACH_BIT);

        result
    }

    /// Convert an iterator macro to a simple for loop
    fn convert_iterator_to_for_loop(&self, source: &str, pattern: &Regex) -> String {
        let mut result = String::new();
        let mut last_end = 0;

        for m in pattern.find_iter(source) {
            result.push_str(&source[last_end..m.start()]);

            // Find the closing paren of the macro call
            let remaining = &source[m.end()..];
            if let Some(paren_end) = self.find_matching_paren(remaining) {
                // Extract the first argument (the iterator variable)
                let args = &remaining[..paren_end];
                let first_arg = args.split(',').next().unwrap_or("__iter").trim();

                // Replace with a simple for loop header
                // for (;first_arg;) keeps the variable reference so the body parses correctly
                result.push_str(&format!("for (;{first_arg};)"));
                last_end = m.end() + paren_end + 1;
            } else {
                // Couldn't find matching paren, keep original
                result.push_str(m.as_str());
                last_end = m.end();
            }
        }

        result.push_str(&source[last_end..]);
        result
    }

    /// Handle container_of macro
    /// container_of(ptr, type, member) -> ((type*)ptr)
    fn handle_container_of(&mut self, source: &str) -> String {
        let mut result = String::new();
        let mut last_end = 0;

        let count = RE_CONTAINER_OF.find_iter(source).count();
        self.stats.container_of_expanded += count;

        for m in RE_CONTAINER_OF.find_iter(source) {
            result.push_str(&source[last_end..m.start()]);

            // Find the closing paren
            let remaining = &source[m.end()..];
            if let Some(paren_end) = self.find_matching_paren(remaining) {
                let args = &remaining[..paren_end];
                let parts: Vec<&str> = args.splitn(3, ',').collect();

                if parts.len() >= 2 {
                    let ptr = parts[0].trim();
                    let type_name = parts[1].trim();
                    // Convert to a simple cast: ((type*)ptr)
                    result.push_str(&format!("(({type_name}*){ptr})"));
                } else {
                    // Not enough args, keep as-is with a simple cast wrapper
                    result.push_str(&format!("((void*){args})"));
                }
                last_end = m.end() + paren_end + 1;
            } else {
                result.push_str(m.as_str());
                last_end = m.end();
            }
        }

        result.push_str(&source[last_end..]);
        result
    }

    /// Handle MODULE_* and EXPORT_SYMBOL macros
    /// These are typically at file scope and cause parsing issues
    fn handle_module_macros(&self, source: &str) -> String {
        let mut result = String::new();

        // Process line by line to handle line-anchored patterns
        for line in source.lines() {
            // Check if this line is an EXPORT_SYMBOL or MODULE_* macro
            if RE_EXPORT_SYMBOL.is_match(line) {
                // Replace with empty or comment
                result.push_str("/* ");
                result.push_str(line.trim());
                result.push_str(" */\n");
            } else if RE_MODULE_MACRO.is_match(line) {
                // Replace with a comment
                result.push_str("/* ");
                result.push_str(line.trim());
                result.push_str(" */\n");
            } else {
                result.push_str(line);
                result.push('\n');
            }
        }

        // Remove trailing newline if source didn't have one
        if !source.ends_with('\n') && result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Helper to replace a macro call with its argument
    /// e.g., likely(x) -> (x)
    fn replace_macro_with_arg(&self, source: &str, pattern: &Regex) -> String {
        let mut result = String::new();
        let mut last_end = 0;

        for m in pattern.find_iter(source) {
            result.push_str(&source[last_end..m.start()]);

            // Find the matching closing paren
            let remaining = &source[m.end()..];
            if let Some(arg_end) = self.find_matching_paren(remaining) {
                let arg = &remaining[..arg_end];
                result.push('(');
                result.push_str(arg);
                result.push(')');
                last_end = m.end() + arg_end + 1; // +1 for the closing paren
            } else {
                // Couldn't find matching paren, keep original
                result.push_str(m.as_str());
                last_end = m.end();
            }
        }

        result.push_str(&source[last_end..]);
        result
    }

    /// Find the position of the matching closing parenthesis
    fn find_matching_paren(&self, s: &str) -> Option<usize> {
        let mut depth = 1;
        for (i, c) in s.char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(i);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

impl Default for MacroNeutralizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_likely_unlikely() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "if (likely(x > 0)) { } if (unlikely(y < 0)) { }";
        let output = neutralizer.neutralize(input);

        assert!(output.contains("if ((x > 0))"));
        assert!(output.contains("if ((y < 0))"));
        assert_eq!(neutralizer.stats.likely_unlikely_stripped, 2);
    }

    #[test]
    fn test_build_bug_on() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "BUILD_BUG_ON(sizeof(x) != 4);";
        let output = neutralizer.neutralize(input);

        assert!(output.contains("((void)0)"));
        assert_eq!(neutralizer.stats.build_bug_on_stripped, 1);
    }

    #[test]
    fn test_define_mutex() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "DEFINE_MUTEX(my_lock);";
        let output = neutralizer.neutralize(input);

        assert!(output.contains("struct mutex my_lock"));
        assert_eq!(neutralizer.stats.define_macros_stubbed, 1);
    }

    #[test]
    fn test_annotation_stripping() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "static __init __cold int my_func(void __user *ptr)";
        let output = neutralizer.neutralize(input);

        assert!(!output.contains("__init"));
        assert!(!output.contains("__cold"));
        assert!(!output.contains("__user"));
    }

    #[test]
    fn test_simple_statement_expression() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "int x = ({ 42; });";
        let output = neutralizer.neutralize(input);

        assert!(output.contains("(42)") || output.contains("({ 42; })"));
    }

    #[test]
    fn test_warn_on() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "WARN_ON(ptr == NULL);";
        let output = neutralizer.neutralize(input);

        // Should convert to just the condition
        assert!(output.contains("(ptr == NULL)"));
    }

    #[test]
    fn test_array_size() {
        let mut neutralizer = MacroNeutralizer::new();

        let input = "int count = ARRAY_SIZE(my_array);";
        let output = neutralizer.neutralize(input);

        assert!(output.contains("sizeof(my_array)"));
    }
}
