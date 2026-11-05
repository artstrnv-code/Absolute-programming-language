use apl_core::{
    Assignment, AssignmentOperator, BinaryOperator, ElseIfBranch, Expression, FunctionDecl,
    IfStatement, InfoAssignment, ListDecl, ListElement, MatchCase, MatchPattern, MatchStatement,
    PickStatement, Program, ProtectionLevel, Statement, UnaryOperator, VTimeDecl, Value, ValueType,
    VariableDecl, WhileStatement,
};

pub fn parse_program(source: &str) -> Result<Program, ParseError> {
    parse_program_internal(source, false).map(|parsed| parsed.program)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementLocation {
    pub path: Vec<usize>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedProgram {
    pub program: Program,
    pub locations: Vec<StatementLocation>,
}

pub fn parse_program_with_locations(source: &str) -> Result<ParsedProgram, ParseError> {
    parse_program_internal(source, true)
}

fn parse_program_internal(
    source: &str,
    track_locations: bool,
) -> Result<ParsedProgram, ParseError> {
    let tokenized = tokenize(source)?;
    let std_namespace_open = tokens_open_std_namespace(&tokenized.tokens);
    let strict_executable = tokens_have_main(&tokenized.tokens, std_namespace_open);
    let mut parser = Parser {
        tokens: tokenized.tokens,
        offsets: tokenized.offsets,
        source,
        cursor: 0,
        std_namespace_open,
        strict_executable,
        track_locations,
        current_path: Vec::new(),
        locations: Vec::new(),
    };
    parser.parse_program()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyntaxKind {
    Keyword,
    Identifier,
    Number,
    String,
    Comment,
    Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyntaxSpan {
    pub start: usize,
    pub end: usize,
    pub kind: SyntaxKind,
}

/// Returns byte ranges produced by the same lexical rules as the parser.
/// Unlike parsing, this is tolerant of incomplete input so editors can keep
/// highlighting while a program is being typed.
pub fn syntax_spans(source: &str) -> Vec<SyntaxSpan> {
    let mut spans = Vec::new();
    let mut cursor = 0;

    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }

        let start = cursor;
        let kind = if ch == '#' {
            while cursor < source.len()
                && !source[cursor..].starts_with('\n')
                && !source[cursor..].starts_with('\r')
            {
                cursor += source[cursor..].chars().next().unwrap().len_utf8();
            }
            SyntaxKind::Comment
        } else if ch == '"' {
            cursor += 1;
            let mut escaped = false;
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap();
                cursor += next.len_utf8();
                if next == '"' && !escaped {
                    break;
                }
                escaped = next == '\\' && !escaped;
                if next != '\\' {
                    escaped = false;
                }
            }
            SyntaxKind::String
        } else if ch.is_ascii_digit() {
            cursor += ch.len_utf8();
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap();
                if !next.is_ascii_digit() && next != '.' {
                    break;
                }
                cursor += next.len_utf8();
            }
            SyntaxKind::Number
        } else if is_ident_start(ch) {
            cursor += ch.len_utf8();
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap();
                if !is_ident_continue(next) {
                    break;
                }
                cursor += next.len_utf8();
            }
            let value = &source[start..cursor];
            if is_keyword(value)
                || value == "limit"
                || ValueType::parse_decl_keyword(value).is_some()
            {
                SyntaxKind::Keyword
            } else {
                SyntaxKind::Identifier
            }
        } else {
            match read_symbol(source, cursor) {
                Ok((_, end)) => cursor = end,
                Err(_) => cursor += ch.len_utf8(),
            }
            SyntaxKind::Symbol
        };
        spans.push(SyntaxSpan {
            start,
            end: cursor,
            kind,
        });
    }

    spans
}

struct Parser<'a> {
    tokens: Vec<Token>,
    offsets: Vec<usize>,
    source: &'a str,
    cursor: usize,
    std_namespace_open: bool,
    strict_executable: bool,
    track_locations: bool,
    current_path: Vec<usize>,
    locations: Vec<StatementLocation>,
}

impl Parser<'_> {
    fn parse_program(&mut self) -> Result<ParsedProgram, ParseError> {
        let mut statements = Vec::new();

        self.skip_newlines();

        while !self.is_done() {
            let statement = if self.track_locations {
                self.parse_statement_at(vec![statements.len()])?
            } else {
                self.parse_statement()?
            };
            statements.push(statement);
            self.expect_statement_end()?;
            self.skip_newlines();
        }

        Ok(ParsedProgram {
            program: Program::new(statements),
            locations: std::mem::take(&mut self.locations),
        })
    }

    fn parse_statement_at(&mut self, path: Vec<usize>) -> Result<Statement, ParseError> {
        let offset = self
            .offsets
            .get(self.cursor)
            .copied()
            .unwrap_or(self.source.len());
        let previous_path = std::mem::replace(&mut self.current_path, path.clone());
        let statement = self.parse_statement();
        self.current_path = previous_path;
        if statement.is_ok() {
            let position = parse_error_at(self.source, offset, "");
            self.locations.push(StatementLocation {
                path,
                line: position.line,
                column: position.column,
            });
        }
        statement
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        if self.eat_symbol(Symbol::At) {
            let mut attributes = Vec::new();
            loop {
                let name = self.expect_ident()?;
                if !matches!(name.as_str(), "marked" | "collector" | "flee") {
                    return Err(
                        self.error_at_previous(format!("unknown memory attribute `{name}`"))
                    );
                }
                if attributes.contains(&name) {
                    return Err(
                        self.error_at_previous(format!("duplicate memory attribute `{name}`"))
                    );
                }
                attributes.push(name);
                self.skip_newlines();
                if !self.eat_symbol(Symbol::At) {
                    break;
                }
            }
            if self.looks_like_main_decl() {
                return self.parse_main().map(Statement::FunctionDecl);
            }
            if self.check_keyword("func") {
                return self.parse_function().map(Statement::FunctionDecl);
            }
            if self.check_keyword("while") {
                return self.parse_while().map(Statement::While);
            }
            return Err(self.error("memory attributes require a function or while"));
        }

        if self.eat_keyword("include") {
            self.expect_symbol(Symbol::RightShift)?;
            return self.expect_ident().map(Statement::Include);
        }

        if self.eat_keyword("exclude") {
            let symbol = self.expect_string()?;
            self.expect_symbol(Symbol::LeftShift)?;
            let library = self.expect_ident()?;
            return Ok(Statement::Exclude { symbol, library });
        }

        if self.eat_keyword("using") {
            self.expect_keyword("namespace")?;
            return self.expect_ident().map(Statement::UsingNamespace);
        }

        if self.eat_keyword("nc") {
            let mut namespaces = vec![self.expect_ident()?];
            while self.eat_symbol(Symbol::Comma) {
                namespaces.push(self.expect_ident()?);
            }
            return Ok(Statement::NoConflicts(namespaces));
        }

        if self.check_keyword("class") {
            return self.parse_class_placeholder().map(Statement::FunctionDecl);
        }

        if self.check_keyword("struct") {
            return self.parse_struct_placeholder().map(Statement::FunctionDecl);
        }

        if self.looks_like_class_instance_decl() {
            return self
                .parse_class_instance_placeholder()
                .map(Statement::VTimeDecl);
        }

        if self.looks_like_main_decl() {
            return self.parse_main().map(Statement::FunctionDecl);
        }

        if self.check_keyword("if") {
            return self.parse_if().map(Statement::If);
        }

        if self.check_keyword("func") {
            return self.parse_function().map(Statement::FunctionDecl);
        }

        if self.check_keyword("while") {
            return self.parse_while().map(Statement::While);
        }

        if self.check_keyword("match") {
            return self.parse_match().map(Statement::Match);
        }

        if self.check_keyword("pick") {
            return self.parse_pick().map(Statement::Pick);
        }

        if self.eat_keyword("List") {
            let name = self.expect_ident()?;
            self.expect_symbol(Symbol::Assign)?;
            let elements = self.parse_list_literal_elements()?;
            return Ok(Statement::ListDecl(ListDecl { name, elements }));
        }

        if self.check_std_function("add") {
            return self.parse_add_to_list();
        }

        if self.check_std_function("set") {
            return self.parse_set_list_element();
        }

        if self.eat_keyword("break") {
            return Ok(Statement::Break);
        }

        if self.eat_keyword("continue") {
            return Ok(Statement::Continue);
        }

        if self.eat_keyword("return") {
            if self.check_symbol(Symbol::LeftParen)
                && matches!(self.tokens.get(self.cursor + 1), Some(Token::Keyword(value)) if value == "ERR")
            {
                return self.parse_error_return();
            }
            return self.parse_expression().map(Statement::Return);
        }

        if self.eat_keyword("secretup") {
            self.expect_symbol(Symbol::LeftParen)?;
            let name = self.expect_ident()?;
            self.expect_symbol(Symbol::RightParen)?;
            return Ok(Statement::SecretUp(name));
        }

        if self.eat_keyword("stop") {
            let reason = if self.next_starts_statement_or_block_end() {
                None
            } else {
                Some(self.parse_expression()?)
            };
            return Ok(Statement::Stop(reason));
        }

        if self.eat_keyword("fail") {
            return self.parse_expression().map(Statement::Fail);
        }

        if self.eat_keyword("out") {
            return self.parse_expression().map(Statement::Out);
        }

        if self.eat_keyword("VTime") {
            let name = self.expect_ident()?;
            self.expect_symbol(Symbol::Assign)?;
            let initial_value = self.parse_expression()?;
            return Ok(Statement::VTimeDecl(VTimeDecl {
                name,
                initial_value,
            }));
        }

        if let Some((kind, value_type)) = self.peek_decl_keyword() {
            self.next();
            let name = self.expect_ident()?;
            self.expect_symbol(Symbol::Assign)?;
            let initial_value = self.parse_expression()?;

            return Ok(Statement::VariableDecl(VariableDecl {
                kind,
                value_type,
                name,
                initial_value,
            }));
        }

        if self.looks_like_info_assignment() {
            return self.parse_info_assignment().map(Statement::InfoAssignment);
        }

        if self.looks_like_call_statement() {
            return self.parse_expression().map(Statement::Expression);
        }

        self.parse_assignment().map(Statement::Assignment)
    }

    fn looks_like_call_statement(&self) -> bool {
        self.qualified_name_end(self.cursor).is_some_and(|end| {
            matches!(self.tokens.get(end), Some(Token::Symbol(Symbol::LeftParen)))
        })
    }

    fn looks_like_class_instance_decl(&self) -> bool {
        matches!(self.tokens.get(self.cursor), Some(Token::Ident(_)))
            && matches!(self.tokens.get(self.cursor + 1), Some(Token::Ident(_)))
            && matches!(
                self.tokens.get(self.cursor + 2),
                Some(Token::Symbol(Symbol::Assign))
            )
    }

    fn parse_class_instance_placeholder(&mut self) -> Result<VTimeDecl, ParseError> {
        let _class_name = self.expect_ident()?;
        let name = self.expect_ident()?;
        self.expect_symbol(Symbol::Assign)?;
        let initial_value = self.parse_expression()?;
        Ok(VTimeDecl {
            name,
            initial_value,
        })
    }

    fn parse_class_placeholder(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword("class")?;
        let name = self.expect_ident()?;
        self.expect_symbol(Symbol::LeftBrace)?;

        self.expect_newline("class body must start on a new line")?;
        self.skip_newlines();

        while !self.check_symbol(Symbol::RightBrace) {
            if self.is_done() {
                return Err(self.error("expected `}` after class body"));
            }

            if self.check_keyword("class") {
                self.next();
                self.expect_symbol(Symbol::Colon)?;
                self.expect_symbol(Symbol::Colon)?;
                let directive = self.expect_name_segment()?;
                match directive.as_str() {
                    "inheritance" => {
                        self.expect_string()?;
                    }
                    "exclude" => {
                        self.expect_string()?;
                        match self.next() {
                            Some(Token::Ident(value)) if value == "method" => {}
                            Some(_) => {
                                return Err(
                                    self.error_at_previous("class exclusion expects `method`")
                                )
                            }
                            None => return Err(self.error("class exclusion expects `method`")),
                        }
                        self.expect_symbol(Symbol::RightShift)?;
                        self.expect_ident()?;
                    }
                    _ => {
                        return Err(self
                            .error_at_previous(format!("unknown class directive `{directive}`")))
                    }
                }
                self.expect_statement_end()?;
                self.skip_newlines();
                continue;
            }

            if self.eat_keyword("override") {
                if !self.check_keyword("func") {
                    return Err(self.error("override expects func"));
                }
                self.parse_function()?;
                self.expect_statement_end()?;
                self.skip_newlines();
                continue;
            }

            if self.check_keyword("func") {
                self.parse_function()?;
                self.expect_statement_end()?;
                self.skip_newlines();
                continue;
            }

            if self.eat_keyword("List") {
                self.expect_ident()?;
                self.expect_symbol(Symbol::Assign)?;
                self.parse_list_literal_elements()?;
                self.expect_statement_end()?;
                self.skip_newlines();
                continue;
            }

            if let Some(_) = self.peek_decl_keyword() {
                self.next();
                self.expect_ident()?;
                self.expect_symbol(Symbol::Assign)?;
                self.parse_expression()?;
                self.expect_statement_end()?;
                self.skip_newlines();
                continue;
            }

            return Err(self.error("class body expects a field, method, or class directive"));
        }
        self.expect_symbol(Symbol::RightBrace)?;

        Ok(FunctionDecl {
            name: format!("aplclass.decl.{name}"),
            params: Vec::new(),
            body: Vec::new(),
        })
    }

    fn parse_struct_placeholder(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword("struct")?;
        let name = self.expect_ident()?;
        self.expect_symbol(Symbol::LeftBrace)?;
        self.expect_newline("struct body must start on a new line")?;
        self.skip_newlines();

        while !self.check_symbol(Symbol::RightBrace) {
            if self.is_done() {
                return Err(self.error("expected `}` after struct body"));
            }
            if self.eat_keyword("List") {
                self.expect_ident()?;
                self.expect_symbol(Symbol::Assign)?;
                self.parse_list_literal_elements()?;
            } else if self.peek_decl_keyword().is_some() {
                self.next();
                self.expect_ident()?;
                self.expect_symbol(Symbol::Assign)?;
                self.parse_expression()?;
            } else if matches!(self.tokens.get(self.cursor), Some(Token::Ident(_)))
                && matches!(self.tokens.get(self.cursor + 1), Some(Token::Ident(_)))
            {
                self.expect_ident()?;
                self.expect_ident()?;
                self.expect_symbol(Symbol::Assign)?;
                self.parse_expression()?;
            } else {
                return Err(self.error("struct body expects a typed field"));
            }
            self.expect_statement_end()?;
            self.skip_newlines();
        }
        self.expect_symbol(Symbol::RightBrace)?;

        Ok(FunctionDecl {
            name: format!("aplstruct.decl.{name}"),
            params: Vec::new(),
            body: Vec::new(),
        })
    }

    fn looks_like_main_decl(&self) -> bool {
        if !matches!(self.tokens.get(self.cursor), Some(Token::Keyword(value)) if value == "AVInt")
        {
            return false;
        }

        let qualified = matches!(self.tokens.get(self.cursor + 1), Some(Token::Ident(value)) if value == "std")
            && self.namespace_width_at(self.cursor + 2).is_some()
            && matches!(self.tokens.get(self.cursor + 4), Some(Token::Ident(value)) if value == "main")
            && matches!(
                self.tokens.get(self.cursor + 5),
                Some(Token::Symbol(Symbol::LeftParen))
            );
        let opened = self.std_namespace_open
            && matches!(self.tokens.get(self.cursor + 1), Some(Token::Ident(value)) if value == "main")
            && matches!(
                self.tokens.get(self.cursor + 2),
                Some(Token::Symbol(Symbol::LeftParen))
            );
        qualified || opened
    }

    fn parse_main(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword("AVInt")?;
        let mut name = self.expect_qualified_ident()?;
        if name == "main" && self.std_namespace_open {
            name = "std::main".to_owned();
        }
        if name != "std::main" {
            return Err(self.error_at_previous("executable entry must be `AVInt std::main()`"));
        }
        self.expect_symbol(Symbol::LeftParen)?;
        self.expect_symbol(Symbol::RightParen)?;
        let body = self.parse_block(0)?;
        Ok(FunctionDecl {
            name,
            params: Vec::new(),
            body,
        })
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, ParseError> {
        self.expect_keyword("func")?;
        let name = self.expect_ident()?;
        self.expect_symbol(Symbol::LeftParen)?;
        let mut params = Vec::new();

        if !self.check_symbol(Symbol::RightParen) {
            loop {
                params.push(self.expect_ident()?);
                if !self.eat_symbol(Symbol::Comma) {
                    break;
                }
            }
        }

        self.expect_symbol(Symbol::RightParen)?;
        let body = self.parse_block(0)?;

        Ok(FunctionDecl { name, params, body })
    }

    fn parse_while(&mut self) -> Result<WhileStatement, ParseError> {
        self.expect_keyword("while")?;
        self.expect_symbol(Symbol::LeftParen)?;
        let condition = self.parse_expression()?;
        self.expect_symbol(Symbol::RightParen)?;
        self.expect_symbol(Symbol::LeftParen)?;
        let limit = self.parse_expression()?;
        self.expect_symbol(Symbol::RightParen)?;
        let body = self.parse_block(0)?;
        let limit_body = if self.eat_word_after_optional_newlines("limit") {
            Some(self.parse_block(1)?)
        } else {
            None
        };
        let else_body = if self.eat_keyword_after_optional_newlines("else") {
            Some(self.parse_block(2)?)
        } else {
            None
        };

        Ok(WhileStatement {
            condition,
            limit,
            body,
            limit_body,
            else_body,
        })
    }

    fn parse_match(&mut self) -> Result<MatchStatement, ParseError> {
        self.expect_keyword("match")?;
        let value = self.parse_expression()?;
        self.expect_symbol(Symbol::LeftBrace)?;
        self.expect_newline("match body must start on a new line")?;
        self.skip_newlines();
        let mut cases = Vec::new();
        let mut default_body = None;

        while !self.check_symbol(Symbol::RightBrace) {
            self.expect_keyword("case")?;
            if self.eat_keyword("default") {
                if default_body.is_some() {
                    return Err(self.error_at_previous("duplicate `case default`"));
                }
                default_body = Some(self.parse_block(cases.len())?);
                self.expect_statement_end()?;
                self.skip_newlines();
                if !self.check_symbol(Symbol::RightBrace) {
                    return Err(self.error("`case default` must be last"));
                }
                continue;
            }

            let mut patterns = Vec::new();
            loop {
                let start = self.parse_expression()?;
                let pattern = if self.eat_symbol(Symbol::Range) {
                    MatchPattern::InclusiveRange {
                        start,
                        end: self.parse_expression()?,
                    }
                } else {
                    MatchPattern::Value(start)
                };
                patterns.push(pattern);
                if !self.eat_symbol(Symbol::Comma) {
                    break;
                }
            }
            let body = self.parse_block(cases.len())?;
            cases.push(MatchCase { patterns, body });
            self.expect_statement_end()?;
            self.skip_newlines();
        }
        self.expect_symbol(Symbol::RightBrace)?;

        if cases.is_empty() && default_body.is_none() {
            return Err(self.error_at_previous("match requires at least one case"));
        }
        Ok(MatchStatement {
            value,
            cases,
            default_body,
        })
    }

    fn parse_error_return(&mut self) -> Result<Statement, ParseError> {
        self.expect_symbol(Symbol::LeftParen)?;
        self.expect_keyword("ERR")?;
        self.expect_symbol(Symbol::Assign)?;
        self.expect_symbol(Symbol::LeftBracket)?;
        let message = self.parse_expression()?;
        self.expect_symbol(Symbol::RightBracket)?;
        self.expect_symbol(Symbol::LeftBracket)?;
        let code = self.parse_expression()?;
        self.expect_symbol(Symbol::RightBracket)?;
        self.expect_symbol(Symbol::RightParen)?;
        Ok(Statement::ErrorReturn { message, code })
    }

    fn parse_pick(&mut self) -> Result<PickStatement, ParseError> {
        self.expect_keyword("pick")?;
        self.expect_symbol(Symbol::LeftParen)?;
        let value = self.parse_expression()?;
        self.expect_symbol(Symbol::RightParen)?;
        self.expect_symbol(Symbol::Colon)?;
        let item_name = self.expect_ident()?;
        let body = self.parse_block(0)?;

        Ok(PickStatement {
            value,
            item_name,
            body,
        })
    }

    fn looks_like_info_assignment(&self) -> bool {
        matches!(
            (
                self.tokens.get(self.cursor),
                self.tokens.get(self.cursor + 1),
                self.tokens.get(self.cursor + 2),
                self.tokens.get(self.cursor + 3),
            ),
            (
                Some(Token::Ident(_)),
                Some(Token::Symbol(Symbol::Comma)),
                Some(Token::Ident(_)),
                Some(Token::Symbol(Symbol::Assign)),
            )
        )
    }

    fn parse_info_assignment(&mut self) -> Result<InfoAssignment, ParseError> {
        let type_target = self.expect_ident()?;
        self.expect_symbol(Symbol::Comma)?;
        let protection_target = self.expect_ident()?;
        self.expect_symbol(Symbol::Assign)?;
        self.expect_keyword("info")?;
        self.expect_symbol(Symbol::LeftParen)?;
        let source = self.expect_ident()?;
        self.expect_symbol(Symbol::RightParen)?;

        Ok(InfoAssignment {
            type_target,
            protection_target,
            source,
        })
    }

    fn parse_add_to_list(&mut self) -> Result<Statement, ParseError> {
        self.expect_std_function("add")?;
        self.expect_symbol(Symbol::LeftParen)?;
        let list_name = self.expect_ident()?;
        self.expect_symbol(Symbol::Comma)?;
        let element = self.parse_list_element()?;
        self.expect_symbol(Symbol::RightParen)?;

        Ok(Statement::AddToList { list_name, element })
    }

    fn parse_set_list_element(&mut self) -> Result<Statement, ParseError> {
        self.expect_std_function("set")?;
        self.expect_symbol(Symbol::LeftParen)?;
        let list_name = self.expect_ident()?;
        self.expect_symbol(Symbol::Comma)?;
        let index = self.parse_expression()?;
        self.expect_symbol(Symbol::Comma)?;
        let element = self.parse_list_element()?;
        self.expect_symbol(Symbol::RightParen)?;

        Ok(Statement::SetListElement {
            list_name,
            index,
            element,
        })
    }

    fn parse_if(&mut self) -> Result<IfStatement, ParseError> {
        self.expect_keyword("if")?;
        let condition = self.parse_expression()?;
        let body = self.parse_block(0)?;
        let mut else_if_branches = Vec::new();
        let mut else_body = None;

        while self.eat_keyword_after_optional_newlines("else") {
            if self.eat_keyword("if") {
                let condition = self.parse_expression()?;
                let body = self.parse_block(else_if_branches.len() + 1)?;
                else_if_branches.push(ElseIfBranch { condition, body });
            } else {
                else_body = Some(self.parse_block(else_if_branches.len() + 1)?);
                break;
            }
        }

        Ok(IfStatement {
            condition,
            body,
            else_if_branches,
            else_body,
        })
    }

    fn parse_block(&mut self, segment: usize) -> Result<Vec<Statement>, ParseError> {
        self.expect_symbol(Symbol::LeftBrace)?;
        self.expect_newline("block body must start on a new line")?;
        self.skip_newlines();
        let mut statements = Vec::new();

        while !self.check_symbol(Symbol::RightBrace) {
            if self.is_done() {
                return Err(self.error("expected `}`"));
            }
            let statement = if self.track_locations {
                let mut path = self.current_path.clone();
                path.extend([segment, statements.len()]);
                self.parse_statement_at(path)?
            } else {
                self.parse_statement()?
            };
            statements.push(statement);
            self.expect_statement_end()?;
            self.skip_newlines();
        }

        self.expect_symbol(Symbol::RightBrace)?;

        Ok(statements)
    }

    fn parse_assignment(&mut self) -> Result<Assignment, ParseError> {
        let name = self.expect_ident()?;
        if self.eat_symbol(Symbol::Increment) {
            return Ok(Assignment {
                name,
                operator: AssignmentOperator::AddAssign,
                value: Expression::Literal(Value::Int(1)),
            });
        }
        if self.eat_symbol(Symbol::Decrement) {
            return Ok(Assignment {
                name,
                operator: AssignmentOperator::SubAssign,
                value: Expression::Literal(Value::Int(1)),
            });
        }
        let operator = match self.next() {
            Some(Token::Symbol(Symbol::Assign)) => AssignmentOperator::Assign,
            Some(Token::Symbol(Symbol::AddAssign)) => AssignmentOperator::AddAssign,
            Some(Token::Symbol(Symbol::SubAssign)) => AssignmentOperator::SubAssign,
            Some(Token::Symbol(Symbol::MulAssign)) => AssignmentOperator::MulAssign,
            Some(Token::Symbol(Symbol::DivAssign)) => AssignmentOperator::DivAssign,
            Some(Token::Symbol(Symbol::RemAssign)) => AssignmentOperator::RemAssign,
            Some(_) => return Err(self.error_at_previous("expected assignment operator")),
            None => return Err(self.error("expected assignment operator")),
        };
        let value = self.parse_expression()?;

        Ok(Assignment {
            name,
            operator,
            value,
        })
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_and()?;

        while self.eat_keyword("or") {
            self.require_grouped_logic_operand(&expression)?;
            let right = self.parse_and()?;
            self.require_grouped_logic_operand(&right)?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::Or,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_and(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_comparison()?;

        while self.eat_keyword("and") {
            self.require_grouped_logic_operand(&expression)?;
            let right = self.parse_comparison()?;
            self.require_grouped_logic_operand(&right)?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: BinaryOperator::And,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn require_grouped_logic_operand(&self, expression: &Expression) -> Result<(), ParseError> {
        match expression {
            Expression::Grouped(_)
            | Expression::Variable(_)
            | Expression::Literal(Value::Bool(_)) => Ok(()),
            _ => {
                Err(self.error("logical operands with `and`/`or` must be grouped with parentheses"))
            }
        }
    }

    fn parse_comparison(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_term()?;

        let Some(operator) = self.peek_comparison_operator() else {
            return Ok(left);
        };

        self.next();
        let right = self.parse_term()?;

        Ok(Expression::Binary {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }

    fn parse_term(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_factor()?;

        loop {
            let operator = if self.eat_symbol(Symbol::Plus) {
                BinaryOperator::Add
            } else if self.eat_symbol(Symbol::Minus) {
                BinaryOperator::Sub
            } else {
                break;
            };

            let right = self.parse_factor()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_factor(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_power()?;

        loop {
            let operator = if self.eat_symbol(Symbol::Star) {
                BinaryOperator::Mul
            } else if self.eat_symbol(Symbol::Slash) {
                BinaryOperator::Div
            } else if self.eat_symbol(Symbol::Percent) {
                BinaryOperator::Rem
            } else {
                break;
            };

            let right = self.parse_power()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            };
        }

        Ok(expression)
    }

    fn parse_power(&mut self) -> Result<Expression, ParseError> {
        let left = self.parse_unary()?;
        if !self.eat_symbol(Symbol::Power) {
            return Ok(left);
        }

        let right = self.parse_power()?;
        Ok(Expression::Binary {
            left: Box::new(left),
            operator: BinaryOperator::Pow,
            right: Box::new(right),
        })
    }

    fn parse_unary(&mut self) -> Result<Expression, ParseError> {
        if self.eat_symbol(Symbol::Minus) {
            if matches!(self.peek(), Some(Token::IntMinMagnitude)) {
                self.next();
                return Ok(Expression::Literal(Value::Int(i64::MIN)));
            }

            return Ok(Expression::Unary {
                operator: UnaryOperator::Negate,
                expression: Box::new(self.parse_unary()?),
            });
        }

        if self.eat_keyword("not") {
            return Ok(Expression::Unary {
                operator: UnaryOperator::Not,
                expression: Box::new(self.parse_unary()?),
            });
        }

        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expression, ParseError> {
        let mut expression = self.parse_primary()?;

        while self.eat_symbol(Symbol::LeftBracket) {
            expression = self.parse_index_or_slice(expression)?;
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, ParseError> {
        match self.next() {
            Some(Token::Int(value)) => Ok(Expression::Literal(Value::Int(value))),
            Some(Token::IntMinMagnitude) => {
                Err(self.error_at_previous("integer literal out of range"))
            }
            Some(Token::Float(value)) => Ok(Expression::Literal(Value::Float(value))),
            Some(Token::Str(value)) => Ok(Expression::Literal(Value::Str(value))),
            Some(Token::Keyword(value)) if value == "true" => {
                Ok(Expression::Literal(Value::Bool(true)))
            }
            Some(Token::Keyword(value)) if value == "false" => {
                Ok(Expression::Literal(Value::Bool(false)))
            }
            Some(Token::Keyword(value)) if value == "NONE" => Ok(Expression::None),
            Some(Token::Keyword(value)) if value == "input" => Ok(Expression::Input),
            Some(Token::Keyword(value)) if value == "secret" => {
                self.expect_keyword("input")?;
                Ok(Expression::SecretInput)
            }
            Some(Token::Symbol(Symbol::LeftBracket)) => {
                self.cursor -= 1;
                self.parse_list_literal_elements()
                    .map(Expression::ListLiteral)
            }
            Some(Token::Ident(mut name)) => {
                if self.namespace_width_at(self.cursor).is_some() {
                    let in_index_expression = self.inside_index_expression(self.cursor - 1);
                    if !in_index_expression {
                        while self.eat_namespace() {
                            name.push_str("::");
                            name.push_str(&self.expect_name_segment()?);
                        }
                    }
                }
                if self.eat_symbol(Symbol::SelfCheck) {
                    Ok(Expression::SelfCheck(name))
                } else if self.eat_symbol(Symbol::LeftParen) {
                    let args = self.parse_call_args()?;
                    Ok(Expression::Call { name, args })
                } else {
                    Ok(Expression::Variable(name.replace("::", ".")))
                }
            }
            Some(Token::Symbol(Symbol::LeftParen)) => {
                let expression = self.parse_expression()?;
                self.expect_symbol(Symbol::RightParen)?;
                Ok(Expression::Grouped(Box::new(expression)))
            }
            Some(_) => Err(self.error_at_previous("expected expression")),
            None => Err(self.error("expected expression")),
        }
    }

    fn inside_index_expression(&self, index: usize) -> bool {
        let mut depth = 0usize;
        for cursor in (0..index).rev() {
            match self.tokens.get(cursor) {
                Some(Token::Symbol(Symbol::RightBracket)) => depth += 1,
                Some(Token::Symbol(Symbol::LeftBracket)) if depth > 0 => depth -= 1,
                Some(Token::Symbol(Symbol::LeftBracket)) => {
                    return matches!(
                        self.tokens.get(cursor.wrapping_sub(1)),
                        Some(Token::Ident(_) | Token::Str(_))
                            | Some(Token::Symbol(Symbol::RightParen | Symbol::RightBracket))
                    );
                }
                _ => {}
            }
        }
        false
    }

    fn parse_index_or_slice(&mut self, target: Expression) -> Result<Expression, ParseError> {
        if self.eat_symbol(Symbol::Colon) {
            let end = if self.check_symbol(Symbol::Colon) || self.check_symbol(Symbol::RightBracket)
            {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            };
            let step = if self.eat_symbol(Symbol::Colon) {
                if self.check_symbol(Symbol::RightBracket) {
                    None
                } else {
                    Some(Box::new(self.parse_expression()?))
                }
            } else {
                None
            };
            self.expect_symbol(Symbol::RightBracket)?;
            return Ok(Expression::Slice {
                target: Box::new(target),
                start: None,
                end,
                step,
            });
        }

        let first = self.parse_expression()?;

        if !self.eat_symbol(Symbol::Colon) {
            self.expect_symbol(Symbol::RightBracket)?;
            return Ok(Expression::Index {
                target: Box::new(target),
                index: Box::new(first),
            });
        }

        let end = if self.check_symbol(Symbol::Colon) || self.check_symbol(Symbol::RightBracket) {
            None
        } else {
            Some(Box::new(self.parse_expression()?))
        };
        let step = if self.eat_symbol(Symbol::Colon) {
            if self.check_symbol(Symbol::RightBracket) {
                None
            } else {
                Some(Box::new(self.parse_expression()?))
            }
        } else {
            None
        };
        self.expect_symbol(Symbol::RightBracket)?;

        Ok(Expression::Slice {
            target: Box::new(target),
            start: Some(Box::new(first)),
            end,
            step,
        })
    }

    fn parse_list_literal_elements(&mut self) -> Result<Vec<ListElement>, ParseError> {
        self.expect_symbol(Symbol::LeftBracket)?;
        let mut elements = Vec::new();

        if self.eat_symbol(Symbol::RightBracket) {
            return Ok(elements);
        }

        loop {
            elements.push(self.parse_list_element()?);
            if self.eat_symbol(Symbol::Comma) {
                continue;
            }

            self.expect_symbol(Symbol::RightBracket)?;
            break;
        }

        Ok(elements)
    }

    fn parse_list_element(&mut self) -> Result<ListElement, ParseError> {
        let value = self.parse_expression()?;
        let protection = if self.eat_symbol(Symbol::Colon) {
            Some(self.expect_protection_level()?)
        } else {
            None
        };

        Ok(ListElement { value, protection })
    }

    fn parse_call_args(&mut self) -> Result<Vec<Expression>, ParseError> {
        let mut args = Vec::new();

        if self.eat_symbol(Symbol::RightParen) {
            return Ok(args);
        }

        loop {
            args.push(self.parse_expression()?);
            if self.eat_symbol(Symbol::Comma) {
                continue;
            }

            self.expect_symbol(Symbol::RightParen)?;
            break;
        }

        Ok(args)
    }

    fn peek_comparison_operator(&self) -> Option<BinaryOperator> {
        match self.peek() {
            Some(Token::Symbol(Symbol::EqualEqual)) => Some(BinaryOperator::Equal),
            Some(Token::Symbol(Symbol::NotEqual)) => Some(BinaryOperator::NotEqual),
            Some(Token::Symbol(Symbol::Greater)) => Some(BinaryOperator::Greater),
            Some(Token::Symbol(Symbol::GreaterEqual)) => Some(BinaryOperator::GreaterEqual),
            Some(Token::Symbol(Symbol::Less)) => Some(BinaryOperator::Less),
            Some(Token::Symbol(Symbol::LessEqual)) => Some(BinaryOperator::LessEqual),
            _ => None,
        }
    }

    fn peek_decl_keyword(&self) -> Option<(apl_core::VariableKind, ValueType)> {
        match self.peek() {
            Some(Token::Keyword(value)) => ValueType::parse_decl_keyword(value),
            _ => None,
        }
    }

    fn expect_ident(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Token::Ident(value)) => Ok(value),
            Some(_) => Err(self.error_at_previous("expected identifier")),
            None => Err(self.error("expected identifier")),
        }
    }

    fn expect_name_segment(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Token::Ident(value)) | Some(Token::Keyword(value)) => Ok(value),
            Some(_) => Err(self.error_at_previous("expected namespace name")),
            None => Err(self.error("expected namespace name")),
        }
    }

    fn expect_qualified_ident(&mut self) -> Result<String, ParseError> {
        let mut name = self.expect_ident()?;
        while self.eat_namespace() {
            name.push_str("::");
            name.push_str(&self.expect_name_segment()?);
        }
        Ok(name)
    }

    fn qualified_name_end(&self, start: usize) -> Option<usize> {
        if !matches!(self.tokens.get(start), Some(Token::Ident(_))) {
            return None;
        }
        let mut end = start + 1;
        while let Some(width) = self.namespace_width_at(end) {
            if !matches!(
                self.tokens.get(end + width),
                Some(Token::Ident(_) | Token::Keyword(_))
            ) {
                return None;
            }
            end += width + 1;
        }
        Some(end)
    }

    fn namespace_width_at(&self, index: usize) -> Option<usize> {
        if matches!(self.tokens.get(index), Some(Token::Symbol(Symbol::Colon)))
            && matches!(
                self.tokens.get(index + 1),
                Some(Token::Symbol(Symbol::Colon))
            )
        {
            Some(2)
        } else {
            None
        }
    }

    fn eat_namespace(&mut self) -> bool {
        let Some(width) = self.namespace_width_at(self.cursor) else {
            return false;
        };
        self.cursor += width;
        true
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.next() {
            Some(Token::Str(value)) => Ok(value),
            Some(_) => Err(self.error_at_previous("expected string literal")),
            None => Err(self.error("expected string literal")),
        }
    }

    fn expect_protection_level(&mut self) -> Result<ProtectionLevel, ParseError> {
        match self.next() {
            Some(Token::Keyword(value)) if value == "AV" => Ok(ProtectionLevel::Av),
            Some(Token::Keyword(value)) if value == "ASV" => Ok(ProtectionLevel::Asv),
            Some(Token::Keyword(value)) if value == "SASV" => Ok(ProtectionLevel::Sasv),
            Some(_) => Err(self.error_at_previous("expected protection level AV, ASV, or SASV")),
            None => Err(self.error("expected protection level AV, ASV, or SASV")),
        }
    }

    fn expect_keyword(&mut self, expected: &'static str) -> Result<(), ParseError> {
        if self.eat_keyword(expected) {
            Ok(())
        } else {
            Err(self.error(format!("expected `{expected}`")))
        }
    }

    fn eat_keyword(&mut self, expected: &str) -> bool {
        match self.peek() {
            Some(Token::Keyword(value)) if value == expected => {
                self.cursor += 1;
                true
            }
            _ => false,
        }
    }

    fn check_keyword(&self, expected: &str) -> bool {
        matches!(self.peek(), Some(Token::Keyword(value)) if value == expected)
    }

    fn check_std_function(&self, expected: &str) -> bool {
        ((!self.strict_executable || self.std_namespace_open)
            && matches!(self.peek(), Some(Token::Ident(value) | Token::Keyword(value)) if value == expected))
            || (matches!(self.tokens.get(self.cursor), Some(Token::Ident(value)) if value == "std")
                && self.namespace_width_at(self.cursor + 1).is_some()
                && matches!(self.tokens.get(self.cursor + 3), Some(Token::Ident(value) | Token::Keyword(value)) if value == expected))
    }

    fn eat_std_function(&mut self, expected: &str) -> bool {
        if !self.check_std_function(expected) {
            return false;
        }
        if matches!(self.tokens.get(self.cursor), Some(Token::Ident(value)) if value == "std") {
            self.cursor += 4;
        } else {
            self.cursor += 1;
        }
        true
    }

    fn expect_std_function(&mut self, expected: &str) -> Result<(), ParseError> {
        if self.eat_std_function(expected) {
            Ok(())
        } else {
            Err(self.error(format!("expected `std::{expected}`")))
        }
    }

    fn check_symbol(&self, expected: Symbol) -> bool {
        matches!(self.peek(), Some(Token::Symbol(value)) if *value == expected)
    }

    fn next_starts_statement_or_block_end(&self) -> bool {
        match self.peek() {
            None | Some(Token::Newline | Token::Symbol(Symbol::RightBrace)) => true,
            Some(Token::Keyword(value)) => {
                matches!(
                    value.as_str(),
                    "include"
                        | "exclude"
                        | "if"
                        | "func"
                        | "while"
                        | "pick"
                        | "break"
                        | "continue"
                        | "return"
                        | "secretup"
                        | "stop"
                        | "fail"
                        | "out"
                        | "VTime"
                        | "List"
                ) || ValueType::parse_decl_keyword(value).is_some()
            }
            _ => false,
        }
    }

    fn expect_symbol(&mut self, expected: Symbol) -> Result<(), ParseError> {
        if self.eat_symbol(expected) {
            Ok(())
        } else {
            Err(self.error(format!("expected `{}`", expected.display())))
        }
    }

    fn eat_symbol(&mut self, expected: Symbol) -> bool {
        match self.peek() {
            Some(Token::Symbol(value)) if *value == expected => {
                self.cursor += 1;
                true
            }
            _ => false,
        }
    }

    fn eat_word(&mut self, expected: &str) -> bool {
        match self.peek() {
            Some(Token::Keyword(value) | Token::Ident(value)) if value == expected => {
                self.cursor += 1;
                true
            }
            _ => false,
        }
    }

    fn eat_keyword_after_optional_newlines(&mut self, expected: &str) -> bool {
        let checkpoint = self.cursor;
        self.skip_newlines();
        if self.eat_keyword(expected) {
            true
        } else {
            self.cursor = checkpoint;
            false
        }
    }

    fn eat_word_after_optional_newlines(&mut self, expected: &str) -> bool {
        let checkpoint = self.cursor;
        self.skip_newlines();
        if self.eat_word(expected) {
            true
        } else {
            self.cursor = checkpoint;
            false
        }
    }

    fn expect_statement_end(&self) -> Result<(), ParseError> {
        if self.is_done() || matches!(self.peek(), Some(Token::Newline)) {
            Ok(())
        } else {
            Err(self.error("expected end of line after statement"))
        }
    }

    fn expect_newline(&mut self, message: &'static str) -> Result<(), ParseError> {
        if matches!(self.peek(), Some(Token::Newline)) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(self.error(message))
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Some(Token::Newline)) {
            self.cursor += 1;
        }
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.cursor).cloned();
        self.cursor += usize::from(token.is_some());
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.cursor)
    }

    fn is_done(&self) -> bool {
        self.cursor >= self.tokens.len()
    }

    fn error(&self, message: impl Into<String>) -> ParseError {
        let offset = self
            .offsets
            .get(self.cursor)
            .copied()
            .unwrap_or(self.source.len());
        parse_error_at(self.source, offset, message)
    }

    fn error_at_previous(&self, message: impl Into<String>) -> ParseError {
        let offset = self
            .cursor
            .checked_sub(1)
            .and_then(|index| self.offsets.get(index))
            .copied()
            .unwrap_or(self.source.len());
        parse_error_at(self.source, offset, message)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
    pub line: usize,
    pub column: usize,
}

struct Tokenized {
    tokens: Vec<Token>,
    offsets: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Newline,
    Keyword(String),
    Ident(String),
    Int(i64),
    IntMinMagnitude,
    Float(f64),
    Str(String),
    Symbol(Symbol),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Symbol {
    At,
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    RemAssign,
    Increment,
    Decrement,
    Power,
    EqualEqual,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    RightShift,
    LeftShift,
    SelfCheck,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Comma,
    Colon,
    Range,
}

impl Symbol {
    fn display(self) -> &'static str {
        match self {
            Self::At => "@",
            Self::Assign => "=",
            Self::AddAssign => "+=",
            Self::SubAssign => "-=",
            Self::MulAssign => "*=",
            Self::DivAssign => "/=",
            Self::RemAssign => "%=",
            Self::Increment => "++",
            Self::Decrement => "--",
            Self::Power => "**",
            Self::EqualEqual => "==",
            Self::NotEqual => "!=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::RightShift => ">>",
            Self::LeftShift => "<<",
            Self::SelfCheck => "=self=",
            Self::Plus => "+",
            Self::Minus => "-",
            Self::Star => "*",
            Self::Slash => "/",
            Self::Percent => "%",
            Self::LeftParen => "(",
            Self::RightParen => ")",
            Self::LeftBrace => "{",
            Self::RightBrace => "}",
            Self::LeftBracket => "[",
            Self::RightBracket => "]",
            Self::Comma => ",",
            Self::Colon => ":",
            Self::Range => "..",
        }
    }
}

fn tokenize(source: &str) -> Result<Tokenized, ParseError> {
    let mut tokens = Vec::new();
    let mut offsets = Vec::new();
    let mut cursor = 0;
    let mut continuation_depth = 0usize;

    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();

        if ch == '\n' || ch == '\r' {
            cursor += ch.len_utf8();
            if ch == '\r' && source[cursor..].starts_with('\n') {
                cursor += 1;
            }
            if continuation_depth == 0 && !matches!(tokens.last(), Some(Token::Newline)) {
                offsets.push(cursor);
                tokens.push(Token::Newline);
            }
            continue;
        }

        if ch.is_whitespace() {
            cursor += ch.len_utf8();
            continue;
        }

        if ch == '#' {
            while cursor < source.len()
                && !source[cursor..].starts_with('\n')
                && !source[cursor..].starts_with('\r')
            {
                cursor += source[cursor..].chars().next().unwrap().len_utf8();
            }
            continue;
        }

        if ch == '"' {
            let (value, next_cursor) = read_string(source, cursor)?;
            offsets.push(cursor);
            tokens.push(Token::Str(value));
            cursor = next_cursor;
            continue;
        }

        if ch.is_ascii_digit() {
            let (token, next_cursor) = read_number(source, cursor)?;
            offsets.push(cursor);
            tokens.push(token);
            cursor = next_cursor;
            continue;
        }

        if is_ident_start(ch) {
            let start = cursor;
            cursor += ch.len_utf8();
            while cursor < source.len() {
                let next = source[cursor..].chars().next().unwrap();
                if !is_ident_continue(next) {
                    break;
                }
                cursor += next.len_utf8();
            }
            let value = &source[start..cursor];
            offsets.push(start);
            if is_keyword(value) || ValueType::parse_decl_keyword(value).is_some() {
                tokens.push(Token::Keyword(value.to_owned()));
            } else {
                tokens.push(Token::Ident(value.to_owned()));
            }
            continue;
        }

        let (symbol, next_cursor) = read_symbol(source, cursor)?;
        offsets.push(cursor);
        tokens.push(Token::Symbol(symbol));
        match symbol {
            Symbol::LeftParen | Symbol::LeftBracket => continuation_depth += 1,
            Symbol::RightParen | Symbol::RightBracket => {
                continuation_depth = continuation_depth.saturating_sub(1)
            }
            _ => {}
        }
        cursor = next_cursor;
    }

    Ok(Tokenized { tokens, offsets })
}

fn tokens_open_std_namespace(tokens: &[Token]) -> bool {
    let mut index = 0;
    while index < tokens.len() {
        if matches!(tokens.get(index), Some(Token::Keyword(value)) if value == "using")
            && matches!(tokens.get(index + 1), Some(Token::Keyword(value)) if value == "namespace")
            && matches!(tokens.get(index + 2), Some(Token::Ident(value)) if value == "std")
        {
            return true;
        }
        if matches!(tokens.get(index), Some(Token::Keyword(value)) if value == "nc") {
            let mut cursor = index + 1;
            loop {
                if matches!(tokens.get(cursor), Some(Token::Ident(value)) if value == "std") {
                    return true;
                }
                cursor += 1;
                if !matches!(tokens.get(cursor), Some(Token::Symbol(Symbol::Comma))) {
                    break;
                }
                cursor += 1;
            }
        }
        index += 1;
    }
    false
}

fn tokens_have_main(tokens: &[Token], std_namespace_open: bool) -> bool {
    let qualified = tokens.windows(6).any(|window| {
        matches!(&window[0], Token::Keyword(value) if value == "AVInt")
            && matches!(&window[1], Token::Ident(value) if value == "std")
            && matches!(&window[2], Token::Symbol(Symbol::Colon))
            && matches!(&window[3], Token::Symbol(Symbol::Colon))
            && matches!(&window[4], Token::Ident(value) if value == "main")
            && matches!(&window[5], Token::Symbol(Symbol::LeftParen))
    });
    let opened = std_namespace_open
        && tokens.windows(3).any(|window| {
            matches!(&window[0], Token::Keyword(value) if value == "AVInt")
                && matches!(&window[1], Token::Ident(value) if value == "main")
                && matches!(&window[2], Token::Symbol(Symbol::LeftParen))
        });
    qualified || opened
}

fn read_string(source: &str, start: usize) -> Result<(String, usize), ParseError> {
    let mut cursor = start + 1;
    let mut value = String::new();

    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if ch == '"' {
            return Ok((value, cursor + 1));
        }

        if ch == '\\' {
            let escape_start = cursor;
            cursor += ch.len_utf8();
            if cursor >= source.len() {
                return Err(parse_error_at(
                    source,
                    escape_start,
                    "unterminated escape sequence",
                ));
            }

            let escaped = source[cursor..].chars().next().unwrap();
            let decoded = match escaped {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '0' => '\0',
                '\\' => '\\',
                '"' => '"',
                _ => {
                    return Err(parse_error_at(
                        source,
                        escape_start,
                        format!("invalid escape sequence `\\{escaped}`"),
                    ))
                }
            };
            value.push(decoded);
            cursor += escaped.len_utf8();
            continue;
        }

        value.push(ch);
        cursor += ch.len_utf8();
    }

    Err(parse_error_at(source, start, "unterminated string literal"))
}

fn read_number(source: &str, start: usize) -> Result<(Token, usize), ParseError> {
    let mut cursor = start;
    let mut has_dot = false;

    while cursor < source.len() {
        let ch = source[cursor..].chars().next().unwrap();
        if ch == '.' && !has_dot && !source[cursor + 1..].starts_with('.') {
            has_dot = true;
            cursor += 1;
        } else if ch.is_ascii_digit() {
            cursor += ch.len_utf8();
        } else {
            break;
        }
    }

    let raw = &source[start..cursor];
    if has_dot {
        if raw.ends_with('.') {
            return Err(parse_error_at(
                source,
                start,
                format!("invalid float literal `{raw}`"),
            ));
        }

        let value = raw
            .parse::<f64>()
            .map_err(|_| parse_error_at(source, start, format!("invalid float literal `{raw}`")))?;
        Ok((Token::Float(value), cursor))
    } else if raw == "9223372036854775808" {
        Ok((Token::IntMinMagnitude, cursor))
    } else {
        let value = raw
            .parse::<i64>()
            .map_err(|_| parse_error_at(source, start, format!("invalid int literal `{raw}`")))?;
        Ok((Token::Int(value), cursor))
    }
}

fn read_symbol(source: &str, cursor: usize) -> Result<(Symbol, usize), ParseError> {
    let rest = &source[cursor..];
    let pairs = [
        ("=self=", Symbol::SelfCheck),
        ("+=", Symbol::AddAssign),
        ("-=", Symbol::SubAssign),
        ("*=", Symbol::MulAssign),
        ("/=", Symbol::DivAssign),
        ("%=", Symbol::RemAssign),
        ("++", Symbol::Increment),
        ("--", Symbol::Decrement),
        ("**", Symbol::Power),
        ("==", Symbol::EqualEqual),
        ("!=", Symbol::NotEqual),
        (">=", Symbol::GreaterEqual),
        ("<=", Symbol::LessEqual),
        (">>", Symbol::RightShift),
        ("<<", Symbol::LeftShift),
        ("..", Symbol::Range),
    ];

    for (raw, symbol) in pairs {
        if rest.starts_with(raw) {
            return Ok((symbol, cursor + raw.len()));
        }
    }

    let ch = rest.chars().next().unwrap();
    let symbol = match ch {
        '@' => Symbol::At,
        '=' => Symbol::Assign,
        '>' => Symbol::Greater,
        '<' => Symbol::Less,
        '+' => Symbol::Plus,
        '-' => Symbol::Minus,
        '*' => Symbol::Star,
        '/' => Symbol::Slash,
        '%' => Symbol::Percent,
        '(' => Symbol::LeftParen,
        ')' => Symbol::RightParen,
        '{' => Symbol::LeftBrace,
        '}' => Symbol::RightBrace,
        '[' => Symbol::LeftBracket,
        ']' => Symbol::RightBracket,
        ',' => Symbol::Comma,
        ':' => Symbol::Colon,
        _ => {
            return Err(parse_error_at(
                source,
                cursor,
                format!("unexpected character `{ch}`"),
            ))
        }
    };

    Ok((symbol, cursor + ch.len_utf8()))
}

fn parse_error_at(source: &str, offset: usize, message: impl Into<String>) -> ParseError {
    let prefix = &source[..offset];
    let mut line = 1;
    let mut column = 1;
    let mut chars = prefix.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                line += 1;
                column = 1;
            }
            '\n' => {
                line += 1;
                column = 1;
            }
            _ => column += 1,
        }
    }
    ParseError {
        message: message.into(),
        offset,
        line,
        column,
    }
}

fn is_keyword(value: &str) -> bool {
    matches!(
        value,
        "if" | "else"
            | "include"
            | "exclude"
            | "using"
            | "namespace"
            | "nc"
            | "while"
            | "match"
            | "case"
            | "default"
            | "pick"
            | "class"
            | "struct"
            | "override"
            | "break"
            | "continue"
            | "func"
            | "return"
            | "ERR"
            | "true"
            | "false"
            | "and"
            | "or"
            | "not"
            | "out"
            | "input"
            | "secret"
            | "secretup"
            | "info"
            | "stop"
            | "fail"
            | "NONE"
            | "VTime"
            | "List"
            | "AV"
            | "ASV"
            | "SASV"
    )
}

fn is_ident_start(ch: char) -> bool {
    ch == '_' || ch.is_ascii_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '_' || ch == '.' || ch.is_ascii_alphanumeric()
}

#[cfg(test)]
mod tests {
    use super::*;
    use apl_core::{validate_program, VariableKind};

    #[test]
    fn parses_struct_surface_and_value_copy_declarations() {
        let program = parse_program(
            r#"struct Point {
  AVInt x = 0
  AVInt y = 0
}
struct Shape {
  Point origin = Point()
  List labels = []
}
AVInt std::main() {
  Shape first = Shape()
  Shape second = first
  return 0
}"#,
        )
        .unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::FunctionDecl(function) if function.name == "aplstruct.decl.Point"
        ));
        assert!(matches!(
            &program.statements[1],
            Statement::FunctionDecl(function) if function.name == "aplstruct.decl.Shape"
        ));
        let Statement::FunctionDecl(main) = &program.statements[2] else {
            panic!("expected main");
        };
        assert!(matches!(
            &main.body[0],
            Statement::VTimeDecl(declaration) if declaration.name == "first"
        ));
        assert!(matches!(
            &main.body[1],
            Statement::VTimeDecl(declaration) if declaration.name == "second"
        ));
    }

    #[test]
    fn rejects_methods_and_inheritance_inside_structs() {
        for source in [
            "struct Bad {\n  func method() {\n    return 0\n  }\n}",
            "struct Bad {\n  class::inheritance \"Base\"\n}",
            "struct Bad {\n  override func method() {\n    return 0\n  }\n}",
        ] {
            assert!(parse_program(source).is_err(), "accepted `{source}`");
        }
    }

    #[test]
    fn parses_class_surface_for_the_self_hosted_frontend() {
        let program = parse_program(
            r#"include >> classes
class Machine {
  AVStr color = "unknown"
  func __init__(color) {
    self.color = color
  }
  func __out__() {
    out self.color
  }
  func label() {
    return self.color
  }
}
class Vehicle {
  class::inheritance "Machine"
  class::exclude "label" method >> Machine
  func __init__(color) {
    self.color = color
  }
  override func __out__() {
    out self.color
  }
}
AVInt std::main() {
  Machine item = Vehicle("red")
  out item
  return 0
}"#,
        )
        .unwrap();

        assert!(program.statements.iter().any(|statement| {
            matches!(statement, Statement::FunctionDecl(function) if function.name == "aplclass.decl.Machine")
        }));
        let Statement::FunctionDecl(main) = program.statements.last().unwrap() else {
            panic!("expected main");
        };
        assert!(matches!(
            &main.body[0],
            Statement::VTimeDecl(declaration) if declaration.name == "item"
        ));
    }

    #[test]
    fn exposes_tolerant_editor_syntax_spans() {
        let source = "AVInt value = 12\n# note\nout \"unfinished";
        let spans = syntax_spans(source);

        assert!(spans.iter().any(|span| {
            span.kind == SyntaxKind::Keyword && &source[span.start..span.end] == "AVInt"
        }));
        assert!(spans.iter().any(|span| {
            span.kind == SyntaxKind::Number && &source[span.start..span.end] == "12"
        }));
        assert!(spans.iter().any(|span| {
            span.kind == SyntaxKind::Comment && &source[span.start..span.end] == "# note"
        }));
        assert!(spans.iter().any(|span| {
            span.kind == SyntaxKind::String && &source[span.start..span.end] == "\"unfinished"
        }));
    }

    #[test]
    fn maps_nested_statements_to_stable_paths() {
        let parsed = parse_program_with_locations(
            "AVInt std::main() {\n  if true {\n    out 1\n  } else {\n    out 2\n  }\n  return 0\n}",
        )
        .unwrap();

        assert!(parsed
            .locations
            .iter()
            .any(|location| location.path == [0, 0, 0] && location.line == 2));
        assert!(parsed
            .locations
            .iter()
            .any(|location| location.path == [0, 0, 0, 0, 0] && location.line == 3));
        assert!(parsed
            .locations
            .iter()
            .any(|location| location.path == [0, 0, 0, 1, 0] && location.line == 5));
        assert!(parsed
            .locations
            .iter()
            .any(|location| location.path == [0, 0, 1] && location.line == 7));
    }

    #[test]
    fn reports_tokenizer_line_and_unicode_column() {
        let error = parse_program("AVStr text = \"Привет\"\n  @").unwrap_err();

        assert_eq!(error.message, "expected identifier");
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 4);
        assert_eq!(error.offset, 31);
    }

    #[test]
    fn reports_parser_location_at_end_of_source() {
        let error = parse_program("AVInt value =\n").unwrap_err();

        assert_eq!(error.message, "expected expression");
        assert_eq!(error.line, 2);
        assert_eq!(error.column, 1);
        assert_eq!(error.offset, 14);
    }

    #[test]
    fn treats_cr_and_crlf_as_comment_line_endings() {
        for source in [
            "AVInt value = 1 # note\rout value",
            "AVInt value = 1 # note\r\nout value",
        ] {
            let program = parse_program(source).unwrap();
            assert_eq!(program.statements.len(), 2);
            assert_eq!(validate_program(&program), Ok(()));
        }

        let error = parse_program("out 1\r  @").unwrap_err();
        assert_eq!((error.line, error.column), (2, 4));
    }

    #[test]
    fn parses_minimum_int_literal() {
        let program = parse_program("AVInt value = -9223372036854775808").unwrap();

        let Statement::VariableDecl(declaration) = &program.statements[0] else {
            panic!("expected declaration");
        };
        assert_eq!(
            declaration.initial_value,
            Expression::Literal(Value::Int(i64::MIN))
        );
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn rejects_positive_int_beyond_i64_range() {
        let error = parse_program("AVInt value = 9223372036854775808").unwrap_err();

        assert_eq!(error.message, "integer literal out of range");
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 15);
    }

    #[test]
    fn rejects_int_below_i64_range() {
        let error = parse_program("AVInt value = -9223372036854775809").unwrap_err();

        assert_eq!(error.message, "invalid int literal `9223372036854775809`");
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 16);
    }

    #[test]
    fn rejects_float_without_fractional_digits() {
        let error = parse_program("AVFloat value = 1.").unwrap_err();

        assert_eq!(error.message, "invalid float literal `1.`");
        assert_eq!(error.line, 1);
        assert_eq!(error.column, 17);
    }

    #[test]
    fn rejects_unclosed_delimiters_and_bad_separators() {
        let invalid_sources = [
            "if true {\nout 1\n",
            "AVInt value = (1 + 2",
            "VTime value = len([1]",
            "List values = [1, 2",
            "VTime value = pow(2 3)",
            "List values = [1 2]",
            "VTime value = pow(2,)",
            "List values = [1]\nVTime value = values[0",
            "out 1\n} out 2",
            "if true {\nout 1\n\"}\"\n",
        ];

        for source in invalid_sources {
            assert!(
                parse_program(source).is_err(),
                "Rust parser unexpectedly accepted `{source}`"
            );
        }
    }

    #[test]
    fn requires_line_boundaries_between_statements_and_inside_blocks() {
        for source in [
            "out 1 out 2",
            "AVInt first = 1 AVInt second = 2",
            "func work() { return 1 }",
            "if true {\n  out 1 }",
            "while (true) (1) { out 1\n}",
            "pick([1]): item { out item\n}",
            "match 1 { case 1 {\n  out 1\n}\n}",
            "class Item {\n  AVInt value = 1 }",
        ] {
            assert!(
                parse_program(source).is_err(),
                "Rust parser unexpectedly accepted `{source}`"
            );
        }

        let program = parse_program(
            r#"AVInt std::main() { # body starts below
  out 1
  out 2
  return 0
}"#,
        )
        .unwrap();
        let Statement::FunctionDecl(main) = &program.statements[0] else {
            panic!("expected main function");
        };
        assert_eq!(main.body.len(), 3);
    }

    #[test]
    fn rejects_non_identifier_name_tokens_and_unterminated_strings() {
        let invalid_sources = [
            "out \"hello",
            "AVInt \"x\" = 1",
            "List \"items\" = []",
            "VTime \"temp\" = 1",
            "func \"f\"() { return 1 }",
            "func f(\"x\") { return 1 }",
            "func f(a b) { return a }",
            "func f(a,) { return a }",
            "pick([1]): \"item\" { out 1 }",
            "secretup(\"value\")",
            "type, \"level\" = info(source)",
            "type, level = info(\"source\")",
            "\"if\" true { out 1 }",
            "VTime value = 1 \"+\" 2",
            "VTime value = 1 \"==\" 1",
            "VTime value = true \"and\" false",
            "AVInt x = 1 VTime value = x \"=self=\"",
            "List values = [1 \":\" ASV]",
            "VTime value = secret \"input\"",
        ];

        for source in invalid_sources {
            assert!(
                parse_program(source).is_err(),
                "Rust parser unexpectedly accepted `{source}`"
            );
        }
    }

    #[test]
    fn decodes_supported_string_escapes() {
        let tokenized = tokenize(r#""line\ncarriage\r tab\t nul\0 slash\\ quote\"""#).unwrap();

        assert_eq!(
            tokenized.tokens,
            vec![Token::Str(
                "line\ncarriage\r tab\t nul\0 slash\\ quote\"".to_owned()
            )]
        );
    }

    #[test]
    fn rejects_invalid_and_unterminated_string_escapes() {
        let invalid = parse_program(r#"out "bad\q""#).unwrap_err();
        assert_eq!(invalid.message, "invalid escape sequence `\\q`");
        assert_eq!(invalid.line, 1);
        assert_eq!(invalid.column, 9);

        let unterminated = parse_program(r#"out "tail\"#).unwrap_err();
        assert_eq!(unterminated.message, "unterminated escape sequence");
        assert_eq!(unterminated.line, 1);
        assert_eq!(unterminated.column, 10);
    }

    #[test]
    fn parses_variable_declarations_and_arithmetic() {
        let program = parse_program(
            r#"
            AVInt x = 4
            AVInt y = (x + 2) * 3
            x += 1
            "#,
        )
        .unwrap();

        assert_eq!(program.statements.len(), 3);
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_power_as_right_associative_above_multiplication() {
        let program = parse_program("AVInt value = -2 ** 3 ** 2 * 4").unwrap();
        let Statement::VariableDecl(declaration) = &program.statements[0] else {
            panic!("expected declaration");
        };
        let Expression::Binary {
            left,
            operator: BinaryOperator::Mul,
            ..
        } = &declaration.initial_value
        else {
            panic!("expected multiplication at the expression root");
        };
        let Expression::Binary {
            left: power_base,
            operator: BinaryOperator::Pow,
            right: power_exponent,
        } = left.as_ref()
        else {
            panic!("expected power before multiplication");
        };
        assert!(matches!(
            power_base.as_ref(),
            Expression::Unary {
                operator: UnaryOperator::Negate,
                ..
            }
        ));
        assert!(matches!(
            power_exponent.as_ref(),
            Expression::Binary {
                operator: BinaryOperator::Pow,
                ..
            }
        ));
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_remainder_with_factor_precedence_and_compound_assignment() {
        let program =
            parse_program("AVInt value = 2 + 10 % 4 * 3\nAVInt cycle = 17\ncycle %= 5").unwrap();
        let Statement::VariableDecl(declaration) = &program.statements[0] else {
            panic!("expected declaration");
        };
        let Expression::Binary {
            operator: BinaryOperator::Add,
            right,
            ..
        } = &declaration.initial_value
        else {
            panic!("expected addition at the expression root");
        };
        assert!(matches!(
            right.as_ref(),
            Expression::Binary {
                operator: BinaryOperator::Mul,
                left,
                ..
            } if matches!(
                left.as_ref(),
                Expression::Binary {
                    operator: BinaryOperator::Rem,
                    ..
                }
            )
        ));
        assert!(matches!(
            &program.statements[2],
            Statement::Assignment(Assignment {
                operator: AssignmentOperator::RemAssign,
                ..
            })
        ));
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_secret_families() {
        let program = parse_program(
            r#"
            ASVStr token = "hidden"
            SASVStr master_key = "raw"
            "#,
        )
        .unwrap();

        let Statement::VariableDecl(first) = &program.statements[0] else {
            panic!("expected declaration");
        };
        let Statement::VariableDecl(second) = &program.statements[1] else {
            panic!("expected declaration");
        };

        assert_eq!(first.kind, VariableKind::Asv);
        assert_eq!(second.kind, VariableKind::Sasv);
    }

    #[test]
    fn parses_self_check() {
        let program = parse_program(
            r#"
            AVInt x = 4
            AVBool unchanged = x =self=
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn rejects_ungrouped_logical_conditions() {
        let error = parse_program(
            r#"
            AVBool a = true
            AVBool b = false
            if a == true and b == false {
              a = false
            }
            "#,
        )
        .unwrap_err();

        assert!(error.message.contains("logical operands"));
    }

    #[test]
    fn parses_grouped_if_else_chain() {
        let program = parse_program(
            r#"
            AVBool a = true
            AVBool b = false
            if (a == true) and (b == false) {
              b = true
            } else if a =self= {
              b = false
            } else {
              a = false
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_none_input_secret_input_out_and_info() {
        let program = parse_program(
            r#"
            AVInt age = input
            AVStr raw = NONE
            ASVStr token = secret input
            SASVStr master_key = "raw"
            AVStr typ = ""
            AVStr level = ""
            typ, level = info(master_key)
            if age != NONE {
              out age
            } else {
              out raw
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_while_with_limit_break_and_continue() {
        let program = parse_program(
            r#"
            AVInt x = 0
            while (x < 10) (-1) {
              x += 1
              if x == 3 {
                continue
              }
              if x == 5 {
                break
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_expression_while_limit() {
        let program = parse_program(
            r#"AVInt max_iterations = 2
AVInt count = 0
while (true) (max_iterations) {
  count += 1
}"#,
        )
        .unwrap();

        let Statement::While(loop_statement) = &program.statements[2] else {
            panic!("expected while statement");
        };
        assert_eq!(
            loop_statement.limit,
            Expression::Variable("max_iterations".to_owned())
        );
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_raw_memory_scope_attributes_at_the_source_boundary() {
        let program = parse_program(
            r#"include >> raw_memory
@marked
@collector
func work() {
  return 1
}
AVInt std::main() {
  @flee
  while (false) (1) {
    out 1
  }
  return 0
}"#,
        )
        .unwrap();
        assert!(
            matches!(&program.statements[1], Statement::FunctionDecl(function) if function.name == "work")
        );
        assert!(parse_program("@marked AVInt value = 1").is_err());
        assert!(parse_program("@unknown func work() { return 1 }").is_err());
    }

    #[test]
    fn parses_match_while_handlers_and_error_return() {
        let program = parse_program(
            r#"
            func abort() {
              return(ERR=["boom"][7])
            }
            AVInt value = 4
            while (value < 4) (2) {
              value += 1
            } limit {
              out "limit"
            } else {
              out "else"
            }
            match value {
              case 1, 2 {
                out "small"
              }
              case 3..5 {
                out "middle"
              }
              case default {
                abort()
              }
            }
            "#,
        )
        .unwrap();

        let Statement::FunctionDecl(function) = &program.statements[0] else {
            panic!("expected function declaration");
        };
        assert!(matches!(
            function.body.as_slice(),
            [Statement::ErrorReturn {
                message: Expression::Literal(Value::Str(message)),
                code: Expression::Literal(Value::Int(7)),
            }] if message == "boom"
        ));

        let Statement::While(loop_statement) = &program.statements[2] else {
            panic!("expected while statement");
        };
        assert!(loop_statement.limit_body.is_some());
        assert!(loop_statement.else_body.is_some());

        let Statement::Match(match_statement) = &program.statements[3] else {
            panic!("expected match statement");
        };
        assert_eq!(match_statement.cases.len(), 2);
        assert_eq!(match_statement.cases[0].patterns.len(), 2);
        assert!(matches!(
            match_statement.cases[1].patterns.as_slice(),
            [MatchPattern::InclusiveRange {
                start: Expression::Literal(Value::Int(3)),
                end: Expression::Literal(Value::Int(5)),
            }]
        ));
        assert!(match_statement.default_body.is_some());
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn rejects_invalid_match_default_layout() {
        for source in [
            "match 1 {\n}",
            "match 1 {\n  case default {\n  }\n  case 1 {\n  }\n}",
            "match 1 {\n  case default {\n  }\n  case default {\n  }\n}",
        ] {
            assert!(parse_program(source).is_err(), "accepted `{source}`");
        }
    }

    #[test]
    fn lowers_postfix_increment_and_decrement() {
        let program = parse_program("AVInt value = 1\nvalue++\nvalue--").unwrap();

        assert!(matches!(
            &program.statements[1],
            Statement::Assignment(Assignment {
                name,
                operator: AssignmentOperator::AddAssign,
                value: Expression::Literal(Value::Int(1)),
            }) if name == "value"
        ));
        assert!(matches!(
            &program.statements[2],
            Statement::Assignment(Assignment {
                name,
                operator: AssignmentOperator::SubAssign,
                value: Expression::Literal(Value::Int(1)),
            }) if name == "value"
        ));
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_vtime_and_pick() {
        let program = parse_program(
            r#"
            AVStr text = "a b"
            pick(text): ch {
              VTime tmp = ch
              tmp = 1
              out tmp
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_functions_calls_conversions_secretup_stop_and_fail() {
        let program = parse_program(
            r#"
            func parse_age(raw) {
              VTime age = int(raw)

              if age == NONE {
                return NONE
              }

              return age
            }

            AVStr raw = input
            AVInt age = parse_age(raw)
            AVStr token = input
            secretup(token)

            if age == NONE {
              fail "bad age"
            } else {
              stop
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_lists_with_tagged_elements_and_stack_ops() {
        let program = parse_program(
            r#"
            List public_items = ["a", "b"]
            List secret_items = [1:SASV, "abc":ASV, true]
            List matrix = [[1, 2], [3, 4]]
            add(public_items, "c")
            add(secret_items, "hidden":ASV)
            set(public_items, 0, "A")
            set(secret_items, -1, "classified":SASV)
            VTime first = get(public_items, 0)
            VTime last = pop(public_items)
            pick(public_items): item {
              out item
            }
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_index_and_slice_access() {
        let program = parse_program(
            r#"
            List values = [1, 2, 3, 4]
            List matrix = [[1, 2], [3, 4]]
            VTime first = values[0]
            VTime cell = matrix[0][1]
            VTime head = values[:3]
            VTime tail = values[1:]
            VTime every_second = values[::2]
            VTime middle = values[1:3:1]
            "#,
        )
        .unwrap();

        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn distinguishes_qualified_values_from_slice_steps() {
        let program =
            parse_program("VTime event = windows::WM_DESTROY\nVTime sliced = values[start::step]")
                .unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::VTimeDecl(declaration)
                if declaration.initial_value == Expression::Variable("windows.WM_DESTROY".to_owned())
        ));
        assert!(matches!(
            &program.statements[1],
            Statement::VTimeDecl(declaration)
                if matches!(
                    &declaration.initial_value,
                    Expression::Slice { start: Some(start), end: None, step: Some(step), .. }
                        if **start == Expression::Variable("start".to_owned())
                            && **step == Expression::Variable("step".to_owned())
                )
        ));
    }

    #[test]
    fn parses_library_composition_directives() {
        let program = parse_program(
            r#"
            include >> base
            exclude "len" << base
            out 1
            "#,
        )
        .unwrap();

        assert_eq!(
            program.statements,
            vec![
                Statement::Include("base".to_owned()),
                Statement::Exclude {
                    symbol: "len".to_owned(),
                    library: "base".to_owned(),
                },
                Statement::Out(Expression::Literal(Value::Int(1))),
            ]
        );
        assert_eq!(validate_program(&program), Ok(()));

        assert!(parse_program("include > base").is_err());
        assert!(parse_program("exclude len << base").is_err());
        assert!(parse_program("exclude \"len\" < base").is_err());
    }

    #[test]
    fn parses_function_call_as_a_statement() {
        let program = parse_program(
            r#"func touch(value) {
  return value
}
touch(42)"#,
        )
        .unwrap();

        assert!(matches!(
            &program.statements[1],
            Statement::Expression(Expression::Call { name, args })
                if name == "touch" && args == &vec![Expression::Literal(Value::Int(42))]
        ));
        assert_eq!(validate_program(&program), Ok(()));
    }

    #[test]
    fn parses_std_entry_and_namespace_directives() {
        let program = parse_program(
            r#"
            using namespace std
            AVInt std::main() {
              AVInt value = int("7")
              return 0
            }
            "#,
        )
        .unwrap();

        assert!(matches!(
            &program.statements[0],
            Statement::UsingNamespace(namespace) if namespace == "std"
        ));
        assert!(matches!(
            &program.statements[1],
            Statement::FunctionDecl(function)
                if function.name == "std::main" && function.params.is_empty()
        ));

        let nc = parse_program("nc std, math").unwrap();
        assert_eq!(
            nc.statements,
            vec![Statement::NoConflicts(vec![
                "std".to_owned(),
                "math".to_owned(),
            ])]
        );

        let unqualified =
            parse_program("using namespace std\nAVInt main() {\n  return 0\n}").unwrap();
        assert!(matches!(
            &unqualified.statements[1],
            Statement::FunctionDecl(function) if function.name == "std::main"
        ));
        assert!(parse_program("AVInt main() {\n  return 0\n}").is_err());
    }
}
