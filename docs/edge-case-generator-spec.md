# APL Edge-Case Generator Contract

```text
spec_id: apl-edge-current-2026-11-05
language: APL (Absoluta Programming Language)
purpose: machine input for generation and minimization of compiler/runtime tests
authority: current repository implementation, not the future-language roadmap
```

## 1. Generator Rules

1. Generate one independent `.apl` source per case.
2. Default to executable source. It must contain exactly one
   `AVInt std::main()` unless the case intentionally tests entrypoint errors.
3. Do not generate planned syntax: constructions, lambdas, package
   TOML, threads, containers, or foreign runtimes.
4. `++` and `--` are implemented statement forms and should be generated near
   compound-assignment cases.
5. Prefer one mutation or boundary per case. Minimize every failure.
6. Record exact exit code, stdout, and stderr. `out` does not append newline.
7. Preserve NUL and Unicode as data; do not infer output from terminal display.
8. A crash, hang, panic, security downgrade, host/portable disagreement, or
   accepted malformed program is higher priority than a diagnostic mismatch.

Recommended sidecar record:

```json
{
  "id": "unique_ascii_id",
  "source": "case.apl",
  "mode": "check|run|module",
  "input": ["line 1", "line 2"],
  "expected_phase": "success|lex|parse|check|link|runtime",
  "expected_exit": 0,
  "expected_stdout": "exact bytes represented as JSON string",
  "expected_stderr_contains": "optional stable fragment",
  "seed": 0,
  "known_issue": null
}
```

## 2. Lexical Contract

```ebnf
IDENT       = (ASCII_LETTER | "_") { ASCII_LETTER | DIGIT | "_" | "." } ;
INT         = decimal integer in signed i64 range ;
FLOAT       = DIGIT { DIGIT } "." DIGIT { DIGIT } ;
STRING      = '"' { normal_char | escape } '"' ;
escape      = "\\n" | "\\r" | "\\t" | "\\0" | "\\\\" | "\\\"" ;
comment     = "#" { any character except newline } ;
namespace   = IDENT "::" IDENT ;
range       = ".." ;
```

- Whitespace, including newline, separates tokens but is otherwise not syntax.
- There are no semicolons.
- Identifiers are ASCII-only. String contents may be Unicode.
- A float requires digits on both sides of `.`.
- Unknown escapes and unterminated strings are parse errors.
- `::` is a namespace separator in names and two slice delimiters in `[::]`.
- Reserved words cannot be declaration identifiers.

## 3. Current Grammar

This EBNF is intentionally compact. `expr` details are in section 4.

```ebnf
program       = { top_level } ;
top_level     = composition | declaration | function | struct | class | main ;
attribute     = "@" ("marked" | "collector" | "flee") ;
attributed    = attribute { attribute } (function | main | while_statement) ;

composition   = "include" ">>" IDENT
              | "exclude" STRING "<<" IDENT
              | "using" "namespace" IDENT
              | "nc" IDENT { "," IDENT } ;

main          = "AVInt" "std::main" "(" ")" block ;
function      = "func" IDENT "(" [ IDENT { "," IDENT } ] ")" block ;

class         = "class" SIMPLE_IDENT "{" class_body "}" ;
class_body    = [ inheritance ] { class_exclusion | class_field | class_method } ;
inheritance   = "class::inheritance" STRING ;
class_exclusion = "class::exclude" STRING "method" ">>" SIMPLE_IDENT ;
class_field   = (ABS_TYPE | "VTime" | "List") SIMPLE_IDENT "=" expr ;
class_method  = [ "override" ] "func" SIMPLE_IDENT
                "(" [ SIMPLE_IDENT { "," SIMPLE_IDENT } ] ")" block ;
class_instance = SIMPLE_IDENT SIMPLE_IDENT "=" call ;

struct        = "struct" SIMPLE_IDENT "{" { struct_field } "}" ;
struct_field  = (ABS_TYPE | "List" | SIMPLE_IDENT) SIMPLE_IDENT "=" expr ;
struct_instance = SIMPLE_IDENT SIMPLE_IDENT "=" expr ;

declaration   = ABS_TYPE IDENT "=" expr
              | "VTime" IDENT "=" expr
              | "List" IDENT "=" list_literal ;

block         = "{" { statement } "}" ;
statement     = declaration
              | struct_instance
              | class_instance
              | attributed
              | IDENT ASSIGN_OP expr
              | standard_mutation
              | info_assignment
              | "secretup" "(" IDENT ")"
              | "out" expr
              | "if" expr block { "else" "if" expr block } [ "else" block ]
              | "match" expr "{" { match_case } [ default_case ] "}"
              | while_statement
              | "pick" "(" expr ")" ":" IDENT block
              | "break" | "continue"
              | "return" expr | error_return
              | "stop" [ expr ]
              | "fail" expr
              | IDENT ("++" | "--")
              | call ;

while_statement = "while" "(" expr ")" "(" expr ")" block
                  [ "limit" block ] [ "else" block ] ;

match_case    = "case" match_pattern { "," match_pattern } block ;
default_case  = "case" "default" block ;
match_pattern = expr [ ".." expr ] ;
error_return  = "return" "(" "ERR" "=" "[" expr "]" "[" expr "]" ")" ;

standard_mutation = std_name("add") "(" IDENT "," list_element ")"
                  | std_name("set") "(" IDENT "," expr "," list_element ")" ;
info_assignment   = IDENT "," IDENT "=" "info" "(" IDENT ")" ;

ASSIGN_OP     = "=" | "+=" | "-=" | "*=" | "/=" | "%=" ;
ABS_TYPE      = PROTECTION ("Int" | "Float" | "Bool" | "Str" | "Bytes" | "Json") ;
PROTECTION    = "AV" | "ASV" | "SASV" ;
SIMPLE_IDENT  = (ASCII_LETTER | "_") { ASCII_LETTER | DIGIT | "_" } ;
```

Notes:

- In executable source, statements must be inside `std::main`; top level may
  contain declarations, functions, and composition directives.
- `std::main` takes no parameters and must definitely return public `AVInt`.
- Module source compiled with `emit-module` has no `std::main` requirement.
- Function parameters are scoped `VTime` values. Function results are dynamic.
- A `while` limit is an integer expression evaluated once at loop entry. Its
  resulting value must be `-1` or nonnegative.
- `while else` runs only when the condition is false before the first
  iteration. `while limit` runs only when a finite counter is exhausted while
  the condition remains true. A later false condition and `break` run neither.
  When both are present, `limit` precedes `else` in source.
- `match` evaluates its subject once and never falls through. `..` ranges are
  inclusive. `case default` is optional and must be the final case. Duplicate
  values, empty numeric ranges, and overlapping numeric ranges are rejected.
- `break`/`continue` require an enclosing `while` or `pick` in the same function.
- `return` requires a function.
- `return(ERR=[message][code])` requires a public Str message and public Int
  code, terminates the whole program, and propagates through nested calls.
- Assignment targets are names, not index expressions. Use `std::set` for list
  element replacement.

### Classes

- A program declaring a class must contain `include >> classes`.
- Class names, field names, method names, and parameters are simple identifiers;
  dotted names and namespace-qualified names are rejected in these positions.
- Every class defines exactly one own `__init__`. Construction always invokes
  that initializer and requires exact argument arity.
- Fields are typed and belong to each instance. Assigning a class value copies
  its stable identity handle, so aliases observe the same field mutations.
- Inheritance is single. The parent declaration may appear later in the source,
  but missing parents and inheritance cycles are compile errors.
- Replacing an inherited method requires `override`. An `override` with no
  inherited target or with a different arity is a compile error.
- `class::exclude "name" method >> Parent` removes one inherited method. The
  named source must be the direct parent, and missing/duplicate exclusions fail.
- `out object` requires a zero-argument `__out__`, which may be inherited. The
  hook preserves field protection and cannot expose ASV/SASV data.
- Dispatch is polymorphic: a base-typed variable holding a derived instance
  calls the derived override. `classes::id`, `classes::type`, and
  `classes::is_a` expose the stable identity and ancestry queries.
- Method overloading, multiple inheritance, `super`, access modifiers, and
  static methods are not implemented syntax.

### Structures

- Structure names and field names are simple identifiers.
- Fields must be typed absolute values, `List`, or another known structure.
- `Type()` is the only constructor form and evaluates declared defaults.
- Assignment copies the structure value. Nested field updates rebuild and
  rebind the root; a copy made before the update remains unchanged.
- Direct or indirect by-value cycles are compile errors.
- Structures have no methods, inheritance, identity, `__init__`, or `__out__`.
  Direct `out structure` is a compile error.
- A structure surface may cross `.aplmod`; an untyped function result assigned
  to a structure variable is validated by its runtime tag.

### Raw Memory

- Classify raw-memory cases under `include >> raw_memory`; calls without the
  include and direct `raw_*` intrinsic calls must fail at compile time.
- Public calls are `memory::malloc(size)`, `load(address, size, permissions,
  abi)`, `mark(pointer)`, parameterless `free()`, `where(source, selector)`,
  `read(pointer, offset, count)`, `write(pointer, offset, bytes)`, and
  `call(pointer, arguments)`.
- Descriptors are `["APL_PTR_1", id, address, origin]`. IDs never repeat.
  Bare numbers may be IDs or exact addresses; an address with multiple
  historical identities is ambiguous and rejected.
- `where` selectors 0/1/2/3 return address/ID/source-name/source-value.
- `mark` revokes before `free`; owned records become marked then freed, loaded
  records become unloaded and are never physically released by APL.
- Attributes may stack before functions, main, and while. Their exit order is
  marked, collector, flee. Continue does not close a loop scope; break and a
  return leaving it do. Obvious marked-pointer returns are compile errors.
- Callable loads require `x` plus `[convention, argument_types, result_type]`.
  The current ABI accepts up to six Int arguments and an Int result.

## 4. Expressions And Precedence

Highest precedence is listed last in this table:

```text
or
and
one comparison: == != > < >= <=
+ -
* / %
**                 right-associative
unary - and not
postfix [index] and [start:end:step]
primary
```

Primaries:

```ebnf
primary       = INT | FLOAT | STRING | "true" | "false" | "NONE"
              | "input" | "secret" "input"
              | IDENT | IDENT "=self="
              | call | list_literal | "(" expr ")" ;

call          = (IDENT | IDENT "::" IDENT) "(" [ expr { "," expr } ] ")" ;
list_literal  = "[" [ list_element { "," list_element } ] "]" ;
list_element  = expr [ ":" ("AV" | "ASV" | "SASV") ] ;
postfix       = primary { "[" index_or_slice "]" } ;
index_or_slice = expr
               | [ expr ] ":" [ expr ] [ ":" [ expr ] ] ;
```

- Chained comparisons are not valid.
- Nontrivial operands of `and`/`or` must be parenthesized. Valid example:
  `(x < 4) and (y != 0)`.
- Indexes and slice bounds/steps are integers; negative indexes and negative
  steps are implemented. A zero step is invalid at runtime.
- `**` exponent must be an integer. Base may be Int or Float.
- Integer overflow and integer division/remainder by zero are runtime errors.
- `=self=` is a variable integrity predicate returning Bool.
- Statement-form `name++` and `name--` lower to `name += 1` and `name -= 1`.

## 5. Types And Protection

```text
value types: Int, Float, Bool, Str, Bytes, Json, List
protection order: AV < ASV < SASV
dynamic value: VTime
absence/failure value: NONE
```

- No implicit Int/Float widening in typed assignment.
- A value may flow to equal or higher protection, never lower protection.
- Operations combine protection using the maximum input level.
- Secret output is rejected. This includes secrets carried through `VTime`,
  lists, conversions, function returns, and mutations.
- `List` is declared separately; effective protection derives from elements.
- Tagged elements use `value:AV`, `value:ASV`, or `value:SASV`.
- `secretup(name)` raises eligible absolute-variable protection.
- `secretup` transitions AV -> ASV -> SASV; applying it to SASV or `VTime` is
  rejected.
- `secret input` may directly initialize ASV only. Direct AV and SASV targets
  are rejected by the current checker.
- `=self=` is restricted to AV absolute variables; it is rejected for secret
  absolute variables and `VTime`.
- `type_name, protection_name = info(name)` writes metadata to two `AVStr`
  targets. `info` is a core operation and does not accept `VTime` as source.
- `input` is converted when assigned to a typed declaration. Failed conversion
  produces `NONE` rather than a process crash.

## 6. Standard Functions

In executable source, use `std::name(...)` unless `using namespace std` or a
valid `nc` directive opens `std`. `out`, `info`, `secretup`, operators, and
control-flow keywords are core syntax and never receive `std::`.

```text
std::int(value)       -> Int
std::float(value)     -> Float
std::bool(value)      -> Bool
std::str(value)       -> Str
std::bytes(value)     -> Bytes
std::json(value)      -> Json
std::get(list, int)   -> VTime
std::pop(named_list)  -> VTime; mutates target
std::split(str, str)  -> List
std::join(list, str)  -> Str
std::len(str|bytes|list) -> Int
std::contains(str, str)  -> Bool
std::ord(str)         -> Int
std::char(int)        -> Str
std::pow(int|float, int) -> same numeric base type
std::add(named_list, value[:protection])          statement only
std::set(named_list, int, value[:protection])     statement only
```

Arity is exact. Calls with missing/trailing commas or extra arguments fail.
`pop`, `add`, and `set` require named mutable List/VTime targets where stated.

## 7. Namespaces And Libraries

- Source-facing qualification is `library::symbol`.
- `using namespace X` may occur at most once in a program.
- `nc A, B` opens multiple namespaces only if their complete public export sets
  do not conflict; a conflict is a check/link error.
- `include >> library` selects a named compiled module.
- `exclude "symbol" << library` removes one export from availability/linking.
- `pick` is keyword syntax but is also an excludable standard export; it is
  written without a `std::` prefix.
- CLI source-relative lookup is `<source-directory>/libraries/<name>.aplmod`.
- Source order of composition directives must not alter dependency resolution.
- Module function declarations currently use internal dotted names such as
  `func math.square(x)`; executable calls use `math::square(x)`.

## 8. Repaired Regressions

Authoritative repaired reproductions are under `tests/regressions/fixed/`.

```text
P0 p0_postfix_increment.apl
   x++ / x-- lower to ordinary increment/decrement.

P0 p0_float_to_int.apl
   std::int(3.7) -> 3 by truncation toward zero.

P1 p1_while_variable_limit.apl
   while (condition) (limit_variable) accepts an integer expression.

P1 p1_match_while_branches.apl
   match and cause-specific while limit/else handlers select the right branch.

P1 p1_error_return.apl
   nested ERR termination preserves message and explicit process code.

P3 p3_unqualified_std_main.apl
   using namespace std + AVInt main() canonicalizes to AVInt std::main().
```

Generate nearby boundaries to guard these repaired contracts against relapse.

## 9. Implementation Oracles

Current source ownership:

```text
host parser/executable guard: crates/apl_parser, crates/apl_core
compiler bridge/library resolver: crates/apl_compiler
shared raw-memory table: crates/apl_raw_memory
reference runtime: crates/apl_runtime
self-host lexer/parser/checker/IR/linker/VM: std/*.apl
CLI: crates/apl_cli
Studio shell: crates/apl_studio
```

Default CLI `check` and `run` compile through the portable APL-written pipeline,
with host executable-boundary validation and filesystem/module loading. `check`
performs linked compilation; it is not parser-only.

Canonical commands from repository root:

```powershell
cargo run --release -p apl -- check D:\path\case.apl
cargo run --release -p apl -- run D:\path\case.apl
cargo run --release -p apl -- emit-module D:\path\module.apl D:\path\module.aplmod
```

For `run`, provide input as lines on stdin in declaration/evaluation order.
Use a timeout for loops, compiler hangs, and native processes. Studio is not an
automation oracle. The native backend is partial and should be compared only
for syntax it claims to lower; the portable CLI is the current primary oracle.

## 10. High-Value Generation Axes

- i64 min/max, overflow, zero division/remainder, power boundaries.
- Float lexical boundaries, conversions, negative zero, large magnitudes.
- Empty/Unicode/NUL/escaped strings and malformed escapes.
- Empty/nested/tagged/secret lists; negative indexes; all slice omissions.
- Protection propagation through calls, `VTime`, lists, `get/pop/add/set`.
- Scope shadowing, duplicate names, recursion, early return, escaped flow.
- Match subject side effects, multiple values, inclusive range boundaries,
  optional/default cases, and no fallthrough.
- Loop limits `-2`, `-1`, `0`, `1`; initial false, later false, exhaustion,
  break/continue, and nested handler boundaries.
- ERR message/code type, protection, i32 boundaries, nested calls, and
  unreachable statements after termination.
- Parser delimiter loss, trailing commas, keyword-as-name, `::` versus slices.
- `include`/`exclude`/`using`/`nc` duplicates, missing libraries, export clashes.
- Class constructor arity, alias identity, independent instances, typed and
  secret fields, parent order/cycles, missing/extra `override`, inherited
  exclusion, polymorphic dispatch, and missing or invalid `__out__` hooks.
- Structure defaults, copy independence, nested updates, mismatched values,
  direct/indirect cycles, forbidden methods/output, and `.aplmod` parity.
- Raw-memory zero/negative/overflow sizes and ranges; descriptor forgery;
  stale/double mark; ID/address ambiguity; loaded ownership; permissions and
  ABI arity; nested/combined attributed scopes; break/continue/return cleanup;
  direct and indirect marked escapes; collector roots; portable/native parity.
- Rust guard versus self-host diagnostic/acceptance disagreement.
