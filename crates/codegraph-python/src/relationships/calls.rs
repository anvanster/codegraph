use serde::{Deserialize, Serialize};

/// Represents a function call from one function to another
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CallRelation {
    /// Name of the calling function
    pub caller: String,

    /// Name of the called function
    pub callee: String,

    /// Line number where the call occurs
    pub line: usize,

    /// Is this a method call (obj.method()) vs function call?
    pub is_method_call: bool,
}

impl CallRelation {
    /// Create a new function call relation
    pub fn new(caller: impl Into<String>, callee: impl Into<String>, line: usize) -> Self {
        Self {
            caller: caller.into(),
            callee: callee.into(),
            line,
            is_method_call: false,
        }
    }

    /// Create a method call relation
    pub fn method_call(caller: impl Into<String>, callee: impl Into<String>, line: usize) -> Self {
        Self {
            caller: caller.into(),
            callee: callee.into(),
            line,
            is_method_call: true,
        }
    }

    /// Set whether this is a method call
    pub fn set_method_call(mut self, is_method_call: bool) -> Self {
        self.is_method_call = is_method_call;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_relation_new() {
        let call = CallRelation::new("foo", "bar", 10);
        assert_eq!(call.caller, "foo");
        assert_eq!(call.callee, "bar");
        assert_eq!(call.line, 10);
        assert!(!call.is_method_call);
    }

    #[test]
    fn test_method_call() {
        let call = CallRelation::method_call("foo", "bar", 20);
        assert_eq!(call.caller, "foo");
        assert_eq!(call.callee, "bar");
        assert_eq!(call.line, 20);
        assert!(call.is_method_call);
    }
}
