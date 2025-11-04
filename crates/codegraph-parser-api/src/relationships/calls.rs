use serde::{Deserialize, Serialize};

/// Represents a function call relationship
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CallRelation {
    /// Caller function name
    pub caller: String,

    /// Callee function name
    pub callee: String,

    /// Line number where the call occurs
    pub call_site_line: usize,

    /// Is this a direct call or indirect (e.g., through function pointer)?
    pub is_direct: bool,
}

impl CallRelation {
    pub fn new(caller: impl Into<String>, callee: impl Into<String>, line: usize) -> Self {
        Self {
            caller: caller.into(),
            callee: callee.into(),
            call_site_line: line,
            is_direct: true,
        }
    }

    pub fn indirect(mut self) -> Self {
        self.is_direct = false;
        self
    }
}
