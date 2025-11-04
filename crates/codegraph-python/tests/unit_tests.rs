//! Comprehensive unit tests for Python parser
//! Aiming for ~90% test coverage

use codegraph_python::error::ParseError;
use codegraph_python::entities::{ClassEntity, Field, FunctionEntity, ModuleEntity, Parameter};
use codegraph_python::relationships::{CallRelation, ImportEntity, InheritanceRelation, ImplementationRelation};
use codegraph_python::config::ParserConfig;
use std::path::PathBuf;

// ====================
// Error Tests
// ====================

mod error_tests {
    use super::*;
    use std::io;

    #[test]
    fn test_io_error_creation() {
        let path = PathBuf::from("test.py");
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let err = ParseError::io_error(path.clone(), io_err);

        match err {
            ParseError::IoError { path: p, .. } => {
                assert_eq!(p, path);
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_file_too_large_creation() {
        let err = ParseError::file_too_large("large.py", 1000, 2000);

        match err {
            ParseError::FileTooLarge { path, max_size, actual_size } => {
                assert_eq!(path, PathBuf::from("large.py"));
                assert_eq!(max_size, 1000);
                assert_eq!(actual_size, 2000);
            }
            _ => panic!("Expected FileTooLarge"),
        }
    }

    #[test]
    fn test_syntax_error_creation() {
        let err = ParseError::syntax_error("test.py", 10, 5, "unexpected token");

        match err {
            ParseError::SyntaxError { file, line, column, message } => {
                assert_eq!(file, "test.py");
                assert_eq!(line, 10);
                assert_eq!(column, 5);
                assert_eq!(message, "unexpected token");
            }
            _ => panic!("Expected SyntaxError"),
        }
    }

    #[test]
    fn test_graph_error_creation() {
        let err = ParseError::graph_error("node insertion failed");

        match err {
            ParseError::GraphError(msg) => {
                assert_eq!(msg, "node insertion failed");
            }
            _ => panic!("Expected GraphError"),
        }
    }

    #[test]
    fn test_invalid_config_creation() {
        let err = ParseError::invalid_config("max_file_size must be positive");

        match err {
            ParseError::InvalidConfig(msg) => {
                assert_eq!(msg, "max_file_size must be positive");
            }
            _ => panic!("Expected InvalidConfig"),
        }
    }

    #[test]
    fn test_unsupported_feature_creation() {
        let err = ParseError::unsupported_feature("test.py", "walrus operator");

        match err {
            ParseError::UnsupportedFeature { file, feature } => {
                assert_eq!(file, "test.py");
                assert_eq!(feature, "walrus operator");
            }
            _ => panic!("Expected UnsupportedFeature"),
        }
    }

    #[test]
    fn test_error_display() {
        let err = ParseError::syntax_error("test.py", 10, 5, "unexpected EOF");
        let display = format!("{}", err);
        assert!(display.contains("test.py"));
        assert!(display.contains("10"));
        assert!(display.contains("5"));
        assert!(display.contains("unexpected EOF"));
    }
}

// ====================
// Config Tests
// ====================

mod config_tests {
    use super::*;

    #[test]
    fn test_config_default_values() {
        let config = ParserConfig::default();
        assert!(config.include_private);
        assert!(config.include_tests);
        assert!(config.parse_docs);
        assert_eq!(config.max_file_size, 10 * 1024 * 1024);
        assert!(!config.parallel);
        assert!(config.num_threads.is_none());
        assert!(config.file_extensions.contains(&".py".to_string()));
    }

    #[test]
    fn test_config_validation_success() {
        let config = ParserConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_zero_threads() {
        let mut config = ParserConfig::default();
        config.num_threads = Some(0);
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("num_threads"));
    }

    #[test]
    fn test_config_validation_zero_file_size() {
        let mut config = ParserConfig::default();
        config.max_file_size = 0;
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("max_file_size"));
    }

    #[test]
    fn test_config_validation_empty_extensions() {
        let mut config = ParserConfig::default();
        config.file_extensions.clear();
        assert!(config.validate().is_err());
        assert!(config.validate().unwrap_err().contains("file_extensions"));
    }

    #[test]
    fn test_should_parse_extension_with_dot() {
        let config = ParserConfig::default();
        assert!(config.should_parse_extension(".py"));
    }

    #[test]
    fn test_should_parse_extension_without_dot() {
        let config = ParserConfig::default();
        assert!(config.should_parse_extension("py"));
    }

    #[test]
    fn test_should_parse_extension_pyw() {
        let mut config = ParserConfig::default();
        config.file_extensions.push(".pyw".to_string());
        assert!(config.should_parse_extension(".pyw"));
        assert!(config.should_parse_extension("pyw"));
    }

    #[test]
    fn test_should_not_parse_wrong_extension() {
        let config = ParserConfig::default();
        assert!(!config.should_parse_extension(".rs"));
        assert!(!config.should_parse_extension(".txt"));
    }

    #[test]
    fn test_should_exclude_pycache() {
        let config = ParserConfig::default();
        assert!(config.should_exclude_dir("__pycache__"));
    }

    #[test]
    fn test_should_exclude_venv() {
        let config = ParserConfig::default();
        assert!(config.should_exclude_dir(".venv"));
        assert!(config.should_exclude_dir("venv"));
        assert!(config.should_exclude_dir("env"));
    }

    #[test]
    fn test_should_exclude_build_dirs() {
        let config = ParserConfig::default();
        assert!(config.should_exclude_dir("dist"));
        assert!(config.should_exclude_dir("build"));
    }

    #[test]
    fn test_should_exclude_glob_pattern() {
        let config = ParserConfig::default();
        assert!(config.should_exclude_dir("mypackage.egg-info"));
        assert!(config.should_exclude_dir("foo.egg-info"));
    }

    #[test]
    fn test_should_not_exclude_regular_dir() {
        let config = ParserConfig::default();
        assert!(!config.should_exclude_dir("src"));
        assert!(!config.should_exclude_dir("tests"));
        assert!(!config.should_exclude_dir("mypackage"));
    }

    #[test]
    fn test_config_with_custom_extensions() {
        let mut config = ParserConfig::default();
        config.file_extensions = vec![".py".to_string(), ".pyw".to_string(), ".pyi".to_string()];

        assert!(config.should_parse_extension(".py"));
        assert!(config.should_parse_extension(".pyw"));
        assert!(config.should_parse_extension(".pyi"));
    }

    #[test]
    fn test_config_with_custom_excludes() {
        let mut config = ParserConfig::default();
        config.exclude_dirs.push("node_modules".to_string());

        assert!(config.should_exclude_dir("node_modules"));
    }

    #[test]
    fn test_config_parallel_settings() {
        let mut config = ParserConfig::default();
        config.parallel = true;
        config.num_threads = Some(4);

        assert!(config.parallel);
        assert_eq!(config.num_threads, Some(4));
        assert!(config.validate().is_ok());
    }
}

// ====================
// Entity Tests
// ====================

mod entity_tests {
    use super::*;

    // Parameter tests
    #[test]
    fn test_parameter_builder() {
        let param = Parameter::new("arg")
            .with_type("str")
            .with_default("\"hello\"");

        assert_eq!(param.name, "arg");
        assert_eq!(param.type_annotation, Some("str".to_string()));
        assert_eq!(param.default_value, Some("\"hello\"".to_string()));
    }

    #[test]
    fn test_parameter_no_type() {
        let param = Parameter::new("arg");
        assert!(param.type_annotation.is_none());
        assert!(param.default_value.is_none());
    }

    // Field tests
    #[test]
    fn test_field_public() {
        let field = Field::new("name");
        assert_eq!(field.visibility, "public");
        assert!(!field.is_static);
    }

    #[test]
    fn test_field_protected() {
        let field = Field::new("_name");
        assert_eq!(field.visibility, "protected");
    }

    #[test]
    fn test_field_private() {
        let field = Field::new("__name");
        assert_eq!(field.visibility, "private");
    }

    #[test]
    fn test_field_dunder_public() {
        let field = Field::new("__init__");
        assert_eq!(field.visibility, "public");
    }

    #[test]
    fn test_field_with_type() {
        let field = Field::new("age").set_type_annotation(Some("int".to_string()));
        assert_eq!(field.type_annotation, Some("int".to_string()));
    }

    #[test]
    fn test_field_static() {
        let field = Field::new("counter").set_static(true);
        assert!(field.is_static);
    }

    // Function tests
    #[test]
    fn test_function_test_detection() {
        assert!(FunctionEntity::new("test_foo", 1, 10).is_test);
        assert!(FunctionEntity::new("TestFoo", 1, 10).is_test);
        assert!(!FunctionEntity::new("foo_test", 1, 10).is_test);
    }

    #[test]
    fn test_function_async() {
        let func = FunctionEntity::new("fetch", 1, 10).set_async(true);
        assert!(func.is_async);
    }

    #[test]
    fn test_function_with_return_type() {
        let func = FunctionEntity::new("get_name", 1, 5)
            .set_return_type(Some("str".to_string()));
        assert_eq!(func.return_type, Some("str".to_string()));
    }

    #[test]
    fn test_function_with_docstring() {
        let func = FunctionEntity::new("foo", 1, 5)
            .set_doc_comment(Some("This is a function".to_string()));
        assert_eq!(func.doc_comment, Some("This is a function".to_string()));
    }

    #[test]
    fn test_function_with_parent() {
        let func = FunctionEntity::new("method", 1, 5)
            .set_parent(Some("MyClass".to_string()));
        assert!(func.is_method());
        assert_eq!(func.parent, Some("MyClass".to_string()));
    }

    #[test]
    fn test_function_decorators() {
        let func = FunctionEntity::new("prop", 1, 5)
            .add_attribute("@property");
        assert!(func.is_property());
        assert!(!func.is_static());
        assert!(!func.is_classmethod());
    }

    #[test]
    fn test_function_staticmethod() {
        let func = FunctionEntity::new("util", 1, 5)
            .add_attribute("@staticmethod");
        assert!(func.is_static());
    }

    #[test]
    fn test_function_classmethod() {
        let func = FunctionEntity::new("create", 1, 5)
            .add_attribute("@classmethod");
        assert!(func.is_classmethod());
    }

    #[test]
    fn test_function_with_parameters() {
        let param1 = Parameter::new("a");
        let param2 = Parameter::with_type("b", "int");

        let func = FunctionEntity::new("add", 1, 5)
            .add_parameter(param1)
            .add_parameter(param2);

        assert_eq!(func.parameters.len(), 2);
    }

    #[test]
    fn test_function_custom_signature() {
        let func = FunctionEntity::new("foo", 1, 5)
            .set_signature("def foo(a: int, b: str) -> bool");
        assert_eq!(func.signature, "def foo(a: int, b: str) -> bool");
    }

    // Class tests
    #[test]
    fn test_class_visibility() {
        let public = ClassEntity::new("MyClass", 1, 10);
        assert_eq!(public.visibility, "public");

        let private = ClassEntity::new("_InternalClass", 1, 10);
        assert_eq!(private.visibility, "private");
    }

    #[test]
    fn test_class_is_dataclass() {
        let class = ClassEntity::new("User", 1, 10)
            .add_attribute("@dataclass");
        assert!(class.is_dataclass());
    }

    #[test]
    fn test_class_is_abstract_explicit() {
        let class = ClassEntity::new("Base", 1, 10).set_abstract(true);
        assert!(class.is_abstract());
    }

    #[test]
    fn test_class_is_abstract_from_abc() {
        let class1 = ClassEntity::new("Base", 1, 10).add_base_class("ABC");
        assert!(class1.is_abstract());

        let class2 = ClassEntity::new("Base", 1, 10).add_base_class("ABCMeta");
        assert!(class2.is_abstract());
    }

    #[test]
    fn test_class_is_abstract_from_decorator() {
        let class = ClassEntity::new("Base", 1, 10)
            .add_attribute("@abstractmethod");
        assert!(class.is_abstract());
    }

    #[test]
    fn test_class_with_bases() {
        let class = ClassEntity::new("Child", 1, 10)
            .add_base_class("Parent1")
            .add_base_class("Parent2");

        assert_eq!(class.base_classes.len(), 2);
        assert!(class.base_classes.contains(&"Parent1".to_string()));
        assert!(class.base_classes.contains(&"Parent2".to_string()));
    }

    #[test]
    fn test_class_with_traits() {
        let class = ClassEntity::new("MyClass", 1, 10)
            .add_trait("Serializable")
            .add_trait("Comparable");

        assert_eq!(class.implemented_traits.len(), 2);
    }

    #[test]
    fn test_class_with_methods() {
        let method1 = FunctionEntity::new("foo", 2, 5);
        let method2 = FunctionEntity::new("bar", 6, 9);

        let class = ClassEntity::new("MyClass", 1, 10)
            .add_method(method1)
            .add_method(method2);

        assert_eq!(class.methods.len(), 2);
        assert!(class.has_method("foo"));
        assert!(class.has_method("bar"));
        assert!(!class.has_method("baz"));
    }

    #[test]
    fn test_class_with_fields() {
        let field1 = Field::new("name");
        let field2 = Field::new("age");

        let class = ClassEntity::new("Person", 1, 10)
            .add_field(field1)
            .add_field(field2);

        assert_eq!(class.fields.len(), 2);
    }

    #[test]
    fn test_class_with_docstring() {
        let class = ClassEntity::new("Foo", 1, 10)
            .set_doc_comment(Some("A test class".to_string()));
        assert_eq!(class.doc_comment, Some("A test class".to_string()));
    }

    // Module tests
    #[test]
    fn test_module_creation() {
        let module = ModuleEntity::new("test", "/path/to/test.py", "python");
        assert_eq!(module.name, "test");
        assert_eq!(module.path, "/path/to/test.py");
        assert_eq!(module.language, "python");
    }

    #[test]
    fn test_module_with_docstring() {
        let module = ModuleEntity::new("test", "/path/to/test.py", "python")
            .set_doc(Some("Module docstring".to_string()));
        assert_eq!(module.doc, Some("Module docstring".to_string()));
    }

    #[test]
    fn test_module_line_count() {
        let module = ModuleEntity::new("test", "/path/to/test.py", "python")
            .set_line_count(100);
        assert_eq!(module.line_count(), 100);
    }
}

// ====================
// Relationship Tests
// ====================

mod relationship_tests {
    use super::*;

    #[test]
    fn test_call_relation_function_call() {
        let call = CallRelation::new("foo", "bar", 10);
        assert_eq!(call.caller, "foo");
        assert_eq!(call.callee, "bar");
        assert_eq!(call.line, 10);
        assert!(!call.is_method_call);
    }

    #[test]
    fn test_call_relation_method_call() {
        let call = CallRelation::method_call("foo", "obj.method", 20);
        assert!(call.is_method_call);
    }

    #[test]
    fn test_call_relation_set_method_call() {
        let call = CallRelation::new("foo", "bar", 10)
            .set_method_call(true);
        assert!(call.is_method_call);
    }

    #[test]
    fn test_import_entity_simple() {
        let import = ImportEntity::new("os");
        assert_eq!(import.module, "os");
        assert!(import.names.is_empty());
        assert!(import.alias.is_none());
        assert!(!import.is_from_import);
    }

    #[test]
    fn test_import_entity_with_alias() {
        let import = ImportEntity::new("numpy")
            .set_alias(Some("np".to_string()));
        assert_eq!(import.alias, Some("np".to_string()));
    }

    #[test]
    fn test_import_entity_from_import() {
        let import = ImportEntity::from_import("pathlib", vec!["Path".to_string()]);
        assert_eq!(import.module, "pathlib");
        assert_eq!(import.names, vec!["Path".to_string()]);
        assert!(import.is_from_import);
    }

    #[test]
    fn test_import_entity_add_name() {
        let import = ImportEntity::new("os")
            .add_name("path")
            .add_name("environ");
        assert_eq!(import.names.len(), 2);
    }

    #[test]
    fn test_inheritance_relation() {
        let inherit = InheritanceRelation::new("Child", "Parent");
        assert_eq!(inherit.child, "Child");
        assert_eq!(inherit.parent, "Parent");
        assert_eq!(inherit.order, 0);
    }

    #[test]
    fn test_inheritance_relation_with_order() {
        let inherit = InheritanceRelation::new("Child", "Parent")
            .set_order(1);
        assert_eq!(inherit.order, 1);
    }

    #[test]
    fn test_inheritance_multiple_parents() {
        let inherit1 = InheritanceRelation::new("Child", "Parent1").set_order(0);
        let inherit2 = InheritanceRelation::new("Child", "Parent2").set_order(1);

        assert_eq!(inherit1.order, 0);
        assert_eq!(inherit2.order, 1);
        assert_eq!(inherit1.child, inherit2.child);
    }

    #[test]
    fn test_implementation_relation() {
        let impl_rel = ImplementationRelation::new("MyClass", "Serializable");
        assert_eq!(impl_rel.implementor, "MyClass");
        assert_eq!(impl_rel.trait_name, "Serializable");
    }
}
