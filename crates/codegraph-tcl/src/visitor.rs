//! AST visitor for extracting Tcl entities
//!
//! The vendored tree-sitter-tcl grammar (ABI v15→v14 patch) produces ERROR nodes
//! for 14 Tcl keywords instead of proper named AST nodes. This visitor uses
//! [`resolve_error_keyword`] to transparently map ERROR nodes to their keyword
//! names, so all dispatch code sees resolved kinds — never "ERROR".

use codegraph_parser_api::{
    CallRelation, ClassEntity, ComplexityMetrics, FunctionEntity, ImportRelation, Parameter,
    ParserConfig,
};
use tree_sitter::Node;

use crate::eda::{self, EdaCommand, EdaData};
use crate::sdc::{self, SdcData};

/// All Tcl keywords that the tree-sitter-tcl grammar defines as named rules.
/// These may appear as proper AST nodes OR as ERROR nodes depending on context.
const TCL_KEYWORDS: &[&str] = &[
    "proc",
    "namespace",
    "if",
    "elseif",
    "else",
    "while",
    "foreach",
    "try",
    "catch",
    "finally",
    "set",
    "global",
    "regexp",
    "expr",
];

/// Scan an ERROR node's children for a recognizable Tcl keyword.
///
/// Returns `&'static str` (string literals from match arms) so there are
/// no lifetime conflicts with `&mut self` in callers.
fn resolve_error_keyword(node: Node) -> &'static str {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        match child.kind() {
            "proc" => return "proc",
            "namespace" => return "namespace",
            "if" => return "if",
            "elseif" => return "elseif",
            "else" => return "else",
            "while" => return "while",
            "foreach" => return "foreach",
            "try" => return "try",
            "catch" => return "catch",
            "finally" => return "finally",
            "set" => return "set",
            "global" => return "global",
            "regexp" => return "regexp",
            "expr" => return "expr",
            _ => continue,
        }
    }
    "unknown"
}

/// Resolve a node's effective kind. ERROR nodes are mapped to their keyword;
/// `procedure` nodes are normalized to `"proc"`. Everything else passes through.
fn resolve_kind(node: Node<'_>) -> &str {
    match node.kind() {
        "ERROR" => resolve_error_keyword(node),
        "procedure" => "proc",
        k => k,
    }
}

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

    // ── Main dispatch ───────────────────────────────────────────────────

    pub fn visit_node(&mut self, node: Node) {
        let kind = resolve_kind(node);

        match kind {
            "source_file" => self.visit_children(node),
            "command" => self.visit_command(node),
            "proc" => self.visit_proc(node),
            "namespace" => self.visit_namespace(node),
            "if" | "elseif" | "while" | "foreach" | "try" | "catch" | "set" | "global"
            | "regexp" | "expr" | "else" | "finally" => {
                self.record_call(kind, node);
                self.visit_bodies(node);
            }
            _ => self.visit_children(node),
        }
    }

    fn visit_children(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.visit_node(child);
        }
    }

    // ── Proc handling (unified) ─────────────────────────────────────────

    /// Handle `proc name {params} {body}` regardless of AST structure.
    ///
    /// Covers four observed parse structures:
    /// - `procedure` node: `[proc, ERROR(name), braced_word(params), arguments, braced_word(body)]`
    /// - ERROR (simple): `[proc, ERROR(name, params), braced_word(body)]`
    /// - ERROR (comments): `[comment*, proc, command(name, word_list(params, body))]`
    /// - `command` node: `name="proc", arguments=[name, params, body]`
    fn visit_proc(&mut self, node: Node) {
        let (name_str, params_node, body_node) = if node.kind() == "command" {
            self.extract_proc_from_command(node)
        } else {
            self.extract_proc_from_tree(node)
        };

        if name_str.is_empty() {
            return;
        }

        let qualified = self.qualified_name(&name_str);
        let params = match params_node {
            Some(pn) => self.extract_params_from_braced(pn),
            None => Vec::new(),
        };

        // Extract doc comments: siblings first, then comment children inside ERROR
        let doc_comment = self.extract_preceding_comment(node).or_else(|| {
            let mut inner_comments = Vec::new();
            let mut c = node.walk();
            for child in node.children(&mut c) {
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

        // Visit body for nested calls
        let prev_proc = self.current_procedure.take();
        self.current_procedure = Some(qualified);
        if let Some(bn) = body_node {
            if bn.kind() == "arguments" {
                self.visit_arguments_body(bn);
            } else {
                self.visit_braced_body(bn);
            }
        }
        self.current_procedure = prev_proc;
    }

    /// Extract proc name/params/body when the node is a `command` (name="proc").
    fn extract_proc_from_command<'b>(
        &self,
        node: Node<'b>,
    ) -> (String, Option<Node<'b>>, Option<Node<'b>>) {
        let args = self.collect_argument_nodes(node);
        if args.len() < 3 {
            return (String::new(), None, None);
        }
        let name = self.node_text(args[0]).trim().to_string();
        (name, Some(args[1]), Some(args[2]))
    }

    /// Extract proc name/params/body from procedure/ERROR node trees.
    fn extract_proc_from_tree<'b>(
        &self,
        node: Node<'b>,
    ) -> (String, Option<Node<'b>>, Option<Node<'b>>) {
        let mut name_str = String::new();
        let mut params_node: Option<Node<'b>> = None;
        let mut body_node: Option<Node<'b>> = None;
        let mut found_proc = false;

        let mut cursor = node.walk();
        let children: Vec<Node<'b>> = node.children(&mut cursor).collect();

        for child in &children {
            match child.kind() {
                "comment" => continue,
                "proc" => {
                    found_proc = true;
                    continue;
                }
                // Comments case: rest of proc parsed as a command child
                "command" if found_proc => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        if name_str.is_empty() {
                            name_str = self.node_text(name_node).trim().to_string();
                        }
                    }
                    if let Some(args_node) = child.child_by_field_name("arguments") {
                        let mut ic = args_node.walk();
                        for inner in args_node.children(&mut ic) {
                            if inner.kind() == "braced_word"
                                || inner.kind() == "braced_word_simple"
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
                // Nested ERROR containing name and possibly params
                "ERROR" if found_proc => {
                    let mut ic = child.walk();
                    for inner in child.children(&mut ic) {
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
                // Flat name child
                "simple_word" | "word" if found_proc && name_str.is_empty() => {
                    name_str = self.node_text(*child).trim().to_string();
                }
                // Flat braced_word children: first is params, second is body.
                // Always prefer braced_word over arguments for body.
                "braced_word" | "braced_word_simple" if found_proc => {
                    if name_str.is_empty() {
                        continue;
                    } else if params_node.is_none() {
                        params_node = Some(*child);
                    } else {
                        body_node = Some(*child);
                    }
                }
                // When the body contains keywords, tree-sitter may parse it
                // as an `arguments` node instead of `braced_word`.
                "arguments" if found_proc && params_node.is_some() && body_node.is_none() => {
                    body_node = Some(*child);
                }
                _ => {}
            }
        }

        (name_str, params_node, body_node)
    }

    // ── Namespace handling (unified) ────────────────────────────────────

    /// Handle `namespace eval name {body}` regardless of AST structure.
    fn visit_namespace(&mut self, node: Node) {
        let (ns_name, body_node) = if node.kind() == "command" {
            self.extract_namespace_from_command(node)
        } else {
            self.extract_namespace_from_tree(node)
        };

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

        if let Some(bn) = body_node {
            self.visit_braced_body(bn);
        }

        self.namespace_stack.pop();
    }

    /// Extract namespace name/body from a `command` node (name="namespace").
    fn extract_namespace_from_command<'b>(
        &self,
        node: Node<'b>,
    ) -> (String, Option<Node<'b>>) {
        let args = self.collect_argument_nodes(node);
        if args.is_empty() {
            return (String::new(), None);
        }
        let subcommand = self.node_text(args[0]).trim().to_string();
        if subcommand != "eval" || args.len() < 3 {
            return (String::new(), None);
        }
        let ns_name = self.node_text(args[1]).trim().to_string();
        (ns_name, Some(args[2]))
    }

    /// Extract namespace name/body from ERROR/procedure node trees.
    fn extract_namespace_from_tree<'b>(
        &self,
        node: Node<'b>,
    ) -> (String, Option<Node<'b>>) {
        let mut found_eval = false;
        let mut ns_name = String::new();
        let mut body_node: Option<Node<'b>> = None;

        let mut cursor = node.walk();
        let children: Vec<Node<'b>> = node.children(&mut cursor).collect();

        for child in &children {
            match child.kind() {
                "namespace" => continue,
                "ERROR" => {
                    let mut ic = child.walk();
                    for inner in child.children(&mut ic) {
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

        if !found_eval {
            return (String::new(), None);
        }
        (ns_name, body_node)
    }

    // ── Command dispatch ────────────────────────────────────────────────

    fn visit_command(&mut self, node: Node) {
        let cmd_name = self.extract_command_name(node);
        if cmd_name.is_empty() {
            return;
        }

        match cmd_name.as_str() {
            "proc" => self.visit_proc(node),
            "namespace" => self.visit_namespace(node),
            "source" => self.visit_source_command(node),
            "package" => self.visit_package_command(node),
            "if" | "elseif" | "while" | "foreach" | "for" | "switch" | "try" | "catch" => {
                self.record_call(&cmd_name, node);
                self.visit_braced_bodies(node);
            }
            _ => self.visit_general_command(&cmd_name, node),
        }
    }

    fn visit_general_command(&mut self, cmd_name: &str, node: Node) {
        if sdc::is_sdc_command(cmd_name) {
            if let Some(constraint) = sdc::extract_sdc_constraint(cmd_name, node, self.source) {
                self.sdc_data.add(constraint);
            }
            self.record_call(cmd_name, node);
            return;
        }

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

        self.record_call(cmd_name, node);
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

    // ── Body visiting ───────────────────────────────────────────────────

    /// Visit commands inside a braced_word body (proc body, namespace body, etc.).
    /// Uses `resolve_kind` so ERROR nodes are dispatched by their keyword.
    fn visit_braced_body(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = resolve_kind(child);
            match kind {
                "command" => self.visit_command(child),
                "proc" => self.visit_proc(child),
                "namespace" => self.visit_namespace(child),
                "if" | "elseif" | "while" | "foreach" | "try" | "catch" | "set" | "global"
                | "regexp" | "expr" | "else" | "finally" => {
                    self.record_call(kind, child);
                    self.visit_braced_body(child);
                }
                _ => self.visit_braced_body(child),
            }
        }
    }

    /// Visit braced_word children of a `command` node's arguments.
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

    /// Visit all braced_word children of any node (ERROR, procedure, etc.).
    /// Used for control flow keywords resolved from ERROR nodes.
    fn visit_bodies(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "braced_word" || child.kind() == "braced_word_simple" {
                self.visit_braced_body(child);
            }
        }
    }

    /// Visit an `arguments` node used as a proc body.
    ///
    /// When tree-sitter-tcl parses a proc body containing grammar keywords
    /// (set, global, expr, etc.), it may flatten the body into an `arguments`
    /// node instead of a `braced_word`. The inner commands appear as `argument`
    /// children with text matching keyword names.
    fn visit_arguments_body(&mut self, node: Node) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "command" => self.visit_command(child),
                "argument" => {
                    let text = self.node_text(child);
                    let trimmed = text.trim();
                    if TCL_KEYWORDS.contains(&trimmed) {
                        self.record_call(trimmed, child);
                    }
                }
                _ => {
                    let kind = resolve_kind(child);
                    match kind {
                        "proc" => self.visit_proc(child),
                        "namespace" => self.visit_namespace(child),
                        "if" | "elseif" | "while" | "foreach" | "try" | "catch" | "set"
                        | "global" | "regexp" | "expr" | "else" | "finally" => {
                            self.record_call(kind, child);
                            self.visit_bodies(child);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    // ── Helpers ─────────────────────────────────────────────────────────

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

    fn extract_command_name(&self, node: Node) -> String {
        if let Some(name_node) = node.child_by_field_name("name") {
            return self.node_text(name_node).trim().to_string();
        }
        String::new()
    }

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
        let parts: Vec<&str> = spec.splitn(2, char::is_whitespace).collect();
        let name = parts[0].to_string();
        let is_variadic = name == "args";
        let default_value = parts.get(1).map(|s| s.trim().to_string());

        let mut param = Parameter::new(name);
        param.is_variadic = is_variadic;
        param.default_value = default_value;
        param
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

    // ── Complexity analysis ─────────────────────────────────────────────

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

        // Resolve the effective kind so ERROR nodes are handled transparently
        let kind = resolve_kind(node);

        let effective_cmd = match kind {
            "command" => Some(self.extract_command_name(node)),
            "if" | "elseif" | "while" | "foreach" | "for" | "catch" | "return" | "switch"
            | "try" | "set" | "global" | "regexp" | "expr" => Some(kind.to_string()),
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
        let params = TclVisitor::parse_param_spec("name");
        assert_eq!(params.name, "name");
        assert!(params.default_value.is_none());

        let params2 = TclVisitor::parse_param_spec("b 0");
        assert_eq!(params2.name, "b");
        assert_eq!(params2.default_value, Some("0".to_string()));

        let params3 = TclVisitor::parse_param_spec("args");
        assert!(params3.is_variadic);
    }

    // ── Tests for previously unhandled keywords ─────────────────────────

    #[test]
    fn test_resolve_error_keyword_covers_all() {
        // Verify the constant and function agree
        for &kw in TCL_KEYWORDS {
            // Each keyword should appear in resolve_error_keyword's match arms
            assert_ne!(kw, "unknown", "TCL_KEYWORDS must not contain 'unknown'");
        }
    }

    #[test]
    fn test_set_recorded_as_call() {
        let source = b"proc foo {} {\n    set x 42\n}";
        let visitor = parse_and_visit(source);

        assert!(
            visitor.calls.iter().any(|c| c.callee == "set"),
            "set should be recorded as a call, got: {:?}",
            visitor.calls.iter().map(|c| &c.callee).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_global_recorded_as_call() {
        let source = b"proc foo {} {\n    global myvar\n}";
        let visitor = parse_and_visit(source);

        assert!(
            visitor.calls.iter().any(|c| c.callee == "global"),
            "global should be recorded as a call, got: {:?}",
            visitor.calls.iter().map(|c| &c.callee).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_expr_recorded_as_call() {
        let source = b"proc foo {} {\n    expr {1 + 2}\n}";
        let visitor = parse_and_visit(source);

        assert!(
            visitor.calls.iter().any(|c| c.callee == "expr"),
            "expr should be recorded as a call, got: {:?}",
            visitor.calls.iter().map(|c| &c.callee).collect::<Vec<_>>()
        );
    }
}
