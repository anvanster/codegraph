// Temporary test file to explore rustpython-ast API
use rustpython_ast::Stmt;
use rustpython_parser::{ast, Parse};

#[test]
fn explore_ast_api() {
    let source = r#"
def foo(x: int, y: str = "default") -> str:
    """A test function."""
    return str(x) + y

class MyClass:
    """A test class."""
    def method(self):
        pass
"#;

    let result = ast::Suite::parse(source, "<test>");
    match result {
        Ok(ast) => {
            for stmt in &ast {
                match stmt {
                    Stmt::FunctionDef(f) => {
                        println!("Function: {}", f.name);
                        println!("Args struct fields:");
                        println!("  - args: {}", f.args.args.len());
                        println!("  - defaults count: {}", f.args.defaults().count());
                        println!(
                            "  Range start: {:?}, end: {:?}",
                            f.range.start(),
                            f.range.end()
                        );
                        // Let's see what's available
                    }
                    Stmt::ClassDef(c) => {
                        println!("Class: {}", c.name);
                        println!("  Range: {:?}", c.range);
                    }
                    _ => {}
                }
            }
        }
        Err(e) => {
            println!("Parse error: {e:?}");
            panic!("Parse failed");
        }
    }
}
