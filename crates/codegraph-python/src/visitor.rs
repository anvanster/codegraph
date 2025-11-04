use rustpython_ast::{Expr, Stmt, StmtAsyncFunctionDef, StmtClassDef, StmtFunctionDef};

/// Context for tracking the current scope during AST traversal
#[derive(Debug, Clone, Default)]
pub struct VisitorContext {
    /// Current class name (if we're inside a class)
    pub current_class: Option<String>,
    
    /// Current function name (if we're inside a function)
    pub current_function: Option<String>,
    
    /// Nesting depth (for tracking nested definitions)
    pub depth: usize,
}

impl VisitorContext {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Enter a class scope
    pub fn enter_class(&mut self, class_name: String) {
        self.current_class = Some(class_name);
        self.depth += 1;
    }
    
    /// Exit a class scope
    pub fn exit_class(&mut self) {
        self.current_class = None;
        if self.depth > 0 {
            self.depth -= 1;
        }
    }
    
    /// Enter a function scope
    pub fn enter_function(&mut self, func_name: String) {
        self.current_function = Some(func_name);
        self.depth += 1;
    }
    
    /// Exit a function scope
    pub fn exit_function(&mut self) {
        self.current_function = None;
        if self.depth > 0 {
            self.depth -= 1;
        }
    }
    
    /// Check if we're at top level (not inside any definition)
    pub fn is_top_level(&self) -> bool {
        self.depth == 0
    }
}

/// Extract the first docstring from a list of statements
pub fn extract_docstring(body: &[Stmt]) -> Option<String> {
    if let Some(first_stmt) = body.first() {
        if let Stmt::Expr(expr_stmt) = first_stmt {
            if let Expr::StringLiteral(string_lit) = &*expr_stmt.value {
                // Get the first string value
                if let Some(string_part) = string_lit.value.first() {
                    return Some(string_part.to_str().to_string());
                }
            }
        }
    }
    None
}

/// Convert decorator expression to string representation
pub fn decorator_to_string(decorator: &Expr) -> String {
    match decorator {
        Expr::Name(name) => format!("@{}", name.id),
        Expr::Call(call) => {
            // Handle decorator with arguments like @decorator(arg)
            if let Expr::Name(name) = &*call.func {
                format!("@{}", name.id)
            } else {
                "@decorator".to_string()
            }
        }
        Expr::Attribute(attr) => {
            // Handle module.decorator like @staticmethod
            format!("@{}", attr.attr)
        }
        _ => "@decorator".to_string(),
    }
}

/// Extract parameter default value as string
pub fn default_to_string(expr: &Expr) -> String {
    match expr {
        Expr::NumberLiteral(num) => format!("{}", num.value),
        Expr::StringLiteral(s) => {
            if let Some(string_part) = s.value.first() {
                format!("\"{}\"", string_part.to_str())
            } else {
                "\"\"".to_string()
            }
        }
        Expr::BooleanLiteral(b) => {
            if b.value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Expr::NoneLiteral(_) => "None".to_string(),
        Expr::Name(name) => name.id.to_string(),
        _ => "...".to_string(),
    }
}

/// Extract type annotation as string
pub fn type_to_string(expr: &Expr) -> String {
    match expr {
        Expr::Name(name) => name.id.to_string(),
        Expr::Subscript(subscript) => {
            // Handle generic types like List[str]
            let base = type_to_string(&subscript.value);
            let slice = type_to_string(&subscript.slice);
            format!("{}[{}]", base, slice)
        }
        Expr::Attribute(attr) => {
            // Handle module.Type like typing.List
            format!("{}.{}", type_to_string(&attr.value), attr.attr)
        }
        Expr::Tuple(tuple) => {
            // Handle tuple types
            let types: Vec<String> = tuple.elts.iter().map(|e| type_to_string(e)).collect();
            format!("({})", types.join(", "))
        }
        Expr::StringLiteral(s) => {
            // Handle string annotations (forward references)
            if let Some(string_part) = s.value.first() {
                string_part.to_str().to_string()
            } else {
                "str".to_string()
            }
        }
        _ => "Any".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visitor_context() {
        let mut ctx = VisitorContext::new();
        assert!(ctx.is_top_level());
        
        ctx.enter_class("MyClass".to_string());
        assert!(!ctx.is_top_level());
        assert_eq!(ctx.current_class, Some("MyClass".to_string()));
        
        ctx.exit_class();
        assert!(ctx.is_top_level());
        assert_eq!(ctx.current_class, None);
    }
    
    #[test]
    fn test_context_depth() {
        let mut ctx = VisitorContext::new();
        assert_eq!(ctx.depth, 0);
        
        ctx.enter_class("Outer".to_string());
        assert_eq!(ctx.depth, 1);
        
        ctx.enter_class("Inner".to_string());
        assert_eq!(ctx.depth, 2);
        
        ctx.exit_class();
        assert_eq!(ctx.depth, 1);
        
        ctx.exit_class();
        assert_eq!(ctx.depth, 0);
    }
}
