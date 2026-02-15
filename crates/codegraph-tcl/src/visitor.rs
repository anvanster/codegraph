//! AST visitor for extracting Tcl entities
//!
//! tree-sitter-tcl parses most Tcl constructs as generic `command` nodes with
//! `name` and `arguments` fields. This visitor dispatches on the command name
//! to classify procedures, namespaces, imports, SDC constraints, and EDA commands.

use codegraph_parser_api::{
    CallRelation, ClassEntity, ComplexityMetrics, FunctionEntity, ImportRelation, Parameter,
    ParserConfig,
};
use tree_sitter::Node;

use crate::eda::{self, EdaCommand, EdaData};
use crate::sdc::{self, SdcData};

pub struct TclVisitor<'a> {
    pub source: &'a [u8],
    #[allow(dead_code)]
    pub config: ParserConfig,

    // Standard CodeIR entities
    pub functions: Vec<FunctionEntity>,
    pub classes: Vec<ClassEntity>,
    pub imports: Vec<ImportRelation>,
    pub calls: Vec<CallRelation>,

    // EDA/SDC data
    pub sdc_data: SdcData,
    pub eda_data: EdaData,

    // Context tracking
    namespace_stack: Vec<String>,
    current_procedure: Option<String>,
}

impl<'a> TclVisitor<'a> {
    pub fn new(source: &'a [u8], config: ParserConfig) -> Self {
        Self {
            source,
            config,
            functions: Vec::new(),
            classes: Vec::new(),
            imports: Vec::new(),
            calls: Vec::new(),
            sdc_data: SdcData::default(),
            eda_data: EdaData::default(),
            namespace_stack: Vec::new(),
            current_procedure: None,
        }
    }

    fn node_text(&self, node: Node) -> String {
        node.utf8_text(self.source).unwrap_or("").to_string()
    }

    fn current_namespace(&self) -> Option<String> {
        if self.namespace_stack.is_empty() {
            None
        } else {
            Some(self.namespace_stack.join("::"))
        }
    }

    fn qualified_name(&self, name: &str) -> String {
        match self.current_namespace() {
            Some(ns) => format!("{}::{}", ns, name),
            None => name.to_string(),
        }
    }

    pub fn visit_node(&mut self, node: Node) {
        match node.kind() {
            "source_file" => self.visit_children(node),
            "command" => self.visit_command(node),
            "procedure" => self.visit_procedure_node(node),
            "if" | "while" | "foreach" | "for" | "switch" | "try" | "catch" => {
                // These appear as proper named nodes in some parse paths
                self.record_call(node.kind(), node);
                self.visit_children(node);
            }
            "ERROR" => self.visit_error_node(node),
            _ => self.visit_children(node),
        }
    }

    fn visit_children(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    /// Handle proper `procedure` nodes (when grammar successfully parses a proc).
    /// Structure: procedure [ proc, ERROR[simple_word(name)], braced_word(params), arguments, braced_word(body) ]
    fn visit_procedure_node(&mut self, node: Node) {
        let mut name_str = String::new();
        let mut params_node: Option<Node> = None;
        let mut body_node: Option<Node> = None;

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "proc" => continue,
                "arguments" => continue,
                "ERROR" => {
                    // Usually contains the proc name
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        if (inner.kind() == "simple_word" || inner.kind() == "word")
                            && name_str.is_empty()
                        {
                            name_str = self.node_text(inner).trim().to_string();
                        }
                    }
                }
                "simple_word" | "word" if name_str.is_empty() => {
                    name_str = self.node_text(child).trim().to_string();
                }
                "braced_word" | "braced_word_simple" => {
                    if name_str.is_empty() {
                        continue;
                    } else if params_node.is_none() {
                        params_node = Some(child);
                    } else if body_node.is_none() {
                        body_node = Some(child);
                    }
                }
                _ => {}
            }
        }

        if name_str.is_empty() {
            return;
        }

        let qualified = self.qualified_name(&name_str);
        let params = match params_node {
            Some(pn) => self.extract_params_from_braced(pn),
            None => Vec::new(),
        };
        let doc_comment = self.extract_preceding_comment(node);
        let complexity = match body_node {
            Some(bn) => self.calculate_complexity(bn),
            None => ComplexityMetrics {
                cyclomatic_complexity: 1,
                branches: 0,
                loops: 0,
                logical_operators: 0,
                max_nesting_depth: 0,
                exception_handlers: 0,
                early_returns: 0,
            },
        };

        let param_str = params
            .iter()
            .map(|p| {
                if let Some(ref default) = p.default_value {
                    format!("{{{} {}}}", p.name, default)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let signature = format!("proc {} {{{}}} {{...}}", name_str, param_str);

        let mut func = FunctionEntity::new(
            &qualified,
            node.start_position().row + 1,
            node.end_position().row + 1,
        )
        .with_visibility("public")
        .with_signature(&signature);

        func.parameters = params;
        func.doc_comment = doc_comment;
        func.parent_class = self.current_namespace();
        func.complexity = Some(complexity);
        self.functions.push(func);

        let prev_proc = self.current_procedure.take();
        self.current_procedure = Some(qualified);
        if let Some(bn) = body_node {
            self.visit_braced_body(bn);
        }
        self.current_procedure = prev_proc;
    }

    /// Handle ERROR nodes which may contain proc/namespace/if/while/etc.
    /// The version-patched grammar (v15→v14) produces ERROR nodes for these
    /// Tcl keywords instead of structured AST nodes.
    /// Note: comments may precede the keyword child, so we scan all children.
    fn visit_error_node(&mut self, node: Node) {
        // Scan children for a keyword to determine what this ERROR node represents
        let mut cursor = node.walk();
        let keyword = node.children(&mut cursor).find(|c| {
            matches!(
                c.kind(),
                "proc"
                    | "namespace"
                    | "if"
                    | "elseif"
                    | "while"
                    | "foreach"
                    | "for"
                    | "switch"
                    | "try"
                    | "catch"
            )
        });

        if let Some(kw) = keyword {
            match kw.kind() {
                "proc" => {
                    self.visit_proc_error(node);
                    return;
                }
                "namespace" => {
                    self.visit_namespace_error(node);
                    return;
                }
                kw_name @ ("if" | "elseif" | "while" | "foreach" | "for" | "switch" | "try"
                | "catch") => {
                    self.record_call(kw_name, node);
                    let mut cursor2 = node.walk();
                    for child in node.children(&mut cursor2) {
                        if child.kind() == "braced_word" || child.kind() == "braced_word_simple" {
                            self.visit_braced_body(child);
                        }
                    }
                    return;
                }
                _ => {}
            }
        }
        // For unrecognized ERROR nodes, visit children to find nested commands
        self.visit_children(node);
    }

    /// Handle `proc name {params} {body}` parsed as ERROR node.
    /// Two structures observed:
    /// 1. ERROR [ proc, ERROR[simple_word(name), braced_word(params)], braced_word(body) ]
    /// 2. ERROR [ comment*, proc, simple_word(name), braced_word(params), braced_word(body) ]
    fn visit_proc_error(&mut self, node: Node) {
        let mut name_str = String::new();
        let mut params_node: Option<Node> = None;
        let mut body_node: Option<Node> = None;
        let mut found_proc = false;

        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        for child in &children {
            match child.kind() {
                "comment" => continue,
                "proc" => {
                    found_proc = true;
                    continue;
                }
                "command" if found_proc => {
                    // In some parse paths (with comments), the rest of the proc
                    // is parsed as a command: command [ simple_word(name), word_list [ braced_word(params), braced_word(body) ] ]
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if name_str.is_empty() {
                            name_str = self.node_text(name_node).trim().to_string();
                        }
                    }
                    if let Some(args_node) = child.child_by_field_name("arguments") {
                        let mut inner_cursor = args_node.walk();
                        for inner in args_node.children(&mut inner_cursor) {
                            if inner.kind() == "braced_word" || inner.kind() == "braced_word_simple"
                            {
                                if params_node.is_none() {
                                    params_node = Some(inner);
                                } else if body_node.is_none() {
                                    body_node = Some(inner);
                                }
                            }
                        }
                    }
                }
                "ERROR" if found_proc => {
                    // May contain name and params as nested children
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        match inner.kind() {
                            "simple_word" | "word" if name_str.is_empty() => {
                                name_str = self.node_text(inner).trim().to_string();
                            }
                            "braced_word" | "braced_word_simple" if params_node.is_none() => {
                                params_node = Some(inner);
                            }
                            _ => {}
                        }
                    }
                }
                "simple_word" | "word" if found_proc && name_str.is_empty() => {
                    name_str = self.node_text(*child).trim().to_string();
                }
                "braced_word" | "braced_word_simple" if found_proc => {
                    if name_str.is_empty() {
                        continue;
                    } else if params_node.is_none() {
                        params_node = Some(*child);
                    } else if body_node.is_none() {
                        body_node = Some(*child);
                    }
                }
                _ => {}
            }
        }

        if name_str.is_empty() {
            return;
        }

        let qualified = self.qualified_name(&name_str);

        let params = match params_node {
            Some(pn) => self.extract_params_from_braced(pn),
            None => Vec::new(),
        };

        // Extract doc comments: check both preceding siblings of the ERROR node
        // and comment children inside the ERROR node (before the proc keyword)
        let doc_comment = self.extract_preceding_comment(node).or_else(|| {
            let mut inner_comments = Vec::new();
            let mut c2 = node.walk();
            for child in node.children(&mut c2) {
                if child.kind() == "comment" {
                    inner_comments.push(self.node_text(child));
                } else if child.kind() == "proc" {
                    break;
                }
            }
            if inner_comments.is_empty() {
                None
            } else {
                Some(inner_comments.join("\n"))
            }
        });

        let complexity = match body_node {
            Some(bn) => self.calculate_complexity(bn),
            None => ComplexityMetrics {
                cyclomatic_complexity: 1,
                branches: 0,
                loops: 0,
                logical_operators: 0,
                max_nesting_depth: 0,
                exception_handlers: 0,
                early_returns: 0,
            },
        };

        let param_str = params
            .iter()
            .map(|p| {
                if let Some(ref default) = p.default_value {
                    format!("{{{} {}}}", p.name, default)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let signature = format!("proc {} {{{}}} {{...}}", name_str, param_str);

        let mut func = FunctionEntity::new(
            &qualified,
            node.start_position().row + 1,
            node.end_position().row + 1,
        )
        .with_visibility("public")
        .with_signature(&signature);

        func.parameters = params;
        func.doc_comment = doc_comment;
        func.parent_class = self.current_namespace();
        func.complexity = Some(complexity);

        self.functions.push(func);

        // Visit body for calls
        let prev_proc = self.current_procedure.take();
        self.current_procedure = Some(qualified);
        if let Some(bn) = body_node {
            self.visit_braced_body(bn);
        }
        self.current_procedure = prev_proc;
    }

    /// Handle `namespace eval name {body}` parsed as ERROR node.
    fn visit_namespace_error(&mut self, node: Node) {
        let mut found_eval = false;
        let mut ns_name = String::new();
        let mut body_node: Option<Node> = None;

        let mut cursor = node.walk();
        let children: Vec<Node> = node.children(&mut cursor).collect();

        for child in &children {
            match child.kind() {
                "namespace" => continue,
                "ERROR" => {
                    // May contain "eval" and name
                    let mut inner_cursor = child.walk();
                    for inner in child.children(&mut inner_cursor) {
                        let text = self.node_text(inner).trim().to_string();
                        if text == "eval" {
                            found_eval = true;
                        } else if found_eval && ns_name.is_empty() {
                            ns_name = text;
                        }
                    }
                }
                "simple_word" | "word" => {
                    let text = self.node_text(*child).trim().to_string();
                    if text == "eval" {
                        found_eval = true;
                    } else if found_eval && ns_name.is_empty() {
                        ns_name = text;
                    }
                }
                "braced_word" | "braced_word_simple" if found_eval && !ns_name.is_empty() => {
                    body_node = Some(*child);
                }
                _ => {}
            }
        }

        if !found_eval || ns_name.is_empty() {
            return;
        }

        self.namespace_stack.push(ns_name);
        let full_ns = self.current_namespace().unwrap_or_default();
        let doc_comment = self.extract_preceding_comment(node);

        let class = ClassEntity {
            name: full_ns,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment,
            attributes: vec!["namespace".to_string()],
            type_parameters: Vec::new(),
        };
        self.classes.push(class);

        if let Some(bn) = body_node {
            self.visit_braced_body(bn);
        }

        self.namespace_stack.pop();
    }

    fn visit_command(&mut self, node: Node) {
        let cmd_name = self.extract_command_name(node);
        if cmd_name.is_empty() {
            return;
        }

        match cmd_name.as_str() {
            "proc" => self.visit_proc_command(node),
            "namespace" => self.visit_namespace_command(node),
            "source" => self.visit_source_command(node),
            "package" => self.visit_package_command(node),
            "if" | "elseif" | "while" | "foreach" | "for" | "switch" => {
                // Record for complexity; visit body arguments for nested commands
                self.record_call(&cmd_name, node);
                self.visit_braced_bodies(node);
            }
            "try" | "catch" => {
                self.record_call(&cmd_name, node);
                self.visit_braced_bodies(node);
            }
            _ => self.visit_general_command(&cmd_name, node),
        }
    }

    fn visit_general_command(&mut self, cmd_name: &str, node: Node) {
        // 1. Check for SDC commands
        if sdc::is_sdc_command(cmd_name) {
            if let Some(constraint) = sdc::extract_sdc_constraint(cmd_name, node, self.source) {
                self.sdc_data.add(constraint);
            }
            self.record_call(cmd_name, node);
            return;
        }

        // 2. Check for EDA commands
        if eda::is_eda_command(cmd_name) {
            if let Some(eda_cmd) = eda::classify_eda_command(cmd_name, node, self.source) {
                match eda_cmd {
                    EdaCommand::DesignFileRead { file_type, path } => {
                        if !path.is_empty() {
                            self.imports.push(ImportRelation {
                                importer: "file".to_string(),
                                imported: path.clone(),
                                symbols: Vec::new(),
                                is_wildcard: false,
                                alias: None,
                            });
                        }
                        self.eda_data.design_reads.push((file_type, path));
                    }
                    EdaCommand::DesignFileWrite { file_type, path } => {
                        self.eda_data.design_writes.push((file_type, path));
                    }
                    EdaCommand::ToolFlowCommand { ref name, .. }
                    | EdaCommand::ObjectQuery { ref name, .. } => {
                        self.record_call(name, node);
                    }
                    EdaCommand::CommandRegistration { name, usage } => {
                        self.eda_data.registered_commands.push((name, usage));
                    }
                    EdaCommand::CollectionIteration { .. } => {
                        self.record_call(cmd_name, node);
                        self.visit_braced_bodies(node);
                    }
                    EdaCommand::AttributeAccess { .. } => {
                        self.record_call(cmd_name, node);
                    }
                }
            }
            return;
        }

        // 3. Generic command call
        self.record_call(cmd_name, node);
    }

    /// Handle `proc name {params} {body}`
    fn visit_proc_command(&mut self, node: Node) {
        let args = self.collect_argument_nodes(node);
        // args[0]=name, args[1]=params, args[2]=body
        if args.len() < 3 {
            return;
        }

        let name = self.node_text(args[0]).trim().to_string();
        if name.is_empty() {
            return;
        }

        let qualified = self.qualified_name(&name);
        let params = self.extract_params_from_braced(args[1]);
        let doc_comment = self.extract_preceding_comment(node);

        // Calculate complexity from body
        let complexity = self.calculate_complexity(args[2]);

        // Build signature
        let param_str = params
            .iter()
            .map(|p| {
                if let Some(ref default) = p.default_value {
                    format!("{{{} {}}}", p.name, default)
                } else {
                    p.name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        let signature = format!("proc {} {{{}}} {{...}}", name, param_str);

        let mut func = FunctionEntity::new(
            &qualified,
            node.start_position().row + 1,
            node.end_position().row + 1,
        )
        .with_visibility("public")
        .with_signature(&signature);

        func.parameters = params;
        func.doc_comment = doc_comment;
        func.parent_class = self.current_namespace();
        func.complexity = Some(complexity);

        self.functions.push(func);

        // Visit body for calls
        let prev_proc = self.current_procedure.take();
        self.current_procedure = Some(qualified);
        self.visit_braced_body(args[2]);
        self.current_procedure = prev_proc;
    }

    /// Handle `namespace eval name {body}`
    fn visit_namespace_command(&mut self, node: Node) {
        let args = self.collect_argument_nodes(node);
        // args[0]="eval", args[1]=name, args[2]=body
        if args.is_empty() {
            return;
        }

        let subcommand = self.node_text(args[0]).trim().to_string();
        if subcommand != "eval" || args.len() < 3 {
            return;
        }

        let ns_name = self.node_text(args[1]).trim().to_string();
        if ns_name.is_empty() {
            return;
        }

        self.namespace_stack.push(ns_name);
        let full_ns = self.current_namespace().unwrap_or_default();
        let doc_comment = self.extract_preceding_comment(node);

        let class = ClassEntity {
            name: full_ns,
            visibility: "public".to_string(),
            line_start: node.start_position().row + 1,
            line_end: node.end_position().row + 1,
            is_abstract: false,
            is_interface: false,
            base_classes: Vec::new(),
            implemented_traits: Vec::new(),
            methods: Vec::new(),
            fields: Vec::new(),
            doc_comment,
            attributes: vec!["namespace".to_string()],
            type_parameters: Vec::new(),
        };
        self.classes.push(class);

        // Visit body
        self.visit_braced_body(args[2]);

        self.namespace_stack.pop();
    }

    fn visit_source_command(&mut self, node: Node) {
        let args = self.collect_argument_nodes(node);
        if let Some(arg) = args.first() {
            let filename = self.node_text(*arg).trim().to_string();
            let cleaned = filename
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !cleaned.is_empty() {
                self.imports.push(ImportRelation {
                    importer: "file".to_string(),
                    imported: cleaned,
                    symbols: Vec::new(),
                    is_wildcard: true,
                    alias: None,
                });
            }
        }
    }

    fn visit_package_command(&mut self, node: Node) {
        let args = self.collect_argument_nodes(node);
        // args[0]="require", args[1]=package_name, args[2..]=version
        if args.is_empty() {
            return;
        }

        let subcommand = self.node_text(args[0]).trim().to_string();
        if subcommand == "require" && args.len() >= 2 {
            let pkg_name = self.node_text(args[1]).trim().to_string();
            if !pkg_name.is_empty() {
                self.imports.push(ImportRelation {
                    importer: "file".to_string(),
                    imported: pkg_name,
                    symbols: Vec::new(),
                    is_wildcard: false,
                    alias: None,
                });
            }
        }
    }

    fn record_call(&mut self, callee: &str, node: Node) {
        let caller = match &self.current_procedure {
            Some(name) => name.clone(),
            None => "::".to_string(),
        };

        self.calls.push(CallRelation {
            caller,
            callee: callee.to_string(),
            call_site_line: node.start_position().row + 1,
            is_direct: true,
        });
    }

    /// Extract the command name from a `command` node (the `name` field)
    fn extract_command_name(&self, node: Node) -> String {
        if let Some(name_node) = node.child_by_field_name("name") {
            return self.node_text(name_node).trim().to_string();
        }
        String::new()
    }

    /// Collect argument child nodes from the `arguments` field (word_list)
    fn collect_argument_nodes<'b>(&self, node: Node<'b>) -> Vec<Node<'b>> {
        let mut result = Vec::new();
        if let Some(args_node) = node.child_by_field_name("arguments") {
            if args_node.kind() == "word_list" {
                let mut cursor = args_node.walk();
                for child in args_node.children(&mut cursor) {
                    if !child.is_extra() {
                        result.push(child);
                    }
                }
            } else {
                result.push(args_node);
            }
        }
        result
    }

    /// Extract parameters from a braced_word like `{a {b 0} args}`
    fn extract_params_from_braced(&self, node: Node) -> Vec<Parameter> {
        let text = self.node_text(node);
        let inner = text
            .trim_start_matches('{')
            .trim_end_matches('}')
            .trim();

        if inner.is_empty() {
            return Vec::new();
        }

        let mut params = Vec::new();
        let chars = inner.chars();
        let mut current = String::new();
        let mut depth = 0;

        for ch in chars {
            match ch {
                '{' => {
                    depth += 1;
                    if depth > 1 {
                        current.push(ch);
                    }
                }
                '}' => {
                    depth -= 1;
                    if depth > 0 {
                        current.push(ch);
                    } else if depth == 0 {
                        // End of a braced param spec
                        let trimmed = current.trim().to_string();
                        if !trimmed.is_empty() {
                            params.push(Self::parse_param_spec(&trimmed));
                        }
                        current.clear();
                    }
                }
                ' ' | '\t' if depth == 0 => {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        params.push(Self::parse_param_spec(&trimmed));
                    }
                    current.clear();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            params.push(Self::parse_param_spec(&trimmed));
        }

        params
    }

    fn parse_param_spec(spec: &str) -> Parameter {
        // spec is either "name" or "name default_value"
        let parts: Vec<&str> = spec.splitn(2, char::is_whitespace).collect();
        let name = parts[0].to_string();
        let is_variadic = name == "args";
        let default_value = parts.get(1).map(|s| s.trim().to_string());

        let mut param = Parameter::new(name);
        param.is_variadic = is_variadic;
        param.default_value = default_value;
        param
    }

    /// Visit commands inside a braced_word body (e.g., proc body, namespace body)
    fn visit_braced_body(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "command" => self.visit_command(child),
                "procedure" => self.visit_procedure_node(child),
                "ERROR" => self.visit_error_node(child),
                "if" | "while" | "foreach" | "for" | "switch" | "try" | "catch" => {
                    self.record_call(child.kind(), child);
                    self.visit_braced_body(child);
                }
                _ => self.visit_braced_body(child),
            }
        }
    }

    /// Visit all braced_word children of a node as potential bodies
    fn visit_braced_bodies(&mut self, node: Node) {
        if let Some(args_node) = node.child_by_field_name("arguments") {
            let mut cursor = args_node.walk();
            for child in args_node.children(&mut cursor) {
                if child.kind() == "braced_word" || child.kind() == "braced_word_simple" {
                    self.visit_braced_body(child);
                }
            }
        }
    }

    fn extract_preceding_comment(&self, node: Node) -> Option<String> {
        let mut comments = Vec::new();
        let mut prev = node.prev_sibling();

        while let Some(sibling) = prev {
            if sibling.kind() == "comment" {
                let text = self.node_text(sibling);
                comments.push(text);
                prev = sibling.prev_sibling();
            } else {
                break;
            }
        }

        if comments.is_empty() {
            return None;
        }

        comments.reverse();
        Some(comments.join("\n"))
    }

    fn calculate_complexity(&self, body_node: Node) -> ComplexityMetrics {
        let mut metrics = ComplexityMetrics {
            cyclomatic_complexity: 1,
            branches: 0,
            loops: 0,
            logical_operators: 0,
            max_nesting_depth: 0,
            exception_handlers: 0,
            early_returns: 0,
        };
        self.walk_complexity(body_node, 0, &mut metrics);
        metrics
    }

    fn walk_complexity(&self, node: Node, depth: u32, metrics: &mut ComplexityMetrics) {
        if depth > metrics.max_nesting_depth {
            metrics.max_nesting_depth = depth;
        }

        // Determine the effective command name from command nodes, ERROR nodes, or direct keyword nodes
        let effective_cmd = match node.kind() {
            "command" => Some(self.extract_command_name(node)),
            // Direct keyword nodes (proper parse path)
            "if" | "elseif" | "while" | "foreach" | "for" | "catch" | "return" | "switch"
            | "try" => Some(node.kind().to_string()),
            "ERROR" => {
                // ERROR nodes for keywords have the keyword as a child
                let mut c = node.walk();
                let result = node
                    .children(&mut c)
                    .find(|child| {
                        matches!(
                            child.kind(),
                            "if" | "elseif"
                                | "while"
                                | "foreach"
                                | "for"
                                | "catch"
                                | "return"
                                | "foreach_in_collection"
                                | "switch"
                                | "try"
                        )
                    })
                    .map(|child| child.kind().to_string());
                result
            }
            _ => None,
        };

        if let Some(cmd_name) = effective_cmd {
            match cmd_name.as_str() {
                "if" | "elseif" => {
                    metrics.cyclomatic_complexity += 1;
                    metrics.branches += 1;
                }
                "while" | "foreach" | "for" => {
                    metrics.cyclomatic_complexity += 1;
                    metrics.loops += 1;
                }
                "foreach_in_collection" => {
                    metrics.cyclomatic_complexity += 1;
                    metrics.loops += 1;
                }
                "catch" => {
                    metrics.cyclomatic_complexity += 1;
                    metrics.exception_handlers += 1;
                }
                "return" => {
                    metrics.early_returns += 1;
                }
                _ => {}
            }
        }

        let new_depth = if node.kind() == "braced_word" || node.kind() == "braced_word_simple" {
            depth + 1
        } else {
            depth
        };

        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.walk_complexity(child, new_depth, metrics);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_and_visit(source: &[u8]) -> TclVisitor<'_> {
        let mut parser = Parser::new();
        let language = crate::ts_tcl::language();
        parser.set_language(&language).unwrap();
        let tree = parser.parse(source, None).unwrap();

        let mut visitor = TclVisitor::new(source, ParserConfig::default());
        visitor.visit_node(tree.root_node());
        visitor
    }

    #[test]
    fn test_visitor_empty() {
        let visitor = TclVisitor::new(b"", ParserConfig::default());
        assert_eq!(visitor.functions.len(), 0);
        assert_eq!(visitor.classes.len(), 0);
    }

    #[test]
    fn test_visit_simple_proc() {
        let source = b"proc greet {name} {\n    puts \"Hello $name\"\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "greet");
        assert_eq!(visitor.functions[0].parameters.len(), 1);
        assert_eq!(visitor.functions[0].parameters[0].name, "name");
    }

    #[test]
    fn test_visit_proc_with_defaults() {
        let source = b"proc add {a {b 0}} {\n    expr {$a + $b}\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert_eq!(visitor.functions[0].name, "add");
        assert!(visitor.functions[0].parameters.len() >= 2);
        assert_eq!(visitor.functions[0].parameters[0].name, "a");
        assert_eq!(visitor.functions[0].parameters[1].name, "b");
        assert_eq!(
            visitor.functions[0].parameters[1].default_value,
            Some("0".to_string())
        );
    }

    #[test]
    fn test_visit_proc_with_args() {
        let source = b"proc variadic {args} {\n    puts $args\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        assert!(visitor.functions[0]
            .parameters
            .iter()
            .any(|p| p.is_variadic));
    }

    #[test]
    fn test_visit_source_import() {
        let source = b"source utils.tcl\nsource \"lib/helpers.tcl\"";
        let visitor = parse_and_visit(source);

        assert!(!visitor.imports.is_empty());
        assert!(visitor
            .imports
            .iter()
            .any(|i| i.imported.contains("utils.tcl")));
    }

    #[test]
    fn test_visit_package_require() {
        let source = b"package require Tcl 8.6\npackage require http";
        let visitor = parse_and_visit(source);

        assert!(visitor
            .imports
            .iter()
            .any(|i| i.imported == "Tcl"));
        assert!(visitor
            .imports
            .iter()
            .any(|i| i.imported == "http"));
    }

    #[test]
    fn test_visit_sdc_create_clock() {
        let source = b"create_clock -name clk -period 10 [get_ports clk_in]";
        let visitor = parse_and_visit(source);

        assert!(!visitor.sdc_data.clocks.is_empty());
        assert_eq!(visitor.sdc_data.clocks[0].name, "clk");
        assert_eq!(visitor.sdc_data.clocks[0].period, "10");
    }

    #[test]
    fn test_visit_eda_read_verilog() {
        let source = b"read_verilog design.v";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.eda_data.design_reads.len(), 1);
        assert_eq!(visitor.eda_data.design_reads[0].0, "verilog");
    }

    #[test]
    fn test_visit_eda_write_def() {
        let source = b"write_def output.def";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.eda_data.design_writes.len(), 1);
        assert_eq!(visitor.eda_data.design_writes[0].0, "def");
    }

    #[test]
    fn test_visit_tool_flow_commands() {
        let source = b"compile\nreport_timing\nglobal_placement";
        let visitor = parse_and_visit(source);

        assert!(visitor.calls.len() >= 3);
    }

    #[test]
    fn test_visit_comment_as_doc() {
        let source = b"# This is a greeting procedure\n# It says hello\nproc greet {name} {\n    puts hello\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        if let Some(ref doc) = visitor.functions[0].doc_comment {
            assert!(doc.contains("greeting procedure"));
        }
    }

    #[test]
    fn test_visit_namespace_eval() {
        let source = b"namespace eval math {\n    proc add {a b} {\n        expr {$a + $b}\n    }\n}";
        let visitor = parse_and_visit(source);

        assert!(!visitor.classes.is_empty());
        assert!(visitor.classes.iter().any(|c| c.name.contains("math")));

        // The proc should be namespace-qualified
        if !visitor.functions.is_empty() {
            assert!(visitor.functions[0].name.contains("math"));
        }
    }

    #[test]
    fn test_complexity_simple() {
        let source = b"proc simple {} {\n    puts hello\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        if let Some(ref c) = visitor.functions[0].complexity {
            assert_eq!(c.cyclomatic_complexity, 1);
        }
    }

    #[test]
    fn test_complexity_with_branches() {
        let source =
            b"proc check {x} {\n    if {$x > 0} {\n        puts positive\n    }\n}";
        let visitor = parse_and_visit(source);

        assert_eq!(visitor.functions.len(), 1);
        if let Some(ref c) = visitor.functions[0].complexity {
            assert!(c.cyclomatic_complexity >= 2);
            assert!(c.branches >= 1);
        }
    }

    #[test]
    fn test_param_parsing() {
        // Test the parameter parsing directly
        let params = TclVisitor::parse_param_spec("name");
        assert_eq!(params.name, "name");
        assert!(params.default_value.is_none());

        let params2 = TclVisitor::parse_param_spec("b 0");
        assert_eq!(params2.name, "b");
        assert_eq!(params2.default_value, Some("0".to_string()));

        let params3 = TclVisitor::parse_param_spec("args");
        assert!(params3.is_variadic);
    }
}
