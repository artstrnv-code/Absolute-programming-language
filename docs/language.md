# Implemented Language Surface

This is a compact current reference, not a future-language proposal.

## Executables

An executable declares exactly one entrypoint and returns public `AVInt`:

```apl
AVInt std::main() {
  out "hello\n"
  return 0
}
```

Executable statements belong inside `std::main`; declarations and function
bodies may remain at file scope. Modules do not declare `std::main`.

## Variables

Absolute variables combine a protection family and value type, for example
`AVInt`, `ASVStr`, and `SASVBool`. `VTime` is dynamically typed and scoped.

```apl
AVInt count = 4
ASVStr token = "hidden"
VTime value = std::int("42")
```

`input` and `secret input` are expressions. Failed dynamic conversions produce
`NONE`; known impossible typed operations are rejected by the checker. Finite
Float-to-Int conversion truncates toward zero. JSON conversion and typed JSON
input validate the complete value and return `NONE` for malformed JSON.

## Control Flow

Implemented forms include `if`/`else if`/`else`, `match`, bounded `while` with
completion handlers, `pick`, `break`, `continue`, `return`, `stop`, and `fail`.

```apl
while (count < 20) (100) {
  count += 1
} limit {
  out "iteration limit reached\n"
} else {
  out "condition was initially false\n"
}

pick ([1, 2, 3]): current {
  out current
}
```

The second `while` group is an integer expression evaluated once at loop entry;
`-1` is the explicit unbounded form. `else` runs only when the condition is
false before the first iteration. `limit` runs only when a finite counter is
exhausted while the condition is still true. A later false condition and
`break` run neither handler. If both handlers are present, `limit` is written
before `else`.

`match` evaluates its subject once, selects the first matching case, and never
falls through:

```apl
match count {
  case 0, 1 {
    out "small"
  }
  case 2..10 {
    out "bounded"
  }
  case default {
    out "other"
  }
}
```

`..` is an inclusive range. `case default` is optional and must be last.
Duplicate values, empty numeric ranges, and overlapping numeric ranges are
compile errors because they would make all or part of a later pattern
unreachable.
Postfix `value++` and `value--` are equivalent to `value += 1` and
`value -= 1`.

## Program Errors

Functions may terminate the entire program with an explicit message and
process code:

```apl
func abort() {
  return(ERR=["configuration is invalid"][7])
}
```

The message must be a public `Str`; the code must be a public `Int` that fits
the signed 32-bit process-code boundary. `ERR` propagates through nested calls,
cannot be caught, prevents later statements from running, and is preserved by
portable and native executables. A normal function error that callers should
handle remains an ordinary returned value. APL deliberately has no
`try`/`catch`/`except` exception mechanism.

## Functions And Standard Calls

```apl
func twice(value) {
  return value * 2
}
```

Standard callable functions require `std::` in executable source:

```apl
AVInt answer = std::int("42")
```

The standard runtime also provides `std::timer_start()`,
`std::timer_stop()`, and `std::timer_elapsed()`. They use a monotonic clock;
elapsed time is returned as Float seconds. Each runtime instance has one
accumulating timer: starting an active timer or stopping a paused timer is a
runtime error. Pausing and starting again resumes the accumulated duration.

`std::now_utc()` returns a seven-item list in this order: year, month, day,
hour, minute, second, and millisecond. The `datetime` library exposes it as
`datetime::now_utc()`. Its `datetime::now_local()` helper applies the fixed
`datetime.TIMEZONE_OFFSET_MINUTES` constant (zero by default); set that constant
to the desired UTC offset in minutes. This is a fixed offset, not a regional
time zone, and does not apply daylight-saving rules.

The standard runtime provides `std::sha1(Bytes)`, `std::sha256(Bytes)`,
`std::sha512(Bytes)`, and `std::sha3_384(Bytes)`, each returning a digest as
`Bytes` and preserving input protection. SHA-1 is retained for compatibility
and should not be used where collision resistance is required. These hashes
are not password-hashing functions or message-authentication codes.

The `crypto` library wraps these as `crypto::sha1(data)`, `crypto::sha256(data)`,
`crypto::sha512(data)`, and `crypto::sha3_384(data)`, with corresponding
`*_hex` functions returning lowercase hexadecimal strings. The library's current
`crypto::safe_rand_int(min, max)` implementation is temporary and insecure;
it uses the shared monotonic timer and must not be used for security-sensitive
randomness.

`using namespace std` and `nc std, other` open callable lookup when their
namespace constraints pass. Either form permits the shortened entrypoint
`AVInt main()`, which is canonicalized to `AVInt std::main()`.

## Composition

```apl
include >> math
exclude "internal" << math
using namespace std
nc std, math
```

Qualified calls and exported values use `::`, for example
`math::square(6)` and `windows::WM_CLOSE`. `using namespace`
may occur only once. `nc` checks the complete public export sets and rejects
conflicting names before compilation continues.

`exclude` uses the public API name, never the compiler's dotted symbol name:

```apl
include >> math
exclude "sqrt" << math
```

The compiler resolves `sqrt` against the exports of `math`, records its
canonical identity internally, and rejects missing or ambiguous names. An
excluded symbol cannot be called directly or imported through `using
namespace`/`nc`. Reachability linking removes it when no retained library
function depends on it.

## Structures

Structures are copied values with typed fields. They require no support
library and have no identity, inheritance, methods, or output hook:

```apl
struct Point {
  AVInt x = 0
  AVInt y = 0
}

struct Rectangle {
  Point origin = Point()
  List labels = []
}

Rectangle first = Rectangle()
Rectangle second = first
first.origin.x = 10
```

Construction is zero-argument and evaluates every field default. Assignment
copies the complete structure record, so changing `first.origin.x` does not
change `second.origin.x`. A `List` field retains normal APL list value/COW
semantics; the structure copy does not recursively duplicate every container
element.

Fields can be read, replaced, or updated with compound assignment. Nested
field assignment rebuilds the affected value path and rebinds the root.
Direct and indirect by-value cycles are rejected because their default value
would be infinite. Direct `out structure` is also rejected; output individual
fields explicitly. Structure surfaces are exported by `.aplmod`, and values
returned through an untyped function are checked against the structure tag at
runtime before entering a typed structure variable.

## Classes

Classes are identity-bearing references backed by the shipped `classes`
module. A source file using class declarations must include it explicitly:

```apl
include >> classes

class Message {
  AVStr text = ""

  func __init__(text) {
    self.text = text
  }

  func value() {
    return self.text
  }

  func __out__() {
    out self.value()
  }
}
```

Every class declares exactly one `__init__`; construction always calls it.
Fields are typed declarations with per-instance values. Assigning an instance
or placing it in `VTime` copies its stable handle, not its fields, so mutations
through an alias affect the same object. `classes::id(value)` returns the
stable identity, `classes::type(value)` returns the concrete class, and
`classes::is_a(value, name)` checks its ancestry.

An instance declaration names its static class and calls a compatible concrete
constructor:

```apl
Message item = Message("hello")
```

`out item` is valid only when the static class provides `__out__`, directly or
through inheritance. The hook owns all formatting and may emit several output
fragments. Field protection is preserved through the object table, so an
ASV/SASV field cannot be exposed by hiding it behind `__out__`.

Inheritance is single and explicit. Replacing an inherited method requires
`override`; accidental replacement is a compile error. Dispatch uses the
concrete instance type even through a base-class variable:

```apl
class LoudMessage {
  class::inheritance "Message"

  func __init__(text) {
    self.text = text
  }

  override func value() {
    return self.text + "!"
  }
}

Message item = LoudMessage("hello")
```

An inherited method can be removed from the derived class explicitly:

```apl
class::exclude "method_name" method >> Message
```

Class names, field names, and method names are simple identifiers. Method
overloading, multiple inheritance, `super`, access modifiers, and static
methods are not part of the current class contract.

A class may inherit a public class exported by an included `.aplmod`. The
linker expands the imported ancestor's dispatch functions with descendants
declared by later modules or by the application, so inherited methods retain
polymorphic calls to newly introduced overrides. Ordinary function duplicates
remain errors; only compiler-owned `aplclass.dispatch.*` functions participate
in this controlled replacement.

## Raw Memory

Raw memory is an explicit library capability:

```apl
include >> raw_memory

AVInt std::main() {
  VTime pointer = memory::malloc(4)
  memory::write(pointer, 1, std::bytes("AB"))
  out std::str(memory::read(pointer, 0, 4))
  memory::mark(pointer)
  memory::free()
  return 0
}
```

`malloc` returns `["APL_PTR_1", id, address, "user"]`; IDs are stable and
never reused. `where(pointer, 0..3)` returns address, ID, source variable name,
or source value respectively. `mark` revokes access, while parameterless
`free` releases all marked owned allocations. `load(address, size,
permissions, abi)` registers borrowed external memory; it is unloaded but
never freed by APL.

`@marked`, `@collector`, and `@flee` may precede a function, main, or `while`.
They run on scope exit in that fixed order. A marked loop spans all iterations,
including `continue`, and closes on condition/limit completion, `break`, or a
return leaving the loop. See `raw-memory.md` for descriptor, root, range,
permission, and indirect-call details.

## Output And Strings

`out expression` does not append a newline. Use escapes such as `\n`, `\r`,
`\t`, `\0`, `\\`, and `\"` inside strings. Multiple comma-separated output
values are not an implemented form; use multiple `out` statements.

The `console` module provides `con::print(values, sep, end)` and
`con::wait_in(prompt)`. `values` is a list; each item uses the existing `out`
rendering, with `sep` between items and `end` appended after the list.
`wait_in` prints its prompt followed by a newline and returns one input line.
If input is exhausted, it terminates the program with `ERR`, message
`input reached EOF`, and code `1`. The checker rejects secret values passed to
either function because they produce public output.

The `console` module also provides `con::lower(value)`, `con::upper(value)`, and
`con::title(value)`. These preserve spaces; `title` uppercases the first
character after each ASCII space and lowercases the remaining characters in
that word. Case conversion currently covers ASCII, Latin-1, basic Greek, and
basic Cyrillic letters; other code points pass through unchanged.

The module also provides `con::split(value, separator)`, `con::join(values,
separator)`, and `con::contains(value, needle)`. `contains` is the substring
test: it returns whether `needle` occurs in `value`. `split` and `join` use
the corresponding standard string/list operations; splitting with an empty
separator returns `NONE`.

`con::replace(value, target, replacement)` replaces non-overlapping matches
from left to right. An empty `target` leaves `value` unchanged.

## Security

Protection downgrades are rejected across ordinary assignments, containers,
function values, conversions, and mutations. `secretup` raises the protection
of an eligible absolute variable. `info` exposes allowed type/protection
metadata. Security primitives remain core language operations.
