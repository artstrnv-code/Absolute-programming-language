use apl_core::{
    AssignmentOperator, BinaryOperator, Expression, FunctionDecl, InfoAssignment, Program,
    ProtectionLevel, Statement, UnaryOperator, Value as AstValue, ValueType, VariableKind,
};
use apl_crypto::{sha1, sha256, sha3_384, sha512};
use apl_raw_memory::{
    AbiSignature, Descriptor, RawMemoryTable, ScopePolicy, State as RawMemoryState,
    DESCRIPTOR_MAGIC,
};
use apl_windows::WindowsHost;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, VecDeque},
    rc::Rc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

fn utc_datetime_fields() -> Result<[i64; 7], &'static str> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "system clock is before the Unix epoch")?;
    let seconds = i64::try_from(elapsed.as_secs()).map_err(|_| "system time is out of range")?;
    let days = seconds / 86_400;
    let second_of_day = seconds % 86_400;

    let shifted_days = days
        .checked_add(719_468)
        .ok_or("system time is out of range")?;
    let era = if shifted_days >= 0 {
        shifted_days / 146_097
    } else {
        (shifted_days - 146_096) / 146_097
    };
    let day_of_era = shifted_days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = month_part + if month_part < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);

    Ok([
        year,
        month,
        day,
        second_of_day / 3_600,
        second_of_day / 60 % 60,
        second_of_day % 60,
        i64::from(elapsed.subsec_millis()),
    ])
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunOutput {
    pub stdout: String,
}

#[derive(Debug, Default)]
struct TimerState {
    accumulated: Duration,
    started_at: Option<Instant>,
}

impl TimerState {
    fn start(&mut self) -> Result<(), &'static str> {
        if self.started_at.is_some() {
            return Err("timer is already running");
        }
        self.started_at = Some(Instant::now());
        Ok(())
    }

    fn stop(&mut self) -> Result<(), &'static str> {
        let Some(started_at) = self.started_at else {
            return Err("timer is not running");
        };
        let accumulated = self
            .accumulated
            .checked_add(started_at.elapsed())
            .ok_or("timer duration overflow")?;
        self.accumulated = accumulated;
        self.started_at = None;
        Ok(())
    }

    fn elapsed(&self) -> Result<Duration, &'static str> {
        match self.started_at {
            Some(started_at) => self
                .accumulated
                .checked_add(started_at.elapsed())
                .ok_or("timer duration overflow"),
            None => Ok(self.accumulated),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NumberedInputArguments {
    values: BTreeMap<usize, String>,
}

impl NumberedInputArguments {
    pub fn iter(&self) -> impl Iterator<Item = (usize, &str)> {
        self.values
            .iter()
            .map(|(index, value)| (*index, value.as_str()))
    }
}

pub fn parse_numbered_input_arguments(
    arguments: &[String],
) -> Result<NumberedInputArguments, String> {
    let mut values = BTreeMap::new();
    let mut cursor = 0;

    while cursor < arguments.len() {
        let flag = &arguments[cursor];
        let Some(index) = flag
            .strip_prefix('-')
            .filter(|value| !value.is_empty() && value.chars().all(|ch| ch.is_ascii_digit()))
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|index| *index > 0)
        else {
            return Err(format!(
                "expected a numbered input flag such as `-1`, got `{flag}`"
            ));
        };
        let Some(value) = arguments.get(cursor + 1) else {
            return Err(format!("input flag `-{index}` requires a value"));
        };
        if values.insert(index, value.clone()).is_some() {
            return Err(format!("duplicate input flag `-{index}`"));
        }
        cursor += 2;
    }

    Ok(NumberedInputArguments { values })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugSourceLocation {
    pub path: Vec<usize>,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugVariable {
    pub name: String,
    pub value: String,
    pub protection: ProtectionLevel,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DebugSnapshot {
    pub location: DebugSourceLocation,
    pub call_stack: Vec<String>,
    pub variables: Vec<DebugVariable>,
    pub stdout: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugAction {
    Continue,
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeError {
    Message(String),
    Failed(String),
    FailedWithCode { message: String, code: i32 },
}

impl RuntimeError {
    fn in_function(self, name: &str) -> Self {
        match self {
            Self::Message(message) => Self::Message(format!("{message}\n  in `{name}`")),
            Self::Failed(message) => Self::Failed(message),
            Self::FailedWithCode { message, code } => Self::FailedWithCode { message, code },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompiledProgram {
    entry: BlockId,
    functions: HashMap<String, CompiledFunction>,
    blocks: Vec<CompiledBlock>,
    code: Vec<StatementOp>,
    source_locations: Vec<Option<DebugSourceLocation>>,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledFunction {
    params: Vec<String>,
    body: BlockId,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledBlock {
    start: usize,
    end: usize,
}

type BlockId = usize;

#[derive(Debug, Clone, PartialEq)]
enum StatementOp {
    VariableDecl(CompiledVariableDecl),
    ListDecl(CompiledListDecl),
    VTimeDecl(CompiledVTimeDecl),
    Assignment(CompiledAssignment),
    InfoAssignment(InfoAssignment),
    AddToList {
        list_name: String,
        element: CompiledListElement,
    },
    SetListElement {
        list_name: String,
        index: CompiledExpression,
        element: CompiledListElement,
    },
    Expression(CompiledExpression),
    SecretUp(String),
    Out(CompiledExpression),
    Stop(Option<CompiledExpression>),
    Fail(CompiledExpression),
    ErrorReturn {
        message: CompiledExpression,
        code: CompiledExpression,
    },
    Return(CompiledExpression),
    Jump {
        target_pc: usize,
    },
    JumpIfFalse {
        condition: CompiledExpression,
        target_pc: usize,
    },
    ExecScopedBlock(BlockId),
    LoopCheck {
        condition: CompiledExpression,
        limit: CompiledExpression,
        normal_exit_pc: usize,
        limit_exit_pc: usize,
        else_exit_pc: usize,
    },
    ExecLoopBody {
        body: BlockId,
        continue_pc: usize,
        break_pc: usize,
    },
    Pick {
        value: CompiledExpression,
        item_name: String,
        body: BlockId,
    },
    Match {
        value: CompiledExpression,
        cases: Vec<(Vec<CompiledMatchPattern>, BlockId)>,
        default_body: Option<BlockId>,
    },
    Break,
    Continue,
}

#[derive(Debug, Clone, PartialEq)]
enum CompiledMatchPattern {
    Value(CompiledExpression),
    InclusiveRange {
        start: CompiledExpression,
        end: CompiledExpression,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledVariableDecl {
    kind: VariableKind,
    value_type: ValueType,
    name: String,
    initial_value: CompiledExpression,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledListDecl {
    name: String,
    elements: Vec<CompiledListElement>,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledVTimeDecl {
    name: String,
    initial_value: CompiledExpression,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledAssignment {
    name: String,
    operator: AssignmentOperator,
    value: CompiledExpression,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledListElement {
    value: CompiledExpression,
    protection: Option<ProtectionLevel>,
}

#[derive(Debug, Clone, PartialEq)]
struct CompiledExpression {
    ops: Vec<ExpressionOp>,
    form: ExpressionForm,
}

#[derive(Debug, Clone, PartialEq)]
enum ExpressionForm {
    Other,
    Input,
    SecretInput,
    Variable(String),
}

#[derive(Debug, Clone, PartialEq)]
enum ExpressionOp {
    PushLiteral(AstValue),
    PushNone,
    ReadInput,
    ReadSecretInput,
    LoadVariable(String),
    Call {
        name: String,
        arg_count: usize,
        arg_forms: Vec<ExpressionForm>,
    },
    Index,
    Slice {
        has_start: bool,
        has_end: bool,
        has_step: bool,
    },
    Unary(UnaryOperator),
    Binary(BinaryOperator),
    SelfCheck(String),
    BuildList(Vec<Option<ProtectionLevel>>),
}

impl From<apl_ir::IrError> for RuntimeError {
    fn from(error: apl_ir::IrError) -> Self {
        Self::Message(format!("IR decode error: {error:?}"))
    }
}

pub fn run_program(program: &Program, input: Vec<String>) -> Result<RunOutput, RuntimeError> {
    let compiled = compile_program(program)?;
    run_compiled_program(&compiled, input)
}

pub fn run_program_with_arguments(
    program: &Program,
    input: Vec<String>,
    arguments: &[String],
) -> Result<RunOutput, RuntimeError> {
    let compiled = compile_program(program)?;
    run_compiled_program_with_arguments(&compiled, input, arguments)
}

pub fn compile_program(program: &Program) -> Result<CompiledProgram, RuntimeError> {
    let mut compiler = StatementCompiler::new(&[]);
    let mut functions = HashMap::new();
    let mut entry_statements = Vec::new();
    let mut has_main = false;

    for statement in &program.statements {
        if let Statement::FunctionDecl(function) = statement {
            has_main |= function.name == "std::main";
            if functions
                .insert(
                    function.name.clone(),
                    compiler.compile_function(function, &[])?,
                )
                .is_some()
            {
                return Err(RuntimeError::Message(format!(
                    "duplicate function `{}`",
                    function.name
                )));
            }
        } else {
            entry_statements.push(statement.clone());
        }
    }
    if has_main {
        entry_statements.push(Statement::Expression(Expression::Call {
            name: "std::main".to_owned(),
            args: Vec::new(),
        }));
    }
    let entry = compiler.compile_block(&entry_statements, &[])?;
    Ok(CompiledProgram {
        entry,
        functions,
        blocks: compiler.blocks,
        code: compiler.code,
        source_locations: compiler.compiled_locations,
    })
}

pub fn compile_program_with_locations(
    program: &Program,
    locations: &[DebugSourceLocation],
) -> Result<CompiledProgram, RuntimeError> {
    let mut compiler = StatementCompiler::new(locations);
    let mut functions = HashMap::new();
    let mut entry_statements = Vec::new();
    let mut has_main = false;

    for (index, statement) in program.statements.iter().enumerate() {
        let path = vec![index];
        if let Statement::FunctionDecl(function) = statement {
            has_main |= function.name == "std::main";
            if functions
                .insert(
                    function.name.clone(),
                    compiler.compile_function(function, &path)?,
                )
                .is_some()
            {
                return Err(RuntimeError::Message(format!(
                    "duplicate function `{}`",
                    function.name
                )));
            }
        } else {
            entry_statements.push((statement, Some(path)));
        }
    }

    let main_entry = has_main.then(|| {
        Statement::Expression(Expression::Call {
            name: "std::main".to_owned(),
            args: Vec::new(),
        })
    });
    if let Some(main_entry) = &main_entry {
        entry_statements.push((main_entry, None));
    }

    let entry = compiler.compile_mapped_block(&entry_statements)?;
    Ok(CompiledProgram {
        entry,
        functions,
        blocks: compiler.blocks,
        code: compiler.code,
        source_locations: compiler.compiled_locations,
    })
}

struct StatementCompiler {
    blocks: Vec<CompiledBlock>,
    code: Vec<StatementOp>,
    source_locations: HashMap<Vec<usize>, DebugSourceLocation>,
    compiled_locations: Vec<Option<DebugSourceLocation>>,
}

impl StatementCompiler {
    fn new(locations: &[DebugSourceLocation]) -> Self {
        Self {
            blocks: Vec::new(),
            code: Vec::new(),
            source_locations: locations
                .iter()
                .cloned()
                .map(|location| (location.path.clone(), location))
                .collect(),
            compiled_locations: Vec::new(),
        }
    }

    fn compile_function(
        &mut self,
        function: &FunctionDecl,
        path: &[usize],
    ) -> Result<CompiledFunction, RuntimeError> {
        let body_path = if self.source_locations.is_empty() {
            Vec::new()
        } else {
            nested_path(Some(path), 0)
        };
        Ok(CompiledFunction {
            params: function.params.clone(),
            body: self.compile_block(&function.body, &body_path)?,
        })
    }

    fn compile_block(
        &mut self,
        statements: &[Statement],
        base_path: &[usize],
    ) -> Result<BlockId, RuntimeError> {
        if self.source_locations.is_empty() {
            let mut ops = Vec::new();
            for statement in statements {
                ops.extend(self.compile_statement(statement, ops.len(), None)?);
            }
            let locations = vec![None; ops.len()];
            return self.finish_block(ops, locations);
        }
        let mapped = statements
            .iter()
            .enumerate()
            .map(|(index, statement)| {
                let mut path = base_path.to_vec();
                path.push(index);
                (statement, Some(path))
            })
            .collect::<Vec<_>>();
        self.compile_mapped_block(&mapped)
    }

    fn compile_mapped_block(
        &mut self,
        statements: &[(&Statement, Option<Vec<usize>>)],
    ) -> Result<BlockId, RuntimeError> {
        if self.source_locations.is_empty() {
            let mut ops = Vec::new();
            for &(statement, _) in statements {
                ops.extend(self.compile_statement(statement, ops.len(), None)?);
            }
            let locations = vec![None; ops.len()];
            return self.finish_block(ops, locations);
        }
        let mut ops = Vec::new();
        let mut locations = Vec::new();
        for &(statement, ref path) in statements {
            let statement_ops = self.compile_statement(statement, ops.len(), path.as_deref())?;
            let location = path
                .as_ref()
                .and_then(|path| self.source_locations.get(path))
                .cloned();
            locations.extend(
                statement_ops
                    .iter()
                    .enumerate()
                    .map(|(index, _)| (index == 0).then(|| location.clone()).flatten()),
            );
            ops.extend(statement_ops);
        }
        self.finish_block(ops, locations)
    }

    fn finish_block(
        &mut self,
        mut ops: Vec<StatementOp>,
        locations: Vec<Option<DebugSourceLocation>>,
    ) -> Result<BlockId, RuntimeError> {
        let start = self.code.len();
        for op in &mut ops {
            absolutize_statement_op_pcs(op, start);
        }
        let end = start + ops.len();
        let id = self.blocks.len();
        self.code.extend(ops);
        self.compiled_locations.extend(locations);
        self.blocks.push(CompiledBlock { start, end });
        Ok(id)
    }

    fn compile_statement(
        &mut self,
        statement: &Statement,
        base_pc: usize,
        path: Option<&[usize]>,
    ) -> Result<Vec<StatementOp>, RuntimeError> {
        let op = match statement {
            Statement::Include(_)
            | Statement::Exclude { .. }
            | Statement::UsingNamespace(_)
            | Statement::NoConflicts(_) => return Ok(Vec::new()),
            Statement::VariableDecl(decl) => StatementOp::VariableDecl(CompiledVariableDecl {
                kind: decl.kind,
                value_type: decl.value_type,
                name: decl.name.clone(),
                initial_value: compile_expression(&decl.initial_value),
            }),
            Statement::ListDecl(decl) => StatementOp::ListDecl(CompiledListDecl {
                name: decl.name.clone(),
                elements: decl.elements.iter().map(compile_list_element).collect(),
            }),
            Statement::VTimeDecl(decl) => StatementOp::VTimeDecl(CompiledVTimeDecl {
                name: decl.name.clone(),
                initial_value: compile_expression(&decl.initial_value),
            }),
            Statement::FunctionDecl(_) => return Ok(Vec::new()),
            Statement::Assignment(assignment) => StatementOp::Assignment(CompiledAssignment {
                name: assignment.name.clone(),
                operator: assignment.operator,
                value: compile_expression(&assignment.value),
            }),
            Statement::InfoAssignment(assignment) => {
                StatementOp::InfoAssignment(assignment.clone())
            }
            Statement::AddToList { list_name, element } => StatementOp::AddToList {
                list_name: list_name.clone(),
                element: compile_list_element(element),
            },
            Statement::SetListElement {
                list_name,
                index,
                element,
            } => StatementOp::SetListElement {
                list_name: list_name.clone(),
                index: compile_expression(index),
                element: compile_list_element(element),
            },
            Statement::Expression(expression) => {
                StatementOp::Expression(compile_expression(expression))
            }
            Statement::SecretUp(name) => StatementOp::SecretUp(name.clone()),
            Statement::Out(expression) => StatementOp::Out(compile_expression(expression)),
            Statement::Stop(reason) => StatementOp::Stop(reason.as_ref().map(compile_expression)),
            Statement::Fail(reason) => StatementOp::Fail(compile_expression(reason)),
            Statement::ErrorReturn { message, code } => StatementOp::ErrorReturn {
                message: compile_expression(message),
                code: compile_expression(code),
            },
            Statement::Return(expression) => StatementOp::Return(compile_expression(expression)),
            Statement::If(if_statement) => {
                return self.compile_if_statement(if_statement, base_pc, path);
            }
            Statement::While(while_statement) => {
                let mut ops = vec![
                    StatementOp::LoopCheck {
                        condition: compile_expression(&while_statement.condition),
                        limit: compile_expression(&while_statement.limit),
                        normal_exit_pc: 0,
                        limit_exit_pc: 0,
                        else_exit_pc: 0,
                    },
                    StatementOp::ExecLoopBody {
                        body: self.compile_nested_block(&while_statement.body, path, 0)?,
                        continue_pc: base_pc,
                        break_pc: 0,
                    },
                    StatementOp::Jump { target_pc: base_pc },
                ];
                let mut limit_jump_index = None;
                let limit_exit_pc = if let Some(body) = &while_statement.limit_body {
                    let pc = base_pc + ops.len();
                    ops.push(StatementOp::ExecScopedBlock(
                        self.compile_nested_block(body, path, 1)?,
                    ));
                    limit_jump_index = Some(ops.len());
                    ops.push(StatementOp::Jump { target_pc: 0 });
                    pc
                } else {
                    0
                };
                let else_exit_pc = if let Some(body) = &while_statement.else_body {
                    let pc = base_pc + ops.len();
                    ops.push(StatementOp::ExecScopedBlock(
                        self.compile_nested_block(body, path, 2)?,
                    ));
                    pc
                } else {
                    0
                };
                let end_pc = base_pc + ops.len();
                if let StatementOp::LoopCheck {
                    normal_exit_pc,
                    limit_exit_pc: target_limit,
                    else_exit_pc: target_else,
                    ..
                } = &mut ops[0]
                {
                    *normal_exit_pc = end_pc;
                    *target_limit = if limit_exit_pc == 0 {
                        end_pc
                    } else {
                        limit_exit_pc
                    };
                    *target_else = if else_exit_pc == 0 {
                        end_pc
                    } else {
                        else_exit_pc
                    };
                }
                if let StatementOp::ExecLoopBody { break_pc, .. } = &mut ops[1] {
                    *break_pc = end_pc;
                }
                if let Some(index) = limit_jump_index {
                    if let StatementOp::Jump { target_pc } = &mut ops[index] {
                        *target_pc = end_pc;
                    }
                }
                return Ok(ops);
            }
            Statement::Match(match_statement) => {
                let mut cases = Vec::with_capacity(match_statement.cases.len());
                for (index, case) in match_statement.cases.iter().enumerate() {
                    let patterns = case
                        .patterns
                        .iter()
                        .map(|pattern| match pattern {
                            apl_core::MatchPattern::Value(value) => {
                                CompiledMatchPattern::Value(compile_expression(value))
                            }
                            apl_core::MatchPattern::InclusiveRange { start, end } => {
                                CompiledMatchPattern::InclusiveRange {
                                    start: compile_expression(start),
                                    end: compile_expression(end),
                                }
                            }
                        })
                        .collect();
                    cases.push((
                        patterns,
                        self.compile_nested_block(&case.body, path, index)?,
                    ));
                }
                let default_body = match_statement
                    .default_body
                    .as_deref()
                    .map(|body| self.compile_nested_block(body, path, match_statement.cases.len()))
                    .transpose()?;
                StatementOp::Match {
                    value: compile_expression(&match_statement.value),
                    cases,
                    default_body,
                }
            }
            Statement::Pick(pick_statement) => StatementOp::Pick {
                value: compile_expression(&pick_statement.value),
                item_name: pick_statement.item_name.clone(),
                body: self.compile_nested_block(&pick_statement.body, path, 0)?,
            },
            Statement::Break => StatementOp::Break,
            Statement::Continue => StatementOp::Continue,
        };
        Ok(vec![op])
    }

    fn compile_if_statement(
        &mut self,
        if_statement: &apl_core::IfStatement,
        base_pc: usize,
        path: Option<&[usize]>,
    ) -> Result<Vec<StatementOp>, RuntimeError> {
        let mut branches = Vec::new();
        branches.push((
            compile_expression(&if_statement.condition),
            self.compile_nested_block(&if_statement.body, path, 0)?,
        ));
        for (index, branch) in if_statement.else_if_branches.iter().enumerate() {
            branches.push((
                compile_expression(&branch.condition),
                self.compile_nested_block(&branch.body, path, index + 1)?,
            ));
        }
        let else_body = if_statement
            .else_body
            .as_deref()
            .map(|body| {
                self.compile_nested_block(body, path, if_statement.else_if_branches.len() + 1)
            })
            .transpose()?;

        let branch_count = branches.len();
        let local_end = branch_count * 3 + usize::from(else_body.is_some());
        let mut ops = Vec::new();
        for (index, (condition, body)) in branches.into_iter().enumerate() {
            let next_branch = (index + 1) * 3;
            let false_target = if index + 1 == branch_count {
                if else_body.is_some() {
                    local_end - 1
                } else {
                    local_end
                }
            } else {
                next_branch
            };
            ops.push(StatementOp::JumpIfFalse {
                condition,
                target_pc: base_pc + false_target,
            });
            ops.push(StatementOp::ExecScopedBlock(body));
            ops.push(StatementOp::Jump {
                target_pc: base_pc + local_end,
            });
        }
        if let Some(body) = else_body {
            ops.push(StatementOp::ExecScopedBlock(body));
        }
        Ok(ops)
    }

    fn compile_nested_block(
        &mut self,
        statements: &[Statement],
        path: Option<&[usize]>,
        segment: usize,
    ) -> Result<BlockId, RuntimeError> {
        if self.source_locations.is_empty() {
            self.compile_block(statements, &[])
        } else {
            self.compile_block(statements, &nested_path(path, segment))
        }
    }
}

fn nested_path(path: Option<&[usize]>, segment: usize) -> Vec<usize> {
    let mut nested = path.unwrap_or_default().to_vec();
    nested.push(segment);
    nested
}

fn absolutize_statement_op_pcs(op: &mut StatementOp, block_start: usize) {
    match op {
        StatementOp::Jump { target_pc } | StatementOp::JumpIfFalse { target_pc, .. } => {
            *target_pc += block_start;
        }
        StatementOp::LoopCheck {
            normal_exit_pc,
            limit_exit_pc,
            else_exit_pc,
            ..
        } => {
            *normal_exit_pc += block_start;
            *limit_exit_pc += block_start;
            *else_exit_pc += block_start;
        }
        StatementOp::ExecLoopBody {
            continue_pc,
            break_pc,
            ..
        } => {
            *continue_pc += block_start;
            *break_pc += block_start;
        }
        _ => {}
    }
}

fn compile_list_element(element: &apl_core::ListElement) -> CompiledListElement {
    CompiledListElement {
        value: compile_expression(&element.value),
        protection: element.protection,
    }
}

fn compile_expression(expression: &Expression) -> CompiledExpression {
    let mut ops = Vec::new();
    compile_expression_ops(expression, &mut ops);
    CompiledExpression {
        ops,
        form: expression_form(expression),
    }
}

fn compile_expression_ops(expression: &Expression, ops: &mut Vec<ExpressionOp>) {
    match expression {
        Expression::Literal(value) => ops.push(ExpressionOp::PushLiteral(value.clone())),
        Expression::ListLiteral(elements) => {
            let protections = elements
                .iter()
                .map(|element| {
                    compile_expression_ops(&element.value, ops);
                    element.protection
                })
                .collect();
            ops.push(ExpressionOp::BuildList(protections));
        }
        Expression::None => ops.push(ExpressionOp::PushNone),
        Expression::Input => ops.push(ExpressionOp::ReadInput),
        Expression::SecretInput => ops.push(ExpressionOp::ReadSecretInput),
        Expression::Variable(name) => ops.push(ExpressionOp::LoadVariable(name.clone())),
        Expression::Call { name, args } => {
            for arg in args {
                compile_expression_ops(arg, ops);
            }
            ops.push(ExpressionOp::Call {
                name: runtime_call_name(name),
                arg_count: args.len(),
                arg_forms: args.iter().map(expression_form).collect(),
            });
        }
        Expression::Index { target, index } => {
            compile_expression_ops(target, ops);
            compile_expression_ops(index, ops);
            ops.push(ExpressionOp::Index);
        }
        Expression::Slice {
            target,
            start,
            end,
            step,
        } => {
            compile_expression_ops(target, ops);
            if let Some(start) = start {
                compile_expression_ops(start, ops);
            }
            if let Some(end) = end {
                compile_expression_ops(end, ops);
            }
            if let Some(step) = step {
                compile_expression_ops(step, ops);
            }
            ops.push(ExpressionOp::Slice {
                has_start: start.is_some(),
                has_end: end.is_some(),
                has_step: step.is_some(),
            });
        }
        Expression::Unary {
            operator,
            expression,
        } => {
            compile_expression_ops(expression, ops);
            ops.push(ExpressionOp::Unary(*operator));
        }
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            compile_expression_ops(left, ops);
            compile_expression_ops(right, ops);
            ops.push(ExpressionOp::Binary(*operator));
        }
        Expression::SelfCheck(name) => ops.push(ExpressionOp::SelfCheck(name.clone())),
        Expression::Grouped(inner) => compile_expression_ops(inner, ops),
    }
}

fn runtime_call_name(name: &str) -> String {
    if name == "std::main" {
        name.to_owned()
    } else if let Some(symbol) = name.strip_prefix("std::") {
        symbol.to_owned()
    } else {
        name.replace("::", ".")
    }
}

fn expression_form(expression: &Expression) -> ExpressionForm {
    match expression {
        Expression::Input => ExpressionForm::Input,
        Expression::SecretInput => ExpressionForm::SecretInput,
        Expression::Variable(name) => ExpressionForm::Variable(name.clone()),
        Expression::Grouped(inner) => expression_form(inner),
        _ => ExpressionForm::Other,
    }
}

pub fn compile_ir_bytes(bytes: &[u8]) -> Result<CompiledProgram, RuntimeError> {
    let program = apl_ir::decode_program(bytes)?;
    compile_program(&program)
}

pub fn run_compiled_program(
    program: &CompiledProgram,
    input: Vec<String>,
) -> Result<RunOutput, RuntimeError> {
    let mut runtime = Runtime::new(input);
    runtime.run(program)
}

pub fn run_compiled_program_with_arguments(
    program: &CompiledProgram,
    input: Vec<String>,
    arguments: &[String],
) -> Result<RunOutput, RuntimeError> {
    let arguments = parse_numbered_input_arguments(arguments)
        .map_err(|message| RuntimeError::Message(format!("input argument error: {message}")))?;
    let mut runtime = Runtime::new_with_arguments(input, arguments);
    runtime.run(program)
}

pub fn run_program_debug<F>(
    program: &Program,
    locations: &[DebugSourceLocation],
    input: Vec<String>,
    mut hook: F,
) -> Result<RunOutput, RuntimeError>
where
    F: FnMut(&DebugSnapshot) -> DebugAction,
{
    let compiled = compile_program_with_locations(program, locations)?;
    let mut runtime = Runtime::new_debug(input, &mut hook);
    runtime.run(&compiled)
}

pub fn run_ir_bytes(bytes: &[u8], input: Vec<String>) -> Result<RunOutput, RuntimeError> {
    let program = compile_ir_bytes(bytes)?;
    run_compiled_program(&program, input)
}

pub fn run_ir_bytes_with_arguments(
    bytes: &[u8],
    input: Vec<String>,
    arguments: &[String],
) -> Result<RunOutput, RuntimeError> {
    let program = compile_ir_bytes(bytes)?;
    run_compiled_program_with_arguments(&program, input, arguments)
}

struct RuntimeInput {
    stdin: VecDeque<String>,
    arguments: BTreeMap<usize, String>,
    next_slot: usize,
}

impl RuntimeInput {
    fn new(stdin: Vec<String>, arguments: NumberedInputArguments) -> Self {
        Self {
            stdin: stdin.into(),
            arguments: arguments.values,
            next_slot: 1,
        }
    }

    fn read(&mut self) -> Result<(Option<String>, bool, usize), RuntimeError> {
        let slot = self.next_slot;
        self.next_slot = self
            .next_slot
            .checked_add(1)
            .ok_or_else(|| RuntimeError::Message("input slot counter overflow".to_owned()))?;
        if let Some(value) = self.arguments.remove(&slot) {
            return Ok((Some(value), true, slot));
        }
        Ok((self.stdin.pop_front(), false, slot))
    }
}

struct Runtime<'a, 'debug> {
    program: Option<&'a CompiledProgram>,
    absolute: HashMap<String, AbsoluteSlot>,
    lists: HashMap<String, RuntimeValue>,
    scopes: Vec<HashMap<String, RuntimeValue>>,
    input: RuntimeInput,
    stdout: String,
    stopped: bool,
    debug_hook: Option<&'debug mut dyn FnMut(&DebugSnapshot) -> DebugAction>,
    call_stack: Vec<String>,
    timer: TimerState,
    raw_memory: RawMemoryTable,
    windows: WindowsHost,
}

#[derive(Debug, Clone, PartialEq)]
struct AbsoluteSlot {
    kind: VariableKind,
    value_type: ValueType,
    initial: RuntimeValue,
    current: RuntimeValue,
}

#[derive(Debug, Clone, PartialEq)]
struct RuntimeValue {
    data: RuntimeData,
    protection: ProtectionLevel,
}

#[derive(Debug, PartialEq)]
struct RuntimeString {
    text: String,
    chars: Vec<char>,
}

impl RuntimeString {
    fn new(text: String) -> Self {
        let chars = text.chars().collect();
        Self { text, chars }
    }

    fn as_str(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, PartialEq)]
enum RuntimeData {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(Rc<RuntimeString>),
    Bytes(Vec<u8>),
    Json(String),
    List(Rc<Vec<RuntimeValue>>),
    None,
}

impl RuntimeData {
    fn type_name(&self) -> &'static str {
        match self {
            Self::Int(_) => "Int",
            Self::Float(_) => "Float",
            Self::Bool(_) => "Bool",
            Self::Str(_) => "Str",
            Self::Bytes(_) => "Bytes",
            Self::Json(_) => "Json",
            Self::List(_) => "List",
            Self::None => "NONE",
        }
    }
}

fn runtime_descriptor(descriptor: Descriptor) -> RuntimeValue {
    RuntimeValue::list(vec![
        RuntimeValue::str(DESCRIPTOR_MAGIC.to_owned()),
        RuntimeValue::new(RuntimeData::Int(descriptor.id), ProtectionLevel::Av),
        RuntimeValue::new(RuntimeData::Int(descriptor.address), ProtectionLevel::Av),
        RuntimeValue::str(descriptor.origin.name().to_owned()),
    ])
}

fn runtime_int(value: &RuntimeValue, label: &str) -> Result<i64, RuntimeError> {
    match value.data {
        RuntimeData::Int(value) => Ok(value),
        _ => Err(RuntimeError::Message(format!("{label} must be Int"))),
    }
}

fn runtime_str(value: &RuntimeValue, label: &str) -> Result<String, RuntimeError> {
    match &value.data {
        RuntimeData::Str(value) => Ok(value.as_str().to_owned()),
        _ => Err(RuntimeError::Message(format!("{label} must be Str"))),
    }
}

fn runtime_bool_value(value: &RuntimeValue, label: &str) -> Result<bool, RuntimeError> {
    match value.data {
        RuntimeData::Bool(value) => Ok(value),
        _ => Err(RuntimeError::Message(format!("{label} must be Bool"))),
    }
}

fn runtime_pointer_id(table: &RawMemoryTable, value: &RuntimeValue) -> Result<i64, RuntimeError> {
    if let RuntimeData::List(values) = &value.data {
        if values.len() == 4
            && matches!(&values[0].data, RuntimeData::Str(magic) if magic.as_str() == DESCRIPTOR_MAGIC)
        {
            return runtime_int(&values[1], "raw-memory descriptor ID");
        }
    }
    let number = runtime_int(value, "raw-memory target")?;
    table.resolve_number(number).map_err(RuntimeError::Message)
}

fn runtime_signature(value: &RuntimeValue) -> Result<Option<AbiSignature>, RuntimeError> {
    if matches!(value.data, RuntimeData::None) {
        return Ok(None);
    }
    let RuntimeData::List(parts) = &value.data else {
        return Err(RuntimeError::Message(
            "memory::load ABI must be NONE or [convention, argument_types, result_type]".to_owned(),
        ));
    };
    if parts.len() != 3 {
        return Err(RuntimeError::Message(
            "memory::load ABI must contain convention, argument types, and result type".to_owned(),
        ));
    }
    Ok(Some(AbiSignature {
        convention: runtime_str(&parts[0], "ABI convention")?,
        argument_types: runtime_string_list(&parts[1], "ABI argument types")?,
        result_type: runtime_str(&parts[2], "ABI result type")?,
    }))
}

fn runtime_string_list(value: &RuntimeValue, label: &str) -> Result<Vec<String>, RuntimeError> {
    let RuntimeData::List(values) = &value.data else {
        return Err(RuntimeError::Message(format!("{label} must be List")));
    };
    values
        .iter()
        .map(|value| runtime_str(value, label))
        .collect()
}

fn runtime_process_output(output: apl_windows::ProcessOutput) -> RuntimeValue {
    RuntimeValue::list(vec![
        RuntimeValue::new(RuntimeData::Int(output.exit_code), ProtectionLevel::Av),
        RuntimeValue::str(output.stdout),
        RuntimeValue::str(output.stderr),
    ])
}

fn runtime_process_snapshot(snapshot: apl_windows::ProcessSnapshot) -> RuntimeValue {
    RuntimeValue::list(vec![
        RuntimeValue::new(RuntimeData::Bool(snapshot.running), ProtectionLevel::Av),
        snapshot
            .exit_code
            .map(|code| RuntimeValue::new(RuntimeData::Int(code), ProtectionLevel::Av))
            .unwrap_or_else(|| RuntimeValue::none(ProtectionLevel::Av)),
        RuntimeValue::str(snapshot.stdout),
        RuntimeValue::str(snapshot.stderr),
    ])
}

fn runtime_accelerator_entries(
    value: &RuntimeValue,
) -> Result<Vec<apl_windows::AcceleratorEntry>, RuntimeError> {
    let RuntimeData::List(entries) = &value.data else {
        return Err(RuntimeError::Message(
            "windows::create_accelerators entries must be List".to_owned(),
        ));
    };
    entries
        .iter()
        .map(|entry| {
            let RuntimeData::List(parts) = &entry.data else {
                return Err(RuntimeError::Message(
                    "each accelerator entry must be [key, command, modifiers]".to_owned(),
                ));
            };
            if parts.len() != 3 {
                return Err(RuntimeError::Message(
                    "each accelerator entry must be [key, command, modifiers]".to_owned(),
                ));
            }
            Ok(apl_windows::AcceleratorEntry {
                key: runtime_int(&parts[0], "accelerator key")?,
                command: runtime_int(&parts[1], "accelerator command")?,
                modifiers: runtime_int(&parts[2], "accelerator modifiers")?,
            })
        })
        .collect()
}

fn runtime_call_arguments(
    table: &RawMemoryTable,
    value: &RuntimeValue,
) -> Result<Vec<i64>, RuntimeError> {
    let RuntimeData::List(values) = &value.data else {
        return Err(RuntimeError::Message(
            "call arguments must be List".to_owned(),
        ));
    };
    values
        .iter()
        .map(|value| {
            if matches!(value.data, RuntimeData::List(_)) {
                let id = runtime_pointer_id(table, value)?;
                return table
                    .descriptor(id)
                    .map(|descriptor| descriptor.address)
                    .map_err(RuntimeError::Message);
            }
            runtime_int(value, "call argument")
        })
        .collect()
}

fn runtime_scope_policy(value: &RuntimeValue) -> Result<ScopePolicy, RuntimeError> {
    let names = runtime_string_list(value, "raw-memory scope policy")?;
    Ok(ScopePolicy {
        marked: names.iter().any(|name| name == "marked"),
        collector: names.iter().any(|name| name == "collector"),
        flee: names.iter().any(|name| name == "flee"),
    })
}

fn collect_runtime_pointer_roots(value: &RuntimeValue, roots: &mut BTreeSet<i64>) {
    let RuntimeData::List(values) = &value.data else {
        return;
    };
    if values.len() == 4
        && matches!(&values[0].data, RuntimeData::Str(magic) if magic.as_str() == DESCRIPTOR_MAGIC)
    {
        if let RuntimeData::Int(id) = values[1].data {
            roots.insert(id);
        }
    }
    for value in values.iter() {
        collect_runtime_pointer_roots(value, roots);
    }
}

enum Flow {
    None,
    Break,
    Continue,
    Return(RuntimeValue),
    Stop,
}

#[derive(Debug, Clone, PartialEq)]
struct BlockFrame {
    block_id: BlockId,
    pc: usize,
    loop_iterations: HashMap<usize, i64>,
    loop_limits: HashMap<usize, i64>,
}

impl<'a, 'debug> Runtime<'a, 'debug> {
    fn new(input: Vec<String>) -> Self {
        Self::new_with_arguments(input, NumberedInputArguments::default())
    }

    fn new_with_arguments(input: Vec<String>, arguments: NumberedInputArguments) -> Self {
        Self {
            program: None,
            absolute: HashMap::new(),
            lists: HashMap::new(),
            scopes: Vec::new(),
            input: RuntimeInput::new(input, arguments),
            stdout: String::new(),
            stopped: false,
            debug_hook: None,
            call_stack: Vec::new(),
            timer: TimerState::default(),
            raw_memory: RawMemoryTable::new(),
            windows: WindowsHost::default(),
        }
    }

    fn new_debug(
        input: Vec<String>,
        hook: &'debug mut dyn FnMut(&DebugSnapshot) -> DebugAction,
    ) -> Self {
        let mut runtime = Self::new(input);
        runtime.debug_hook = Some(hook);
        runtime
    }

    fn raw_memory_roots(&self) -> BTreeSet<i64> {
        let mut roots = BTreeSet::new();
        for slot in self.absolute.values() {
            collect_runtime_pointer_roots(&slot.current, &mut roots);
        }
        for value in self.lists.values() {
            collect_runtime_pointer_roots(value, &mut roots);
        }
        for scope in &self.scopes {
            for value in scope.values() {
                collect_runtime_pointer_roots(value, &mut roots);
            }
        }
        roots
    }

    fn run(&mut self, program: &'a CompiledProgram) -> Result<RunOutput, RuntimeError> {
        self.program = Some(program);

        match self.exec_active_block(program.entry)? {
            Flow::None | Flow::Stop => Ok(RunOutput {
                stdout: self.stdout.clone(),
            }),
            Flow::Break => Err(RuntimeError::Message("break outside loop".to_owned())),
            Flow::Continue => Err(RuntimeError::Message("continue outside loop".to_owned())),
            Flow::Return(_) => Err(RuntimeError::Message("return outside function".to_owned())),
        }
    }

    fn exec_block(&mut self, block_id: BlockId) -> Result<Flow, RuntimeError> {
        let block = self
            .program
            .and_then(|program| program.blocks.get(block_id))
            .ok_or_else(|| RuntimeError::Message(format!("unknown block `{block_id}`")))?;
        let mut frame = BlockFrame {
            block_id,
            pc: block.start,
            loop_iterations: HashMap::new(),
            loop_limits: HashMap::new(),
        };

        while let Some((_, op)) = self.next_statement_op(&mut frame)? {
            let flow = self.exec_statement_op(&mut frame, &op)?;
            if self.stopped {
                return Ok(Flow::Stop);
            }
            if !matches!(flow, Flow::None) {
                return Ok(flow);
            }
        }
        Ok(Flow::None)
    }

    fn exec_debug_block(&mut self, block_id: BlockId) -> Result<Flow, RuntimeError> {
        let block = self
            .program
            .and_then(|program| program.blocks.get(block_id))
            .ok_or_else(|| RuntimeError::Message(format!("unknown block `{block_id}`")))?;
        let mut frame = BlockFrame {
            block_id,
            pc: block.start,
            loop_iterations: HashMap::new(),
            loop_limits: HashMap::new(),
        };

        while let Some((pc, op)) = self.next_statement_op(&mut frame)? {
            if self.debug_before(pc) == DebugAction::Stop {
                self.stopped = true;
                return Ok(Flow::Stop);
            }
            let flow = self.exec_statement_op(&mut frame, &op)?;
            if self.stopped {
                return Ok(Flow::Stop);
            }
            if !matches!(flow, Flow::None) {
                return Ok(flow);
            }
        }
        Ok(Flow::None)
    }

    fn exec_active_block(&mut self, block_id: BlockId) -> Result<Flow, RuntimeError> {
        if self.debug_hook.is_some() {
            self.exec_debug_block(block_id)
        } else {
            self.exec_block(block_id)
        }
    }

    fn next_statement_op(
        &self,
        frame: &mut BlockFrame,
    ) -> Result<Option<(usize, StatementOp)>, RuntimeError> {
        let block = self
            .program
            .and_then(|program| program.blocks.get(frame.block_id))
            .ok_or_else(|| RuntimeError::Message(format!("unknown block `{}`", frame.block_id)))?;
        if frame.pc >= block.end {
            return Ok(None);
        }
        let pc = frame.pc;
        let op = self
            .program
            .and_then(|program| program.code.get(frame.pc))
            .cloned()
            .ok_or_else(|| RuntimeError::Message(format!("unknown pc `{}`", frame.pc)))?;
        frame.pc += 1;
        Ok(Some((pc, op)))
    }

    fn debug_before(&mut self, pc: usize) -> DebugAction {
        let Some(location) = self
            .program
            .and_then(|program| program.source_locations.get(pc))
            .and_then(Option::as_ref)
            .cloned()
        else {
            return DebugAction::Continue;
        };
        let snapshot = self.debug_snapshot(location);
        self.debug_hook
            .as_mut()
            .map_or(DebugAction::Continue, |hook| hook(&snapshot))
    }

    fn debug_snapshot(&self, location: DebugSourceLocation) -> DebugSnapshot {
        let mut visible = HashMap::new();
        for (name, slot) in &self.absolute {
            visible.insert(name.clone(), debug_variable(name, &slot.current));
        }
        for (name, value) in &self.lists {
            visible.insert(name.clone(), debug_variable(name, value));
        }
        for scope in &self.scopes {
            for (name, value) in scope {
                visible.insert(name.clone(), debug_variable(name, value));
            }
        }
        let mut variables = visible.into_values().collect::<Vec<_>>();
        variables.sort_by(|left, right| left.name.cmp(&right.name));
        DebugSnapshot {
            location,
            call_stack: self.call_stack.clone(),
            variables,
            stdout: self.stdout.clone(),
        }
    }

    fn exec_scoped_block(&mut self, block_id: BlockId) -> Result<Flow, RuntimeError> {
        self.with_scope(|runtime| runtime.exec_active_block(block_id))
    }

    fn exec_statement_op(
        &mut self,
        frame: &mut BlockFrame,
        op: &StatementOp,
    ) -> Result<Flow, RuntimeError> {
        match op {
            StatementOp::VariableDecl(decl) => {
                let protection = variable_kind_protection(decl.kind);
                let value = self
                    .eval_for_type(&decl.initial_value, decl.value_type, protection)?
                    .with_protection(protection);
                self.absolute.insert(
                    decl.name.clone(),
                    AbsoluteSlot {
                        kind: decl.kind,
                        value_type: decl.value_type,
                        initial: value.clone(),
                        current: value,
                    },
                );
                Ok(Flow::None)
            }
            StatementOp::ListDecl(decl) => {
                let values = decl
                    .elements
                    .iter()
                    .map(|element| self.eval_list_element(element))
                    .collect::<Result<Vec<_>, _>>()?;
                self.lists
                    .insert(decl.name.clone(), RuntimeValue::list(values));
                Ok(Flow::None)
            }
            StatementOp::VTimeDecl(decl) => {
                let value = self.eval_expression(&decl.initial_value)?;
                self.declare_vtime(decl.name.clone(), value);
                Ok(Flow::None)
            }
            StatementOp::Assignment(assignment) => {
                self.exec_assignment(assignment)?;
                Ok(Flow::None)
            }
            StatementOp::InfoAssignment(assignment) => {
                let Some(slot) = self.absolute.get(&assignment.source) else {
                    return Err(RuntimeError::Message(format!(
                        "unknown absolute variable `{}`",
                        assignment.source
                    )));
                };
                let type_name = slot.value_type.to_string();
                let protection_name = slot.kind.to_string();
                self.assign_existing(&assignment.type_target, RuntimeValue::str(type_name))?;
                self.assign_existing(
                    &assignment.protection_target,
                    RuntimeValue::str(protection_name),
                )?;
                Ok(Flow::None)
            }
            StatementOp::AddToList { list_name, element } => {
                let value = self.eval_list_element(element)?;
                self.add_to_list(list_name, value)?;
                Ok(Flow::None)
            }
            StatementOp::SetListElement {
                list_name,
                index,
                element,
            } => {
                let index = self.eval_expression(index)?;
                let value = self.eval_list_element(element)?;
                self.set_list_element(list_name, index, value)?;
                Ok(Flow::None)
            }
            StatementOp::Expression(expression) => {
                self.eval_expression(expression)?;
                Ok(Flow::None)
            }
            StatementOp::SecretUp(name) => {
                let Some(slot) = self.absolute.get_mut(name) else {
                    return Err(RuntimeError::Message(format!(
                        "unknown absolute variable `{name}`"
                    )));
                };
                slot.kind = match slot.kind {
                    VariableKind::Av => VariableKind::Asv,
                    VariableKind::Asv => VariableKind::Sasv,
                    VariableKind::Sasv => {
                        return Err(RuntimeError::Message(format!("`{name}` is already SASV")))
                    }
                };
                let protection = variable_kind_protection(slot.kind);
                slot.current.protection = protection;
                slot.initial.protection = protection;
                Ok(Flow::None)
            }
            StatementOp::Out(expression) => {
                let value = self.eval_expression(expression)?;
                if value.effective_protection() != ProtectionLevel::Av {
                    return Err(RuntimeError::Message(
                        "refusing to output secret value".to_owned(),
                    ));
                }
                self.stdout.push_str(&value.render());
                Ok(Flow::None)
            }
            StatementOp::Stop(reason) => {
                if let Some(reason) = reason {
                    let value = self.eval_expression(reason)?;
                    if value.effective_protection() != ProtectionLevel::Av {
                        return Err(RuntimeError::Message(
                            "refusing to stop with secret reason".to_owned(),
                        ));
                    }
                    self.stdout.push_str(&value.render());
                    self.stdout.push('\n');
                }
                self.stopped = true;
                Ok(Flow::Stop)
            }
            StatementOp::Fail(reason) => {
                let value = self.eval_expression(reason)?;
                if value.effective_protection() != ProtectionLevel::Av {
                    return Err(RuntimeError::Message(
                        "refusing to fail with secret reason".to_owned(),
                    ));
                }
                Err(RuntimeError::Failed(value.render()))
            }
            StatementOp::ErrorReturn { message, code } => {
                let message = self.eval_expression(message)?;
                if message.effective_protection() != ProtectionLevel::Av {
                    return Err(RuntimeError::Message(
                        "refusing to return secret ERR message".to_owned(),
                    ));
                }
                let RuntimeData::Str(message) = message.data else {
                    return Err(RuntimeError::Message(
                        "ERR message must evaluate to Str".to_owned(),
                    ));
                };
                let code = self.eval_expression(code)?;
                if code.effective_protection() != ProtectionLevel::Av {
                    return Err(RuntimeError::Message(
                        "refusing to return secret ERR code".to_owned(),
                    ));
                }
                let RuntimeData::Int(code) = code.data else {
                    return Err(RuntimeError::Message(
                        "ERR code must evaluate to Int".to_owned(),
                    ));
                };
                let code = i32::try_from(code).map_err(|_| {
                    RuntimeError::Message("ERR code must fit signed 32-bit range".to_owned())
                })?;
                Err(RuntimeError::FailedWithCode {
                    message: message.text.clone(),
                    code,
                })
            }
            StatementOp::Return(expression) => Ok(Flow::Return(self.eval_expression(expression)?)),
            StatementOp::Jump { target_pc } => {
                frame.pc = *target_pc;
                Ok(Flow::None)
            }
            StatementOp::JumpIfFalse {
                condition,
                target_pc,
            } => {
                if !self.eval_bool(condition)? {
                    frame.pc = *target_pc;
                }
                Ok(Flow::None)
            }
            StatementOp::ExecScopedBlock(block_id) => self.exec_scoped_block(*block_id),
            StatementOp::LoopCheck {
                condition,
                limit,
                normal_exit_pc,
                limit_exit_pc,
                else_exit_pc,
            } => {
                let op_pc = frame.pc.saturating_sub(1);
                let limit = if let Some(limit) = frame.loop_limits.get(&op_pc) {
                    *limit
                } else {
                    let value = self.eval_expression(limit)?;
                    let RuntimeData::Int(limit) = value.data else {
                        return Err(RuntimeError::Message(
                            "while limit must evaluate to Int".to_owned(),
                        ));
                    };
                    if limit < -1 {
                        return Err(RuntimeError::Message(format!(
                            "invalid loop limit `{limit}`"
                        )));
                    }
                    frame.loop_limits.insert(op_pc, limit);
                    limit
                };
                if !self.eval_bool(condition)? {
                    let entered = frame
                        .loop_iterations
                        .get(&op_pc)
                        .is_some_and(|iterations| *iterations > 0);
                    frame.loop_iterations.remove(&op_pc);
                    frame.loop_limits.remove(&op_pc);
                    frame.pc = if entered {
                        *normal_exit_pc
                    } else {
                        *else_exit_pc
                    };
                    return Ok(Flow::None);
                }
                let iterations = frame.loop_iterations.entry(op_pc).or_insert(0);
                if limit != -1 && *iterations >= limit {
                    frame.loop_iterations.remove(&op_pc);
                    frame.loop_limits.remove(&op_pc);
                    frame.pc = *limit_exit_pc;
                    return Ok(Flow::None);
                }
                *iterations += 1;
                Ok(Flow::None)
            }
            StatementOp::ExecLoopBody {
                body,
                continue_pc,
                break_pc,
            } => match self.exec_scoped_block(*body)? {
                Flow::None => Ok(Flow::None),
                Flow::Continue => {
                    frame.pc = *continue_pc;
                    Ok(Flow::None)
                }
                Flow::Break => {
                    frame.loop_iterations.remove(continue_pc);
                    frame.loop_limits.remove(continue_pc);
                    frame.pc = *break_pc;
                    Ok(Flow::None)
                }
                other => Ok(other),
            },
            StatementOp::Pick {
                value,
                item_name,
                body,
            } => {
                let value = self.eval_expression(value)?;
                let items = self.pick_items(value)?;
                for item in items {
                    match self.with_scope(|runtime| {
                        runtime.declare_vtime(item_name.clone(), item);
                        runtime.exec_active_block(*body)
                    })? {
                        Flow::None | Flow::Continue => {}
                        Flow::Break => break,
                        other => return Ok(other),
                    }
                }
                Ok(Flow::None)
            }
            StatementOp::Match {
                value,
                cases,
                default_body,
            } => {
                let value = self.eval_expression(value)?;
                for (patterns, body) in cases {
                    let mut matched = false;
                    for pattern in patterns {
                        matched = match pattern {
                            CompiledMatchPattern::Value(pattern) => {
                                let pattern = self.eval_expression(pattern)?;
                                self.runtime_bool(self.eval_binary(
                                    value.clone(),
                                    BinaryOperator::Equal,
                                    pattern,
                                )?)?
                            }
                            CompiledMatchPattern::InclusiveRange { start, end } => {
                                let start = self.eval_expression(start)?;
                                let lower = self.eval_binary(
                                    value.clone(),
                                    BinaryOperator::GreaterEqual,
                                    start,
                                )?;
                                if !self.runtime_bool(lower)? {
                                    false
                                } else {
                                    let end = self.eval_expression(end)?;
                                    let upper = self.eval_binary(
                                        value.clone(),
                                        BinaryOperator::LessEqual,
                                        end,
                                    )?;
                                    self.runtime_bool(upper)?
                                }
                            }
                        };
                        if matched {
                            break;
                        }
                    }
                    if matched {
                        return self.exec_scoped_block(*body);
                    }
                }
                if let Some(body) = default_body {
                    self.exec_scoped_block(*body)
                } else {
                    Ok(Flow::None)
                }
            }
            StatementOp::Break => Ok(Flow::Break),
            StatementOp::Continue => Ok(Flow::Continue),
        }
    }

    fn exec_assignment(&mut self, assignment: &CompiledAssignment) -> Result<(), RuntimeError> {
        if self.vtime_exists(&assignment.name) {
            let current = self.lookup(&assignment.name)?;
            let value = match assignment.operator {
                AssignmentOperator::Assign => self.eval_expression(&assignment.value)?,
                operator => {
                    let right = self.eval_expression(&assignment.value)?;
                    self.apply_arithmetic(current, right, operator)?
                }
            };
            self.update_vtime(&assignment.name, value);
            return Ok(());
        }

        let current = self.lookup_absolute(&assignment.name)?.current.clone();
        let value = match assignment.operator {
            AssignmentOperator::Assign => self.eval_expression(&assignment.value)?,
            operator => {
                let right = self.eval_expression(&assignment.value)?;
                self.apply_arithmetic(current, right, operator)?
            }
        };
        self.assign_existing(&assignment.name, value)
    }

    fn eval_expression(
        &mut self,
        expression: &CompiledExpression,
    ) -> Result<RuntimeValue, RuntimeError> {
        let mut stack = Vec::new();

        for op in &expression.ops {
            match op {
                ExpressionOp::PushLiteral(value) => {
                    stack.push(RuntimeValue::from_ast(value.clone()));
                }
                ExpressionOp::PushNone => stack.push(RuntimeValue::none(ProtectionLevel::Av)),
                ExpressionOp::ReadInput => stack.push(self.read_input(ProtectionLevel::Av)?),
                ExpressionOp::ReadSecretInput => stack.push(self.read_input(ProtectionLevel::Asv)?),
                ExpressionOp::LoadVariable(name) => stack.push(self.lookup(name)?),
                ExpressionOp::Call {
                    name,
                    arg_count,
                    arg_forms,
                } => {
                    let args = pop_stack_values(&mut stack, *arg_count)?;
                    stack.push(self.eval_call(name, args, arg_forms)?);
                }
                ExpressionOp::Index => {
                    let index = pop_stack_value(&mut stack)?;
                    let target = pop_stack_value(&mut stack)?;
                    stack.push(self.eval_index(target, index)?);
                }
                ExpressionOp::Slice {
                    has_start,
                    has_end,
                    has_step,
                } => {
                    let step = pop_optional_stack_int(&mut stack, *has_step)?;
                    let end = pop_optional_stack_int(&mut stack, *has_end)?;
                    let start = pop_optional_stack_int(&mut stack, *has_start)?;
                    let target = pop_stack_value(&mut stack)?;
                    stack.push(self.eval_slice(target, start, end, step)?);
                }
                ExpressionOp::Unary(operator) => {
                    let value = pop_stack_value(&mut stack)?;
                    stack.push(self.eval_unary(*operator, value)?);
                }
                ExpressionOp::Binary(operator) => {
                    let right = pop_stack_value(&mut stack)?;
                    let left = pop_stack_value(&mut stack)?;
                    stack.push(self.eval_binary(left, *operator, right)?);
                }
                ExpressionOp::SelfCheck(name) => {
                    let Some(slot) = self.absolute.get(name) else {
                        return Err(RuntimeError::Message(format!(
                            "unknown absolute variable `{name}`"
                        )));
                    };
                    stack.push(RuntimeValue::new(
                        RuntimeData::Bool(slot.current.data == slot.initial.data),
                        variable_kind_protection(slot.kind),
                    ));
                }
                ExpressionOp::BuildList(protections) => {
                    let mut values = pop_stack_values(&mut stack, protections.len())?;
                    for (value, protection) in values.iter_mut().zip(protections) {
                        if let Some(protection) = protection {
                            if value.effective_protection() > *protection {
                                return Err(RuntimeError::Message(
                                    "refusing to downgrade list element protection".to_owned(),
                                ));
                            }
                            value.protection = *protection;
                        }
                    }
                    stack.push(RuntimeValue::list(values));
                }
            }
        }

        if stack.len() != 1 {
            return Err(RuntimeError::Message(
                "invalid compiled expression stack".to_owned(),
            ));
        }

        Ok(stack.pop().unwrap())
    }

    fn eval_for_type(
        &mut self,
        expression: &CompiledExpression,
        value_type: ValueType,
        protection: ProtectionLevel,
    ) -> Result<RuntimeValue, RuntimeError> {
        let value = self.eval_expression(expression)?;
        if value.effective_protection() > protection {
            return Err(RuntimeError::Message(
                "refusing to downgrade declaration protection".to_owned(),
            ));
        }
        if matches!(value.data, RuntimeData::None) {
            return Ok(value.with_protection(protection));
        }
        if matches!(
            expression.form,
            ExpressionForm::Input | ExpressionForm::SecretInput
        ) {
            return Ok(value.convert(value_type).with_protection(protection));
        }
        if value.matches_type(value_type) {
            Ok(value.with_protection(protection))
        } else {
            Ok(RuntimeValue::none(protection))
        }
    }
    fn eval_call(
        &mut self,
        name: &str,
        args: Vec<RuntimeValue>,
        arg_forms: &[ExpressionForm],
    ) -> Result<RuntimeValue, RuntimeError> {
        if let Some(value_type) = conversion_type(name) {
            let value = args[0].clone();
            return Ok(value.convert(value_type));
        }

        match name {
            "windows_available" => Ok(RuntimeValue::new(
                RuntimeData::Bool(self.windows.available()),
                ProtectionLevel::Av,
            )),
            "windows_load_library" => {
                let name = runtime_str(&args[0], "windows::load_library name")?;
                let library = self
                    .windows
                    .load_library(&name)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(library),
                    ProtectionLevel::Av,
                ))
            }
            "windows_get_procedure" => {
                let library = runtime_int(&args[0], "windows::get_procedure library")?;
                let name = runtime_str(&args[1], "windows::get_procedure name")?;
                let argument_types =
                    runtime_string_list(&args[2], "windows::get_procedure argument types")?;
                let result_type = runtime_str(&args[3], "windows::get_procedure result type")?;
                let address = self
                    .windows
                    .procedure_address(library, &name)
                    .map_err(RuntimeError::Message)?;
                let descriptor = self
                    .raw_memory
                    .load(
                        address,
                        1,
                        "rx",
                        Some(AbiSignature {
                            convention: "win64".to_owned(),
                            argument_types,
                            result_type,
                        }),
                    )
                    .map_err(RuntimeError::Message)?;
                self.windows
                    .attach_procedure(library, descriptor.id)
                    .map_err(RuntimeError::Message)?;
                Ok(runtime_descriptor(descriptor))
            }
            "windows_create_callback" => {
                let kind = runtime_str(&args[0], "windows::create_callback kind")?;
                let result = runtime_int(&args[1], "windows::create_callback result")?;
                let registration = self
                    .windows
                    .create_callback(&kind, result)
                    .map_err(RuntimeError::Message)?;
                let descriptor = match self.raw_memory.register_runtime(
                    registration.address,
                    1,
                    "rx",
                    Some(AbiSignature {
                        convention: "win64".to_owned(),
                        argument_types: registration.argument_types,
                        result_type: registration.result_type,
                    }),
                ) {
                    Ok(descriptor) => descriptor,
                    Err(error) => {
                        self.windows.discard_callback(registration.identity);
                        return Err(RuntimeError::Message(error));
                    }
                };
                if let Err(error) = self
                    .windows
                    .attach_callback(registration.identity, descriptor.id)
                {
                    let _ = self.raw_memory.release_runtime(descriptor.id);
                    self.windows.discard_callback(registration.identity);
                    return Err(RuntimeError::Message(error));
                }
                Ok(runtime_descriptor(descriptor))
            }
            "windows_next_callback" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                if self.raw_memory.state(id).map_err(RuntimeError::Message)? != RawMemoryState::Live
                {
                    return Err(RuntimeError::Message(format!(
                        "Windows callback pointer `{id}` is not live"
                    )));
                }
                Ok(
                    match self
                        .windows
                        .next_callback(id)
                        .map_err(RuntimeError::Message)?
                    {
                        Some(arguments) => RuntimeValue::list(
                            arguments
                                .into_iter()
                                .map(|value| {
                                    RuntimeValue::new(RuntimeData::Int(value), ProtectionLevel::Av)
                                })
                                .collect(),
                        ),
                        None => RuntimeValue::none(ProtectionLevel::Av),
                    },
                )
            }
            "windows_destroy_callback" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                self.windows
                    .callback_identity(id)
                    .map_err(RuntimeError::Message)?;
                self.raw_memory
                    .release_runtime(id)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(
                        self.windows
                            .destroy_callback(id)
                            .map_err(RuntimeError::Message)?,
                    ),
                    ProtectionLevel::Av,
                ))
            }
            "windows_call" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                let call_args = runtime_call_arguments(&self.raw_memory, &args[1])?;
                let value = self
                    .raw_memory
                    .call(id, &call_args)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(value),
                    ProtectionLevel::Av,
                ))
            }
            "windows_close_library" => {
                let library = runtime_int(&args[0], "windows::close_library library")?;
                let pointers = self
                    .windows
                    .close_library(library)
                    .map_err(RuntimeError::Message)?;
                for pointer in pointers {
                    if self
                        .raw_memory
                        .state(pointer)
                        .map_err(RuntimeError::Message)?
                        == RawMemoryState::Live
                    {
                        self.raw_memory
                            .mark(pointer)
                            .map_err(RuntimeError::Message)?;
                    }
                }
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "windows_last_error" => Ok(RuntimeValue::new(
                RuntimeData::Int(apl_windows::last_error()),
                ProtectionLevel::Av,
            )),
            "windows_utf16" => {
                let value = runtime_str(&args[0], "windows::utf16 value")?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bytes(apl_windows::utf16_bytes(&value)),
                    args[0].protection,
                ))
            }
            "windows_message_box" => {
                let text = runtime_str(&args[0], "windows::message_box text")?;
                let caption = runtime_str(&args[1], "windows::message_box caption")?;
                let kind = runtime_int(&args[2], "windows::message_box kind")?;
                let result = apl_windows::message_box(&text, &caption, kind)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_create_window" => {
                let title = runtime_str(&args[0], "windows::create_window title")?;
                let width = runtime_int(&args[1], "windows::create_window width")?;
                let height = runtime_int(&args[2], "windows::create_window height")?;
                let window = self
                    .windows
                    .create_window(&title, width, height)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(window),
                    ProtectionLevel::Av,
                ))
            }
            "windows_load_icon" => {
                let path = runtime_str(&args[0], "windows::load_icon path")?;
                let icon = self
                    .windows
                    .load_icon(
                        &path,
                        runtime_int(&args[1], "windows::load_icon width")?,
                        runtime_int(&args[2], "windows::load_icon height")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(icon),
                    ProtectionLevel::Av,
                ))
            }
            "windows_load_resource_icon" => {
                let icon = self
                    .windows
                    .load_resource_icon(
                        runtime_int(&args[0], "windows::load_resource_icon resource")?,
                        runtime_int(&args[1], "windows::load_resource_icon width")?,
                        runtime_int(&args[2], "windows::load_resource_icon height")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(icon),
                    ProtectionLevel::Av,
                ))
            }
            "windows_create_accelerators" => {
                let entries = runtime_accelerator_entries(&args[0])?;
                let accelerators = self
                    .windows
                    .create_accelerators(&entries)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(accelerators),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_accelerators" => {
                let changed = self
                    .windows
                    .set_accelerators(
                        runtime_int(&args[0], "windows::set_accelerators window")?,
                        runtime_int(&args[1], "windows::set_accelerators table")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_destroy_accelerators" => {
                let destroyed = self
                    .windows
                    .destroy_accelerators(runtime_int(
                        &args[0],
                        "windows::destroy_accelerators table",
                    )?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(destroyed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_window_icon" => {
                let changed = self
                    .windows
                    .set_window_icon(
                        runtime_int(&args[0], "windows::set_window_icon window")?,
                        runtime_int(&args[1], "windows::set_window_icon icon")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_deferred_close" => {
                let changed = self
                    .windows
                    .set_deferred_close(
                        runtime_int(&args[0], "windows::set_deferred_close window")?,
                        runtime_bool_value(&args[1], "windows::set_deferred_close enabled")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_timer" => {
                let changed = self
                    .windows
                    .set_timer(
                        runtime_int(&args[0], "windows::set_timer window")?,
                        runtime_int(&args[1], "windows::set_timer timer_id")?,
                        runtime_int(&args[2], "windows::set_timer interval_ms")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_kill_timer" => {
                let changed = self
                    .windows
                    .kill_timer(
                        runtime_int(&args[0], "windows::kill_timer window")?,
                        runtime_int(&args[1], "windows::kill_timer timer_id")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_enable_window" => {
                let changed = self
                    .windows
                    .enable_window(
                        runtime_int(&args[0], "windows::enable_window window")?,
                        runtime_bool_value(&args[1], "windows::enable_window enabled")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_invalidate_window" => {
                let changed = self
                    .windows
                    .invalidate_window(
                        runtime_int(&args[0], "windows::invalidate_window window")?,
                        runtime_bool_value(&args[1], "windows::invalidate_window erase")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_client_size" => {
                let (width, height) = self
                    .windows
                    .client_size(runtime_int(&args[0], "windows::client_size window")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::list(vec![
                    RuntimeValue::new(RuntimeData::Int(width), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(height), ProtectionLevel::Av),
                ]))
            }
            "windows_set_minimum_size" => {
                let changed = self
                    .windows
                    .set_minimum_size(
                        runtime_int(&args[0], "windows::set_minimum_size window")?,
                        runtime_int(&args[1], "windows::set_minimum_size width")?,
                        runtime_int(&args[2], "windows::set_minimum_size height")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_destroy_icon" => {
                let destroyed = self
                    .windows
                    .destroy_icon(runtime_int(&args[0], "windows::destroy_icon icon")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(destroyed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_create_menu" => {
                let popup = runtime_bool_value(&args[0], "windows::create_menu popup")?;
                let menu = self
                    .windows
                    .create_menu(popup)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(menu),
                    ProtectionLevel::Av,
                ))
            }
            "windows_append_menu_item" => {
                let appended = self
                    .windows
                    .append_menu_item(
                        runtime_int(&args[0], "windows::append_menu_item menu")?,
                        runtime_int(&args[1], "windows::append_menu_item command id")?,
                        &runtime_str(&args[2], "windows::append_menu_item text")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(appended),
                    ProtectionLevel::Av,
                ))
            }
            "windows_append_submenu" => {
                let appended = self
                    .windows
                    .append_submenu(
                        runtime_int(&args[0], "windows::append_submenu menu")?,
                        runtime_int(&args[1], "windows::append_submenu submenu")?,
                        &runtime_str(&args[2], "windows::append_submenu text")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(appended),
                    ProtectionLevel::Av,
                ))
            }
            "windows_append_menu_separator" => {
                let appended = self
                    .windows
                    .append_menu_separator(runtime_int(
                        &args[0],
                        "windows::append_menu_separator menu",
                    )?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(appended),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_menu" => {
                let changed = self
                    .windows
                    .set_menu(
                        runtime_int(&args[0], "windows::set_menu window")?,
                        runtime_int(&args[1], "windows::set_menu menu")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_menu_item_enabled" => {
                let changed = self
                    .windows
                    .set_menu_item_enabled(
                        runtime_int(&args[0], "windows::set_menu_item_enabled menu")?,
                        runtime_int(&args[1], "windows::set_menu_item_enabled command id")?,
                        runtime_bool_value(&args[2], "windows::set_menu_item_enabled enabled")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_menu_item_checked" => {
                let changed = self
                    .windows
                    .set_menu_item_checked(
                        runtime_int(&args[0], "windows::set_menu_item_checked menu")?,
                        runtime_int(&args[1], "windows::set_menu_item_checked command id")?,
                        runtime_bool_value(&args[2], "windows::set_menu_item_checked checked")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_destroy_menu" => {
                let destroyed = self
                    .windows
                    .destroy_menu(runtime_int(&args[0], "windows::destroy_menu menu")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(destroyed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_show_window" => {
                let window = runtime_int(&args[0], "windows::show_window window")?;
                let shown = self
                    .windows
                    .show_window(window)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(shown),
                    ProtectionLevel::Av,
                ))
            }
            "windows_destroy_window" => {
                let window = runtime_int(&args[0], "windows::destroy_window window")?;
                let destroyed = self
                    .windows
                    .destroy_window(window)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(destroyed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_run_message_loop" => {
                let result = self
                    .windows
                    .run_message_loop()
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_pump_messages" => {
                let running = self
                    .windows
                    .pump_messages()
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(running),
                    ProtectionLevel::Av,
                ))
            }
            "windows_next_event" => Ok(match self.windows.next_event() {
                Some(event) => RuntimeValue::list(vec![
                    RuntimeValue::new(RuntimeData::Int(event.window), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.message), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.wparam), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.lparam), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.source), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.code), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(event.control_id), ProtectionLevel::Av),
                ]),
                None => RuntimeValue::none(ProtectionLevel::Av),
            }),
            "windows_wait_event" => Ok(
                match self.windows.wait_event().map_err(RuntimeError::Message)? {
                    Some(event) => RuntimeValue::list(vec![
                        RuntimeValue::new(RuntimeData::Int(event.window), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.message), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.wparam), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.lparam), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.source), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.code), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(event.control_id), ProtectionLevel::Av),
                    ]),
                    None => RuntimeValue::none(ProtectionLevel::Av),
                },
            ),
            "windows_create_control" => {
                let parent = runtime_int(&args[0], "windows::create_control parent")?;
                let class_name = runtime_str(&args[1], "windows::create_control class_name")?;
                let text = runtime_str(&args[2], "windows::create_control text")?;
                let control = self
                    .windows
                    .create_control(
                        parent,
                        &class_name,
                        &text,
                        runtime_int(&args[3], "windows::create_control id")?,
                        runtime_int(&args[4], "windows::create_control x")?,
                        runtime_int(&args[5], "windows::create_control y")?,
                        runtime_int(&args[6], "windows::create_control width")?,
                        runtime_int(&args[7], "windows::create_control height")?,
                        runtime_int(&args[8], "windows::create_control style")?,
                        runtime_int(&args[9], "windows::create_control extended_style")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(control),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_window_text" => {
                let window = runtime_int(&args[0], "windows::set_window_text window")?;
                let text = runtime_str(&args[1], "windows::set_window_text text")?;
                let changed = self
                    .windows
                    .set_window_text(window, &text)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_window_text" => {
                let window = runtime_int(&args[0], "windows::window_text window")?;
                let text = self
                    .windows
                    .window_text(window)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::str(text))
            }
            "windows_move_window" => {
                let moved = self
                    .windows
                    .move_window(
                        runtime_int(&args[0], "windows::move_window window")?,
                        runtime_int(&args[1], "windows::move_window x")?,
                        runtime_int(&args[2], "windows::move_window y")?,
                        runtime_int(&args[3], "windows::move_window width")?,
                        runtime_int(&args[4], "windows::move_window height")?,
                        runtime_bool_value(&args[5], "windows::move_window repaint")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(moved),
                    ProtectionLevel::Av,
                ))
            }
            "windows_selection" => {
                let control = runtime_int(&args[0], "windows::selection control")?;
                let (start, end) = self
                    .windows
                    .selection(control)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::list(vec![
                    RuntimeValue::new(RuntimeData::Int(start), ProtectionLevel::Av),
                    RuntimeValue::new(RuntimeData::Int(end), ProtectionLevel::Av),
                ]))
            }
            "windows_set_selection" => {
                let changed = self
                    .windows
                    .set_selection(
                        runtime_int(&args[0], "windows::set_selection control")?,
                        runtime_int(&args[1], "windows::set_selection start")?,
                        runtime_int(&args[2], "windows::set_selection end")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_replace_selection" => {
                let changed = self
                    .windows
                    .replace_selection(
                        runtime_int(&args[0], "windows::replace_selection control")?,
                        &runtime_str(&args[1], "windows::replace_selection text")?,
                        runtime_bool_value(&args[2], "windows::replace_selection can_undo")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_line_count" => {
                let count = self
                    .windows
                    .line_count(runtime_int(&args[0], "windows::line_count control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(count),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_text_color" => {
                let changed = self
                    .windows
                    .set_text_color(
                        runtime_int(&args[0], "windows::set_text_color control")?,
                        runtime_int(&args[1], "windows::set_text_color start")?,
                        runtime_int(&args[2], "windows::set_text_color end")?,
                        runtime_int(&args[3], "windows::set_text_color color")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_background_color" => {
                let changed = self
                    .windows
                    .set_background_color(
                        runtime_int(&args[0], "windows::set_background_color control")?,
                        runtime_int(&args[1], "windows::set_background_color start")?,
                        runtime_int(&args[2], "windows::set_background_color end")?,
                        runtime_int(&args[3], "windows::set_background_color color")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_text_bold" => {
                let changed = self
                    .windows
                    .set_text_bold(
                        runtime_int(&args[0], "windows::set_text_bold control")?,
                        runtime_int(&args[1], "windows::set_text_bold start")?,
                        runtime_int(&args[2], "windows::set_text_bold end")?,
                        runtime_bool_value(&args[3], "windows::set_text_bold enabled")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_text_font" => {
                let changed = self
                    .windows
                    .set_text_font(
                        runtime_int(&args[0], "windows::set_text_font control")?,
                        runtime_int(&args[1], "windows::set_text_font start")?,
                        runtime_int(&args[2], "windows::set_text_font end")?,
                        &runtime_str(&args[3], "windows::set_text_font face")?,
                        runtime_int(&args[4], "windows::set_text_font points")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(changed),
                    ProtectionLevel::Av,
                ))
            }
            "windows_run_process" => {
                let output = apl_windows::run_process(
                    &runtime_str(&args[0], "windows::run_process program")?,
                    &runtime_string_list(&args[1], "windows::run_process arguments")?,
                    &runtime_str(&args[2], "windows::run_process input")?,
                    &runtime_str(&args[3], "windows::run_process working_directory")?,
                )
                .map_err(RuntimeError::Message)?;
                Ok(runtime_process_output(output))
            }
            "windows_start_process" => {
                let process = self
                    .windows
                    .start_process(
                        &runtime_str(&args[0], "windows::start_process program")?,
                        &runtime_string_list(&args[1], "windows::start_process arguments")?,
                        &runtime_str(&args[2], "windows::start_process input")?,
                        &runtime_str(&args[3], "windows::start_process working_directory")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(process),
                    ProtectionLevel::Av,
                ))
            }
            "windows_poll_process" => {
                let snapshot = self
                    .windows
                    .poll_process(runtime_int(&args[0], "windows::poll_process process")?)
                    .map_err(RuntimeError::Message)?;
                Ok(runtime_process_snapshot(snapshot))
            }
            "windows_wait_process" | "windows_terminate_process" => {
                let process = runtime_int(&args[0], "Windows process identity")?;
                let output = if name == "windows_wait_process" {
                    self.windows.wait_process(process)
                } else {
                    self.windows.terminate_process(process)
                }
                .map_err(RuntimeError::Message)?;
                Ok(runtime_process_output(output))
            }
            "windows_open_file_dialog" | "windows_save_file_dialog" => {
                let owner = runtime_int(&args[0], "Windows file dialog owner")?;
                let title = runtime_str(&args[1], "Windows file dialog title")?;
                let initial_directory =
                    runtime_str(&args[2], "Windows file dialog initial directory")?;
                let file_name = runtime_str(&args[3], "Windows file dialog file name")?;
                let filter_name = runtime_str(&args[4], "Windows file dialog filter name")?;
                let filter_pattern = runtime_str(&args[5], "Windows file dialog filter pattern")?;
                let default_extension =
                    runtime_str(&args[6], "Windows file dialog default extension")?;
                let selected = if name == "windows_open_file_dialog" {
                    self.windows.open_file_dialog(
                        owner,
                        &title,
                        &initial_directory,
                        &file_name,
                        &filter_name,
                        &filter_pattern,
                        &default_extension,
                    )
                } else {
                    self.windows.save_file_dialog(
                        owner,
                        &title,
                        &initial_directory,
                        &file_name,
                        &filter_name,
                        &filter_pattern,
                        &default_extension,
                    )
                }
                .map_err(RuntimeError::Message)?;
                Ok(match selected {
                    Some(path) => RuntimeValue::str(path),
                    None => RuntimeValue::none(ProtectionLevel::Av),
                })
            }
            "windows_can_undo" => {
                let result = self
                    .windows
                    .can_undo(runtime_int(&args[0], "windows::can_undo control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_undo" => {
                let result = self
                    .windows
                    .undo(runtime_int(&args[0], "windows::undo control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_can_redo" => {
                let result = self
                    .windows
                    .can_redo(runtime_int(&args[0], "windows::can_redo control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_redo" => {
                let result = self
                    .windows
                    .redo(runtime_int(&args[0], "windows::redo control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_clear_history" => {
                let result = self
                    .windows
                    .clear_history(runtime_int(&args[0], "windows::clear_history control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_event_mask" => {
                let mask = self
                    .windows
                    .event_mask(runtime_int(&args[0], "windows::event_mask control")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(mask),
                    ProtectionLevel::Av,
                ))
            }
            "windows_set_event_mask" => {
                let previous = self
                    .windows
                    .set_event_mask(
                        runtime_int(&args[0], "windows::set_event_mask control")?,
                        runtime_int(&args[1], "windows::set_event_mask mask")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(previous),
                    ProtectionLevel::Av,
                ))
            }
            "windows_first_visible_line" => {
                let line = self
                    .windows
                    .first_visible_line(runtime_int(
                        &args[0],
                        "windows::first_visible_line control",
                    )?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(line),
                    ProtectionLevel::Av,
                ))
            }
            "windows_scroll_lines" => {
                let result = self
                    .windows
                    .scroll_lines(
                        runtime_int(&args[0], "windows::scroll_lines control")?,
                        runtime_int(&args[1], "windows::scroll_lines lines")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_focus" => {
                let result = self
                    .windows
                    .focus(runtime_int(&args[0], "windows::focus window")?)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(result),
                    ProtectionLevel::Av,
                ))
            }
            "windows_find_text" => {
                let found = self
                    .windows
                    .find_text(
                        runtime_int(&args[0], "windows::find_text control")?,
                        &runtime_str(&args[1], "windows::find_text text")?,
                        runtime_int(&args[2], "windows::find_text start")?,
                        runtime_int(&args[3], "windows::find_text end")?,
                        runtime_bool_value(&args[4], "windows::find_text match_case")?,
                        runtime_bool_value(&args[5], "windows::find_text whole_word")?,
                    )
                    .map_err(RuntimeError::Message)?;
                Ok(match found {
                    Some((start, end)) => RuntimeValue::list(vec![
                        RuntimeValue::new(RuntimeData::Int(start), ProtectionLevel::Av),
                        RuntimeValue::new(RuntimeData::Int(end), ProtectionLevel::Av),
                    ]),
                    None => RuntimeValue::none(ProtectionLevel::Av),
                })
            }
            "timer_start" => {
                self.timer
                    .start()
                    .map_err(|error| RuntimeError::Message(error.to_owned()))?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "timer_stop" => {
                self.timer
                    .stop()
                    .map_err(|error| RuntimeError::Message(error.to_owned()))?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "timer_elapsed" => Ok(RuntimeValue::new(
                RuntimeData::Float(
                    self.timer
                        .elapsed()
                        .map_err(|error| RuntimeError::Message(error.to_owned()))?
                        .as_secs_f64(),
                ),
                ProtectionLevel::Av,
            )),
            "now_utc" => Ok(RuntimeValue::list(
                utc_datetime_fields()
                    .map_err(|error| RuntimeError::Message(error.to_owned()))?
                    .into_iter()
                    .map(|field| RuntimeValue::new(RuntimeData::Int(field), ProtectionLevel::Av))
                    .collect(),
            )),
            "sha1" | "sha256" | "sha512" | "sha3_384" => {
                let RuntimeData::Bytes(bytes) = &args[0].data else {
                    return Err(RuntimeError::Message(format!(
                        "{name}(value) requires Bytes"
                    )));
                };
                let digest = match name {
                    "sha1" => sha1(bytes),
                    "sha256" => sha256(bytes),
                    "sha512" => sha512(bytes),
                    "sha3_384" => sha3_384(bytes),
                    _ => unreachable!(),
                };
                Ok(RuntimeValue::new(
                    RuntimeData::Bytes(digest),
                    args[0].protection,
                ))
            }
            "raw_malloc" => {
                let size = runtime_int(&args[0], "memory::malloc size")?;
                let descriptor = self
                    .raw_memory
                    .malloc(size)
                    .map_err(RuntimeError::Message)?;
                Ok(runtime_descriptor(descriptor))
            }
            "raw_load" => {
                let address = runtime_int(&args[0], "memory::load address")?;
                let size = runtime_int(&args[1], "memory::load size")?;
                let permissions = runtime_str(&args[2], "memory::load permissions")?;
                let signature = runtime_signature(&args[3])?;
                let descriptor = self
                    .raw_memory
                    .load(address, size, &permissions, signature)
                    .map_err(RuntimeError::Message)?;
                Ok(runtime_descriptor(descriptor))
            }
            "raw_mark" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                self.raw_memory.mark(id).map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "raw_free" => Ok(RuntimeValue::new(
                RuntimeData::Int(self.raw_memory.free_pending() as i64),
                ProtectionLevel::Av,
            )),
            "raw_where" => {
                let selector = runtime_int(&args[1], "memory::where selector")?;
                match selector {
                    0 => {
                        let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                        Ok(RuntimeValue::new(
                            RuntimeData::Int(
                                self.raw_memory
                                    .descriptor(id)
                                    .map_err(RuntimeError::Message)?
                                    .address,
                            ),
                            ProtectionLevel::Av,
                        ))
                    }
                    1 => Ok(RuntimeValue::new(
                        RuntimeData::Int(runtime_pointer_id(&self.raw_memory, &args[0])?),
                        ProtectionLevel::Av,
                    )),
                    2 => Ok(match &arg_forms[0] {
                        ExpressionForm::Variable(name) => RuntimeValue::str(name.clone()),
                        _ => RuntimeValue::none(ProtectionLevel::Av),
                    }),
                    3 => Ok(args[0].clone()),
                    _ => Err(RuntimeError::Message(
                        "memory::where selector must be 0, 1, 2, or 3".to_owned(),
                    )),
                }
            }
            "raw_read" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                let offset = runtime_int(&args[1], "memory::read offset")?;
                let count = runtime_int(&args[2], "memory::read count")?;
                let bytes = self
                    .raw_memory
                    .read(id, offset, count)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bytes(bytes),
                    ProtectionLevel::Av,
                ))
            }
            "raw_write" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                let offset = runtime_int(&args[1], "memory::write offset")?;
                let RuntimeData::Bytes(bytes) = &args[2].data else {
                    return Err(RuntimeError::Message(
                        "memory::write value must be Bytes".to_owned(),
                    ));
                };
                self.raw_memory
                    .write(id, offset, bytes)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "raw_call" => {
                let id = runtime_pointer_id(&self.raw_memory, &args[0])?;
                let call_args = runtime_call_arguments(&self.raw_memory, &args[1])?;
                let value = self
                    .raw_memory
                    .call(id, &call_args)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(value),
                    ProtectionLevel::Av,
                ))
            }
            "raw_scope_enter" => {
                let policy = runtime_scope_policy(&args[0])?;
                self.raw_memory.enter_scope(policy);
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(true),
                    ProtectionLevel::Av,
                ))
            }
            "raw_scope_exit" => {
                let roots = self.raw_memory_roots();
                let released = self
                    .raw_memory
                    .exit_scope(&roots)
                    .map_err(RuntimeError::Message)?;
                Ok(RuntimeValue::new(
                    RuntimeData::Int(released as i64),
                    ProtectionLevel::Av,
                ))
            }
            "raw_scope_leave" => {
                let value = args[0].clone();
                let mut roots = self.raw_memory_roots();
                collect_runtime_pointer_roots(&value, &mut roots);
                self.raw_memory
                    .exit_scope(&roots)
                    .map_err(RuntimeError::Message)?;
                Ok(value)
            }
            "get" => {
                let target = args[0].clone();
                let index = args[1].clone();
                self.eval_index(target, index)
            }
            "pop" => {
                let ExpressionForm::Variable(name) = &arg_forms[0] else {
                    return Err(RuntimeError::Message("pop expects list name".to_owned()));
                };
                self.pop_list(name)
            }
            "split" => {
                let target = args[0].clone();
                let delim = args[1].clone();
                let protection = target
                    .effective_protection()
                    .max(delim.effective_protection());
                let RuntimeData::Str(text) = &target.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let RuntimeData::Str(separator) = &delim.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                if separator.as_str().is_empty() {
                    return Ok(RuntimeValue::none(protection));
                }
                let parts: Vec<RuntimeValue> = text
                    .as_str()
                    .split(separator.as_str())
                    .map(|s| RuntimeValue::str(s.to_owned()).with_protection(protection))
                    .collect();
                Ok(RuntimeValue::list(parts).with_protection(protection))
            }
            "len" => {
                let target = args[0].clone();
                let protection = target.effective_protection();
                let length = match &target.data {
                    RuntimeData::Str(s) => s.chars.len() as i64,
                    RuntimeData::List(values) => values.len() as i64,
                    RuntimeData::Bytes(b) => b.len() as i64,
                    _ => return Ok(RuntimeValue::none(protection)),
                };
                Ok(RuntimeValue::new(RuntimeData::Int(length), protection))
            }
            "contains" => {
                let target = args[0].clone();
                let sub = args[1].clone();
                let protection = target
                    .effective_protection()
                    .max(sub.effective_protection());
                let RuntimeData::Str(text) = &target.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let RuntimeData::Str(substr) = &sub.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                Ok(RuntimeValue::new(
                    RuntimeData::Bool(text.as_str().contains(substr.as_str())),
                    protection,
                ))
            }
            "join" => {
                let values = args[0].clone();
                let separator = args[1].clone();
                let protection = values
                    .effective_protection()
                    .max(separator.effective_protection());
                let RuntimeData::List(items) = &values.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let RuntimeData::Str(separator) = &separator.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let mut parts = Vec::with_capacity(items.len());
                for item in items.iter() {
                    let RuntimeData::Str(value) = &item.data else {
                        return Ok(RuntimeValue::none(protection));
                    };
                    parts.push(value.as_str());
                }
                Ok(RuntimeValue::str(parts.join(separator.as_str())).with_protection(protection))
            }
            "ord" => {
                let value = args[0].clone();
                let protection = value.effective_protection();
                let RuntimeData::Str(text) = &value.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let Some(ch) = text.chars.first() else {
                    return Ok(RuntimeValue::none(protection));
                };
                Ok(RuntimeValue::new(
                    RuntimeData::Int(u32::from(*ch) as i64),
                    protection,
                ))
            }
            "char" => {
                let value = args[0].clone();
                let protection = value.effective_protection();
                let RuntimeData::Int(code) = value.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                let Ok(code) = u32::try_from(code) else {
                    return Ok(RuntimeValue::none(protection));
                };
                let Some(ch) = char::from_u32(code) else {
                    return Ok(RuntimeValue::none(protection));
                };
                Ok(RuntimeValue::str(ch.to_string()).with_protection(protection))
            }
            "pow" => {
                let base = args[0].clone();
                let exponent = args[1].clone();
                let protection = base
                    .effective_protection()
                    .max(exponent.effective_protection());
                let RuntimeData::Int(exponent) = exponent.data else {
                    return Ok(RuntimeValue::none(protection));
                };
                if exponent < 0 {
                    return Ok(RuntimeValue::none(protection));
                }
                match base.data {
                    RuntimeData::Int(base) => {
                        let Ok(exponent) = u32::try_from(exponent) else {
                            return Ok(RuntimeValue::none(protection));
                        };
                        let Some(value) = base.checked_pow(exponent) else {
                            return Ok(RuntimeValue::none(protection));
                        };
                        Ok(RuntimeValue::new(RuntimeData::Int(value), protection))
                    }
                    RuntimeData::Float(base) => {
                        let Ok(exponent) = i32::try_from(exponent) else {
                            return Ok(RuntimeValue::none(protection));
                        };
                        Ok(RuntimeValue::new(
                            RuntimeData::Float(base.powi(exponent)),
                            protection,
                        ))
                    }
                    _ => Ok(RuntimeValue::none(protection)),
                }
            }
            _ => {
                let Some(function) = self
                    .program
                    .and_then(|program| program.functions.get(name))
                    .cloned()
                else {
                    return Err(RuntimeError::Message(format!("unknown function `{name}`")));
                };
                let error_context = match (name, args.first().map(|value| &value.data)) {
                    ("vm.call_func_slot", Some(RuntimeData::Int(slot))) => {
                        format!("vm.call_func_slot[{slot}]")
                    }
                    _ => name.to_owned(),
                };
                let debugging = self.debug_hook.is_some();
                if debugging {
                    self.call_stack.push(error_context.clone());
                }
                let result = self.with_scope(|runtime| {
                    for (param, value) in function.params.iter().zip(args) {
                        runtime.declare_vtime(param.clone(), value);
                    }
                    match runtime.exec_active_block(function.body)? {
                        Flow::Return(value) => Ok(value),
                        Flow::None | Flow::Stop => Ok(RuntimeValue::none(ProtectionLevel::Av)),
                        Flow::Break => {
                            Err(RuntimeError::Message("break escaped function".to_owned()))
                        }
                        Flow::Continue => Err(RuntimeError::Message(
                            "continue escaped function".to_owned(),
                        )),
                    }
                });
                if debugging {
                    self.call_stack.pop();
                }
                result.map_err(|error| error.in_function(&error_context))
            }
        }
    }

    fn eval_index(
        &mut self,
        target: RuntimeValue,
        index: RuntimeValue,
    ) -> Result<RuntimeValue, RuntimeError> {
        let RuntimeData::Int(index) = index.data else {
            return Ok(RuntimeValue::none(target.effective_protection()));
        };
        let protection = target.effective_protection();
        match target.data {
            RuntimeData::List(values) => Ok(resolve_index(values.as_ref(), index)
                .cloned()
                .unwrap_or_else(|| RuntimeValue::none(protection))),
            RuntimeData::Str(value) => Ok(value
                .chars
                .get(resolve_raw_index(value.chars.len(), index).unwrap_or(usize::MAX))
                .map(|ch| RuntimeValue::str(ch.to_string()).with_protection(protection))
                .unwrap_or_else(|| RuntimeValue::none(protection))),
            RuntimeData::Bytes(value) => Ok(value
                .get(resolve_raw_index(value.len(), index).unwrap_or(usize::MAX))
                .map(|b| RuntimeValue::new(RuntimeData::Int(*b as i64), protection))
                .unwrap_or_else(|| RuntimeValue::none(protection))),
            _ => Ok(RuntimeValue::none(protection)),
        }
    }

    fn eval_slice(
        &self,
        target: RuntimeValue,
        start: Option<i64>,
        end: Option<i64>,
        step: Option<i64>,
    ) -> Result<RuntimeValue, RuntimeError> {
        let protection = target.effective_protection();
        let step = step.unwrap_or(1);
        if step == 0 {
            return Err(RuntimeError::Message("zero slice step".to_owned()));
        }
        match target.data {
            RuntimeData::Str(value) => {
                let result: String = slice_indices(value.chars.len(), start, end, step)
                    .into_iter()
                    .map(|index| value.chars[index])
                    .collect();
                Ok(RuntimeValue::str(result).with_protection(protection))
            }
            RuntimeData::List(values) => {
                let result: Vec<RuntimeValue> = slice_indices(values.len(), start, end, step)
                    .into_iter()
                    .map(|index| values[index].clone())
                    .collect();
                Ok(RuntimeValue::list(result).with_protection(protection))
            }
            RuntimeData::Bytes(value) => {
                let result: Vec<u8> = slice_indices(value.len(), start, end, step)
                    .into_iter()
                    .map(|index| value[index])
                    .collect();
                Ok(RuntimeValue::new(RuntimeData::Bytes(result), protection))
            }
            _ => Ok(RuntimeValue::none(protection)),
        }
    }
    fn eval_list_element(
        &mut self,
        element: &CompiledListElement,
    ) -> Result<RuntimeValue, RuntimeError> {
        let value = self.eval_expression(&element.value)?;
        if let Some(protection) = element.protection {
            if value.effective_protection() > protection {
                return Err(RuntimeError::Message(
                    "refusing to downgrade list element protection".to_owned(),
                ));
            }
            Ok(value.with_protection(protection))
        } else {
            Ok(value)
        }
    }
    fn eval_unary(
        &self,
        operator: UnaryOperator,
        value: RuntimeValue,
    ) -> Result<RuntimeValue, RuntimeError> {
        let protection = value.protection;
        match (operator, value.data) {
            (_, RuntimeData::None) => Ok(RuntimeValue::none(protection)),
            (UnaryOperator::Negate, RuntimeData::Int(val)) => val
                .checked_neg()
                .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                .ok_or_else(|| RuntimeError::Message("integer overflow in negate".to_owned())),
            (UnaryOperator::Negate, RuntimeData::Float(val)) => {
                Ok(RuntimeValue::new(RuntimeData::Float(-val), protection))
            }
            (UnaryOperator::Not, RuntimeData::Bool(val)) => {
                Ok(RuntimeValue::new(RuntimeData::Bool(!val), protection))
            }
            _ => Err(RuntimeError::Message("invalid unary operands".to_owned())),
        }
    }

    fn eval_binary(
        &self,
        left: RuntimeValue,
        operator: BinaryOperator,
        right: RuntimeValue,
    ) -> Result<RuntimeValue, RuntimeError> {
        let protection = left.protection.max(right.protection);
        match operator {
            BinaryOperator::Add => match (left.data, right.data) {
                (RuntimeData::Int(a), RuntimeData::Int(b)) => a
                    .checked_add(b)
                    .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                    .ok_or_else(|| RuntimeError::Message("integer overflow in add".to_owned())),
                (RuntimeData::Float(a), RuntimeData::Float(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Float(a + b), protection))
                }
                (RuntimeData::Str(a), RuntimeData::Str(b)) => {
                    Ok(RuntimeValue::str(format!("{}{}", a.as_str(), b.as_str()))
                        .with_protection(protection))
                }
                (RuntimeData::Int(a), RuntimeData::Float(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a as f64 + b),
                    protection,
                )),
                (RuntimeData::Float(a), RuntimeData::Int(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a + b as f64),
                    protection,
                )),
                _ => Err(RuntimeError::Message("invalid add operands".to_owned())),
            },
            BinaryOperator::Sub => match (left.data, right.data) {
                (RuntimeData::Int(a), RuntimeData::Int(b)) => a
                    .checked_sub(b)
                    .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                    .ok_or_else(|| RuntimeError::Message("integer overflow in sub".to_owned())),
                (RuntimeData::Float(a), RuntimeData::Float(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Float(a - b), protection))
                }
                (RuntimeData::Int(a), RuntimeData::Float(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a as f64 - b),
                    protection,
                )),
                (RuntimeData::Float(a), RuntimeData::Int(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a - b as f64),
                    protection,
                )),
                _ => Err(RuntimeError::Message("invalid sub operands".to_owned())),
            },
            BinaryOperator::Mul => match (left.data, right.data) {
                (RuntimeData::Int(a), RuntimeData::Int(b)) => a
                    .checked_mul(b)
                    .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                    .ok_or_else(|| RuntimeError::Message("integer overflow in mul".to_owned())),
                (RuntimeData::Float(a), RuntimeData::Float(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Float(a * b), protection))
                }
                (RuntimeData::Int(a), RuntimeData::Float(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a as f64 * b),
                    protection,
                )),
                (RuntimeData::Float(a), RuntimeData::Int(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a * b as f64),
                    protection,
                )),
                _ => Err(RuntimeError::Message("invalid mul operands".to_owned())),
            },
            BinaryOperator::Div => match (left.data, right.data) {
                (RuntimeData::Int(a), RuntimeData::Int(b)) => {
                    if b == 0 {
                        return Err(RuntimeError::Message("division by zero".to_owned()));
                    }
                    a.checked_div(b)
                        .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                        .ok_or_else(|| RuntimeError::Message("integer overflow in div".to_owned()))
                }
                (RuntimeData::Float(a), RuntimeData::Float(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Float(a / b), protection))
                }
                (RuntimeData::Int(a), RuntimeData::Float(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a as f64 / b),
                    protection,
                )),
                (RuntimeData::Float(a), RuntimeData::Int(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a / b as f64),
                    protection,
                )),
                _ => Err(RuntimeError::Message("invalid div operands".to_owned())),
            },
            BinaryOperator::Rem => match (left.data, right.data) {
                (RuntimeData::Int(a), RuntimeData::Int(b)) => {
                    if b == 0 {
                        return Err(RuntimeError::Message("remainder by zero".to_owned()));
                    }
                    a.checked_rem(b)
                        .map(|value| RuntimeValue::new(RuntimeData::Int(value), protection))
                        .ok_or_else(|| RuntimeError::Message("integer overflow in rem".to_owned()))
                }
                (RuntimeData::Float(a), RuntimeData::Float(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Float(a % b), protection))
                }
                (RuntimeData::Int(a), RuntimeData::Float(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a as f64 % b),
                    protection,
                )),
                (RuntimeData::Float(a), RuntimeData::Int(b)) => Ok(RuntimeValue::new(
                    RuntimeData::Float(a % b as f64),
                    protection,
                )),
                _ => Err(RuntimeError::Message("invalid rem operands".to_owned())),
            },
            BinaryOperator::Pow => {
                let data = match (left.data, right.data) {
                    (RuntimeData::Int(base), RuntimeData::Int(exponent)) if exponent >= 0 => {
                        u32::try_from(exponent)
                            .ok()
                            .and_then(|exponent| base.checked_pow(exponent))
                            .map(RuntimeData::Int)
                    }
                    (RuntimeData::Float(base), RuntimeData::Int(exponent)) if exponent >= 0 => {
                        i32::try_from(exponent)
                            .ok()
                            .map(|exponent| RuntimeData::Float(base.powi(exponent)))
                    }
                    _ => None,
                };
                Ok(data
                    .map(|data| RuntimeValue::new(data, protection))
                    .unwrap_or_else(|| RuntimeValue::none(protection)))
            }
            BinaryOperator::Equal => Ok(RuntimeValue::new(
                RuntimeData::Bool(left.data == right.data),
                protection,
            )),
            BinaryOperator::NotEqual => Ok(RuntimeValue::new(
                RuntimeData::Bool(left.data != right.data),
                protection,
            )),
            BinaryOperator::Greater
            | BinaryOperator::GreaterEqual
            | BinaryOperator::Less
            | BinaryOperator::LessEqual => {
                self.eval_ordering(left.data, operator, right.data, protection)
            }
            BinaryOperator::And => match (left.data, right.data) {
                (RuntimeData::Bool(a), RuntimeData::Bool(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Bool(a && b), protection))
                }
                _ => Err(RuntimeError::Message("invalid and operands".to_owned())),
            },
            BinaryOperator::Or => match (left.data, right.data) {
                (RuntimeData::Bool(a), RuntimeData::Bool(b)) => {
                    Ok(RuntimeValue::new(RuntimeData::Bool(a || b), protection))
                }
                _ => Err(RuntimeError::Message("invalid or operands".to_owned())),
            },
        }
    }

    fn eval_ordering(
        &self,
        left: RuntimeData,
        operator: BinaryOperator,
        right: RuntimeData,
        protection: ProtectionLevel,
    ) -> Result<RuntimeValue, RuntimeError> {
        let left_type = left.type_name();
        let right_type = right.type_name();
        let result = match (left, right) {
            (RuntimeData::Int(a), RuntimeData::Int(b)) => compare_values(a, b, operator),
            (RuntimeData::Float(a), RuntimeData::Float(b)) => compare_values(a, b, operator),
            _ => {
                return Err(RuntimeError::Message(format!(
                    "invalid ordering operands for {operator:?}: {left_type} and {right_type}"
                )));
            }
        };
        Ok(RuntimeValue::new(RuntimeData::Bool(result), protection))
    }

    fn eval_bool(&mut self, expression: &CompiledExpression) -> Result<bool, RuntimeError> {
        match self.eval_expression(expression)?.data {
            RuntimeData::Bool(value) => Ok(value),
            RuntimeData::None => Ok(false),
            _ => Err(RuntimeError::Message("condition is not Bool".to_owned())),
        }
    }

    fn runtime_bool(&self, value: RuntimeValue) -> Result<bool, RuntimeError> {
        match value.data {
            RuntimeData::Bool(value) => Ok(value),
            RuntimeData::None => Ok(false),
            _ => Err(RuntimeError::Message("condition is not Bool".to_owned())),
        }
    }

    fn apply_arithmetic(
        &self,
        current: RuntimeValue,
        right: RuntimeValue,
        operator: AssignmentOperator,
    ) -> Result<RuntimeValue, RuntimeError> {
        let binary = match operator {
            AssignmentOperator::Assign => unreachable!(),
            AssignmentOperator::AddAssign => BinaryOperator::Add,
            AssignmentOperator::SubAssign => BinaryOperator::Sub,
            AssignmentOperator::MulAssign => BinaryOperator::Mul,
            AssignmentOperator::DivAssign => BinaryOperator::Div,
            AssignmentOperator::RemAssign => BinaryOperator::Rem,
        };
        self.eval_binary(current, binary, right)
    }
    fn pop_list(&mut self, name: &str) -> Result<RuntimeValue, RuntimeError> {
        if let Some(list) = self.vtime_mut(name) {
            return pop_list_value(list);
        }
        let Some(list) = self.lists.get_mut(name) else {
            return Ok(RuntimeValue::none(ProtectionLevel::Av));
        };
        pop_list_value(list)
    }

    fn add_to_list(&mut self, name: &str, value: RuntimeValue) -> Result<(), RuntimeError> {
        if let Some(list) = self.vtime_mut(name) {
            return push_list_value(name, list, value);
        }
        let Some(list) = self.lists.get_mut(name) else {
            return Err(RuntimeError::Message(format!("unknown list `{name}`")));
        };
        push_list_value(name, list, value)
    }

    fn set_list_element(
        &mut self,
        name: &str,
        index: RuntimeValue,
        value: RuntimeValue,
    ) -> Result<(), RuntimeError> {
        if let Some(list) = self.vtime_mut(name) {
            return replace_list_value(name, list, index, value);
        }
        let Some(list) = self.lists.get_mut(name) else {
            return Err(RuntimeError::Message(format!("unknown list `{name}`")));
        };
        replace_list_value(name, list, index, value)
    }

    fn vtime_mut(&mut self, name: &str) -> Option<&mut RuntimeValue> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(value) = scope.get_mut(name) {
                return Some(value);
            }
        }
        None
    }

    fn pick_items(&self, value: RuntimeValue) -> Result<Vec<RuntimeValue>, RuntimeError> {
        let protection = value.effective_protection();
        match value.data {
            RuntimeData::List(values) => Ok(values.as_ref().clone()),
            RuntimeData::Str(value) => Ok(value
                .chars
                .iter()
                .copied()
                .map(|ch| RuntimeValue::str(ch.to_string()).with_protection(protection))
                .collect()),
            RuntimeData::Bytes(value) => Ok(value
                .into_iter()
                .map(|byte| RuntimeValue::new(RuntimeData::Int(i64::from(byte)), protection))
                .collect()),
            _ => Err(RuntimeError::Message("value is not pickable".to_owned())),
        }
    }

    fn assign_existing(&mut self, name: &str, value: RuntimeValue) -> Result<(), RuntimeError> {
        if self.vtime_exists(name) {
            self.update_vtime(name, value);
            return Ok(());
        }
        let Some(slot) = self.absolute.get_mut(name) else {
            return Err(RuntimeError::Message(format!("unknown variable `{name}`")));
        };
        let target_protection = variable_kind_protection(slot.kind);
        if value.effective_protection() > target_protection {
            return Err(RuntimeError::Message(format!(
                "refusing to downgrade protection of `{name}`"
            )));
        }
        let value =
            if value.matches_type(slot.value_type) || matches!(value.data, RuntimeData::None) {
                value.with_protection(target_protection)
            } else {
                RuntimeValue::none(target_protection)
            };
        slot.current = value;
        Ok(())
    }

    fn lookup(&self, name: &str) -> Result<RuntimeValue, RuntimeError> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Ok(value.clone());
            }
        }
        if let Some(slot) = self.absolute.get(name) {
            return Ok(slot.current.clone());
        }
        if let Some(value) = self.lists.get(name) {
            return Ok(value.clone());
        }
        Err(RuntimeError::Message(format!("unknown variable `{name}`")))
    }

    fn lookup_absolute(&self, name: &str) -> Result<&AbsoluteSlot, RuntimeError> {
        self.absolute
            .get(name)
            .ok_or_else(|| RuntimeError::Message(format!("unknown absolute variable `{name}`")))
    }

    fn with_scope<T>(
        &mut self,
        body: impl FnOnce(&mut Self) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        self.scopes.push(HashMap::new());
        let result = body(self);
        self.scopes.pop();
        result
    }

    fn declare_vtime(&mut self, name: String, value: RuntimeValue) {
        if self.scopes.is_empty() {
            self.scopes.push(HashMap::new());
        }
        self.scopes.last_mut().unwrap().insert(name, value);
    }

    fn update_vtime(&mut self, name: &str, value: RuntimeValue) {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_owned(), value);
                return;
            }
        }
    }

    fn vtime_exists(&self, name: &str) -> bool {
        self.scopes
            .iter()
            .rev()
            .any(|scope| scope.contains_key(name))
    }

    fn read_input(&mut self, protection: ProtectionLevel) -> Result<RuntimeValue, RuntimeError> {
        let (input, from_argument, slot) = self.input.read()?;
        if from_argument && protection != ProtectionLevel::Av {
            return Err(RuntimeError::Message(format!(
                "secret input cannot be supplied by process argument `-{slot}`"
            )));
        }
        Ok(input
            .map(RuntimeValue::str)
            .unwrap_or_else(|| RuntimeValue::none(protection))
            .with_protection(protection))
    }
}

fn pop_stack_value(stack: &mut Vec<RuntimeValue>) -> Result<RuntimeValue, RuntimeError> {
    stack
        .pop()
        .ok_or_else(|| RuntimeError::Message("compiled expression stack underflow".to_owned()))
}

fn pop_stack_values(
    stack: &mut Vec<RuntimeValue>,
    count: usize,
) -> Result<Vec<RuntimeValue>, RuntimeError> {
    if stack.len() < count {
        return Err(RuntimeError::Message(
            "compiled expression stack underflow".to_owned(),
        ));
    }
    let start = stack.len() - count;
    Ok(stack.split_off(start))
}

fn pop_optional_stack_int(
    stack: &mut Vec<RuntimeValue>,
    has_value: bool,
) -> Result<Option<i64>, RuntimeError> {
    if !has_value {
        return Ok(None);
    }
    match pop_stack_value(stack)?.data {
        RuntimeData::Int(value) => Ok(Some(value)),
        _ => Ok(None),
    }
}

fn push_list_value(
    name: &str,
    list: &mut RuntimeValue,
    value: RuntimeValue,
) -> Result<(), RuntimeError> {
    let RuntimeData::List(values) = &mut list.data else {
        return Err(RuntimeError::Message(format!("`{name}` is not a list")));
    };
    list.protection = list.protection.max(value.effective_protection());
    Rc::make_mut(values).push(value);
    Ok(())
}

fn replace_list_value(
    name: &str,
    list: &mut RuntimeValue,
    index: RuntimeValue,
    value: RuntimeValue,
) -> Result<(), RuntimeError> {
    let RuntimeData::Int(index) = index.data else {
        return Ok(());
    };
    let RuntimeData::List(values) = &mut list.data else {
        return Err(RuntimeError::Message(format!("`{name}` is not a list")));
    };
    let Some(index) = resolve_raw_index(values.len(), index) else {
        return Ok(());
    };
    Rc::make_mut(values)[index] = value;
    list.refresh_list_protection();
    Ok(())
}

fn pop_list_value(list: &mut RuntimeValue) -> Result<RuntimeValue, RuntimeError> {
    let protection = list.effective_protection();
    let RuntimeData::List(values) = &mut list.data else {
        return Ok(RuntimeValue::none(protection));
    };
    let popped = Rc::make_mut(values)
        .pop()
        .unwrap_or_else(|| RuntimeValue::none(protection));
    list.refresh_list_protection();
    Ok(popped)
}

impl RuntimeValue {
    fn new(data: RuntimeData, protection: ProtectionLevel) -> Self {
        Self { data, protection }
    }

    fn none(protection: ProtectionLevel) -> Self {
        Self::new(RuntimeData::None, protection)
    }

    fn list(values: Vec<RuntimeValue>) -> Self {
        let protection = values
            .iter()
            .map(RuntimeValue::effective_protection)
            .max()
            .unwrap_or(ProtectionLevel::Av);
        Self::new(RuntimeData::List(Rc::new(values)), protection)
    }

    fn str(value: String) -> Self {
        Self::new(
            RuntimeData::Str(Rc::new(RuntimeString::new(value))),
            ProtectionLevel::Av,
        )
    }

    fn from_ast(value: AstValue) -> Self {
        let data = match value {
            AstValue::Int(value) => RuntimeData::Int(value),
            AstValue::Float(value) => RuntimeData::Float(value),
            AstValue::Bool(value) => RuntimeData::Bool(value),
            AstValue::Str(value) => RuntimeData::Str(Rc::new(RuntimeString::new(value))),
            AstValue::Bytes(value) => RuntimeData::Bytes(value),
            AstValue::Json(value) => RuntimeData::Json(value),
        };
        Self::new(data, ProtectionLevel::Av)
    }

    fn with_protection(mut self, protection: ProtectionLevel) -> Self {
        self.protection = if matches!(&self.data, RuntimeData::List(_)) {
            self.protection.max(protection)
        } else {
            protection
        };
        self
    }

    fn effective_protection(&self) -> ProtectionLevel {
        self.protection
    }

    fn refresh_list_protection(&mut self) {
        if let RuntimeData::List(values) = &self.data {
            self.protection = values
                .iter()
                .map(RuntimeValue::effective_protection)
                .max()
                .unwrap_or(ProtectionLevel::Av);
        }
    }

    fn matches_type(&self, value_type: ValueType) -> bool {
        matches!(
            (&self.data, value_type),
            (RuntimeData::Int(_), ValueType::Int)
                | (RuntimeData::Float(_), ValueType::Float)
                | (RuntimeData::Bool(_), ValueType::Bool)
                | (RuntimeData::Str(_), ValueType::Str)
                | (RuntimeData::Bytes(_), ValueType::Bytes)
                | (RuntimeData::Json(_), ValueType::Json)
                | (RuntimeData::List(_), ValueType::List)
        )
    }

    fn convert(self, value_type: ValueType) -> Self {
        let protection = self.protection;
        if matches!(self.data, RuntimeData::None) {
            return Self::none(protection);
        }
        let converted = match value_type {
            ValueType::Int => match &self.data {
                RuntimeData::Int(value) => Some(RuntimeData::Int(*value)),
                RuntimeData::Float(value)
                    if value.is_finite()
                        && *value >= i64::MIN as f64
                        && *value < -(i64::MIN as f64) =>
                {
                    Some(RuntimeData::Int(*value as i64))
                }
                _ => self.render().parse::<i64>().ok().map(RuntimeData::Int),
            },
            ValueType::Float => self.render().parse::<f64>().ok().map(RuntimeData::Float),
            ValueType::Bool => match self.render().as_str() {
                "true" => Some(RuntimeData::Bool(true)),
                "false" => Some(RuntimeData::Bool(false)),
                _ => None,
            },
            ValueType::Str => Some(RuntimeData::Str(Rc::new(RuntimeString::new(self.render())))),
            ValueType::Bytes => Some(RuntimeData::Bytes(self.render().into_bytes())),
            ValueType::Json => match &self.data {
                RuntimeData::Json(value) => Some(RuntimeData::Json(value.clone())),
                _ => {
                    let rendered = self.render();
                    serde_json::from_str::<serde_json::Value>(&rendered)
                        .ok()
                        .map(|_| RuntimeData::Json(rendered))
                }
            },
            ValueType::List => match self.data {
                RuntimeData::List(values) => Some(RuntimeData::List(values)),
                _ => None,
            },
        };
        converted
            .map(|data| Self::new(data, protection))
            .unwrap_or_else(|| Self::none(protection))
    }

    fn render(&self) -> String {
        match &self.data {
            RuntimeData::Int(value) => value.to_string(),
            RuntimeData::Float(value) => value.to_string(),
            RuntimeData::Bool(value) => value.to_string(),
            RuntimeData::Str(value) => value.text.clone(),
            RuntimeData::Bytes(value) => format!("{value:?}"),
            RuntimeData::Json(value) => value.clone(),
            RuntimeData::List(values) => {
                let inner = values
                    .iter()
                    .map(RuntimeValue::render)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            RuntimeData::None => "NONE".to_owned(),
        }
    }
}

fn debug_variable(name: &str, value: &RuntimeValue) -> DebugVariable {
    let protection = value.effective_protection();
    let rendered = match protection {
        ProtectionLevel::Av => value.render(),
        ProtectionLevel::Asv => "<ASV protected>".to_owned(),
        ProtectionLevel::Sasv => "<SASV protected>".to_owned(),
    };
    DebugVariable {
        name: name.to_owned(),
        value: rendered,
        protection,
    }
}

fn variable_kind_protection(kind: VariableKind) -> ProtectionLevel {
    match kind {
        VariableKind::Av => ProtectionLevel::Av,
        VariableKind::Asv => ProtectionLevel::Asv,
        VariableKind::Sasv => ProtectionLevel::Sasv,
    }
}

fn conversion_type(name: &str) -> Option<ValueType> {
    match name {
        "int" => Some(ValueType::Int),
        "float" => Some(ValueType::Float),
        "bool" => Some(ValueType::Bool),
        "str" => Some(ValueType::Str),
        "bytes" => Some(ValueType::Bytes),
        "json" => Some(ValueType::Json),
        _ => None,
    }
}

fn compare_values<T: PartialOrd>(a: T, b: T, operator: BinaryOperator) -> bool {
    match operator {
        BinaryOperator::Greater => a > b,
        BinaryOperator::GreaterEqual => a >= b,
        BinaryOperator::Less => a < b,
        BinaryOperator::LessEqual => a <= b,
        _ => unreachable!(),
    }
}

fn resolve_index<T>(values: &[T], index: i64) -> Option<&T> {
    let index = resolve_raw_index(values.len(), index)?;
    values.get(index)
}

fn resolve_raw_index(len: usize, index: i64) -> Option<usize> {
    let index = if index < 0 {
        len.checked_sub(index.unsigned_abs() as usize)
    } else {
        usize::try_from(index).ok()
    }?;
    (index < len).then_some(index)
}

fn slice_indices(len: usize, start: Option<i64>, end: Option<i64>, step: i64) -> Vec<usize> {
    debug_assert_ne!(step, 0);

    if step > 0 {
        let mut index = normalize_forward_slice_bound(len, start.unwrap_or(0));
        let end = normalize_forward_slice_bound(len, end.unwrap_or(len as i64));
        let mut indices = Vec::new();

        while index < end {
            indices.push(index as usize);
            index += step as isize;
        }

        return indices;
    }

    let mut index = normalize_reverse_slice_start(len, start);
    let end = normalize_reverse_slice_end(len, end);
    let mut indices = Vec::new();

    while index > end {
        if (0..len as isize).contains(&index) {
            indices.push(index as usize);
        }
        index += step as isize;
    }

    indices
}

fn normalize_forward_slice_bound(len: usize, bound: i64) -> isize {
    let len = len as isize;
    let index = if bound < 0 {
        len + bound as isize
    } else {
        bound as isize
    };
    index.clamp(0, len)
}

fn normalize_reverse_slice_start(len: usize, start: Option<i64>) -> isize {
    let len = len as isize;
    let Some(start) = start else {
        return len - 1;
    };
    let index = if start < 0 {
        len + start as isize
    } else {
        start as isize
    };
    index.clamp(-1, len - 1)
}

fn normalize_reverse_slice_end(len: usize, end: Option<i64>) -> isize {
    let len = len as isize;
    let Some(end) = end else {
        return -1;
    };
    let index = if end < 0 {
        len + end as isize
    } else {
        end as isize
    };
    index.clamp(-1, len - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use apl_core::{
        validate_program, Assignment, BinaryOperator, Expression, ListDecl, ListElement, Statement,
        Value, VariableDecl, WhileStatement,
    };

    #[test]
    fn timer_accumulates_pauses_and_rejects_invalid_transitions() {
        let mut timer = TimerState::default();
        assert_eq!(timer.elapsed(), Ok(Duration::ZERO));
        assert_eq!(timer.stop(), Err("timer is not running"));
        assert_eq!(timer.start(), Ok(()));
        assert_eq!(timer.start(), Err("timer is already running"));

        timer.started_at = Some(Instant::now() - Duration::from_secs(2));
        timer.stop().unwrap();
        let first = timer.elapsed().unwrap();
        assert!(first >= Duration::from_secs(2));
        assert!(first < Duration::from_secs(3));
        assert_eq!(timer.elapsed(), Ok(first));

        timer.start().unwrap();
        timer.started_at = Some(Instant::now() - Duration::from_secs(3));
        timer.stop().unwrap();
        let resumed = timer.elapsed().unwrap();
        assert!(resumed >= Duration::from_secs(5));
        assert!(resumed < Duration::from_secs(6));
    }

    fn run(source: Vec<Statement>) -> Result<String, RuntimeError> {
        run_program(&Program::new(source), vec![]).map(|output| output.stdout)
    }

    fn run_source(source: &str) -> Result<String, RuntimeError> {
        let program = apl_parser::parse_program(source).unwrap();
        validate_program(&program).unwrap();
        run_program(&program, vec![]).map(|output| output.stdout)
    }

    #[test]
    fn debugger_tracks_source_stack_and_redacts_secrets() {
        let source = "ASVInt hidden = 73\nfunc touch(value) {\n  VTime local = value\n  return local\n}\nAVInt std::main() {\n  AVInt visible = 4\n  touch(visible)\n  return 0\n}";
        let parsed = apl_parser::parse_program_with_locations(source).unwrap();
        validate_program(&parsed.program).unwrap();
        let locations = parsed
            .locations
            .into_iter()
            .map(|location| DebugSourceLocation {
                path: location.path,
                line: location.line,
                column: location.column,
            })
            .collect::<Vec<_>>();
        let mut snapshots = Vec::new();

        let output = run_program_debug(&parsed.program, &locations, Vec::new(), |snapshot| {
            snapshots.push(snapshot.clone());
            DebugAction::Continue
        })
        .unwrap();

        assert_eq!(output.stdout, "");
        assert!(snapshots.iter().any(|snapshot| snapshot.location.line == 7));
        assert!(snapshots.iter().any(|snapshot| {
            snapshot.location.line == 3 && snapshot.call_stack.iter().any(|frame| frame == "touch")
        }));
        assert!(snapshots.iter().any(|snapshot| {
            snapshot.variables.iter().any(|variable| {
                variable.name == "hidden"
                    && variable.value == "<ASV protected>"
                    && variable.protection == ProtectionLevel::Asv
            })
        }));
        assert!(!snapshots
            .iter()
            .flat_map(|snapshot| &snapshot.variables)
            .any(|variable| variable.name == "hidden" && variable.value.contains("73")));
    }

    fn run_source_with_prelude(source: &str) -> Result<String, RuntimeError> {
        run_source_with_prelude_and_input(source, vec![])
    }

    fn run_source_with_prelude_and_input(
        source: &str,
        input: Vec<String>,
    ) -> Result<String, RuntimeError> {
        std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("apl-self-hosted-runtime-test".to_owned())
                .stack_size(8 * 1024 * 1024)
                .spawn_scoped(scope, move || {
                    run_source_with_prelude_on_current_thread(source, input)
                })
                .expect("spawn self-hosted runtime test")
                .join()
                .expect("self-hosted runtime test panicked")
        })
    }

    fn run_source_with_prelude_on_current_thread(
        source: &str,
        input: Vec<String>,
    ) -> Result<String, RuntimeError> {
        let prelude = concat!(
            include_str!("../../../std/runtime.apl"),
            "\n\n",
            include_str!("../../../std/lexer.apl"),
            "\n\n",
            include_str!("../../../std/parser.apl"),
            "\n\n",
            include_str!("../../../std/raw_memory.apl"),
            "\n\n",
            include_str!("../../../std/structures.apl"),
            "\n\n",
            include_str!("../../../std/classes.apl"),
            "\n\n",
            include_str!("../../../std/checker.apl"),
            "\n\n",
            include_str!("../../../std/ir.apl"),
            "\n\n",
            include_str!("../../../std/linker.apl"),
            "\n\n",
            include_str!("../../../std/verifier.apl"),
            "\n\n",
            include_str!("../../../std/vm.apl"),
            "\n\n",
            include_str!("../../../std/artifact.apl"),
            "\n\n",
            include_str!("../../../std/native.apl"),
            "\n\n",
            include_str!("../../../std/abi.apl"),
            "\n\n",
            include_str!("../../../std/assembly.apl"),
            "\n\n",
            include_str!("../../../std/bootstrap.apl")
        );
        let source = format!("{}\n\n{}\n", prelude.trim(), source.trim());
        let program = apl_parser::parse_program(&source).unwrap();
        validate_program(&program).unwrap();
        run_program(&program, input).map(|output| output.stdout)
    }

    fn apl_compile_outcomes(sources: &[&str]) -> Vec<String> {
        let mut input = Vec::with_capacity(sources.len() + 1);
        input.push(sources.len().to_string());
        input.extend(sources.iter().map(|source| (*source).to_owned()));

        let output = run_source_with_prelude_and_input(
            r#"
            VTime apltest.remaining = int(input)
            VTime apltest.index = 0

            while (apltest.index < apltest.remaining) (-1) {
              VTime apltest.report = bootstrap.compile_report(input)
              out get(apltest.report, 0)
              out char(0)
              apltest.index += 1
            }
            "#,
            input,
        )
        .unwrap();

        output.split_terminator('\0').map(str::to_owned).collect()
    }

    fn apl_run_outcomes(sources: &[&str]) -> Vec<(String, String)> {
        let mut input = Vec::with_capacity(sources.len() + 1);
        input.push(sources.len().to_string());
        input.extend(sources.iter().map(|source| (*source).to_owned()));

        let output = run_source_with_prelude_and_input(
            r#"
            VTime apltest.remaining = int(input)
            VTime apltest.index = 0

            while (apltest.index < apltest.remaining) (-1) {
              VTime apltest.report = bootstrap.run_report(input)
              out get(apltest.report, 0)
              out char(0)
              out join(get(apltest.report, 1), "")
              out char(0)
              apltest.index += 1
            }
            "#,
            input,
        )
        .unwrap();

        let fields = output.split_terminator('\0').collect::<Vec<_>>();
        assert_eq!(fields.len(), sources.len() * 2);
        fields
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0].to_owned(), pair[1].to_owned()))
            .collect()
    }

    fn apl_run_outcomes_with_input(cases: &[(&str, &[&str])]) -> Vec<(String, String)> {
        let mut input = Vec::new();
        input.push(cases.len().to_string());
        for (source, program_input) in cases {
            input.push((*source).to_owned());
            input.push(program_input.len().to_string());
            input.extend(program_input.iter().map(|value| (*value).to_owned()));
        }

        let output = run_source_with_prelude_and_input(
            r#"
            VTime apltest.remaining = int(input)
            VTime apltest.index = 0

            while (apltest.index < apltest.remaining) (-1) {
              VTime apltest.source = input
              VTime apltest.input_remaining = int(input)
              VTime apltest.inputs = []
              VTime apltest.input_index = 0

              while (apltest.input_index < apltest.input_remaining) (-1) {
                add(apltest.inputs, input)
                apltest.input_index += 1
              }

              VTime apltest.report = bootstrap.run_with_input_report(apltest.source, apltest.inputs)
              out get(apltest.report, 0)
              out char(0)
              out join(get(apltest.report, 1), "")
              out char(0)
              apltest.index += 1
            }
            "#,
            input,
        )
        .unwrap();

        let fields = output.split_terminator('\0').collect::<Vec<_>>();
        assert_eq!(fields.len(), cases.len() * 2);
        fields
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| (pair[0].to_owned(), pair[1].to_owned()))
            .collect()
    }

    fn checked_program(source: &str) -> Program {
        let program = apl_parser::parse_program(source).unwrap();
        validate_program(&program).unwrap();
        program
    }

    fn block_ops(program: &CompiledProgram, block_id: BlockId) -> &[StatementOp] {
        let block = &program.blocks[block_id];
        &program.code[block.start..block.end]
    }

    #[test]
    fn runs_arithmetic_and_out() {
        let output = run(vec![
            Statement::VariableDecl(VariableDecl {
                kind: VariableKind::Av,
                value_type: ValueType::Int,
                name: "x".to_owned(),
                initial_value: Expression::Literal(Value::Int(1)),
            }),
            Statement::Assignment(Assignment {
                name: "x".to_owned(),
                operator: AssignmentOperator::AddAssign,
                value: Expression::Literal(Value::Int(2)),
            }),
            Statement::Out(Expression::Variable("x".to_owned())),
        ])
        .unwrap();

        assert_eq!(output, "3");
    }

    #[test]
    fn rust_and_apl_frontends_agree_on_core_source_acceptance() {
        let cases = [
            ("AVInt value = 1 \nout value", true),
            ("func halt() { \nstop \n} \nout 1", true),
            ("out 1 \nstop \nout 2", true),
            ("VTime value = [1, 2][0] \nout value", true),
            ("AVStr value = \"line\\nquote=\\\"\" \nout value", true),
            ("out 1 out 2", false),
            ("func work() { return 1 }", false),
            ("func work() {\n  return 1\n}\nout work()", true),
            ("AVInt value =", false),
            ("AVFloat value = 1.", false),
            ("func pair(left right) {\n  return left\n}", false),
            ("List values = [1 2]", false),
            ("out \"bad\\q\"", false),
            ("if true { \nout 1", false),
            ("break", false),
            ("return 1", false),
            ("AVInt value = 1 \nout missing", false),
            ("VTime value = 1 \nvalue = str(value) \nout value", true),
            ("out len([1, 2, 3])", true),
            (
                "List values = [1] \nset(values, -1, 2) \nout values[0]",
                true,
            ),
            ("List values = [1] \nset(values, true, 2)", false),
            (
                "List values = [1] \nVTime result = set(values, 0, 2)",
                false,
            ),
            ("out len(1)", false),
            ("out pow(2)", false),
            ("AVInt value = 7 % 3 \nout value", true),
            ("AVInt value = 7 % 3.0", false),
            ("AVStr value = \"x\" \nvalue %= 2", false),
            ("AVInt value = true", false),
            ("ASVInt hidden = 1 \nout hidden", false),
            ("while true (-2) {\n  break\n}", false),
            ("func same(value, value) { \nreturn value \n}", false),
            ("out missing()", false),
            (
                "func touch(value) { \nreturn value \n} \ntouch(42) \nout 1",
                true,
            ),
            ("include >> base \nout 1", true),
            ("exclude \"len\" << base \nout 1", true),
            ("exclude \"len\" << base \nout len([1])", false),
            (
                "exclude \"set\" << base \nList values = [1] \nset(values, 0, 2)",
                false,
            ),
            (
                "exclude \"pick\" << base \npick(\"A\"): item { \nout item \n}",
                false,
            ),
            ("include >> math \nout 1", false),
            ("exclude \"missing\" << base \nout 1", false),
            ("include >> base \ninclude >> base \nout 1", false),
            (
                "exclude \"len\" << base \nexclude \"len\" << base \nout 1",
                false,
            ),
            ("if true { \ninclude >> base \n}", false),
        ];
        let sources = cases.iter().map(|(source, _)| *source).collect::<Vec<_>>();
        let apl_outcomes = apl_compile_outcomes(&sources);

        assert_eq!(apl_outcomes.len(), cases.len());
        for ((source, expected), apl_status) in cases.iter().zip(apl_outcomes) {
            let rust_accepts = apl_parser::parse_program(source)
                .ok()
                .and_then(|program| validate_program(&program).ok())
                .is_some();
            let apl_accepts = apl_status == "OK";

            assert_eq!(
                rust_accepts, *expected,
                "unexpected Rust result for `{source}`"
            );
            assert_eq!(
                apl_accepts, *expected,
                "unexpected APL result for `{source}`"
            );
        }
    }

    #[test]
    fn source_runtime_apl_frontend_enforces_base_composition_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            VTime ok_source = "include >> base \nexclude \"pick\" << base \nout 42"
            VTime excluded_source = "exclude \"len\" << base \nout len([1])"
            VTime unknown_source = "include >> math \nout 1"
            VTime nested_source = "if true { \ninclude >> base \n}"
            VTime ok_report = bootstrap.compile_report(ok_source)
            VTime excluded_report = bootstrap.compile_report(excluded_source)
            VTime unknown_report = bootstrap.compile_report(unknown_source)
            VTime nested_report = bootstrap.compile_report(nested_source)

            out get(ok_report, 0)
            out len(get(ok_report, 1))
            out ir.opcode(get(get(ok_report, 1), 0))
            out get(excluded_report, 0)
            out get(excluded_report, 1)
            out get(unknown_report, 0)
            out get(unknown_report, 1)
            out get(nested_report, 0)
            out get(nested_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OK1OUTFAILsemantic error at 2:1: excluded symbol `len` from library `base`FAILsemantic error at 1:1: unknown library `math`FAILsemantic error at 2:1: composition directive must be top-level"
        );
    }

    #[test]
    fn rust_and_apl_runtimes_agree_on_core_public_programs() {
        let cases = [
            ("arithmetic", "out (2 + 3) * 4", "OK", "20"),
            (
                "remainder",
                "AVInt value = 17 \nvalue %= 5 \nout 2 + 10 % 4 * 3 \nout value \nout -17 % 5 \nout 2 ** 5 % 5",
                "OK",
                "82-22",
            ),
            ("float", "AVFloat value = -1.5 \nout value", "OK", "-1.5"),
            (
                "escapes",
                r#"out "line\nquote=\" slash=\\ tab=\t""#,
                "OK",
                "line\nquote=\" slash=\\ tab=\t",
            ),
            (
                "function",
                "func double(value) { \nreturn value * 2 \n} \nout double(21)",
                "OK",
                "42",
            ),
            (
                "while",
                "AVInt value = 0 \nwhile (value < 4) (10) { \nvalue += 1 \n} \nout value",
                "OK",
                "4",
            ),
            ("pick", r#"pick("APL"): item {
out item
}"#, "OK", "APL"),
            (
                "nested index",
                "List matrix = [[1, 2], [3, 4]] \nout matrix[1][0]",
                "OK",
                "3",
            ),
            (
                "reverse slice",
                r#"AVStr value = "APL"
out value[::-1]"#,
                "OK",
                "LPA",
            ),
            (
                "conversions",
                r#"out int("42")
out str(true)
out bool(0)"#,
                "OK",
                "42trueNONE",
            ),
            ("logical", "out (3 < 4) and (4 != 5)", "OK", "true"),
            (
                "self",
                "AVInt value = 1 \nout value =self= \nvalue += 1 \nout value =self=",
                "OK",
                "truefalse",
            ),
            ("none", "VTime value = NONE \nout value", "OK", "NONE"),
            (
                "list mutation",
                "List values = [1, 2] \nadd(values, 3) \nout pop(values) \nout len(values)",
                "OK",
                "32",
            ),
            (
                "list replacement",
                "List values = [1, 2, 3] \nset(values, 0, 9) \nset(values, -1, 7) \nset(values, 99, 5) \nout values[0] \nout values[2] \nout len(values)",
                "OK",
                "973",
            ),
            (
                "info",
                r#"SASVStr key = "raw"
AVStr typ = ""
AVStr level = ""
typ, level = info(key)
out typ
out level"#,
                "OK",
                "StrSASV",
            ),
            (
                "bare stop",
                r#"out "before"
if true {
stop
}
out "after""#,
                "STOP",
                "before",
            ),
            (
                "if else-if",
                "AVInt value = 2 \nif value == 1 { \nout 1 \n} else if value == 2 { \nout 2 \n} else { \nout 3 \n}",
                "OK",
                "2",
            ),
            (
                "while limit",
                "AVInt value = 0 \nwhile (true) (3) { \nvalue += 1 \n} \nout value",
                "OK",
                "3",
            ),
            (
                "continue \nand break",
                "AVInt value = 0 \nwhile (value < 10) (20) { \nvalue += 1 \nif value == 2 { \ncontinue \n} \nif value == 4 { \nbreak \n} \nout value \n} \nout value",
                "OK",
                "134",
            ),
            (
                "builtins",
                r#"out len("APL")
out get([7], 0)
out join(split("a,b", ","), "-")
out contains("apl", "p")
out ord("A")
out char(66)
out pow(2, 5)"#,
                "OK",
                "37a-btrue65B32",
            ),
            (
                "slices",
                r#"out "abcdef"[1:5:2]
out [1, 2, 3][-1]"#,
                "OK",
                "bd3",
            ),
            (
                "bytes and json",
                r#"out bytes("AB")
out json("42")"#,
                "OK",
                "[65, 66]42",
            ),
            (
                "list rendering",
                r#"out [1, "x", true, NONE]"#,
                "OK",
                "[1, x, true, NONE]",
            ),
            (
                "nested functions",
                "func add_one(value) { \nreturn value + 1 \n} \nfunc twice(value) { \nreturn add_one(add_one(value)) \n} \nout twice(40)",
                "OK",
                "42",
            ),
            (
                "nested scope",
                "AVInt value = 1 \nif true { \nVTime amount = 2 \nvalue += amount \n} \nout value",
                "OK",
                "3",
            ),
            (
                "base composition",
                "include >> base \nexclude \"len\" << base \nout 42",
                "OK",
                "42",
            ),
        ];
        let sources = cases
            .iter()
            .map(|(_, source, _, _)| *source)
            .collect::<Vec<_>>();
        let apl_outcomes = apl_run_outcomes(&sources);
        assert_eq!(apl_outcomes.len(), cases.len());
        for ((name, source, expected_status, expected_output), (apl_status, apl_output)) in
            cases.iter().zip(apl_outcomes)
        {
            let program = apl_parser::parse_program(source).unwrap();
            validate_program(&program).unwrap();
            let rust_output = run_program(&program, vec![]).unwrap().stdout;

            assert_eq!(rust_output, *expected_output, "Rust output for {name}");
            assert_eq!(apl_status, *expected_status, "APL status for {name}");
            assert_eq!(apl_output, *expected_output, "APL output for {name}");
        }
    }

    #[test]
    fn rust_and_apl_runtimes_agree_on_input_programs() {
        let cases: [(&str, &[&str], &str); 6] = [
            ("VTime value = input \nout value", &["hello"], "hello"),
            (
                "AVInt whole = input \nAVFloat ratio = input \nAVBool flag = input \nAVStr text = input \nout whole \nout ratio \nout flag \nout text",
                &["42", "2.5", "true", "abc"],
                "422.5trueabc",
            ),
            (
                "AVInt whole = input \nAVFloat ratio = input \nAVBool flag = input \nout whole \nout ratio \nout flag",
                &["bad", "bad", "0"],
                "NONENONENONE",
            ),
            (
                r#"VTime first = input
VTime second = input
out first
out "|"
out second"#,
                &["only"],
                "only|NONE",
            ),
            (
                r#"ASVStr hidden = secret input
AVStr typ = ""
AVStr level = ""
typ, level = info(hidden)
out typ
out level"#,
                &["hidden"],
                "StrASV",
            ),
            (
                "func read() { \nreturn input \n} \nout read() \nout input",
                &["A", "B"],
                "AB",
            ),
        ];
        let apl_cases = cases
            .iter()
            .map(|(source, input, _)| (*source, *input))
            .collect::<Vec<_>>();
        let apl_outcomes = apl_run_outcomes_with_input(&apl_cases);

        assert_eq!(apl_outcomes.len(), cases.len());
        for ((source, input, expected_output), (apl_status, apl_output)) in
            cases.iter().zip(apl_outcomes)
        {
            let program = apl_parser::parse_program(source).unwrap();
            validate_program(&program).unwrap();
            let rust_output = run_program(
                &program,
                input.iter().map(|value| (*value).to_owned()).collect(),
            )
            .unwrap()
            .stdout;

            assert_eq!(rust_output, *expected_output, "Rust output for `{source}`");
            assert_eq!(apl_status, "OK", "APL status for `{source}`");
            assert_eq!(apl_output, *expected_output, "APL output for `{source}`");
        }
    }

    #[test]
    fn integer_overflow_is_a_runtime_error() {
        let binary_cases = [
            (i64::MAX, BinaryOperator::Add, 1, "integer overflow in add"),
            (i64::MIN, BinaryOperator::Sub, 1, "integer overflow in sub"),
            (i64::MAX, BinaryOperator::Mul, 2, "integer overflow in mul"),
            (i64::MIN, BinaryOperator::Div, -1, "integer overflow in div"),
        ];

        for (left, operator, right, message) in binary_cases {
            let expression = Expression::Binary {
                left: Box::new(Expression::Literal(Value::Int(left))),
                operator,
                right: Box::new(Expression::Literal(Value::Int(right))),
            };
            assert_eq!(
                run(vec![Statement::Out(expression)]),
                Err(RuntimeError::Message(message.to_owned()))
            );
        }

        let negation = Expression::Unary {
            operator: UnaryOperator::Negate,
            expression: Box::new(Expression::Literal(Value::Int(i64::MIN))),
        };
        assert_eq!(
            run(vec![Statement::Out(negation)]),
            Err(RuntimeError::Message(
                "integer overflow in negate".to_owned()
            ))
        );
    }

    #[test]
    fn integer_division_by_zero_remains_a_runtime_error() {
        let expression = Expression::Binary {
            left: Box::new(Expression::Literal(Value::Int(1))),
            operator: BinaryOperator::Div,
            right: Box::new(Expression::Literal(Value::Int(0))),
        };

        assert_eq!(
            run(vec![Statement::Out(expression)]),
            Err(RuntimeError::Message("division by zero".to_owned()))
        );
    }

    #[test]
    fn source_runtime_apl_vm_propagates_integer_overflow_prelude() {
        let error = run_source_with_prelude(
            r#"
            AVStr source = "AVInt value = input \nout -value"
            VTime output = vm.run_source_with_input(source, ["-9223372036854775808"])
            out output
            "#,
        )
        .unwrap_err();

        let RuntimeError::Message(message) = error else {
            panic!("expected runtime error");
        };
        assert!(message.starts_with("integer overflow in sub"));
        assert!(message.contains("in `vm.eval_unary`"));
    }

    #[test]
    fn source_runtime_uses_pow_builtin() {
        let output = run_source(
            r#"
            AVInt x = pow(2, 3)
            AVFloat y = pow(2.0, 3)
            AVInt bad = pow(2, -1)
            ASVInt hidden = 2
            ASVInt hidden_pow = pow(hidden, 2)
            out x
            out y == 8.0
            out bad
            "#,
        )
        .unwrap();

        assert_eq!(output, "8trueNONE");
    }

    #[test]
    fn source_runtime_uses_power_operator() {
        let output = run_source(
            r#"
            AVInt nested = 2 ** 3 ** 2
            AVInt signed = -2 ** 2
            AVFloat decimal = 2.0 ** 3
            AVInt negative = 2 ** -1
            AVInt overflow = 2 ** 63
            ASVInt hidden = 3
            ASVInt protected = hidden ** 2
            AVStr type_name = ""
            AVStr protection = ""
            type_name, protection = info(protected)
            out nested
            out signed
            out decimal
            out negative
            out overflow
            out type_name
            out protection
            "#,
        )
        .unwrap();

        assert_eq!(output, "51248NONENONEIntASV");
    }

    #[test]
    fn source_runtime_uses_remainder_operator() {
        let output = run_source(
            r#"
            AVInt cycle = 17
            cycle %= 5
            AVFloat decimal = 5.5 % 2.0
            ASVInt hidden = -17
            ASVInt protected = hidden % 5
            AVStr type_name = ""
            AVStr protection = ""
            type_name, protection = info(protected)
            out 2 + 10 % 4 * 3
            out cycle
            out decimal
            out type_name
            out protection
            "#,
        )
        .unwrap();

        assert_eq!(output, "821.5IntASV");
    }

    #[test]
    fn integer_remainder_errors_are_explicit() {
        for (source, expected) in [
            ("out 1 % 0", "remainder by zero"),
            (
                "AVInt value = -9223372036854775808 \nout value % -1",
                "integer overflow in rem",
            ),
        ] {
            assert_eq!(
                run_source(source),
                Err(RuntimeError::Message(expected.to_owned()))
            );
        }
    }

    #[test]
    fn source_runtime_out_appends_and_strings_decode_escapes() {
        let output = run_source(
            r#"
            out "value="
            out 42
            out "\n"
            out "quote: \" slash: \\ tab:\t nul:\0"
            "#,
        )
        .unwrap();

        assert_eq!(output, "value=42\nquote: \" slash: \\ tab:\t nul:\0");
    }

    #[test]
    fn runs_while_with_limit() {
        let output = run(vec![
            Statement::VariableDecl(VariableDecl {
                kind: VariableKind::Av,
                value_type: ValueType::Int,
                name: "x".to_owned(),
                initial_value: Expression::Literal(Value::Int(0)),
            }),
            Statement::While(WhileStatement {
                condition: Expression::Binary {
                    left: Box::new(Expression::Variable("x".to_owned())),
                    operator: BinaryOperator::Less,
                    right: Box::new(Expression::Literal(Value::Int(10))),
                },
                limit: Expression::Literal(Value::Int(3)),
                body: vec![Statement::Assignment(Assignment {
                    name: "x".to_owned(),
                    operator: AssignmentOperator::AddAssign,
                    value: Expression::Literal(Value::Int(1)),
                })],
                limit_body: None,
                else_body: None,
            }),
            Statement::Out(Expression::Variable("x".to_owned())),
        ])
        .unwrap();

        assert_eq!(output, "3");
    }

    #[test]
    fn source_runtime_supports_dynamic_loop_limits_and_postfix_assignment() {
        let output = run_source(
            r#"
            AVInt limit = 2
            AVInt count = 0
            while (true) (limit) {
              count++
              limit = 0
            }
            count--
            out count
            "#,
        )
        .unwrap();

        assert_eq!(output, "1");
    }

    #[test]
    fn match_and_while_handlers_follow_their_completion_causes() {
        let output = run_source(
            r#"
            AVInt count = 0
            match 4 {
              case 1, 2 {
out "small"
}
              case 3..5 {
out "range"
}
              case default {
out "default"
}
            }
            while (false) (3) {
out "bad-body"
} limit {
out "bad-limit"
} else {
out ":else"
}
            while (true) (2) {
count += 1
} limit {
out ":limit"
} else {
out "bad-else"
}
            while (count < 3) (5) {
count += 1
} limit {
out "bad-limit"
} else {
out "bad-else"
}
            while (true) (1) {
break
} limit {
out "bad-break"
} else {
out "bad-break"
}
            out ":"
            out count
            "#,
        )
        .unwrap();

        assert_eq!(output, "range:else:limit:3");
    }

    #[test]
    fn error_return_terminates_the_whole_program_with_its_code() {
        let error = run_source(
            r#"
            func abort() {
              return(ERR=["boom"][7])
            }
            abort()
            out "unreachable"
            "#,
        )
        .unwrap_err();

        assert_eq!(
            error,
            RuntimeError::FailedWithCode {
                message: "boom".to_owned(),
                code: 7,
            }
        );
    }

    #[test]
    fn float_to_int_truncates_toward_zero() {
        let output = run_source("out int(3.7) \nout int(-3.7)").unwrap();

        assert_eq!(output, "3-3");
    }

    #[test]
    fn json_conversion_and_typed_input_validate_json() {
        let output = run_source("out json(\"{\") \nout json(\"42\")").unwrap();
        assert_eq!(output, "NONE42");

        let program = apl_parser::parse_program("AVJson value = input \nout value").unwrap();
        validate_program(&program).unwrap();
        let invalid = run_program(&program, vec!["{".to_owned()]).unwrap();
        let valid = run_program(&program, vec!["[1, 2]".to_owned()]).unwrap();
        assert_eq!(invalid.stdout, "NONE");
        assert_eq!(valid.stdout, "[1, 2]");
    }

    #[test]
    fn runs_nested_list_indexing() {
        let output = run(vec![
            Statement::ListDecl(ListDecl {
                name: "matrix".to_owned(),
                elements: vec![ListElement {
                    value: Expression::ListLiteral(vec![
                        ListElement {
                            value: Expression::Literal(Value::Int(1)),
                            protection: None,
                        },
                        ListElement {
                            value: Expression::Literal(Value::Int(2)),
                            protection: None,
                        },
                    ]),
                    protection: None,
                }],
            }),
            Statement::VTimeDecl(apl_core::VTimeDecl {
                name: "cell".to_owned(),
                initial_value: Expression::Index {
                    target: Box::new(Expression::Index {
                        target: Box::new(Expression::Variable("matrix".to_owned())),
                        index: Box::new(Expression::Literal(Value::Int(0))),
                    }),
                    index: Box::new(Expression::Literal(Value::Int(1))),
                },
            }),
            Statement::Out(Expression::Variable("cell".to_owned())),
        ])
        .unwrap();

        assert_eq!(output, "2");
    }

    #[test]
    fn source_runtime_handles_functions_lists_loops_and_slices() {
        let output = run_source(
            r#"
            List nums = [1, 2, 3, 4]
            AVInt total = 0

            func inc(value) {
              return value + 1
            }

            pick(nums[:3]): item {
              VTime next = inc(item)
              total += next
            }

            out total
            "#,
        )
        .unwrap();

        assert_eq!(output, "9");
    }

    #[test]
    fn source_runtime_handles_negative_step_slices() {
        let output = run_source(
            r#"
            List nums = [1, 2, 3]
            VTime reversed_nums = nums[::-1]
            AVStr text = "APL"
            VTime reversed_text = text[::-1]
            List empty = []
            VTime reversed_empty = empty[::-1]

            out get(reversed_nums, 0)
            out get(reversed_nums, 2)
            out reversed_text
            out len(reversed_empty)
            "#,
        )
        .unwrap();

        assert_eq!(output, "31LPA0");
    }

    #[test]
    fn source_runtime_rejects_zero_slice_step() {
        assert_eq!(
            run_source("List values = [1, 2, 3] \nVTime invalid = values[::0]"),
            Err(RuntimeError::Message("zero slice step".to_owned()))
        );
    }

    #[test]
    fn source_runtime_uses_apl_lexer_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = join(["AVInt x += 42 out ", char(34), "ok", char(34), " # ignored"], "")
            VTime tokens = lexer.tokenize(source)
            VTime first = get(tokens, 0)
            VTime op = get(tokens, 2)
            VTime string = get(tokens, 5)
            out len(tokens)
            out lexer.token_value(first)
            out lexer.token_kind(first)
            out lexer.token_value(op)
            out lexer.token_kind(op)
            out lexer.token_value(string)
            out lexer.token_kind(string)
            "#,
        )
        .unwrap();

        assert_eq!(output, "6AVIntKeyword+=SymbolokStr");
    }

    #[test]
    fn source_runtime_apl_lexer_decodes_and_validates_escapes() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = join([char(34), "line", char(92), "n", " tab", char(92), "t", " nul", char(92), "0", " slash", char(92), char(92), " quote", char(92), char(34), char(34)], "")
            VTime token = get(lexer.tokenize(source), 0)

            AVStr invalid_source = join([char(34), "bad", char(92), "q", char(34)], "")
            VTime invalid = get(lexer.tokenize(invalid_source), 0)

            AVStr unfinished_source = join([char(34), "bad", char(92)], "")
            VTime unfinished = get(lexer.tokenize(unfinished_source), 0)

            out lexer.token_value(token)
            out "|"
            out lexer.token_kind(invalid)
            out ":"
            out lexer.token_value(invalid)
            out "|"
            out lexer.token_kind(unfinished)
            out ":"
            out lexer.token_value(unfinished)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "line\n tab\t nul\0 slash\\ quote\"|Error:invalid escape sequence `\\q`|Error:unterminated escape sequence"
        );
    }

    #[test]
    fn source_runtime_supports_vtime_list_add_and_join() {
        let output = run_source(
            r#"
            VTime chars = []
            add(chars, "A")
            add(chars, "P")
            add(chars, "L")
            out join(chars, "")
            out ord("A")
            out char(65)
            "#,
        )
        .unwrap();

        assert_eq!(output, "APL65A");
    }

    #[test]
    fn source_runtime_uses_apl_parser_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 42 \nout x \nx += 1"
            VTime statements = parser.parse_source(source)
            VTime decl = get(statements, 0)
            VTime out_node = get(statements, 1)
            VTime assign = get(statements, 2)
            VTime literal = get(decl, 3)
            AVStr loop_source = "while (true) (-1) { \nbreak \n}"
            VTime loop_node = get(parser.parse_source(loop_source), 0)

            out len(statements)
            out parser.node_kind(decl)
            out get(decl, 1)
            out get(decl, 2)
            out parser.expr_kind(literal)
            out parser.expr_value(literal)
            out parser.node_kind(out_node)
            out parser.node_kind(assign)
            out get(assign, 2)
            out parser.node_kind(loop_node)
            out get(loop_node, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3DeclAVIntxInt42OutAssign+=While[Int, -1]");
    }

    #[test]
    fn source_runtime_parser_keeps_positions_out_of_the_success_ast() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 1 \nif true { \nout missing \n}"
            VTime compact = parser.parse_source(source)
            VTime located = parser.parse_source_with_positions(source)
            VTime compact_decl = get(compact, 0)
            VTime located_decl = get(located, 0)
            VTime nested_out = get(get(get(located, 1), 2), 0)

            out len(compact_decl)
            out parser.node_token_index(compact_decl) == NONE
            out len(located_decl)
            out parser.node_token_index(located_decl)
            out parser.node_token_index(nested_out)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4true609");
    }

    #[test]
    fn source_runtime_apl_parser_keeps_syntax_like_strings_literal() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            AVStr source = "out \"(\" \nout \"[\" \nout \"]\" \nout \"input\" \nout \"secret\" \nout \"-\""
            VTime statements = parser.parse_source(source)
            out len(statements)

            pick(statements): statement {
              VTime expression = get(statement, 1)
              out parser.expr_kind(expression)
              out parser.expr_value(expression)
            }

            AVStr index_source = "List values = [\"a\"] \nVTime value = values[\":\"] \nout value"
            VTime index_statements = parser.parse_source(index_source)
            VTime index_expression = get(get(index_statements, 1), 2)
            VTime string_index = get(index_expression, 2)
            out parser.expr_kind(index_expression)
            out parser.expr_kind(string_index)
            out parser.expr_value(string_index)
            "#,
        )
        .unwrap();

        assert_eq!(output, "6Str(Str[Str]StrinputStrsecretStr-IndexStr:");
    }

    #[test]
    fn source_runtime_apl_parser_accepts_bare_stop_at_statement_boundaries() {
        let output = run_source_with_prelude(
            r#"
            AVStr block_source = "func halt() { \nstop \n} \nout \"before\" \nVTime unused = halt() \nout \"after\""
            AVStr statement_source = "out \"before\" \nstop \nout \"after\""
            VTime block_report = bootstrap.run_report(block_source)
            VTime statement_report = bootstrap.run_report(statement_source)

            out get(block_report, 0)
            out len(get(block_report, 1))
            out get(get(block_report, 1), 0)
            out get(statement_report, 0)
            out len(get(statement_report, 1))
            out get(get(statement_report, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "STOP1beforeSTOP1before");
    }

    #[test]
    fn source_runtime_uses_apl_ir_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 42 \nout x \nx += 1"
            VTime program = ir.compile_source(source)
            VTime decl = get(program, 0)
            VTime out_instruction = get(program, 1)
            VTime assign = get(program, 2)
            VTime literal = get(decl, 3)

            out len(program)
            out ir.opcode(decl)
            out get(decl, 1)
            out get(decl, 2)
            out ir.expr_opcode(literal)
            out ir.expr_value(literal)
            out ir.opcode(out_instruction)
            out ir.opcode(assign)
            out get(assign, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3DECLAVIntxLITERAL42OUTASSIGN+=");
    }

    #[test]
    fn source_runtime_uses_apl_vm_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 41 \nx += 1 \nout x"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "142");
    }

    #[test]
    fn source_runtime_uses_apl_vm_if_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 1 \nif x == 1 { \nx += 41 \nout x \n} \nif x != 42 { \nout 0 \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "142");
    }

    #[test]
    fn source_runtime_uses_apl_vm_else_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 1 \nif x == 2 { \nout 0 \n} else { \nx += 41 \nout x \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "142");
    }

    #[test]
    fn source_runtime_uses_apl_vm_else_if_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 2 \nif x == 1 { \nout 1 \n} else if x == 2 { \nout 42 \n} else { \nout 0 \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "142");
    }

    #[test]
    fn source_runtime_uses_apl_vm_while_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 0 \nwhile ( x < 3 ) ( 10 ) { \nx += 1 \nout x \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3123");
    }

    #[test]
    fn source_runtime_uses_apl_vm_loop_flow_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 0 \nwhile ( x < 10 ) ( 20 ) { \nx += 1 \nif x == 2 { \ncontinue \n} \nif x == 5 { \nbreak \n} \nout x \n} \nout x"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "41345");
    }

    #[test]
    fn source_runtime_uses_apl_vm_function_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func sum_two(left, right) { \nreturn left + right \n} \nAVInt x = sum_two(20, 22) \nout x"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "142");
    }

    #[test]
    fn source_runtime_apl_vm_scopes_calls_and_blocks() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt counter = 0 \nfunc bump(value) { \nVTime index = value \nif true { \nVTime local = index \nindex += 1 \n} \ncounter += index \nreturn index \n} \nfunc wrap(value) { \nVTime index = 40 \nreturn bump(value) + index \n} \nout wrap(1) \nout counter"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "2422");
    }

    #[test]
    fn source_runtime_uses_apl_vm_list_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List items = [1, 2] \nadd(items, 3) \nVTime first = get(items, 0) \nVTime last = pop(items) \nout first \nout last \nout len(items)"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3132");
    }

    #[test]
    fn source_runtime_uses_apl_vm_index_slice_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List items = [1, 2, 3, 4] \nVTime first = items[0] \nVTime tail = items[1:] \nAVStr text = \"APL\" \nout first \nout get(tail, 0) \nout get(tail, 2) \nout text[::-1]"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4124LPA");
    }

    #[test]
    fn source_runtime_uses_apl_vm_pick_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List items = [1, 2, 3, 4] \nAVInt total = 0 \npick(items): item { \nif item == 2 { \ncontinue \n} \nif item == 4 { \nbreak \n} \ntotal += item \n} \nout total \nout item"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "24NONE");
    }

    #[test]
    fn source_runtime_uses_apl_vm_info_and_secret_out_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr typ = \"\" \nAVStr level = \"\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ \nout level \nout master"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3StrSASVDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_derived_secret_out_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "ASVStr token = \"root\" \nVTime tmp = token \nList items = [token] \nout tmp \nout token == token \nout items \nout get(items, 0)"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4DENIEDDENIEDDENIEDDENIED");
    }

    #[test]
    fn source_runtime_rejects_apl_vm_secret_downgrade_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "out \"before\" \nASVStr token = \"root\" \nVTime tmp = token \nAVStr public = tmp \nout public \nout \"after\""
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "1beforeNONE");
    }

    #[test]
    fn source_runtime_rejects_apl_vm_duplicate_names_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr duplicate_var = "AVInt x = 1 \nout x \nAVStr x = \"bad\" \nout x"
            AVStr duplicate_func = "func same() { \nreturn 1 \n} \nfunc same() { \nreturn 2 \n} \nout same()"
            VTime var_output = vm.run_source(duplicate_var)
            VTime func_output = vm.run_source(duplicate_func)

            out len(var_output)
            out get(var_output, 0)
            out get(var_output, 1)
            out len(func_output)
            out get(func_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "11NONE0NONE");
    }

    #[test]
    fn source_runtime_rejects_apl_vm_unknown_mutation_targets_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr assign_source = "out \"before\" \nmissing = 4 \nout \"after\""
            AVStr secretup_source = "out \"start\" \nsecretup(missing) \nout \"after\""
            VTime assign_output = vm.run_source(assign_source)
            VTime secretup_output = vm.run_source(secretup_source)

            out len(assign_output)
            out get(assign_output, 0)
            out get(assign_output, 1)
            out len(secretup_output)
            out get(secretup_output, 0)
            out get(secretup_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "1beforeNONE1startNONE");
    }

    #[test]
    fn source_runtime_rejects_apl_vm_bad_compound_assignments_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr public_source = "AVInt x = 1 \nx += 2 \nout x"
            AVStr secret_source = "ASVInt x = 1 \nx += 2 \nout x"
            AVStr string_source = "AVStr text = \"a\" \ntext += \"b\" \nout text"
            AVStr list_source = "List items = [1] \nitems = [2] \nout items"
            VTime public_output = vm.run_source(public_source)
            VTime secret_output = vm.run_source(secret_source)
            VTime string_output = vm.run_source(string_source)
            VTime list_output = vm.run_source(list_source)

            out get(public_output, 0)
            out len(secret_output)
            out len(string_output)
            out len(list_output)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3000");
    }

    #[test]
    fn source_runtime_rejects_apl_vm_bad_info_targets_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr ok_source = "AVStr typ = \"\" \nAVStr level = \"\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ \nout level"
            AVStr missing_target_source = "out \"before\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ"
            AVStr secret_target_source = "ASVStr typ = \"\" \nAVStr level = \"\" \nAVStr public = \"x\" \ntyp, level = info(public) \nout level"
            AVStr vtime_source = "AVStr typ = \"\" \nAVStr level = \"\" \nVTime tmp = 1 \ntyp, level = info(tmp) \nout typ"
            VTime ok_output = vm.run_source(ok_source)
            VTime missing_target_output = vm.run_source(missing_target_source)
            VTime secret_target_output = vm.run_source(secret_target_source)
            VTime vtime_output = vm.run_source(vtime_source)

            out get(ok_output, 0)
            out get(ok_output, 1)
            out len(missing_target_output)
            out get(missing_target_output, 0)
            out len(secret_target_output)
            out len(vtime_output)
            "#,
        )
        .unwrap();

        assert_eq!(output, "StrSASV1before00");
    }

    #[test]
    fn source_runtime_guards_apl_vm_list_mutation_builtins_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List items = [1] \nadd(items, 2) \nAVInt x = 7 \nVTime bad_add = add(x, 9) \nVTime missing_add = add(missing, 1) \nVTime missing_pop = pop(missing) \nout len(items) \nout pop(items) \nout len(items) \nout bad_add \nout missing_add \nout missing_pop"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 5)
            "#,
        )
        .unwrap();

        assert_eq!(output, "6221NONENONENONE");
    }

    #[test]
    fn source_runtime_uses_apl_vm_secret_aware_stop_fail_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr public_stop = "stop \"done\" \nout \"after\""
            AVStr secret_stop = "ASVStr token = \"root\" \nstop token \nout \"after\""
            AVStr public_fail = "fail \"bad\" \nout \"after\""
            VTime stop_output = vm.run_source(public_stop)
            VTime secret_output = vm.run_source(secret_stop)
            VTime fail_output = vm.run_source(public_fail)

            out len(stop_output)
            out get(stop_output, 0)
            out len(secret_output)
            out get(secret_output, 0)
            out len(fail_output)
            out get(fail_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "1done1DENIED1bad");
    }

    #[test]
    fn source_runtime_propagates_apl_vm_stop_fail_through_function_calls_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr stop_source = "func halt() { \nstop \"done\" \n} \nout \"before\" \nVTime unused = halt() \nout \"after\""
            AVStr fail_source = "func reject() { \nfail \"bad\" \n} \nout \"before\" \nVTime unused = reject() \nout \"after\""
            AVStr nested_source = "func halt() { \nstop \"deep\" \n} \nfunc late() { \nout \"late\" \nreturn 1 \n} \nfunc pair(left, right) { \nreturn left \n} \nout \"before\" \nVTime unused = pair(halt(), late()) \nout \"after\""
            VTime stop_report = vm.run_source_report(stop_source)
            VTime fail_report = vm.run_source_report(fail_source)
            VTime nested_report = vm.run_source_report(nested_source)

            out get(stop_report, 0)
            out len(get(stop_report, 1))
            out get(get(stop_report, 1), 0)
            out get(get(stop_report, 1), 1)
            out get(fail_report, 0)
            out len(get(fail_report, 1))
            out get(get(fail_report, 1), 0)
            out get(get(fail_report, 1), 1)
            out get(nested_report, 0)
            out len(get(nested_report, 1))
            out get(get(nested_report, 1), 0)
            out get(get(nested_report, 1), 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "STOP2beforedoneFAIL2beforebadSTOP2beforedeep");
    }

    #[test]
    fn source_runtime_uses_apl_vm_grouped_logical_conditions_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVBool a = true \nAVBool b = false \nASVBool hidden = true \nif (a == true) and (b != true) { \nout \"ok\" \n} \nif (hidden == true) or (b == true) { \nout hidden == true \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "2okDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_expression_precedence_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 1 + 2 * 3 \nAVInt y = (1 + 2) * 3 \nAVBool ok = (x == 7) and (y == 9) or false \nout x \nout y \nout ok"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "379true");
    }

    #[test]
    fn source_runtime_uses_apl_vm_float_literals_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVFloat x = 1.5 \nAVFloat y = -2.5 \nAVFloat z = x + y \nout z \nout z < 0.0"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "2-1true");
    }

    #[test]
    fn source_runtime_uses_apl_vm_unary_expressions_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 4 \nAVInt y = -x \nAVFloat z = -1.5 \nAVBool flag = false \nout y \nout z \nout not flag \nout -(x + 1)"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4-4-1.5true-5");
    }

    #[test]
    fn source_runtime_uses_apl_vm_pow_builtin_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = pow(2, 3) \nAVFloat y = pow(2.0, 3) \nAVInt bad = pow(2, -1) \nASVInt hidden = 2 \nout x \nout y == 8.0 \nout bad \nout pow(hidden, 2)"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "48trueNONEDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_power_operator_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt nested = 2 ** 3 ** 2 \nAVInt signed = -2 ** 2 \nAVFloat decimal = 2.0 ** 3 \nAVInt negative = 2 ** -1 \nAVInt overflow = 2 ** 63 \nout nested \nout signed \nout decimal \nout negative \nout overflow"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            pick(vm_output): item {
              out item
            }
            "#,
        )
        .unwrap();

        assert_eq!(output, "551248NONENONE");
    }

    #[test]
    fn source_runtime_uses_apl_vm_remainder_operator_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt cycle = 17 \ncycle %= 5 \nAVFloat decimal = 5.5 % 2.0 \nout 2 + 10 % 4 * 3 \nout cycle \nout -17 % 5 \nout decimal"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            pick(vm_output): item {
              out item
            }
            "#,
        )
        .unwrap();

        assert_eq!(output, "482-21.5");
    }

    #[test]
    fn source_runtime_apl_vm_propagates_remainder_errors_prelude() {
        for (driver, expected) in [
            (
                r#"AVStr source = "out 1 % 0"
VTime output = vm.run_source(source)
out output"#,
                "remainder by zero",
            ),
            (
                r#"AVStr source = "AVInt value = -9223372036854775808 \nout value % -1"
VTime output = vm.run_source(source)
out output"#,
                "integer overflow in rem",
            ),
        ] {
            let error = run_source_with_prelude(driver).unwrap_err();
            let RuntimeError::Message(message) = error else {
                panic!("expected runtime error");
            };
            assert!(message.starts_with(expected), "unexpected error: {message}");
            assert!(message.contains("in `vm.eval_binary`"));
        }
    }

    #[test]
    fn source_runtime_apl_checker_validates_power_operator_prelude() {
        assert_eq!(
            apl_compile_outcomes(&[
                "AVInt value = 2 ** 8",
                "AVFloat value = 2.0 ** 8",
                "AVInt hidden = 2 \nASVInt result = hidden ** 8",
                "AVInt value = true ** 2",
                "AVInt value = 2 ** 2.0",
                "ASVInt hidden = 2 \nAVInt leaked = hidden ** 8",
            ]),
            vec!["OK", "OK", "OK", "FAIL", "FAIL", "FAIL"]
        );
    }

    #[test]
    fn source_runtime_uses_apl_vm_self_check_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 4 \nout x =self= \nx += 1 \nout x =self= \nASVStr token = \"root\" \nout token =self="
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3truefalseDENIED");
    }

    #[test]
    fn source_runtime_executes_standalone_function_calls() {
        let output = run_source(
            r#"
            AVInt total = 0
            func add_to_total(value) {
              total += value
              return total
            }
            add_to_total(40)
            add_to_total(2)
            out total
            "#,
        )
        .unwrap();

        assert_eq!(output, "42");
    }

    #[test]
    fn source_runtime_uses_apl_vm_conversion_builtins_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr raw = \"42\" \nAVStr bad = \"no\" \nASVStr hidden = \"7\" \nout int(raw) \nout int(bad) \nout bool(\"true\") \nout str(42) \nout int(hidden)"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 1) == "NONE"
            "#,
        )
        .unwrap();

        assert_eq!(output, "542NONEtrue42DENIEDtrue");
    }

    #[test]
    fn source_runtime_uses_apl_vm_secretup_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr token = \"root\" \nAVStr typ = \"\" \nAVStr level = \"\" \nsecretup(token) \ntyp, level = info(token) \nout level \nout token \nsecretup(token) \ntyp, level = info(token) \nout level \nout token =self="
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4ASVDENIEDSASVDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_string_helper_builtins_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr text = \"A P L\" \nList parts = split(text, \" \") \nASVStr hidden = \"secret words\" \nout len(parts) \nout join(parts, \"-\") \nout contains(text, \"P\") \nout ord(\"A\") \nout char(90) \nout split(hidden, \" \")"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 5)
            "#,
        )
        .unwrap();

        assert_eq!(output, "63A-P-Ltrue65ZDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_tagged_list_elements_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr typ = \"\" \nAVStr level = \"\" \nList items = [1:SASV, \"public\"] \nadd(items, \"hidden\":ASV) \nset(items, 1, \"replaced\":ASV) \ntyp, level = info(items) \nout typ \nout level \nout items \nout get(items, 0) \nout get(items, 1) \nout get(items, 2) \npick(items): item { \nout item \n}"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 5)
            out get(vm_output, 6)
            out get(vm_output, 7)
            out get(vm_output, 8)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "9ListSASVDENIEDDENIEDDENIEDDENIEDDENIEDDENIEDDENIED"
        );
    }

    #[test]
    fn source_runtime_uses_apl_vm_input_placeholders_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVStr typ = \"\" \nAVStr level = \"\" \ntyp, level = info(public) \nout typ \nout level \nout public \ntyp, level = info(hidden) \nout typ \nout level \nout hidden"
            VTime vm_output = vm.run_source(source)

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 5)
            "#,
        )
        .unwrap();

        assert_eq!(output, "6StrAVNONEStrASVDENIED");
    }

    #[test]
    fn source_runtime_uses_apl_vm_input_stream_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVStr missing = input \nout public \nout hidden \nout missing"
            VTime vm_output = vm.run_source_with_input(source, ["hello", "token"])

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            "#,
        )
        .unwrap();

        assert_eq!(output, "3helloDENIEDNONE");
    }

    #[test]
    fn source_runtime_runs_compiled_bootstrap_scenario_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVInt age = input \nList values = [age, 2, [3, 4]] \nfunc inc(x) { \nreturn x + 1 \n} \nVTime next = inc(age) \nVTime first = values[0] \nout public \nout next \nout first \nAVStr missing = input \nout missing"
            VTime vm_output = bootstrap.run_with_input(source, ["hello", "token", "41"])

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            "#,
        )
        .unwrap();

        assert_eq!(output, "4hello4241NONE");
    }

    #[test]
    fn source_runtime_uses_apl_bootstrap_facade_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = input \nx += 1 \nout x"
            VTime tokens = bootstrap.tokens(source)
            VTime ast = bootstrap.ast(source)
            VTime program = bootstrap.ir(source)
            VTime vm_output = bootstrap.run_with_input(source, ["4"])

            out len(tokens)
            out parser.node_kind(get(ast, 0))
            out ir.opcode(get(program, 0))
            out get(vm_output, 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "11DeclDECL5");
    }

    #[test]
    fn source_runtime_uses_apl_bootstrap_report_facade_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr ok_source = "AVInt x = input \nx += 1 \nout x"
            AVStr stop_source = "out \"before\" \nstop \"done\" \nout \"after\""
            AVStr fail_source = "out \"before\" \nfail \"bad\" \nout \"after\""

            VTime ok_report = bootstrap.run_with_input_report(ok_source, ["4"])
            VTime stop_report = bootstrap.run_report(stop_source)
            VTime fail_report = bootstrap.run_report(fail_source)

            out get(ok_report, 0)
            out get(get(ok_report, 1), 0)
            out get(stop_report, 0)
            out get(get(stop_report, 1), 0)
            out get(get(stop_report, 1), 1)
            out get(fail_report, 0)
            out get(get(fail_report, 1), 0)
            out get(get(fail_report, 1), 1)
            "#,
        )
        .unwrap();

        assert_eq!(output, "OK5STOPbeforedoneFAILbeforebad");
    }

    #[test]
    fn source_runtime_uses_apl_bootstrap_compile_report_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr ok_source = "AVInt x = 1 \nout x"
            AVStr bad_source = "AVInt x 1 out x"

            VTime ok_report = bootstrap.compile_report(ok_source)
            VTime bad_report = bootstrap.compile_report(bad_source)
            VTime bad_run = bootstrap.run_report(bad_source)

            out get(ok_report, 0)
            out ir.opcode(get(get(ok_report, 1), 0))
            out get(bad_report, 0)
            out get(bad_report, 1)
            out get(bad_run, 0)
            out get(get(bad_run, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKDECLFAILparse error at 1:9: declaration expects =FAILparse error at 1:9: declaration expects ="
        );
    }

    #[test]
    fn source_runtime_apl_frontend_reports_unicode_source_positions() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            VTime slash = char(92)
            VTime unicode_source = "out \"ёж\" \nAVInt x 1"
            VTime crlf_source = join(["out 1", char(13), char(10), "AVInt x 1"], "")
            VTime escape_source = join(["out ", quote, char(1105), char(1078), slash, "q", quote], "")
            VTime tokenized = lexer.tokenize_with_offsets(unicode_source)
            VTime offsets = get(tokenized, 1)
            VTime declaration_position = lexer.position_at(unicode_source, get(offsets, 3))
            VTime bad_value_position = lexer.position_at(unicode_source, get(offsets, 5))
            VTime unicode_report = bootstrap.compile_report(unicode_source)
            VTime crlf_report = bootstrap.compile_report(crlf_source)
            VTime escape_report = bootstrap.compile_report(escape_source)

            out get(declaration_position, 0)
            out ":"
            out get(declaration_position, 1)
            out "|"
            out get(bad_value_position, 0)
            out ":"
            out get(bad_value_position, 1)
            out "|"
            out get(unicode_report, 0)
            out get(unicode_report, 1)
            out "|"
            out get(crlf_report, 0)
            out get(crlf_report, 1)
            out "|"
            out get(escape_report, 0)
            out get(escape_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "2:1|2:9|FAILparse error at 2:9: declaration expects =|FAILparse error at 2:9: declaration expects =|FAILparse error at 1:8: invalid escape sequence `\\q`"
        );
    }

    #[test]
    fn source_runtime_checker_reports_nested_statement_position() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            VTime source = join(["out ", quote, char(1105), char(1078), quote, char(10), "if true {", char(10), "  out missing", char(10), "}"], "")
            VTime report = bootstrap.compile_report(source)

            out get(report, 0)
            out get(report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "FAILsemantic error at 3:3: unknown variable `missing`"
        );
    }

    #[test]
    fn source_runtime_apl_frontend_checks_numeric_literal_boundaries() {
        let output = run_source_with_prelude(
            r#"
            AVStr min_source = "AVInt value = -9223372036854775808 \nout value"
            AVStr positive_overflow = "AVInt value = 9223372036854775808"
            AVStr negative_overflow = "AVInt value = -9223372036854775809"
            AVStr trailing_dot = "AVFloat value = 1."

            VTime min_report = bootstrap.run_report(min_source)
            VTime positive_report = bootstrap.compile_report(positive_overflow)
            VTime negative_report = bootstrap.compile_report(negative_overflow)
            VTime float_report = bootstrap.compile_report(trailing_dot)

            out get(min_report, 0)
            out get(get(min_report, 1), 0)
            out get(positive_report, 0)
            out get(positive_report, 1)
            out get(negative_report, 0)
            out get(negative_report, 1)
            out get(float_report, 0)
            out get(float_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OK-9223372036854775808FAILsemantic error at 1:1: integer literal out of rangeFAILsemantic error at 1:1: integer literal out of rangeFAILsemantic error at 1:1: invalid float literal"
        );
    }

    #[test]
    fn source_runtime_apl_frontend_rejects_unclosed_delimiters_and_bad_separators() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            List sources = [
              "if true {\nout 1\n",
              "AVInt value = (1 + 2",
              "VTime value = len([1]",
              "List values = [1, 2",
              "VTime value = pow(2 3)",
              "List values = [1 2]",
              "VTime value = pow(2,)",
              "List values = [1] \nVTime value = values[0",
              "out 1 \n} out 2",
              "if true {\nout 1\n\"}\"\n"
            ]

            pick(sources): source {
              VTime report = bootstrap.compile_report(source)
              out get(report, 0)
              out get(report, 1)
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "FAILparse error at 3:1: expected }FAILparse error at 1:21: group expects )FAILparse error at 1:22: call expects )FAILparse error at 1:20: list expects ]FAILparse error at 1:21: call expects , or )FAILparse error at 1:18: list expects , or ]FAILparse error at 1:21: expected expression after ,FAILparse error at 2:23: index expects ]FAILparse error at 2:1: unexpected }FAILparse error at 3:1: unknown statement"
        );
    }

    #[test]
    fn source_runtime_apl_frontend_enforces_token_roles_and_string_termination() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            List sources = [
              join(["out ", quote, "hello"], ""),
              join(["AVInt ", quote, "x", quote, " = 1"], ""),
              join(["List ", quote, "items", quote, " = []"], ""),
              join(["VTime ", quote, "temp", quote, " = 1"], ""),
              join(["func ", quote, "f", quote, "() { return 1 }"], ""),
              join(["func f(", quote, "x", quote, ") { return 1 }"], ""),
              "func f(a b) { return a }",
              "func f(a,) { return a }",
              join(["pick([1]): ", quote, "item", quote, " { out 1 }"], ""),
              join(["secretup(", quote, "value", quote, ")"], ""),
              join(["type, ", quote, "level", quote, " = info(source)"], ""),
              join(["type, level = info(", quote, "source", quote, ")"], ""),
              join([quote, "if", quote, " true { out 1 }"], ""),
              "VTime value = 1 \n\"+\" 2",
              "VTime value = 1 \n\"==\" 1",
              "VTime value = true \n\"and\" false",
              "AVInt x = 1 \nVTime value = x \n\"=self=\"",
              join(["List values = [1 ", quote, ":", quote, " ASV]"], ""),
              join(["VTime value = secret ", quote, "input", quote], "")
            ]

            pick(sources): source {
              VTime report = bootstrap.compile_report(source)
              out get(report, 0)
              out get(report, 1)
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "FAILparse error at 1:5: unterminated string literalFAILparse error at 1:7: declaration expects identifierFAILparse error at 1:6: list declaration expects identifierFAILparse error at 1:7: vtime declaration expects identifierFAILparse error at 1:6: func expects identifierFAILparse error at 1:8: func expects parameter identifierFAILparse error at 1:10: func expects , or )FAILparse error at 1:10: func expects parameter identifierFAILparse error at 1:12: pick expects item identifierFAILparse error at 1:10: secretup expects identifierFAILparse error at 1:7: info assignment expects identifierFAILparse error at 1:20: info assignment expects source identifierFAILparse error at 1:1: unknown statementFAILparse error at 2:1: unknown statementFAILparse error at 2:1: unknown statementFAILparse error at 2:1: unknown statementFAILparse error at 3:1: unknown statementFAILparse error at 1:18: list expects , or ]FAILsemantic error at 1:1: unknown variable `secret`"
        );
    }

    #[test]
    fn source_runtime_uses_apl_bootstrap_artifact_report_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr ok_source = "AVInt x = 1 \nout x"
            AVStr bad_source = "AVInt x 1 out x"

            VTime ok_report = bootstrap.artifact_report(ok_source)
            VTime bad_report = bootstrap.artifact_report(bad_source)
            VTime artifact = get(ok_report, 1)

            out get(ok_report, 0)
            out artifact[:7]
            out contains(artifact, "DECL")
            out contains(artifact, "OUT")
            out get(bad_report, 0)
            out get(bad_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKAPLIR1:truetrueFAILparse error at 1:9: declaration expects ="
        );
    }

    #[test]
    fn source_runtime_runs_apl_bootstrap_artifact_images_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func inc(x) { \nreturn x + 1 \n} \nAVInt x = input \nout inc(x)"
            VTime image_report = bootstrap.artifact_image_report(source)
            VTime image = get(image_report, 1)
            VTime first_run = bootstrap.run_artifact_image_with_input_report(image, ["4"])
            VTime second_run = bootstrap.run_artifact_image_with_input_report(image, ["9"])
            VTime invalid_run = bootstrap.run_artifact_image_report(["BAD"])

            out get(image_report, 0)
            out get(image, 0)
            out ir.opcode(get(get(image, 1), 0))
            out get(first_run, 0)
            out get(get(first_run, 1), 0)
            out get(second_run, 0)
            out get(get(second_run, 1), 0)
            out get(invalid_run, 0)
            out get(get(invalid_run, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "OKAPLIR1FUNCOK5OK10FAILinvalid artifact image");
    }

    #[test]
    fn source_runtime_runs_loaded_apl_images_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func inc(x) { \nreturn add_one(x) \n} \nfunc add_one(x) { \nreturn x + 1 \n} \nAVInt x = input \nout inc(x)"
            VTime loaded_report = bootstrap.loaded_image_report(source)
            VTime loaded = get(loaded_report, 1)
            VTime entry_call = get(get(get(loaded, 1), 1), 1)
            VTime inc_call = get(get(get(get(loaded, 2), 2), 0), 0)[1]
            VTime first_run = bootstrap.run_loaded_image_with_input_report(loaded, ["4"])
            VTime second_run = bootstrap.run_loaded_image_with_input_report(loaded, ["9"])
            VTime direct_run = bootstrap.run_with_input_report(source, ["14"])
            VTime invalid_load = bootstrap.load_artifact_image_report(["BAD"])
            VTime invalid_run = bootstrap.run_loaded_image_report(["BAD"])

            out get(loaded_report, 0)
            out get(loaded, 0)
            out ir.opcode(get(get(loaded, 1), 0))
            out len(get(loaded, 1))
            out ir.expr_opcode(entry_call)
            out get(entry_call, 1)
            out ir.expr_opcode(inc_call)
            out get(inc_call, 1)
            out get(first_run, 0)
            out get(get(first_run, 1), 0)
            out get(second_run, 0)
            out get(get(second_run, 1), 0)
            out get(direct_run, 0)
            out get(get(direct_run, 1), 0)
            out get(invalid_load, 0)
            out get(get(invalid_load, 1), 0)
            out get(invalid_run, 0)
            out get(get(invalid_run, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKAPLLOAD2DECL2CALL_SLOT0CALL_SLOT1OK5OK10OK15FAILinvalid artifact imageFAILinvalid loaded image"
        );
    }

    #[test]
    fn source_runtime_apl_linker_retains_only_reachable_functions_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func dead(x) { \nreturn x + 100 \n} \nfunc even(x) { \nif x == 0 { \nreturn true \n} \nreturn odd(x - 1) \n} \nfunc unused_middle() { \nreturn dead(1) \n} \nfunc odd(x) { \nif x == 0 { \nreturn false \n} \nreturn even(x - 1) \n} \nfunc recurse(x) { \nif x == 0 { \nreturn 0 \n} \nreturn recurse(x - 1) \n} \nout even(4) \nout recurse(3)"
            VTime encoded_report = bootstrap.linked_artifact_report(source)
            VTime encoded = get(encoded_report, 1)
            VTime loaded_report = bootstrap.load_linked_artifact_report(encoded)
            VTime loaded = get(loaded_report, 1)
            VTime entry = vm.loaded_program(loaded)
            VTime functions = vm.loaded_functions(loaded)
            VTime first_call = get(get(entry, 0), 1)
            VTime second_call = get(get(entry, 1), 1)
            VTime run_report = bootstrap.run_loaded_image_report(loaded)

            out get(encoded_report, 0)
            out get(loaded_report, 0)
            out join(vm.func_names(functions), ",")
            out len(vm.func_names(functions))
            out get(first_call, 1)
            out get(second_call, 1)
            out contains(encoded, "dead")
            out contains(encoded, "unused_middle")
            out get(run_report, 0)
            out join(get(run_report, 1), "")
            "#,
        )
        .unwrap();

        assert_eq!(output, "OKOKeven,odd,recurse302falsefalseOKtrue0");
    }

    #[test]
    fn source_runtime_lowers_linked_programs_to_linear_native_ir_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func twice(value) { \nreturn value * 2 \n} \nAVInt x = 1 \nout twice(x + 2)"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime lowered = native.lower_linked_artifact_report(get(linked, 1))
            VTime encoded = get(lowered, 1)
            VTime decoded = native.decode_image_report(encoded)
            VTime image = get(decoded, 1)
            VTime entry = get(image, 1)
            VTime functions = get(image, 2)
            VTime function = get(functions, 0)
            VTime body = get(function, 3)

            out get(linked, 0)
            out get(lowered, 0)
            out encoded[:len(native.HEADER)]
            out get(decoded, 0)
            out native.image_is_valid(image)
            out len(entry)

            pick(entry): instruction {
              out native.opcode(instruction)
            }

            out len(functions)
            out get(function, 0)
            out get(function, 1)
            out join(get(function, 2), ",")
            out len(body)

            pick(body): instruction {
              out native.opcode(instruction)
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKAPLLOW1:OKtrue7CONSTDECLARELOADCONSTBINARYCALL_SLOTOUT10twicevalue6LOADCONSTBINARYRETURNNONERETURN"
        );
    }

    #[test]
    fn source_runtime_native_ir_flattens_control_flow_and_scope_unwinds_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt x = 0 \nif true { \nx = 1 \n} else { \nx = 2 \n} \nwhile (x < 3) (4) { \nif x == 1 { \nx += 1 \ncontinue \n} \nif x == 2 { \nbreak \n} \n} \npick([1, 2]): item { \nif item == 1 { \ncontinue \n} \nbreak \n}"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime lowered = native.lower_linked_artifact_report(get(linked, 1))
            VTime image = get(native.decode_image_report(get(lowered, 1)), 1)
            VTime code = get(image, 1)
            VTime labels = 0
            VTime conditional_jumps = 0
            VTime loop_enters = 0
            VTime pick_enters = 0
            List unwinds = []

            pick(code): instruction {
              VTime opcode = native.opcode(instruction)

              if opcode == native.OP_LABEL {
                labels += 1
              }

              if opcode == native.OP_JUMP_IF_FALSE {
                conditional_jumps += 1
              }

              if opcode == native.OP_LOOP_ENTER {
                loop_enters += 1
              }

              if opcode == native.OP_PICK_ENTER {
                pick_enters += 1
              }

              if opcode == native.OP_UNWIND_JUMP {
                add(unwinds, str(get(instruction, 1)))
              }
            }

            out get(linked, 0)
            out get(lowered, 0)
            out native.image_is_valid(image)
            out labels
            out conditional_jumps
            out loop_enters
            out pick_enters
            out join(unwinds, ",")
            "#,
        )
        .unwrap();

        assert_eq!(output, "OKOKtrue166112,2,2,1");
    }

    #[test]
    fn source_runtime_native_ir_covers_current_value_and_statement_surface_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func echo(value) { \nreturn value \n} \nAVInt x = input \nAVFloat ratio = 2.5 \nASVStr hidden = secret input \nAVBool same = x =self= \nAVStr type_name = \"\" \nAVStr protection = \"\" \ntype_name, protection = info(x) \nList items = [1:ASV, 2] \nVTime first = items[0] \nVTime part = items[:] \nVTime neg = -x \nx += 1 \nadd(items, 3) \nset(items, 0, 4) \nVTime last = pop(items) \nVTime returned = echo(neg + 2) \nsecretup(x) \nif false { \nfail \"never\" \n} \nout \"ok\" \nstop"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime lowered = native.lower_linked_artifact_report(get(linked, 1))
            VTime image = get(native.decode_image_report(get(lowered, 1)), 1)
            List opcodes = []
            List mutation_targets = []

            pick(get(image, 1)): instruction {
              add(opcodes, native.opcode(instruction))

              if native.opcode(instruction) == native.OP_CALL_BUILTIN {
                VTime name = get(instruction, 1)

                if (name == "add") or (name == "pop") {
                  add(mutation_targets, get(get(instruction, 3), 0))
                }

                if name == "set" {
                  add(mutation_targets, get(get(instruction, 3), 0))
                }
              }
            }

            pick(get(image, 2)): function {
              pick(get(function, 3)): instruction {
                add(opcodes, native.opcode(instruction))
              }
            }

            List expected = [
              native.OP_CONST,
              native.OP_NONE,
              native.OP_INPUT,
              native.OP_SECRET_INPUT,
              native.OP_LOAD,
              native.OP_UNARY,
              native.OP_BINARY,
              native.OP_CALL_BUILTIN,
              native.OP_CALL_SLOT,
              native.OP_LIST,
              native.OP_INDEX,
              native.OP_SLICE,
              native.OP_SELF,
              native.OP_TAG,
              native.OP_DECLARE,
              native.OP_DECLARE_LIST,
              native.OP_DECLARE_VTIME,
              native.OP_ASSIGN,
              native.OP_INFO_ASSIGN,
              native.OP_DROP,
              native.OP_SECRETUP,
              native.OP_OUT,
              native.OP_STOP,
              native.OP_FAIL,
              native.OP_RETURN
            ]
            List missing = []

            pick(expected): opcode {
              if checker.value_exists(opcodes, opcode) != true {
                add(missing, opcode)
              }
            }

            out get(linked, 0)
            out get(lowered, 0)
            out native.image_is_valid(image)
            out len(missing)
            out join(missing, ",")
            out join(mutation_targets, ",")
            "#,
        )
        .unwrap();

        assert_eq!(output, "OKOKtrue0items,items,items");
    }

    #[test]
    fn source_runtime_native_ir_codec_rejects_malformed_images_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime manifest = [linker.MANIFEST_MAGIC, [["base", []]]]
            VTime bad_label = [native.IMAGE_MAGIC, [[native.OP_JUMP, 7]], [], manifest]
            VTime bad_slot = [native.IMAGE_MAGIC, [[native.OP_CALL_SLOT, 0, 0]], [], manifest]
            VTime duplicate_label = [native.IMAGE_MAGIC, [[native.OP_LABEL, 1], [native.OP_LABEL, 1]], [], manifest]
            VTime stack_underflow = [native.IMAGE_MAGIC, [[native.OP_OUT]], [], manifest]
            VTime stack_residue = [native.IMAGE_MAGIC, [[native.OP_CONST, parser.EXPR_INT, 1]], [], manifest]
            VTime bad_arity_function = [0, "f", ["value"], [[native.OP_NONE], [native.OP_RETURN, 0]]]
            VTime bad_arity = [native.IMAGE_MAGIC, [[native.OP_CALL_SLOT, 0, 0], [native.OP_DROP]], [bad_arity_function], manifest]
            VTime missing_mutation_target = [native.IMAGE_MAGIC, [[native.OP_CONST, parser.EXPR_INT, 1], [native.OP_CONST, parser.EXPR_INT, 2], [native.OP_CALL_BUILTIN, "add", 2, [NONE, NONE]], [native.OP_DROP]], [], manifest]
            VTime bad_label_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(bad_label))], "")
            VTime bad_slot_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(bad_slot))], "")
            VTime duplicate_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(duplicate_label))], "")
            VTime underflow_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(stack_underflow))], "")
            VTime residue_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(stack_residue))], "")
            VTime scalar_text = join([native.HEADER, artifact.encode_node(artifact.node_int(1))], "")
            VTime bad_arity_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(bad_arity))], "")
            VTime mutation_text = join([native.HEADER, artifact.encode_node(artifact.wrap_value(missing_mutation_target))], "")
            VTime valid = native.lower_linked_artifact_report(get(bootstrap.linked_artifact_report("out 1"), 1))
            VTime encoded = get(valid, 1)
            VTime bad_header = native.decode_image_report("BAD")
            VTime trailing = native.decode_image_report(join([encoded, "x"], ""))
            VTime label_report = native.decode_image_report(bad_label_text)
            VTime slot_report = native.decode_image_report(bad_slot_text)
            VTime duplicate_report = native.decode_image_report(duplicate_text)
            VTime underflow_report = native.decode_image_report(underflow_text)
            VTime residue_report = native.decode_image_report(residue_text)
            VTime scalar_report = native.decode_image_report(scalar_text)
            VTime arity_report = native.decode_image_report(bad_arity_text)
            VTime mutation_report = native.decode_image_report(mutation_text)
            VTime encode_report = native.encode_image_report(bad_label)

            out get(valid, 0)
            out get(bad_header, 0)
            out get(bad_header, 1)
            out get(trailing, 0)
            out get(trailing, 1)
            out get(label_report, 0)
            out get(label_report, 1)
            out get(slot_report, 0)
            out get(slot_report, 1)
            out get(duplicate_report, 0)
            out get(duplicate_report, 1)
            out get(underflow_report, 0)
            out get(underflow_report, 1)
            out get(residue_report, 0)
            out get(residue_report, 1)
            out get(scalar_report, 0)
            out get(scalar_report, 1)
            out get(arity_report, 0)
            out get(arity_report, 1)
            out get(mutation_report, 0)
            out get(mutation_report, 1)
            out get(encode_report, 0)
            out get(encode_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKFAILinvalid low IR headerFAILtrailing low IR dataFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR imageFAILinvalid low IR image"
        );
    }

    #[test]
    fn source_runtime_lowers_native_ir_to_stable_platform_abi_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func twice(value) { \nreturn value * 2 \n} \nAVInt x = 1 \nout twice(x + 2)"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime low = native.lower_linked_artifact_report(get(linked, 1))
            VTime lowered = abi.lower_encoded_report(get(low, 1), abi.TARGET_WINDOWS_X64)
            VTime decoded = abi.decode_image_report(get(lowered, 1))
            VTime image = get(decoded, 1)
            VTime target = get(image, 1)
            VTime pool = get(image, 2)
            VTime entry = get(image, 3)
            VTime functions = get(image, 4)
            VTime function = get(functions, 0)

            out get(linked, 0)
            out get(low, 0)
            out get(lowered, 0)
            out get(lowered, 1)[:len(abi.HEADER)]
            out get(decoded, 0)
            out abi.image_is_valid(image)
            out get(target, 1)
            out get(target, 3)
            out get(target, 4)
            out get(target, 6)
            out get(target, 7)
            out len(pool)
            out get(get(pool, 0), 0)
            out get(get(pool, 0), 1)
            out len(entry)
            out get(get(entry, 0), 0)
            out get(get(entry, len(entry) - 1), 0)
            out len(functions)
            out abi.pool_value(pool, get(function, 1))
            out len(get(function, 2))
            out get(get(function, 3), 0)[0]
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKOKAPLABI1:OKtruex86_64-pc-windows-msvccoffwin64163251170211twice14"
        );
    }

    #[test]
    fn source_runtime_native_abi_covers_complete_low_ir_surface_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func echo(value) { \nreturn value \n} \nfunc abort() { \nreturn(ERR=[\"never\"][7]) \n} \nAVInt x = input \nAVFloat ratio = 2.5 \nASVStr hidden = secret input \nAVBool same = x =self= \nAVStr type_name = \"\" \nAVStr protection = \"\" \ntype_name, protection = info(x) \nList items = [1:ASV, 2] \nVTime first = items[0] \nVTime part = items[:] \nVTime neg = -x \nx += 1 \nadd(items, 3) \nset(items, 0, 4) \nVTime last = pop(items) \nVTime returned = echo(neg + 2) \necho(x) \nif false { \nfail \"never\" \nabort() \n} \nwhile (x < 3) (4) { \nx += 1 \nif x == 2 { \ncontinue \n} \nbreak \n} \npick(items): item { \nif item == 1 { \ncontinue \n} \nbreak \n} \nsecretup(x) \nout \"ok\" \nstop"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime low = native.lower_linked_artifact_report(get(linked, 1))
            VTime lowered = abi.lower_encoded_report(get(low, 1), abi.TARGET_LINUX_X64)

            if get(lowered, 0) != vm.FLOW_OK {
              out get(linked, 0)
              out get(linked, 1)
              out get(low, 0)
              out get(low, 1)
              out get(lowered, 0)
              out get(lowered, 1)
              stop
            }

            VTime image = get(abi.decode_image_report(get(lowered, 1)), 1)
            VTime target = get(image, 1)
            VTime pool = get(image, 2)
            VTime opcodes = []
            VTime mutation_targets = []

            pick(get(image, 3)): instruction {
              add(opcodes, get(instruction, 0))

              if get(instruction, 0) == abi.OP_CALL_BUILTIN {
                VTime builtin_name = get(abi.BUILTINS, get(instruction, 1))

                if (builtin_name == "add") or (builtin_name == "pop") {
                  add(mutation_targets, abi.pool_value(pool, get(get(instruction, 3), 0)))
                }

                if builtin_name == "set" {
                  add(mutation_targets, abi.pool_value(pool, get(get(instruction, 3), 0)))
                }
              }
            }

            pick(get(image, 4)): function {
              pick(get(function, 3)): instruction {
                add(opcodes, get(instruction, 0))
              }
            }

            VTime missing = []
            VTime opcode = 0

            while (opcode < len(abi.OPCODE_NAMES)) (-1) {
              if checker.value_exists(opcodes, opcode) != true {
                add(missing, opcode)
              }

              opcode += 1
            }

            out get(linked, 0)
            out get(lowered, 0)
            out abi.image_is_valid(image)
            out len(missing)
            out join(mutation_targets, ",")
            out get(target, 3)
            out get(target, 4)
            out get(target, 7)
            "#,
        )
        .unwrap();

        assert_eq!(output, "OKOKtrue0items,items,itemselfsysv640");
    }

    #[test]
    fn source_runtime_native_abi_codec_rejects_malformed_images_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime linked = bootstrap.linked_artifact_report("out 1")
            VTime low = native.lower_linked_artifact_report(get(linked, 1))
            VTime lowered = abi.lower_encoded_report(get(low, 1), abi.TARGET_WINDOWS_X64)
            VTime encoded = get(lowered, 1)
            VTime image = get(abi.decode_image_report(encoded), 1)
            VTime target = get(image, 1)
            VTime pool = get(image, 2)
            VTime entry = get(image, 3)
            VTime functions = get(image, 4)
            VTime manifest = get(image, 5)
            VTime duplicate_pool = pool[:]
            add(duplicate_pool, get(pool, 0))

            VTime bad_target = [abi.IMAGE_MAGIC, [99, "bad", "x86_64", "coff", "win64", 8, 16, 32, ["rcx"]], pool, entry, functions, manifest]
            VTime bad_pool = [abi.IMAGE_MAGIC, target, duplicate_pool, entry, functions, manifest]
            VTime bad_pool_entry = [abi.IMAGE_MAGIC, target, [[99, "bad"]], entry, functions, manifest]
            VTime bad_ref = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_CONST, len(pool)], [abi.OP_OUT]], functions, manifest]
            VTime bad_name_ref = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_LOAD, 0], [abi.OP_DROP]], functions, manifest]
            VTime bad_label = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_JUMP, 7]], functions, manifest]
            VTime bad_slot = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_CALL_SLOT, 0, 0], [abi.OP_DROP]], functions, manifest]
            VTime underflow = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_OUT]], functions, manifest]
            VTime residue = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_CONST, 0]], functions, manifest]
            VTime bad_builtin = [abi.IMAGE_MAGIC, target, pool, [[abi.OP_CONST, 0], [abi.OP_CONST, 0], [abi.OP_CALL_BUILTIN, abi.index_of(abi.BUILTINS, "len"), 2, [-1, -1]], [abi.OP_DROP]], functions, manifest]
            VTime invalid_images = [bad_target, bad_pool, bad_pool_entry, bad_ref, bad_name_ref, bad_label, bad_slot, underflow, residue, bad_builtin]

            out get(lowered, 0)
            out abi.image_is_valid(image)

            pick(invalid_images): invalid_image {
              out abi.image_is_valid(invalid_image)
              out get(abi.encode_image_report(invalid_image), 0)
            }

            VTime bad_ref_text = join([abi.HEADER, artifact.encode_node(artifact.wrap_value(bad_ref))], "")
            out get(abi.lower_encoded_report(get(low, 1), "unknown-target"), 0)
            out get(abi.lower_encoded_report(get(low, 1), "unknown-target"), 1)
            out get(abi.decode_image_report("BAD"), 0)
            out get(abi.decode_image_report("BAD"), 1)
            out get(abi.decode_image_report(join([encoded, "x"], "")), 0)
            out get(abi.decode_image_report(join([encoded, "x"], "")), 1)
            out get(abi.decode_image_report(bad_ref_text), 0)
            out get(abi.decode_image_report(bad_ref_text), 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKtruefalseFAILfalseFAILfalseFAILfalseFAILfalseFAILfalseFAILfalseFAILfalseFAILfalseFAILfalseFAILFAILunsupported native target: unknown-targetFAILinvalid native ABI headerFAILtrailing native ABI dataFAILinvalid native ABI image"
        );
    }

    #[test]
    fn source_runtime_emits_windows_and_linux_x64_assembly_prelude() {
        let output = run_source_with_prelude(
            r##"
            AVStr source = "func increment(value) { \nreturn value + 1 \n} \nAVStr text = \"AЖ🙂\" \nout text \nout increment(41)"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime low = native.lower_linked_artifact_report(get(linked, 1))
            VTime windows_abi = abi.lower_encoded_report(get(low, 1), abi.TARGET_WINDOWS_X64)
            VTime linux_abi = abi.lower_encoded_report(get(low, 1), abi.TARGET_LINUX_X64)
            VTime windows = assembly.emit_encoded_report(get(windows_abi, 1))
            VTime linux = assembly.emit_encoded_report(get(linux_abi, 1))
            VTime windows_text = get(windows, 1)
            VTime linux_text = get(linux, 1)

            out get(linked, 0)
            out get(windows, 0)
            out get(linux, 0)
            out contains(windows_text, "# APLASM1 runtime-abi=2 target=x86_64-pc-windows-msvc")
            out contains(windows_text, ".def apl_program_entry; .scl 2; .type 32; .endef")
            out contains(windows_text, "sub rsp, 40")
            out contains(windows_text, "mov r12, rcx")
            out contains(windows_text, ".section .rdata,\"dr\"")
            out contains(windows_text, ".byte 65, 208, 150, 240, 159, 153, 130")
            out contains(windows_text, "call apl_fn_0")
            out contains(windows_text, "call apl_rt_function_enter")
            out contains(windows_text, "call apl_rt_finish")
            out contains(linux_text, "# APLASM1 runtime-abi=2 target=x86_64-unknown-linux-gnu")
            out contains(linux_text, ".type apl_program_entry, @function")
            out contains(linux_text, "sub rsp, 8")
            out contains(linux_text, "mov r12, rdi")
            out contains(linux_text, ".section .rodata")
            out contains(linux_text, ".section .note.GNU-stack,\"\",@progbits")
            "##,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKOKtruetruetruetruetruetruetruetruetruetruetruetruetruetruetrue"
        );
    }

    #[test]
    fn source_runtime_x64_assembly_covers_complete_abi_surface_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "func echo(value) { \nreturn value \n} \nfunc abort() { \nreturn(ERR=[\"never\"][7]) \n} \nAVInt x = input \nAVFloat ratio = 2.5 \nASVStr hidden = secret input \nAVBool same = x =self= \nAVStr type_name = \"\" \nAVStr protection = \"\" \ntype_name, protection = info(x) \nList items = [1:ASV, 2] \nVTime first = items[0] \nVTime part = items[:] \nVTime neg = -x \nx += 1 \nadd(items, 3) \nVTime last = pop(items) \nVTime returned = echo(neg + 2) \necho(x) \nif false { \nfail \"never\" \nabort() \n} \nwhile (x < 3) (4) { \nx += 1 \nif x == 2 { \ncontinue \n} \nbreak \n} \npick(items): item { \nif item == 1 { \ncontinue \n} \nbreak \n} \nsecretup(x) \nout \"ok\" \nstop"
            VTime linked = bootstrap.linked_artifact_report(source)
            VTime low = native.lower_linked_artifact_report(get(linked, 1))
            VTime lowered = abi.lower_encoded_report(get(low, 1), abi.TARGET_WINDOWS_X64)
            VTime emitted = assembly.emit_encoded_report(get(lowered, 1))
            VTime text = get(emitted, 1)
            VTime expected = [
              "call apl_rt_push_const", "call apl_rt_push_none", "call apl_rt_input",
              "call apl_rt_secret_input", "call apl_rt_load", "call apl_rt_unary",
              "call apl_rt_binary", "call apl_rt_builtin_add", "call apl_rt_builtin_pop",
              "call apl_fn_0", "call apl_rt_list", "call apl_rt_index", "call apl_rt_slice",
              "call apl_rt_self", "call apl_rt_tag", "call apl_rt_declare",
              "call apl_rt_declare_list", "call apl_rt_declare_vtime", "call apl_rt_assign",
              "call apl_rt_info_assign", "call apl_rt_drop", "call apl_rt_secretup",
              "call apl_rt_out", "call apl_rt_stop", "call apl_rt_fail", "call apl_rt_error_return",
              "call apl_rt_return", "call apl_rt_branch_false", "call apl_rt_scope_enter",
              "call apl_rt_scope_exit", "call apl_rt_unwind", "call apl_rt_loop_enter",
              "call apl_rt_loop_check", "call apl_rt_loop_exit", "call apl_rt_pick_enter",
              "call apl_rt_pick_next", "call apl_rt_pick_exit", "call apl_rt_function_enter",
              "call apl_rt_finish", ".Lapl_block_entry_", "jmp .Lapl_block_entry_",
              ".Lapl_targets_entry_", ".Lapl_params_0", ".quad .Lapl_pool_"
            ]
            VTime missing = []

            pick(expected): fragment {
              if contains(text, fragment) != true {
                add(missing, fragment)
              }
            }

            VTime builtin_index = 0
            VTime bad_builtin_symbols = 0

            while (builtin_index < len(abi.BUILTINS)) (-1) {
              VTime expected_symbol = join(["apl_rt_builtin_", get(abi.BUILTINS, builtin_index)], "")

              if assembly.builtin_symbol(builtin_index) != expected_symbol {
                bad_builtin_symbols += 1
              }

              builtin_index += 1
            }

            out get(linked, 0)
            out get(lowered, 0)
            out get(emitted, 0)
            out len(missing)
            out bad_builtin_symbols
            out get(assembly.emit_encoded_report("BAD"), 0)
            out get(assembly.emit_encoded_report("BAD"), 1)
            out get(assembly.emit_image_report(1), 0)
            out get(assembly.emit_image_report(1), 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKOK00FAILinvalid native ABI headerFAILinvalid native ABI image"
        );
    }

    #[test]
    fn source_runtime_round_trips_linked_apl_artifacts_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime quote = char(34)
            AVStr source = "func say() { \nreturn echo(\"a:b,[x]\")[::-1] \n} \nfunc echo(x) { \nreturn x \n} \nout say() \nAVInt x = input \nout echo(x) + 1 \nout 2.5 \nout true \nout NONE"

            VTime encoded_report = bootstrap.linked_artifact_report(source)
            VTime encoded = get(encoded_report, 1)
            VTime loaded_report = bootstrap.load_linked_artifact_report(encoded)
            VTime loaded = get(loaded_report, 1)
            VTime entry_call = get(get(get(loaded, 1), 0), 1)
            VTime say_expr = get(get(get(get(loaded, 2), 2), 0), 0)[1]
            VTime say_call = get(say_expr, 1)
            VTime run_report = bootstrap.run_linked_artifact_with_input_report(encoded, ["4"])
            VTime encoded_again = artifact.encode_loaded(loaded)
            VTime bad_header = bootstrap.load_linked_artifact_report("BAD")
            VTime trailing = bootstrap.load_linked_artifact_report(join([encoded, "x"], ""))
            VTime truncated = bootstrap.load_linked_artifact_report(encoded[:-1])
            VTime bad_slot_image = ["APLLOAD2", [["OUT", ["CALL_SLOT", 9, []]]], [[], [], [], true]]
            VTime bad_slot_encoded = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded(bad_slot_image))], "")
            VTime bad_slot_load = bootstrap.load_linked_artifact_report(bad_slot_encoded)
            VTime bad_slot_encode = artifact.encode_loaded_report(bad_slot_image)
            VTime bad_builtin_image = ["APLLOAD2", [["OUT", ["CALL", "len", []]]], [[], [], [], true]]
            VTime bad_builtin_encoded = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded(bad_builtin_image))], "")
            VTime bad_builtin_load = bootstrap.load_linked_artifact_report(bad_builtin_encoded)
            VTime bad_functions_image = ["APLLOAD2", [], [["f"], [[]], [], true]]
            VTime bad_functions_encoded = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded(bad_functions_image))], "")
            VTime bad_functions_load = bootstrap.load_linked_artifact_report(bad_functions_encoded)

            out get(encoded_report, 0)
            out encoded[:len(artifact.HEADER)]
            out get(loaded_report, 0)
            out get(loaded, 0)
            out ir.expr_opcode(entry_call)
            out get(entry_call, 1)
            out ir.expr_opcode(say_call)
            out get(say_call, 1)
            out encoded_again == encoded
            out get(run_report, 0)
            out get(get(run_report, 1), 0)
            out get(get(run_report, 1), 1)
            out get(get(run_report, 1), 2)
            out get(get(run_report, 1), 3)
            out get(get(run_report, 1), 4)
            out get(bad_header, 0)
            out get(bad_header, 1)
            out get(trailing, 0)
            out get(trailing, 1)
            out get(truncated, 0)
            out get(bad_slot_load, 0)
            out get(bad_slot_load, 1)
            out get(bad_slot_encode, 0)
            out get(bad_slot_encode, 1)
            out get(bad_builtin_load, 0)
            out get(bad_builtin_load, 1)
            out get(bad_functions_load, 0)
            out get(bad_functions_load, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKAPLLINK3:OKAPLLOAD2CALL_SLOT0CALL_SLOT1trueOK]x[,b:a52.5trueNONEFAILinvalid linked artifact headerFAILtrailing linked artifact dataFAILFAILinvalid linked artifact imageFAILinvalid loaded imageFAILinvalid linked artifact imageFAILinvalid linked artifact image"
        );
    }

    #[test]
    fn source_runtime_migrates_legacy_linked_artifacts_and_validates_manifests_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime loaded = get(bootstrap.loaded_image_report("out 7"), 1)
            VTime legacy = join([artifact.LEGACY_HEADER, artifact.encode_node(artifact.wrap_loaded(loaded))], "")
            VTime legacy_load = bootstrap.load_linked_artifact_report(legacy)
            VTime legacy_manifest = bootstrap.linked_artifact_manifest_report(legacy)
            VTime legacy_run = bootstrap.run_linked_artifact_report(legacy)
            VTime migrated = artifact.encode_loaded(get(legacy_load, 1))
            VTime migrated_manifest = bootstrap.linked_artifact_manifest(migrated)
            VTime valid_manifest = [linker.MANIFEST_MAGIC, [["base", []]]]
            VTime bad_manifest = [linker.MANIFEST_MAGIC, [["math", []]]]
            VTime loaded_with_manifest = [get(loaded, 0), get(loaded, 1), get(loaded, 2), valid_manifest]
            VTime missing_manifest_v3 = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded(loaded))], "")
            VTime extra_manifest_v2 = join([artifact.LEGACY_HEADER, artifact.encode_node(artifact.wrap_loaded(loaded_with_manifest))], "")
            VTime invalid_manifest_v3 = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded([get(loaded, 0), get(loaded, 1), get(loaded, 2), bad_manifest]))], "")
            VTime duplicate_library_manifest = [linker.MANIFEST_MAGIC, [["base", []], ["math", []], ["math", []]]]
            VTime duplicate_exclusion_manifest = [linker.MANIFEST_MAGIC, [["base", ["len", "len"]]]]
            VTime duplicate_library_v3 = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded([get(loaded, 0), get(loaded, 1), get(loaded, 2), duplicate_library_manifest]))], "")
            VTime duplicate_exclusion_v3 = join([artifact.HEADER, artifact.encode_node(artifact.wrap_loaded([get(loaded, 0), get(loaded, 1), get(loaded, 2), duplicate_exclusion_manifest]))], "")
            VTime missing_manifest_load = bootstrap.load_linked_artifact_report(missing_manifest_v3)
            VTime extra_manifest_load = bootstrap.load_linked_artifact_report(extra_manifest_v2)
            VTime invalid_manifest_load = bootstrap.load_linked_artifact_report(invalid_manifest_v3)
            VTime duplicate_library_load = bootstrap.load_linked_artifact_report(duplicate_library_v3)
            VTime duplicate_exclusion_load = bootstrap.load_linked_artifact_report(duplicate_exclusion_v3)

            out legacy[:len(artifact.LEGACY_HEADER)]
            out get(legacy_load, 0)
            out get(legacy_manifest, 0)
            out get(legacy_manifest, 1) == NONE
            out get(legacy_run, 0)
            out get(get(legacy_run, 1), 0)
            out migrated[:len(artifact.HEADER)]
            out get(get(get(migrated_manifest, 1), 0), 0)
            out get(missing_manifest_load, 0)
            out get(missing_manifest_load, 1)
            out get(extra_manifest_load, 0)
            out get(extra_manifest_load, 1)
            out get(invalid_manifest_load, 0)
            out get(invalid_manifest_load, 1)
            out get(duplicate_library_load, 0)
            out get(duplicate_library_load, 1)
            out get(duplicate_exclusion_load, 0)
            out get(duplicate_exclusion_load, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "APLLINK2:OKOKtrueOK7APLLINK3:baseFAILinvalid linked artifact imageFAILinvalid linked artifact imageFAILinvalid linked artifact imageFAILinvalid linked artifact imageFAILinvalid linked artifact image"
        );
    }

    #[test]
    fn source_runtime_links_relocatable_apl_modules_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr module_source = "AVInt module.answer = 40 \nList module.values = [2] \nfunc module.add_one(value) { \nreturn value + 1 \n} \nfunc module.total() { \nreturn module.add_one(module.answer) + get(module.values, 0) - 1 \n} \nfunc module.later(value) { \nreturn value + 100 \n}"
            VTime module_report = bootstrap.module_report(module_source)
            VTime encoded_module = get(module_report, 1)
            VTime loaded_module = bootstrap.load_module_report(encoded_module)
            VTime linked_report = bootstrap.linked_artifact_with_module_report(encoded_module, "out module.answer \nout get(module.values, 0) \nout module.total()")
            VTime run_report = bootstrap.run_linked_artifact_report(get(linked_report, 1))
            VTime later_linked_report = bootstrap.linked_artifact_with_module_report(encoded_module, "out module.later(1)")
            VTime later_run_report = bootstrap.run_linked_artifact_report(get(later_linked_report, 1))
            VTime unknown_report = bootstrap.linked_artifact_with_module_report(encoded_module, "out module.missing(1)")
            VTime duplicate_report = bootstrap.linked_artifact_with_module_report(encoded_module, "AVInt module.answer = 1")
            VTime bad_header = bootstrap.load_module_report("BAD")
            VTime prelinked_program = [["OUT", ["CALL_SLOT", 0, []]], ["FUNC", "f", [], [["RETURN", ["LITERAL", "Int", 1]]]]]
            VTime prelinked_encode = artifact.encode_module_report(prelinked_program)
            VTime prelinked_image = artifact.node_list([
              artifact.node_str("APLMOD2"),
              artifact.wrap_program(prelinked_program),
              artifact.wrap_symbol_table([["f", "Func", 0]])
            ])
            VTime prelinked_text = join([artifact.MODULE_HEADER, artifact.encode_node(prelinked_image)], "")
            VTime prelinked_load = bootstrap.load_module_report(prelinked_text)
            VTime module_image = get(loaded_module, 1)
            VTime legacy_image = artifact.node_list([
              artifact.node_str("APLMOD1"),
              artifact.wrap_program(artifact.module_program(module_image)),
              artifact.wrap_str_list(["module.add_one", "module.total", "module.later"]),
              artifact.wrap_int_list([1, 0, 1])
            ])
            VTime legacy_text = join([artifact.MODULE_HEADER_V1, artifact.encode_node(legacy_image)], "")
            VTime legacy_load = bootstrap.load_module_report(legacy_text)

            out get(module_report, 0)
            out encoded_module[:len(artifact.MODULE_HEADER)]
            out get(loaded_module, 0)
            out get(linked_report, 0)
            out get(run_report, 0)
            out get(get(run_report, 1), 0)
            out get(get(run_report, 1), 1)
            out get(get(run_report, 1), 2)
            out get(later_linked_report, 0)
            out get(later_run_report, 0)
            out get(get(later_run_report, 1), 0)
            out get(unknown_report, 0)
            out get(unknown_report, 1)
            out get(duplicate_report, 0)
            out get(duplicate_report, 1)
            out get(bad_header, 0)
            out get(bad_header, 1)
            out get(prelinked_encode, 0)
            out get(prelinked_encode, 1)
            out get(prelinked_load, 0)
            out get(prelinked_load, 1)
            out get(legacy_load, 0)
            out get(get(legacy_load, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKAPLMOD3:OKOKOK40242OKOK101FAILsemantic error at 1:1: unknown function `module.missing`FAILsemantic error at 1:1: duplicate name `module.answer`FAILinvalid module artifact headerFAILmodule program is already linkedFAILinvalid module artifact imageOKAPLMOD1"
        );
    }

    #[test]
    fn source_runtime_links_named_libraries_over_precompiled_base_module_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime base_report = bootstrap.module_report("func base.one() { \nreturn 1 \n}")
            VTime math_report = bootstrap.module_report("func math.hidden(value) { \nreturn value + 1 \n} \nfunc math.answer() { \nreturn math.hidden(40) \n}")
            VTime libraries = [["math", get(math_report, 1)]]
            VTime linked = bootstrap.linked_artifact_with_module_and_libraries_report(get(base_report, 1), libraries, "include >> math \nexclude \"hidden\" << math \nout base.one() \nout math.answer()")
            VTime manifest_report = bootstrap.linked_artifact_manifest_report(get(linked, 1))
            VTime manifest = get(manifest_report, 1)
            VTime run = bootstrap.run_linked_artifact_report(get(linked, 1))
            VTime excluded = bootstrap.linked_artifact_with_module_and_libraries_report(get(base_report, 1), libraries, "include >> math \nexclude \"hidden\" << math \nout math.hidden(1)")
            VTime missing_include = bootstrap.linked_artifact_with_module_and_libraries_report(get(base_report, 1), libraries, "out math.answer()")

            out get(linked, 0)
            out get(linked, 1)[:len(artifact.HEADER)]
            out get(manifest_report, 0)
            out get(manifest, 0)
            out get(get(get(manifest, 1), 0), 0)
            out len(get(manifest, 1))
            out get(get(get(manifest, 1), 1), 0)
            out get(get(get(get(manifest, 1), 1), 1), 0)
            out get(run, 0)
            out get(get(run, 1), 0)
            out get(get(run, 1), 1)
            out get(excluded, 0)
            out get(excluded, 1)
            out get(missing_include, 0)
            out get(missing_include, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKAPLLINK3:OKAPLMAN1base2mathmath.hiddenOK141FAILsemantic error at 3:1: excluded symbol `math.hidden` from library `math`FAILsemantic error at 1:1: unknown function `math.answer`"
        );
    }

    #[test]
    fn source_runtime_preserves_protection_across_apl_modules_prelude() {
        let output = run_source_with_prelude(
            r#"
            VTime secret_list_report = bootstrap.module_report("List module.items = [1:ASV]")
            VTime secret_list_text = get(secret_list_report, 1)
            VTime secret_list_image = get(bootstrap.load_module_report(secret_list_text), 1)
            VTime secret_list_symbols = artifact.module_symbols(secret_list_image)
            VTime secret_list_link = bootstrap.linked_artifact_with_module_report(secret_list_text, "out get(module.items, 0)")

            VTime promoted_report = bootstrap.module_report("AVInt module.value = 1 \nsecretup(module.value)")
            VTime promoted_text = get(promoted_report, 1)
            VTime promoted_image = get(bootstrap.load_module_report(promoted_text), 1)
            VTime promoted_symbols = artifact.module_symbols(promoted_image)
            VTime promoted_link = bootstrap.linked_artifact_with_module_report(promoted_text, "out module.value")

            VTime legacy_program = artifact.module_program(secret_list_image)
            VTime legacy_image = artifact.node_list([
              artifact.node_str("APLMOD2"),
              artifact.wrap_program(legacy_program),
              artifact.wrap_symbol_table(artifact.program_symbols(legacy_program))
            ])
            VTime legacy_text = join([artifact.MODULE_HEADER_V2, artifact.encode_node(legacy_image)], "")
            VTime legacy_report = bootstrap.load_module_report(legacy_text)
            VTime legacy_symbols = artifact.module_symbols(get(legacy_report, 1))

            VTime invalid_name_program = [["DECL", "AVInt", "bad?name", ["LITERAL", "Int", 1]]]
            VTime invalid_name_image = artifact.node_list([
              artifact.node_str("APLMOD3"),
              artifact.wrap_program(invalid_name_program),
              artifact.wrap_symbol_table([["bad?name", "Absolute", "AVInt", "AV"]])
            ])
            VTime invalid_name_text = join([artifact.MODULE_HEADER, artifact.encode_node(invalid_name_image)], "")
            VTime invalid_name_report = bootstrap.load_module_report(invalid_name_text)

            out secret_list_text[:len(artifact.MODULE_HEADER)]
            out get(get(secret_list_symbols, 0), 3)
            out get(secret_list_link, 0)
            out get(secret_list_link, 1)
            out get(get(promoted_symbols, 0), 3)
            out get(promoted_link, 0)
            out get(promoted_link, 1)
            out get(legacy_report, 0)
            out get(get(legacy_symbols, 0), 3)
            out get(invalid_name_report, 0)
            out get(invalid_name_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "APLMOD3:ASVFAILsemantic error at 1:1: secret expression denied `out`ASVFAILsemantic error at 1:1: secret expression denied `out`OKSASVFAILinvalid module artifact image"
        );
    }

    #[test]
    fn source_runtime_uses_apl_checker_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr ok_source = "AVInt x = 1 \nList items = [x] \nfunc done() { \nreturn x \n} \nout done()"
            AVStr duplicate_var = "AVInt x = 1 \nAVStr x = 2 \nout x"
            AVStr duplicate_func = "func go() { \nreturn 1 \n} \nfunc go() { \nreturn 2 \n} \nout go()"
            AVStr nested_duplicate = "AVInt x = 1 \nif true { \nAVInt x = 2 \n} \nout x"
            AVStr nested_func_duplicate = "func go() { \nreturn 1 \n} \nif true { \nfunc go() { \nreturn 2 \n} \n} \nout go()"

            VTime ok_report = bootstrap.compile_report(ok_source)
            VTime var_report = bootstrap.compile_report(duplicate_var)
            VTime func_report = bootstrap.compile_report(duplicate_func)
            VTime nested_report = bootstrap.compile_report(nested_duplicate)
            VTime nested_func_report = bootstrap.compile_report(nested_func_duplicate)

            out get(ok_report, 0)
            out ir.opcode(get(get(ok_report, 1), 0))
            out get(var_report, 0)
            out get(var_report, 1)
            out get(func_report, 0)
            out get(func_report, 1)
            out get(nested_report, 0)
            out get(nested_report, 1)
            out get(nested_func_report, 0)
            out get(nested_func_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKDECLFAILsemantic error at 2:1: duplicate name `x`FAILsemantic error at 4:1: duplicate name `go`FAILsemantic error at 3:1: duplicate name `x`FAILsemantic error at 5:1: duplicate name `go`"
        );
    }

    #[test]
    fn source_runtime_apl_checker_scopes_vtime_names() {
        let output = run_source_with_prelude(
            r#"
            AVStr scoped_source = "func first(value) { \nVTime text = value \nif true { \nVTime local = text \ntext = local \n} \nreturn text \n} \nfunc second(value) { \nVTime text = value \nwhile (false) (-1) { \nVTime local = text \n} \npick([value]): item { \nVTime local = item \n} \nreturn text \n} \nout first(1) \nout second(2)"
            AVStr leaked_source = "func make() { \nVTime local = 1 \nreturn local \n} \nout local"
            VTime scoped_report = bootstrap.compile_report(scoped_source)
            VTime leaked_report = bootstrap.compile_report(leaked_source)

            out get(scoped_report, 0)
            out get(leaked_report, 0)
            out get(leaked_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKFAILsemantic error at 5:1: unknown variable `local`"
        );
    }

    #[test]
    fn source_runtime_checker_rejects_unknown_mutation_targets_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr missing_assign = "missing = 1 \nout missing"
            AVStr list_assign = "List items = [] \nitems = 1"
            AVStr func_assign = "func go() { \nreturn 1 \n} \ngo = 2"
            AVStr secret_compound = "ASVInt hidden = 1 \nhidden += 1"
            AVStr string_compound = "AVStr text = \"\" \ntext += 1"
            AVStr vtime_compound = "VTime temp = 1 \ntemp += 1 \nout temp"
            AVStr missing_secretup = "secretup(missing)"
            AVStr list_secretup = "List items = [] \nsecretup(items)"
            AVStr missing_info_target = "AVStr level = \"\" \ntyp, level = info(level)"
            AVStr bad_info_target = "AVInt typ = 1 \nAVStr level = \"\" \ntyp, level = info(typ)"
            AVStr missing_info_source = "AVStr typ = \"\" \nAVStr level = \"\" \ntyp, level = info(missing)"

            VTime assign_report = bootstrap.compile_report(missing_assign)
            VTime list_assign_report = bootstrap.compile_report(list_assign)
            VTime func_assign_report = bootstrap.compile_report(func_assign)
            VTime secret_compound_report = bootstrap.compile_report(secret_compound)
            VTime string_compound_report = bootstrap.compile_report(string_compound)
            VTime vtime_compound_report = bootstrap.run_report(vtime_compound)
            VTime secretup_report = bootstrap.compile_report(missing_secretup)
            VTime list_report = bootstrap.compile_report(list_secretup)
            VTime info_target_report = bootstrap.compile_report(missing_info_target)
            VTime bad_info_report = bootstrap.compile_report(bad_info_target)
            VTime info_source_report = bootstrap.compile_report(missing_info_source)

            out get(assign_report, 0)
            out get(assign_report, 1)
            out get(list_assign_report, 0)
            out get(list_assign_report, 1)
            out get(func_assign_report, 0)
            out get(func_assign_report, 1)
            out get(secret_compound_report, 0)
            out get(secret_compound_report, 1)
            out get(string_compound_report, 0)
            out get(string_compound_report, 1)
            out get(vtime_compound_report, 0)
            out get(get(vtime_compound_report, 1), 0)
            out get(secretup_report, 0)
            out get(secretup_report, 1)
            out get(list_report, 0)
            out get(list_report, 1)
            out get(info_target_report, 0)
            out get(info_target_report, 1)
            out get(bad_info_report, 0)
            out get(bad_info_report, 1)
            out get(info_source_report, 0)
            out get(info_source_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "FAILsemantic error at 1:1: unknown assignment target `missing`FAILsemantic error at 2:1: invalid assignment target `items`FAILsemantic error at 4:1: invalid assignment target `go`FAILsemantic error at 2:1: invalid compound assignment target `hidden`FAILsemantic error at 2:1: invalid compound assignment target `text`OK2FAILsemantic error at 1:1: unknown secretup target `missing`FAILsemantic error at 2:1: invalid secretup target `items`FAILsemantic error at 2:1: unknown info target `typ`FAILsemantic error at 3:1: invalid info target `typ`FAILsemantic error at 3:1: unknown info source `missing`"
        );
    }

    #[test]
    fn source_runtime_checker_rejects_unknown_expression_refs_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid = "AVInt x = 1 \nVTime y = x + 1 \nout y"
            AVStr missing_out = "out missing"
            AVStr missing_decl = "AVInt x = missing"
            AVStr missing_assignment_value = "AVInt x = 1 \nx = missing"
            AVStr missing_list = "List items = [missing]"
            AVStr missing_if = "if missing { \nout 1 \n}"
            AVStr bad_self = "VTime temp = 1 \nout temp =self="
            AVStr missing_slice = "List items = [1] \nVTime part = items[:missing]"

            VTime valid_report = bootstrap.run_report(valid)
            VTime out_report = bootstrap.compile_report(missing_out)
            VTime decl_report = bootstrap.compile_report(missing_decl)
            VTime assign_report = bootstrap.compile_report(missing_assignment_value)
            VTime list_report = bootstrap.compile_report(missing_list)
            VTime if_report = bootstrap.compile_report(missing_if)
            VTime self_report = bootstrap.compile_report(bad_self)
            VTime slice_report = bootstrap.compile_report(missing_slice)

            out get(valid_report, 0)
            out get(get(valid_report, 1), 0)
            out get(out_report, 0)
            out get(out_report, 1)
            out get(decl_report, 0)
            out get(decl_report, 1)
            out get(assign_report, 0)
            out get(assign_report, 1)
            out get(list_report, 0)
            out get(list_report, 1)
            out get(if_report, 0)
            out get(if_report, 1)
            out get(self_report, 0)
            out get(self_report, 1)
            out get(slice_report, 0)
            out get(slice_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OK2FAILsemantic error at 1:1: unknown variable `missing`FAILsemantic error at 1:1: unknown variable `missing`FAILsemantic error at 2:1: unknown variable `missing`FAILsemantic error at 1:1: unknown variable `missing`FAILsemantic error at 1:1: unknown variable `missing`FAILsemantic error at 2:1: invalid self target `temp`FAILsemantic error at 2:1: unknown variable `missing`"
        );
    }

    #[test]
    fn source_runtime_checker_validates_function_calls_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_forward = "out go() \nfunc go() { \nreturn 7 \n}"
            AVStr builtin_ok = "out len([1, 2])"
            AVStr missing_func = "out missing()"
            AVStr var_as_func = "AVInt x = 1 \nout x()"
            AVStr bad_arity = "func go(a) { \nreturn a \n} \nout go()"
            AVStr missing_arg = "out len(missing)"
            AVStr builtin_missing = "out len()"
            AVStr builtin_extra = "out pow(2, 3, 4)"
            AVStr mutating_builtin_missing = "add()"

            VTime valid_report = bootstrap.run_report(valid_forward)
            VTime builtin_report = bootstrap.run_report(builtin_ok)
            VTime missing_func_report = bootstrap.compile_report(missing_func)
            VTime var_func_report = bootstrap.compile_report(var_as_func)
            VTime arity_report = bootstrap.compile_report(bad_arity)
            VTime missing_arg_report = bootstrap.compile_report(missing_arg)
            VTime builtin_missing_report = bootstrap.compile_report(builtin_missing)
            VTime builtin_extra_report = bootstrap.compile_report(builtin_extra)
            VTime mutating_builtin_report = bootstrap.compile_report(mutating_builtin_missing)

            out get(valid_report, 0)
            out get(get(valid_report, 1), 0)
            out get(builtin_report, 0)
            out get(get(builtin_report, 1), 0)
            out get(missing_func_report, 0)
            out get(missing_func_report, 1)
            out get(var_func_report, 0)
            out get(var_func_report, 1)
            out get(arity_report, 0)
            out get(arity_report, 1)
            out get(missing_arg_report, 0)
            out get(missing_arg_report, 1)
            out get(builtin_missing_report, 0)
            out get(builtin_missing_report, 1)
            out get(builtin_extra_report, 0)
            out get(builtin_extra_report, 1)
            out get(mutating_builtin_report, 0)
            out get(mutating_builtin_report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OK7OK2FAILsemantic error at 1:1: unknown function `missing`FAILsemantic error at 2:1: invalid function `x`FAILsemantic error at 4:1: wrong argument count `go`FAILsemantic error at 1:1: unknown variable `missing`FAILsemantic error at 1:1: wrong argument count `len`FAILsemantic error at 1:1: wrong argument count `pow`FAILsemantic error at 1:1: wrong argument count `add`"
        );
    }

    #[test]
    fn source_runtime_checker_builtin_arities_match_verifier_prelude() {
        let output = run_source_with_prelude(
            r#"
            List binary_builtins = ["get", "split", "join", "contains", "pow", "add"]
            List unary_builtins = ["len", "ord", "char", "pop", "int", "float", "bool", "str", "bytes", "json"]
            List ternary_builtins = ["set"]
            List zero_builtins = ["timer_start", "timer_stop", "timer_elapsed"]
            AVBool builtin_arity_matches = true

            pick(binary_builtins): builtin_name {
              if checker.builtin_arity(builtin_name) != 2 {
                builtin_arity_matches = false
              }

              if checker.builtin_arity(builtin_name) != verifier.builtin_arity(builtin_name) {
                builtin_arity_matches = false
              }
            }

            pick(unary_builtins): builtin_name {
              if checker.builtin_arity(builtin_name) != 1 {
                builtin_arity_matches = false
              }

              if checker.builtin_arity(builtin_name) != verifier.builtin_arity(builtin_name) {
                builtin_arity_matches = false
              }
            }

            pick(ternary_builtins): builtin_name {
              if checker.builtin_arity(builtin_name) != 3 {
                builtin_arity_matches = false
              }

              if checker.builtin_arity(builtin_name) != verifier.builtin_arity(builtin_name) {
                builtin_arity_matches = false
              }
            }

            pick(zero_builtins): builtin_name {
              if checker.builtin_arity(builtin_name) != 0 {
                builtin_arity_matches = false
              }

              if checker.builtin_arity(builtin_name) != verifier.builtin_arity(builtin_name) {
                builtin_arity_matches = false
              }
            }

            out builtin_arity_matches
            out checker.builtin_arity("missing")
            "#,
        )
        .unwrap();

        assert_eq!(output, "true-1");
    }

    #[test]
    fn source_runtime_checker_validates_builtin_argument_types_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_static = "List values = [1] \nout len(values) \nout get(values, 0) \nout char(65) \nout pow(2, 3)"
            AVStr valid_dynamic = "VTime value = 1 \nout len(value) \nout split(value, value) \nout pow(value, value)"
            AVStr bad_len = "out len(1)"
            AVStr bad_get = "out get(1, 0)"
            AVStr bad_split = "out split(1, 2)"
            AVStr bad_join = "out join(1, 2)"
            AVStr bad_contains = "out contains(1, 2)"
            AVStr bad_ord = "out ord(1)"
            AVStr bad_char = "out char(true)"
            AVStr bad_pow = "out pow(true, 2)"
            AVStr bad_pop = "VTime value = pop(1)"
            AVStr bad_add = "AVInt value = 1 \nadd(value, 2)"

            List reports = [
              bootstrap.compile_report(valid_static),
              bootstrap.compile_report(valid_dynamic),
              bootstrap.compile_report(bad_len),
              bootstrap.compile_report(bad_get),
              bootstrap.compile_report(bad_split),
              bootstrap.compile_report(bad_join),
              bootstrap.compile_report(bad_contains),
              bootstrap.compile_report(bad_ord),
              bootstrap.compile_report(bad_char),
              bootstrap.compile_report(bad_pow),
              bootstrap.compile_report(bad_pop),
              bootstrap.compile_report(bad_add)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKFAILsemantic error at 1:1: len(value) requires Str, Bytes, List, or VTimeFAILsemantic error at 1:1: get(list, index) requires List and IntFAILsemantic error at 1:1: split(value, separator) requires Str and StrFAILsemantic error at 1:1: join(values, separator) requires List and StrFAILsemantic error at 1:1: contains(value, needle) requires Str and StrFAILsemantic error at 1:1: ord(value) requires StrFAILsemantic error at 1:1: char(value) requires IntFAILsemantic error at 1:1: pow(base, exponent) requires Int/Float and IntFAILsemantic error at 1:1: pop(list) requires ListFAILsemantic error at 2:1: add(list, value) requires List"
        );
    }

    #[test]
    fn source_runtime_checker_validates_list_mutation_contracts_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_pop = "List items = [1] \nVTime item = pop(items) \nout item"
            AVStr bad_expression_add = "List items = [1] \nVTime result = add(items, 2)"
            AVStr bad_temporary_add = "add([1], 2)"
            AVStr bad_temporary_pop = "VTime item = pop([1])"
            AVStr bad_vtime_out = "VTime items = [1] \nASVInt hidden = 2 \nadd(items, hidden) \nout items"

            List reports = [
              bootstrap.compile_report(valid_pop),
              bootstrap.compile_report(bad_expression_add),
              bootstrap.compile_report(bad_temporary_add),
              bootstrap.compile_report(bad_temporary_pop),
              bootstrap.compile_report(bad_vtime_out)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKFAILsemantic error at 2:1: add(list, value) is only valid as a statementFAILsemantic error at 1:1: add(list, value) requires a named List or VTime targetFAILsemantic error at 1:1: pop(list) requires a named List or VTime targetFAILsemantic error at 4:1: secret expression denied `out`"
        );
    }

    #[test]
    fn source_runtime_checker_tracks_prefixed_list_mutation_protection_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr bad_add_get = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nstd::add(values, secret_value) \nVTime selected = std::get(values, 1) \nout selected"
            AVStr bad_set_index = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nstd::set(values, 0, secret_value) \nVTime selected = values[0] \nout selected"
            AVStr bad_add_pop = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nstd::add(values, secret_value) \nVTime removed = std::pop(values) \nout removed"
            AVStr bad_alias_add = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nVTime alias = values \nstd::add(alias, secret_value) \nout std::get(values, 1)"
            AVStr bad_alias_set = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nVTime alias = values \nstd::set(alias, 0, secret_value) \nout std::get(values, 0)"
            AVStr bad_original_add = "SASVStr secret_value = \"hidden\" \nList values = [\"public\"] \nVTime alias = values \nstd::add(values, secret_value) \nout std::get(alias, 1)"
            List reports = [
              bootstrap.compile_report(bad_add_get),
              bootstrap.compile_report(bad_set_index),
              bootstrap.compile_report(bad_add_pop),
              bootstrap.compile_report(bad_alias_add),
              bootstrap.compile_report(bad_alias_set),
              bootstrap.compile_report(bad_original_add)
            ]

            pick(reports): report {
              if contains(get(report, 1), "secret expression denied") {
                out "DENIED"
              } else {
                out get(report, 0)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(output, "DENIEDDENIEDDENIEDDENIEDDENIEDDENIED");
    }

    #[test]
    fn source_runtime_apl_vm_reports_zero_slice_step_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List values = [1, 2, 3] \nVTime invalid = values[::0] \nout len(invalid)"
            VTime report = bootstrap.run_report(source)
            out get(report, 0)
            out get(get(report, 1), 0)
            "#,
        )
        .unwrap();

        assert_eq!(output, "FAILzero slice step");
    }

    #[test]
    fn source_runtime_checker_validates_decl_and_assignment_types_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_exact = "AVInt integer = 1 \nAVFloat decimal = 1.0 \nAVBool flag = true \nAVStr text = str(1) \nAVBytes raw = bytes(text) \nAVJson document = json(text) \ninteger = 2 \ndecimal = 2.0 \nflag = false \ntext = str(2) \nraw = bytes(text) \ndocument = json(text)"
            AVStr valid_dynamic = "AVInt value = input \nvalue = NONE \nVTime dynamic = 1 \ndynamic = str(dynamic) \nvalue = dynamic"
            AVStr bad_int = "AVInt value = str(1)"
            AVStr bad_float = "AVFloat value = 1"
            AVStr bad_bool = "AVBool value = 1"
            AVStr bad_str = "AVStr value = 1"
            AVStr bad_bytes = "AVBytes value = str(1)"
            AVStr bad_json = "AVJson value = str(1)"
            AVStr bad_assignment = "AVInt value = 1 \nvalue = str(2)"
            AVStr bad_compound = "AVInt value = 1 \nvalue += 1.0"

            List reports = [
              bootstrap.compile_report(valid_exact),
              bootstrap.compile_report(valid_dynamic),
              bootstrap.compile_report(bad_int),
              bootstrap.compile_report(bad_float),
              bootstrap.compile_report(bad_bool),
              bootstrap.compile_report(bad_str),
              bootstrap.compile_report(bad_bytes),
              bootstrap.compile_report(bad_json),
              bootstrap.compile_report(bad_assignment),
              bootstrap.compile_report(bad_compound)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKFAILsemantic error at 1:1: type mismatch `value`: expected Int, got StrFAILsemantic error at 1:1: type mismatch `value`: expected Float, got IntFAILsemantic error at 1:1: type mismatch `value`: expected Bool, got IntFAILsemantic error at 1:1: type mismatch `value`: expected Str, got IntFAILsemantic error at 1:1: type mismatch `value`: expected Bytes, got StrFAILsemantic error at 1:1: type mismatch `value`: expected Json, got StrFAILsemantic error at 2:1: type mismatch `value`: expected Int, got StrFAILsemantic error at 2:1: type mismatch `value`: expected Int, got Float"
        );
    }

    #[test]
    fn source_runtime_checker_validates_expression_operand_types_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_exact = "AVInt integer = 1 \nAVFloat decimal = 1.0 \nAVBool flag = true \nAVInt sum = integer + 2 \nAVFloat quotient = decimal / 2.0 \nAVBool equal = integer == 1 \nAVBool ordered = decimal >= 1.0 \nAVBool logical = flag and true \nif flag { \nout integer \n} \nwhile (integer < 2) (1) { \ninteger += 1 \n} \npick([integer]): item { \nout item \n}"
            AVStr valid_dynamic = "VTime value = 1 \nVTime other = value + true \nVTime index = true \nVTime item = value[index] \nVTime part = value[index:index:index] \nif value { \nout other \n} \nwhile (value) (1) { \nvalue = false \n} \npick(value): picked { \nout picked \n}"
            AVStr bad_negate = "VTime value = -true"
            AVStr bad_not = "VTime value = not 1"
            AVStr bad_arithmetic_shape = "VTime value = 1 + 1.0"
            AVStr bad_arithmetic_type = "VTime value = bytes(1) + bytes(2)"
            AVStr bad_equality = "VTime value = 1 == bytes(1)"
            AVStr bad_ordered = "VTime value = 1 < 1.0"
            AVStr bad_logical = "VTime value = true and 1"
            AVStr bad_if = "if 1 { \nout 1 \n}"
            AVStr bad_while = "while (bytes(1)) (1) { \nout 1 \n}"
            AVStr bad_pick = "pick(1): item { \nout item \n}"
            AVStr bad_index_target = "VTime value = 1[0]"
            AVStr bad_index_value = "VTime value = [1][true]"
            AVStr bad_slice_target = "VTime value = true[:]"
            AVStr bad_slice_bound = "VTime value = [1][1.0:]"

            List reports = [
              bootstrap.compile_report(valid_exact),
              bootstrap.compile_report(valid_dynamic),
              bootstrap.compile_report(bad_negate),
              bootstrap.compile_report(bad_not),
              bootstrap.compile_report(bad_arithmetic_shape),
              bootstrap.compile_report(bad_arithmetic_type),
              bootstrap.compile_report(bad_equality),
              bootstrap.compile_report(bad_ordered),
              bootstrap.compile_report(bad_logical),
              bootstrap.compile_report(bad_if),
              bootstrap.compile_report(bad_while),
              bootstrap.compile_report(bad_pick),
              bootstrap.compile_report(bad_index_target),
              bootstrap.compile_report(bad_index_value),
              bootstrap.compile_report(bad_slice_target),
              bootstrap.compile_report(bad_slice_bound)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKFAILsemantic error at 1:1: invalid unary operation for BoolFAILsemantic error at 1:1: invalid unary operation for IntFAILsemantic error at 1:1: arithmetic requires matching Int or Float values, got Int and FloatFAILsemantic error at 1:1: arithmetic requires matching Int or Float values, got Bytes and BytesFAILsemantic error at 1:1: comparison requires matching types, got Int and BytesFAILsemantic error at 1:1: ordered comparison requires matching Int or Float values, got Int and FloatFAILsemantic error at 1:1: logical operation requires Bool values, got Bool and IntFAILsemantic error at 1:1: type mismatch `condition`: expected Bool, got IntFAILsemantic error at 1:1: type mismatch `condition`: expected Bool, got BytesFAILsemantic error at 1:1: pick requires Str, Bytes, List, or VTime, got IntFAILsemantic error at 1:1: index access requires Str, Bytes, List, or VTime, got IntFAILsemantic error at 1:1: index must be Int or VTime, got BoolFAILsemantic error at 1:1: slice requires Str, Bytes, List, or VTime, got BoolFAILsemantic error at 1:1: slice bounds must be Int or VTime, got Float"
        );
    }

    #[test]
    fn source_runtime_checker_validates_control_flow_context_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_function = "func choose(value) { \nif true { \nreturn value \n} \nreturn NONE \n} \nVTime result = choose(1)"
            AVStr valid_loops = "func work() { \nwhile (true) (1) { \ncontinue \n} \npick([1]): item { \nbreak \n} \nreturn NONE \n}"
            AVStr valid_unlimited = "while (true) (-1) { \nbreak \n}"
            AVStr bad_return = "return NONE"
            AVStr bad_nested_return = "if true { \nreturn NONE \n}"
            AVStr bad_break = "break"
            AVStr bad_continue = "continue"
            AVStr bad_nested_break = "if true { \nbreak \n}"
            AVStr bad_function_break = "func work() { \nbreak \n}"
            AVStr bad_limit = "while (true) (-2) { \nbreak \n}"
            AVStr bad_nested_function_break = "while (true) (1) { \nfunc nested() { \nbreak \n} \n}"

            List reports = [
              bootstrap.compile_report(valid_function),
              bootstrap.compile_report(valid_loops),
              bootstrap.compile_report(valid_unlimited),
              bootstrap.compile_report(bad_return),
              bootstrap.compile_report(bad_nested_return),
              bootstrap.compile_report(bad_break),
              bootstrap.compile_report(bad_continue),
              bootstrap.compile_report(bad_nested_break),
              bootstrap.compile_report(bad_function_break),
              bootstrap.compile_report(bad_limit),
              bootstrap.compile_report(bad_nested_function_break)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKOKFAILsemantic error at 1:1: return outside functionFAILsemantic error at 2:1: return outside functionFAILsemantic error at 1:1: break outside loopFAILsemantic error at 1:1: continue outside loopFAILsemantic error at 2:1: break outside loopFAILsemantic error at 2:1: break outside loopFAILsemantic error at 1:1: invalid loop limit `-2`FAILsemantic error at 3:1: break outside loop"
        );
    }

    #[test]
    fn source_runtime_checker_validates_names_and_parameter_scopes_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_names = "AVInt alpha.beta_2 = 1 \nfunc echo(value) { \nreturn value \n} \nVTime result = echo(alpha.beta_2)"
            AVStr valid_shadow = "AVStr value = \"global\" \nfunc increment(value) { \nreturn value + 1 \n} \nfunc apply(increment) { \nreturn increment(increment) \n} \nout apply(1) \nout value"
            AVStr bad_symbol = "AVInt bad?name = 1"
            AVStr bad_leading_dot = "AVInt .hidden = 1"
            AVStr bad_reserved = "AVInt len = 1"
            AVStr bad_function = "func bad?name(value) { return value }"
            AVStr bad_duplicate_param = "func duplicate(value, value) { \nreturn value \n}"
            AVStr bad_param = "func identity(bad?name) { return bad?name }"
            AVStr bad_pick_collision = "AVInt item = 1 \npick([1]): item { \nout item \n}"
            AVStr bad_pick_name = "pick([1]): bad?name { out bad?name }"
            AVStr bad_shadow_secretup = "AVInt value = 1 \nfunc promote(value) { \nsecretup(value) \n}"
            AVStr bad_shadow_self = "AVInt value = 1 \nfunc same(value) { \nreturn value =self= \n}"
            AVStr bad_function_value = "func go() { \nreturn 1 \n} \nout go"

            VTime valid_names_report = bootstrap.compile_report(valid_names)
            VTime valid_shadow_report = bootstrap.run_report(valid_shadow)
            List bad_reports = [
              bootstrap.compile_report(bad_symbol),
              bootstrap.compile_report(bad_leading_dot),
              bootstrap.compile_report(bad_reserved),
              bootstrap.compile_report(bad_function),
              bootstrap.compile_report(bad_duplicate_param),
              bootstrap.compile_report(bad_param),
              bootstrap.compile_report(bad_pick_collision),
              bootstrap.compile_report(bad_pick_name),
              bootstrap.compile_report(bad_shadow_secretup),
              bootstrap.compile_report(bad_shadow_self),
              bootstrap.compile_report(bad_function_value)
            ]

            out get(valid_names_report, 0)
            out get(valid_shadow_report, 0)

            pick(get(valid_shadow_report, 1)): line {
              out line
            }

            pick(bad_reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOK2globalFAILsemantic error at 1:1: invalid name `bad?name`FAILsemantic error at 1:1: invalid name `.hidden`FAILsemantic error at 1:1: invalid name `len`FAILsemantic error at 1:1: invalid name `bad?name`FAILsemantic error at 1:1: duplicate parameter `value`FAILsemantic error at 1:1: invalid name `bad?name`FAILsemantic error at 2:1: duplicate name `item`FAILsemantic error at 1:1: invalid name `bad?name`FAILsemantic error at 3:1: invalid secretup target `value`FAILsemantic error at 3:1: invalid self target `value`FAILsemantic error at 4:1: unknown variable `go`"
        );
    }

    #[test]
    fn source_runtime_checker_validates_protection_flow_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr valid_assignments = "ASVInt hidden = 1 \nSASVInt vault = hidden \nVTime temporary = hidden \nASVInt copy = temporary"
            AVStr valid_tags = "List items = [1:ASV, 2:SASV] \nSASVInt vault = 1"
            AVStr valid_secret_input = "ASVStr value = secret input"
            AVStr bad_decl = "ASVInt hidden = 1 \nAVInt public = hidden"
            AVStr bad_assign = "ASVInt hidden = 1 \nAVInt public = 0 \npublic = hidden"
            AVStr bad_secret_input_av = "AVStr public = secret input"
            AVStr bad_input_sasv = "SASVStr vault = input"
            AVStr bad_secret_input_sasv = "SASVStr vault = secret input"
            AVStr bad_tag = "ASVInt hidden = 1 \nList items = [hidden:AV]"
            AVStr bad_out = "ASVInt hidden = 1 \nout hidden"
            AVStr bad_derived_out = "ASVInt hidden = 1 \nout hidden + 1"
            AVStr bad_vtime = "ASVInt hidden = 1 \nVTime temporary = hidden \nAVInt public = temporary"
            AVStr bad_list_out = "List items = [1:ASV] \nout items"
            AVStr bad_get_out = "List items = [1:ASV] \nout get(items, 0)"
            AVStr bad_self = "ASVInt hidden = 1 \nout hidden =self="
            AVStr bad_stop = "ASVInt hidden = 1 \nstop hidden"
            AVStr bad_fail = "ASVInt hidden = 1 \nfail hidden"
            AVStr bad_secretup_out = "AVInt value = 1 \nsecretup(value) \nout value"
            AVStr bad_secretup_limit = "AVInt value = 1 \nsecretup(value) \nsecretup(value) \nsecretup(value)"
            AVStr bad_add_out = "List items = [1] \nASVInt hidden = 2 \nadd(items, hidden) \nout items"
            AVStr bad_call_out = "func echo(value) { \nreturn value \n} \nASVInt hidden = 1 \nout echo(hidden)"

            List reports = [
              bootstrap.compile_report(valid_assignments),
              bootstrap.compile_report(valid_tags),
              bootstrap.compile_report(valid_secret_input),
              bootstrap.compile_report(bad_decl),
              bootstrap.compile_report(bad_assign),
              bootstrap.compile_report(bad_secret_input_av),
              bootstrap.compile_report(bad_input_sasv),
              bootstrap.compile_report(bad_secret_input_sasv),
              bootstrap.compile_report(bad_tag),
              bootstrap.compile_report(bad_out),
              bootstrap.compile_report(bad_derived_out),
              bootstrap.compile_report(bad_vtime),
              bootstrap.compile_report(bad_list_out),
              bootstrap.compile_report(bad_get_out),
              bootstrap.compile_report(bad_self),
              bootstrap.compile_report(bad_stop),
              bootstrap.compile_report(bad_fail),
              bootstrap.compile_report(bad_secretup_out),
              bootstrap.compile_report(bad_secretup_limit),
              bootstrap.compile_report(bad_add_out),
              bootstrap.compile_report(bad_call_out)
            ]

            pick(reports): report {
              out get(report, 0)

              if get(report, 0) == checker.STATUS_FAIL {
                out get(report, 1)
              }
            }
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "OKOKOKFAILsemantic error at 2:1: protection downgrade `public`: source ASV, target AVFAILsemantic error at 3:1: protection downgrade `public`: source ASV, target AVFAILsemantic error at 1:1: secret expression denied `public`FAILsemantic error at 1:1: secret expression denied `vault`FAILsemantic error at 1:1: secret expression denied `vault`FAILsemantic error at 2:1: protection downgrade `list element`: source ASV, target AVFAILsemantic error at 2:1: secret expression denied `out`FAILsemantic error at 2:1: secret expression denied `out`FAILsemantic error at 3:1: protection downgrade `public`: source ASV, target AVFAILsemantic error at 2:1: secret expression denied `out`FAILsemantic error at 2:1: secret expression denied `out`FAILsemantic error at 2:1: secret expression denied `hidden`FAILsemantic error at 2:1: secret expression denied `stop`FAILsemantic error at 2:1: secret expression denied `fail`FAILsemantic error at 3:1: secret expression denied `out`FAILsemantic error at 4:1: invalid secretup target `value`FAILsemantic error at 4:1: secret expression denied `out`FAILsemantic error at 5:1: secret expression denied `out`"
        );
    }

    #[test]
    fn source_runtime_checker_blocks_compound_vtime_laundering_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "List values = [4:SASV] \npick(values): item { \nitem += 1 \nVTime temporary = str(item) \nAVStr stolen = temporary \n} \nout stolen"
            VTime report = bootstrap.compile_report(source)
            out get(report, 0)
            out get(report, 1)
            "#,
        )
        .unwrap();

        assert_eq!(
            output,
            "FAILsemantic error at 5:1: protection downgrade `stolen`: source SASV, target AV"
        );
    }

    #[test]
    fn runtime_refuses_compound_vtime_laundering_without_checker() {
        let program = apl_parser::parse_program(
            r#"
            List values = [4:SASV]
            pick(values): item {
              item += 1
              VTime temporary = str(item)
              AVStr stolen = temporary
            }
            out stolen
            "#,
        )
        .unwrap();

        assert_eq!(
            run_program(&program, vec![]),
            Err(RuntimeError::Message(
                "refusing to downgrade declaration protection".to_owned()
            ))
        );
    }

    #[test]
    fn source_runtime_uses_apl_vm_typed_input_coercion_prelude() {
        let output = run_source_with_prelude(
            r#"
            AVStr source = "AVInt good = input \nAVInt bad = input \nAVFloat ratio = input \nAVBool flag = input \nAVStr text = input \nAVInt assigned = 0 \nassigned = input \nout good \nout bad \nout ratio \nout flag \nout text \nout assigned"
            VTime vm_output = vm.run_source_with_input(source, ["42", "oops", "2.5", "true", "abc", "7"])

            out len(vm_output)
            out get(vm_output, 0)
            out get(vm_output, 1)
            out get(vm_output, 2)
            out get(vm_output, 3)
            out get(vm_output, 4)
            out get(vm_output, 5)
            "#,
        )
        .unwrap();

        assert_eq!(output, "642NONE2.5trueabc7");
    }

    #[test]
    fn checker_keeps_split_secret_values_off_output() {
        let program = apl_parser::parse_program(
            r#"
            ASVStr source = "secret words"
            VTime parts = split(source, " ")
            out get(parts, 0)
            "#,
        )
        .unwrap();

        validate_program(&program).unwrap_err();
    }

    #[test]
    fn source_runtime_handles_loop_break_and_continue_with_jump_opcodes() {
        let output = run_source(
            r#"
            AVInt x = 0
            AVInt total = 0

            while (x < 10) (20) {
              x += 1

              if x == 2 {
                continue
              }

              if x == 5 {
                break
              }

              total += x
            }

            out total
            "#,
        )
        .unwrap();

        assert_eq!(output, "8");
    }

    #[test]
    fn compiled_program_splits_entry_and_functions() {
        let program = checked_program(
            r#"
            func add_one(value) {
              return value + 1
            }

            AVInt x = add_one(4)
            out x
            "#,
        );
        let compiled = compile_program(&program).unwrap();

        assert_eq!(block_ops(&compiled, compiled.entry).len(), 2);
        assert!(compiled.functions.contains_key("add_one"));
        let function_body = compiled.functions["add_one"].body;
        assert_eq!(block_ops(&compiled, function_body).len(), 1);
        assert!(matches!(
            block_ops(&compiled, function_body)[0],
            StatementOp::Return(_)
        ));
    }

    #[test]
    fn compiled_program_uses_single_linear_code_segment() {
        let program = checked_program(
            r#"
            func add_one(value) {
              return value + 1
            }

            AVInt x = 0
            while (x < 2) (5) {
              x = add_one(x)
            }
            out x
            "#,
        );
        let compiled = compile_program(&program).unwrap();

        assert!(!compiled.code.is_empty());
        for block in &compiled.blocks {
            assert!(block.start <= block.end);
            assert!(block.end <= compiled.code.len());
        }
        assert!(compiled
            .blocks
            .windows(2)
            .all(|pair| pair[0].end <= pair[1].start));
    }

    #[test]
    fn compiled_while_uses_loop_jump_opcodes() {
        let program = checked_program(
            r#"
            AVInt x = 0
            while (x < 3) (10) {
              x += 1
            }
            out x
            "#,
        );
        let compiled = compile_program(&program).unwrap();
        let entry_start = compiled.blocks[compiled.entry].start;
        let entry_ops = block_ops(&compiled, compiled.entry);

        assert!(matches!(entry_ops[1], StatementOp::LoopCheck { .. }));
        assert!(matches!(entry_ops[2], StatementOp::ExecLoopBody { .. }));
        assert!(matches!(
            entry_ops[3],
            StatementOp::Jump { target_pc } if target_pc == entry_start + 1
        ));
        let StatementOp::LoopCheck { normal_exit_pc, .. } = entry_ops[1] else {
            unreachable!();
        };
        assert_eq!(normal_exit_pc, entry_start + 4);
        let StatementOp::ExecLoopBody {
            body,
            continue_pc,
            break_pc,
        } = entry_ops[2]
        else {
            unreachable!();
        };
        assert_eq!(continue_pc, entry_start + 1);
        assert_eq!(break_pc, entry_start + 4);
        assert_ne!(body, compiled.entry);
        assert!(matches!(
            block_ops(&compiled, body)[0],
            StatementOp::Assignment(_)
        ));
    }

    #[test]
    fn block_frame_advances_with_program_counter() {
        let program = checked_program(
            r#"
            AVInt x = 1
            out x
            "#,
        );
        let compiled = compile_program(&program).unwrap();
        let runtime = Runtime::new(vec![]);
        let mut frame = BlockFrame {
            block_id: compiled.entry,
            pc: compiled.blocks[compiled.entry].start,
            loop_iterations: HashMap::new(),
            loop_limits: HashMap::new(),
        };
        let mut runtime = runtime;
        runtime.program = Some(&compiled);

        assert!(matches!(
            runtime.next_statement_op(&mut frame).unwrap(),
            Some((_, StatementOp::VariableDecl(_)))
        ));
        assert_eq!(frame.pc, compiled.blocks[compiled.entry].start + 1);
        assert!(matches!(
            runtime.next_statement_op(&mut frame).unwrap(),
            Some((_, StatementOp::Out(_)))
        ));
        assert_eq!(frame.pc, compiled.blocks[compiled.entry].start + 2);
        assert_eq!(runtime.next_statement_op(&mut frame).unwrap(), None);
    }

    #[test]
    fn compiled_if_uses_jump_opcodes() {
        let program = checked_program(
            r#"
            AVBool flag = false
            if flag == true {
              out "yes"
            } else {
              out "no"
            }
            "#,
        );
        let compiled = compile_program(&program).unwrap();
        let entry_ops = block_ops(&compiled, compiled.entry);

        assert!(matches!(entry_ops[1], StatementOp::JumpIfFalse { .. }));
        assert!(matches!(entry_ops[2], StatementOp::ExecScopedBlock(_)));
        assert!(matches!(entry_ops[3], StatementOp::Jump { .. }));
        assert!(matches!(entry_ops[4], StatementOp::ExecScopedBlock(_)));

        let StatementOp::JumpIfFalse { target_pc, .. } = entry_ops[1] else {
            unreachable!();
        };
        assert_eq!(target_pc, compiled.blocks[compiled.entry].start + 4);
        let StatementOp::Jump { target_pc } = entry_ops[3] else {
            unreachable!();
        };
        assert_eq!(target_pc, compiled.blocks[compiled.entry].start + 5);
    }

    #[test]
    fn compiled_expressions_are_stack_opcodes() {
        let program = checked_program("AVInt value = (2 + 3) * 4");
        let compiled = compile_program(&program).unwrap();

        let StatementOp::VariableDecl(decl) = &block_ops(&compiled, compiled.entry)[0] else {
            panic!("expected variable declaration");
        };

        assert_eq!(
            decl.initial_value.ops,
            vec![
                ExpressionOp::PushLiteral(Value::Int(2)),
                ExpressionOp::PushLiteral(Value::Int(3)),
                ExpressionOp::Binary(BinaryOperator::Add),
                ExpressionOp::PushLiteral(Value::Int(4)),
                ExpressionOp::Binary(BinaryOperator::Mul),
            ]
        );
    }

    #[test]
    fn runs_compiled_ir_bytes() {
        let program = checked_program(
            r#"
            func add_one(value) {
              return value + 1
            }

            AVInt x = add_one(4)
            out x
            "#,
        );
        let ir = apl_ir::encode_program(&program).unwrap();
        let compiled = compile_ir_bytes(&ir).unwrap();
        let output = run_compiled_program(&compiled, vec![]).unwrap();

        assert_eq!(output.stdout, "5");
    }

    #[test]
    fn source_runtime_keeps_nested_secret_list_values_off_output() {
        let program = apl_parser::parse_program(
            r#"
            List outer = [[1:SASV], "public"]
            out outer
            "#,
        )
        .unwrap();
        validate_program(&program).unwrap_err();
    }

    #[test]
    fn runtime_keeps_dynamically_added_secret_list_values_off_output() {
        let program = Program::new(vec![
            Statement::ListDecl(ListDecl {
                name: "items".to_owned(),
                elements: vec![],
            }),
            Statement::AddToList {
                list_name: "items".to_owned(),
                element: ListElement {
                    value: Expression::Literal(Value::Str("hidden".to_owned())),
                    protection: Some(ProtectionLevel::Asv),
                },
            },
            Statement::Out(Expression::Variable("items".to_owned())),
        ]);

        assert_eq!(
            run_program(&program, vec![]),
            Err(RuntimeError::Message(
                "refusing to output secret value".to_owned()
            ))
        );
    }

    #[test]
    fn runtime_keeps_dynamically_replaced_secret_list_values_off_output() {
        let program = Program::new(vec![
            Statement::ListDecl(ListDecl {
                name: "items".to_owned(),
                elements: vec![ListElement {
                    value: Expression::Literal(Value::Int(1)),
                    protection: None,
                }],
            }),
            Statement::SetListElement {
                list_name: "items".to_owned(),
                index: Expression::Literal(Value::Int(0)),
                element: ListElement {
                    value: Expression::Literal(Value::Str("hidden".to_owned())),
                    protection: Some(ProtectionLevel::Sasv),
                },
            },
            Statement::Out(Expression::Variable("items".to_owned())),
        ]);

        assert_eq!(
            run_program(&program, vec![]),
            Err(RuntimeError::Message(
                "refusing to output secret value".to_owned()
            ))
        );
    }

    #[test]
    fn negative_index_too_far_returns_none() {
        let output = run_source(
            r#"
            List items = ["a", "b"]
            VTime item = items[-3]
            out item
            "#,
        )
        .unwrap();

        assert_eq!(output, "NONE");
    }

    #[test]
    fn stop_inside_function_stops_program() {
        let output = run_source(
            r#"
            func halt() {
              stop "done"
            }

            VTime unused = halt()
            out "after"
            "#,
        )
        .unwrap();

        assert_eq!(output, "done\n");
    }
}
