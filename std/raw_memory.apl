# Lowering for explicit raw-memory lifetime attributes.

AVInt rawlower.next_id = 0

func rawlower.ok(value) {
  return [true, value]
}

func rawlower.fail(message, node) {
  return [false, message, node]
}

func rawlower.includes(values, requested) {
  pick(values): value {
    if value == requested {
      return true
    }
  }
  return false
}

func rawlower.unique(prefix) {
  VTime id = rawlower.next_id
  rawlower.next_id += 1
  return join([prefix, str(id)], "")
}

func rawlower.policy_expr(attributes) {
  VTime values = []
  pick(attributes): attribute {
    add(values, [parser.EXPR_STR, attribute])
  }
  return [parser.EXPR_LIST, values]
}

func rawlower.call_expr(name, args) {
  return [parser.EXPR_CALL, name, args]
}

func rawlower.call_statement(name, args) {
  return [parser.NODE_EXPR, rawlower.call_expr(name, args)]
}

func rawlower.enter_statement(attributes) {
  return rawlower.call_statement("raw_scope_enter", [rawlower.policy_expr(attributes)])
}

func rawlower.exit_statement() {
  return rawlower.call_statement("raw_scope_exit", [])
}

func rawlower.leave_expr(value) {
  return rawlower.call_expr("raw_scope_leave", [value])
}

func rawlower.append_all(target, values) {
  pick(values): value {
    add(target, value)
  }
  return target
}

func rawlower.with_source(node, original) {
  VTime token_index = parser.node_token_index(original)
  if token_index != NONE {
    add(node, parser.SOURCE_AT)
    add(node, token_index)
  }
  return node
}

func rawlower.block_has_attribute(statements) {
  pick(statements): statement {
    if rawlower.statement_has_attribute(statement) {
      return true
    }
  }
  return false
}

func rawlower.statement_has_attribute(statement) {
  VTime kind = parser.node_kind(statement)
  if kind == parser.NODE_ATTRIBUTED {
    return true
  }
  if kind == parser.NODE_FUNC {
    return rawlower.block_has_attribute(get(statement, 3))
  }
  if kind == parser.NODE_CLASS {
    pick(get(statement, 5)): entry {
      if rawlower.block_has_attribute(get(get(entry, 1), 3)) {
        return true
      }
    }
    return false
  }
  if kind == parser.NODE_IF {
    return (rawlower.block_has_attribute(get(statement, 2))) or (rawlower.block_has_attribute(get(statement, 3)))
  }
  if kind == parser.NODE_WHILE {
    if rawlower.block_has_attribute(get(statement, 3)) {
      return true
    }
    if rawlower.block_has_attribute(get(statement, 4)) {
      return true
    }
    return rawlower.block_has_attribute(get(statement, 5))
  }
  if kind == parser.NODE_MATCH {
    pick(get(statement, 2)): case_entry {
      if rawlower.block_has_attribute(get(case_entry, 1)) {
        return true
      }
    }
    return rawlower.block_has_attribute(get(statement, 3))
  }
  if kind == parser.NODE_PICK {
    return rawlower.block_has_attribute(get(statement, 3))
  }
  return false
}

func rawlower.internal_name(name) {
  VTime qualified = split(name, "::")
  if len(qualified) > 1 {
    return rawlower.internal_name(get(qualified, len(qualified) - 1))
  }
  if name == "raw_malloc" {
    return true
  }
  if name == "raw_load" {
    return true
  }
  if name == "raw_mark" {
    return true
  }
  if name == "raw_free" {
    return true
  }
  if name == "raw_where" {
    return true
  }
  if name == "raw_read" {
    return true
  }
  if name == "raw_write" {
    return true
  }
  if name == "raw_call" {
    return true
  }
  if name == "raw_scope_enter" {
    return true
  }
  if name == "raw_scope_exit" {
    return true
  }
  if name == "raw_scope_leave" {
    return true
  }
  return false
}

func rawlower.reserved_name(name) {
  VTime parts = split(name, ".")
  return get(parts, 0) == "aplraw"
}

func rawlower.expr_has_internal(expression) {
  if expression == NONE {
    return false
  }
  VTime kind = parser.expr_kind(expression)
  if kind == parser.EXPR_CALL {
    if rawlower.internal_name(get(expression, 1)) {
      return true
    }
    pick(get(expression, 2)): arg {
      if rawlower.expr_has_internal(arg) {
        return true
      }
    }
    return false
  }
  if kind == parser.EXPR_UNARY {
    return rawlower.expr_has_internal(get(expression, 2))
  }
  if kind == parser.EXPR_BINARY {
    return (rawlower.expr_has_internal(get(expression, 2))) or (rawlower.expr_has_internal(get(expression, 3)))
  }
  if kind == parser.EXPR_LIST {
    pick(get(expression, 1)): item {
      if rawlower.expr_has_internal(item) {
        return true
      }
    }
    return false
  }
  if kind == parser.EXPR_INDEX {
    return (rawlower.expr_has_internal(get(expression, 1))) or (rawlower.expr_has_internal(get(expression, 2)))
  }
  if kind == parser.EXPR_SLICE {
    if rawlower.expr_has_internal(get(expression, 1)) {
      return true
    }
    if rawlower.expr_has_internal(get(expression, 2)) {
      return true
    }
    if rawlower.expr_has_internal(get(expression, 3)) {
      return true
    }
    return rawlower.expr_has_internal(get(expression, 4))
  }
  if kind == parser.EXPR_TAG {
    return rawlower.expr_has_internal(get(expression, 1))
  }
  return false
}

func rawlower.validate_block(statements, allow_internal) {
  pick(statements): statement {
    VTime state = rawlower.validate_statement(statement, allow_internal)
    if get(state, 0) != true {
      return state
    }
  }
  return rawlower.ok(statements)
}

func rawlower.validate_expr(expression, allow_internal, node) {
  if allow_internal {
    return rawlower.ok(node)
  }
  if rawlower.expr_has_internal(expression) {
    return rawlower.fail("raw-memory runtime intrinsics are not source-callable", node)
  }
  return rawlower.ok(node)
}

func rawlower.validate_statement(statement, allow_internal) {
  VTime kind = parser.node_kind(statement)
  if kind == parser.NODE_ATTRIBUTED {
    return rawlower.validate_statement(get(statement, 2), allow_internal)
  }
  if kind == parser.NODE_FUNC {
    if rawlower.reserved_name(get(statement, 1)) {
      return rawlower.fail("names under aplraw.* are reserved for compiler lowering", statement)
    }
    pick(get(statement, 2)): parameter {
      if rawlower.reserved_name(parameter) {
        return rawlower.fail("names under aplraw.* are reserved for compiler lowering", statement)
      }
    }
    VTime body_allow = get(statement, 1) == "vm.call_builtin"
    return rawlower.validate_block(get(statement, 3), body_allow)
  }
  if kind == parser.NODE_CLASS {
    pick(get(statement, 4)): field {
      VTime field_kind = parser.node_kind(field)
      VTime value_index = 2
      if field_kind == parser.NODE_DECL {
        value_index = 3
      }
      VTime field_state = rawlower.validate_expr(get(field, value_index), false, field)
      if get(field_state, 0) != true {
        return field_state
      }
    }
    pick(get(statement, 5)): entry {
      VTime method = get(entry, 1)
      VTime method_state = rawlower.validate_block(get(method, 3), false)
      if get(method_state, 0) != true {
        return method_state
      }
    }
    return rawlower.ok(statement)
  }
  if kind == parser.NODE_IF {
    VTime state = rawlower.validate_expr(get(statement, 1), allow_internal, statement)
    if get(state, 0) != true {
      return state
    }
    state = rawlower.validate_block(get(statement, 2), allow_internal)
    if get(state, 0) != true {
      return state
    }
    return rawlower.validate_block(get(statement, 3), allow_internal)
  }
  if kind == parser.NODE_WHILE {
    VTime state = rawlower.validate_expr(get(statement, 1), allow_internal, statement)
    if get(state, 0) != true {
      return state
    }
    state = rawlower.validate_expr(get(statement, 2), allow_internal, statement)
    if get(state, 0) != true {
      return state
    }
    state = rawlower.validate_block(get(statement, 3), allow_internal)
    if get(state, 0) != true {
      return state
    }
    state = rawlower.validate_block(get(statement, 4), allow_internal)
    if get(state, 0) != true {
      return state
    }
    return rawlower.validate_block(get(statement, 5), allow_internal)
  }
  if kind == parser.NODE_MATCH {
    VTime state = rawlower.validate_expr(get(statement, 1), allow_internal, statement)
    if get(state, 0) != true {
      return state
    }
    pick(get(statement, 2)): case_entry {
      pick(get(case_entry, 0)): pattern {
        if rawlower.expr_has_internal(get(pattern, 1)) {
          return rawlower.fail("raw-memory runtime intrinsics are not source-callable", statement)
        }
        if len(pattern) > 2 {
          if rawlower.expr_has_internal(get(pattern, 2)) {
            return rawlower.fail("raw-memory runtime intrinsics are not source-callable", statement)
          }
        }
      }
      state = rawlower.validate_block(get(case_entry, 1), allow_internal)
      if get(state, 0) != true {
        return state
      }
    }
    return rawlower.validate_block(get(statement, 3), allow_internal)
  }
  if kind == parser.NODE_PICK {
    VTime state = rawlower.validate_expr(get(statement, 1), allow_internal, statement)
    if get(state, 0) != true {
      return state
    }
    return rawlower.validate_block(get(statement, 3), allow_internal)
  }

  VTime expression = NONE
  if kind == parser.NODE_DECL {
    expression = get(statement, 3)
  }
  if kind == parser.NODE_LIST_DECL {
    expression = get(statement, 2)
  }
  if kind == parser.NODE_VTIME_DECL {
    expression = get(statement, 2)
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    expression = get(statement, 3)
  }
  if kind == parser.NODE_ASSIGN {
    expression = get(statement, 3)
  }
  if kind == parser.NODE_EXPR {
    expression = get(statement, 1)
  }
  if kind == parser.NODE_OUT {
    expression = get(statement, 1)
  }
  if kind == parser.NODE_RETURN {
    expression = get(statement, 1)
  }
  if kind == parser.NODE_FAIL {
    expression = get(statement, 1)
  }
  if kind == parser.NODE_STOP {
    expression = get(statement, 1)
  }
  VTime declared_name = NONE
  if kind == parser.NODE_DECL {
    declared_name = get(statement, 2)
  }
  if kind == parser.NODE_LIST_DECL {
    declared_name = get(statement, 1)
  }
  if kind == parser.NODE_VTIME_DECL {
    declared_name = get(statement, 1)
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    declared_name = get(statement, 2)
  }
  if declared_name != NONE {
    if rawlower.reserved_name(declared_name) {
      return rawlower.fail("names under aplraw.* are reserved for compiler lowering", statement)
    }
  }
  if expression != NONE {
    return rawlower.validate_expr(expression, allow_internal, statement)
  }
  if kind == parser.NODE_ERR {
    if rawlower.expr_has_internal(get(statement, 1)) {
      return rawlower.fail("raw-memory runtime intrinsics are not source-callable", statement)
    }
    if rawlower.expr_has_internal(get(statement, 2)) {
      return rawlower.fail("raw-memory runtime intrinsics are not source-callable", statement)
    }
  }
  return rawlower.ok(statement)
}

func rawlower.inject_return_exit(statements) {
  VTime result = []

  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if kind == parser.NODE_RETURN {
      add(result, rawlower.with_source([parser.NODE_RETURN, rawlower.leave_expr(get(statement, 1))], statement))
      continue
    }

    if kind == parser.NODE_IF {
      add(result, rawlower.with_source([kind, get(statement, 1), rawlower.inject_return_exit(get(statement, 2)), rawlower.inject_return_exit(get(statement, 3))], statement))
      continue
    }

    if kind == parser.NODE_WHILE {
      add(result, rawlower.with_source([kind, get(statement, 1), get(statement, 2), rawlower.inject_return_exit(get(statement, 3)), rawlower.inject_return_exit(get(statement, 4)), rawlower.inject_return_exit(get(statement, 5))], statement))
      continue
    }

    if kind == parser.NODE_MATCH {
      VTime cases = []
      pick(get(statement, 2)): case_entry {
        add(cases, [get(case_entry, 0), rawlower.inject_return_exit(get(case_entry, 1))])
      }
      add(result, rawlower.with_source([kind, get(statement, 1), cases, rawlower.inject_return_exit(get(statement, 3))], statement))
      continue
    }

    if kind == parser.NODE_PICK {
      add(result, rawlower.with_source([kind, get(statement, 1), get(statement, 2), rawlower.inject_return_exit(get(statement, 3))], statement))
      continue
    }

    add(result, statement)
  }

  return result
}

func rawlower.is_pointer_constructor(name) {
  if name == "memory::malloc" {
    return true
  }
  if name == "memory.malloc" {
    return true
  }
  if name == "memory::load" {
    return true
  }
  if name == "memory.load" {
    return true
  }
  if name == "raw_malloc" {
    return true
  }
  return name == "raw_load"
}

func rawlower.expr_is_scoped_pointer(expression, pointers) {
  if expression == NONE {
    return false
  }
  VTime kind = parser.expr_kind(expression)
  if kind == parser.EXPR_VAR {
    return rawlower.includes(pointers, get(expression, 1))
  }
  if kind == parser.EXPR_CALL {
    if rawlower.is_pointer_constructor(get(expression, 1)) {
      return true
    }
    VTime name = get(expression, 1)
    if (name == "memory::where") or (name == "memory.where") {
      VTime args = get(expression, 2)
      if len(args) == 2 {
        VTime selector = get(args, 1)
        if parser.expr_kind(selector) == parser.EXPR_INT {
          if get(selector, 1) == 3 {
            return rawlower.expr_is_scoped_pointer(get(args, 0), pointers)
          }
        }
      }
    }
  }
  return false
}

func rawlower.marked_escape(statements, pointers) {
  pick(statements): statement {
    VTime kind = parser.node_kind(statement)
    if kind == parser.NODE_RETURN {
      if rawlower.expr_is_scoped_pointer(get(statement, 1), pointers) {
        return rawlower.fail("@marked memory cannot escape through return", statement)
      }
      continue
    }

    VTime declared_name = NONE
    VTime value = NONE
    if kind == parser.NODE_VTIME_DECL {
      declared_name = get(statement, 1)
      value = get(statement, 2)
    }
    if kind == parser.NODE_CLASS_INSTANCE {
      declared_name = get(statement, 2)
      value = get(statement, 3)
    }
    if kind == parser.NODE_ASSIGN {
      declared_name = get(statement, 1)
      value = get(statement, 3)
    }
    if declared_name != NONE {
      if rawlower.expr_is_scoped_pointer(value, pointers) {
        if rawlower.includes(pointers, declared_name) != true {
          add(pointers, declared_name)
        }
      }
    }

    if kind == parser.NODE_IF {
      VTime state = rawlower.marked_escape(get(statement, 2), pointers[:])
      if get(state, 0) != true {
        return state
      }
      state = rawlower.marked_escape(get(statement, 3), pointers[:])
      if get(state, 0) != true {
        return state
      }
    }
    if kind == parser.NODE_WHILE {
      VTime state = rawlower.marked_escape(get(statement, 3), pointers[:])
      if get(state, 0) != true {
        return state
      }
      state = rawlower.marked_escape(get(statement, 4), pointers[:])
      if get(state, 0) != true {
        return state
      }
      state = rawlower.marked_escape(get(statement, 5), pointers[:])
      if get(state, 0) != true {
        return state
      }
    }
    if kind == parser.NODE_MATCH {
      pick(get(statement, 2)): case_entry {
        VTime state = rawlower.marked_escape(get(case_entry, 1), pointers[:])
        if get(state, 0) != true {
          return state
        }
      }
      VTime state = rawlower.marked_escape(get(statement, 3), pointers[:])
      if get(state, 0) != true {
        return state
      }
    }
    if kind == parser.NODE_PICK {
      VTime state = rawlower.marked_escape(get(statement, 3), pointers[:])
      if get(state, 0) != true {
        return state
      }
    }
  }
  return rawlower.ok(pointers)
}

func rawlower.lower_ordinary(statement, includes) {
  VTime kind = parser.node_kind(statement)

  if rawlower.statement_has_attribute(statement) != true {
    return rawlower.ok([statement])
  }

  if kind == parser.NODE_FUNC {
    VTime body = rawlower.lower_block(get(statement, 3), includes)
    if get(body, 0) != true {
      return body
    }
    return rawlower.ok([[kind, get(statement, 1), get(statement, 2), get(body, 1)]])
  }

  if kind == parser.NODE_IF {
    VTime body = rawlower.lower_block(get(statement, 2), includes)
    if get(body, 0) != true {
      return body
    }
    VTime else_body = rawlower.lower_block(get(statement, 3), includes)
    if get(else_body, 0) != true {
      return else_body
    }
    return rawlower.ok([[kind, get(statement, 1), get(body, 1), get(else_body, 1)]])
  }

  if kind == parser.NODE_WHILE {
    VTime body = rawlower.lower_block(get(statement, 3), includes)
    if get(body, 0) != true {
      return body
    }
    VTime limit_body = rawlower.lower_block(get(statement, 4), includes)
    if get(limit_body, 0) != true {
      return limit_body
    }
    VTime else_body = rawlower.lower_block(get(statement, 5), includes)
    if get(else_body, 0) != true {
      return else_body
    }
    return rawlower.ok([[kind, get(statement, 1), get(statement, 2), get(body, 1), get(limit_body, 1), get(else_body, 1)]])
  }

  if kind == parser.NODE_MATCH {
    VTime cases = []
    pick(get(statement, 2)): case_entry {
      VTime case_body = rawlower.lower_block(get(case_entry, 1), includes)
      if get(case_body, 0) != true {
        return case_body
      }
      add(cases, [get(case_entry, 0), get(case_body, 1)])
    }
    VTime default_body = rawlower.lower_block(get(statement, 3), includes)
    if get(default_body, 0) != true {
      return default_body
    }
    return rawlower.ok([[kind, get(statement, 1), cases, get(default_body, 1)]])
  }

  if kind == parser.NODE_PICK {
    VTime body = rawlower.lower_block(get(statement, 3), includes)
    if get(body, 0) != true {
      return body
    }
    return rawlower.ok([[kind, get(statement, 1), get(statement, 2), get(body, 1)]])
  }

  if kind == parser.NODE_CLASS {
    VTime methods = []
    pick(get(statement, 5)): entry {
      VTime method = get(entry, 1)
      VTime body = rawlower.lower_block(get(method, 3), includes)
      if get(body, 0) != true {
        return body
      }
      add(methods, [get(entry, 0), [parser.NODE_FUNC, get(method, 1), get(method, 2), get(body, 1)]])
    }
    return rawlower.ok([[kind, get(statement, 1), get(statement, 2), get(statement, 3), get(statement, 4), methods]])
  }

  return rawlower.ok([statement])
}

func rawlower.lower_attributed(attributes, target, includes) {
  if rawlower.includes(includes, "raw_memory") != true {
    return rawlower.fail("memory attributes require include >> raw_memory", target)
  }

  if rawlower.includes(attributes, "marked") {
    VTime target_kind = parser.node_kind(target)
    VTime escape = NONE
    if target_kind == parser.NODE_FUNC {
      escape = rawlower.marked_escape(get(target, 3), [])
    } else {
      escape = rawlower.marked_escape(get(target, 3), [])
    }
    if get(escape, 0) != true {
      return escape
    }
  }

  VTime lowered = rawlower.lower_ordinary(target, includes)
  if get(lowered, 0) != true {
    return lowered
  }
  VTime ordinary = get(get(lowered, 1), 0)
  VTime kind = parser.node_kind(ordinary)

  if kind == parser.NODE_FUNC {
    VTime original_name = get(ordinary, 1)
    VTime inner_name = NONE
    if original_name == "std::main" {
      inner_name = rawlower.unique("aplraw.main.n")
    } else {
      inner_name = rawlower.unique("aplraw.scope.n")
    }
    VTime result_name = rawlower.unique("aplraw.result.n")
    VTime params = get(ordinary, 2)
    VTime call_args = []
    pick(params): param {
      add(call_args, [parser.EXPR_VAR, param])
    }

    VTime wrapper = [rawlower.enter_statement(attributes)]
    VTime call = rawlower.call_expr(inner_name, call_args)
    if original_name == "std::main" {
      add(wrapper, [parser.NODE_DECL, "AVInt", result_name, call])
    } else {
      add(wrapper, [parser.NODE_VTIME_DECL, result_name, call])
    }
    add(wrapper, rawlower.exit_statement())
    add(wrapper, [parser.NODE_RETURN, [parser.EXPR_VAR, result_name]])

    VTime inner = rawlower.with_source([parser.NODE_FUNC, inner_name, params, get(ordinary, 3)], target)
    VTime outer = rawlower.with_source([parser.NODE_FUNC, original_name, params, wrapper], target)
    return rawlower.ok([inner, outer])
  }

  if kind == parser.NODE_WHILE {
    VTime guarded = rawlower.with_source([kind, get(ordinary, 1), get(ordinary, 2), rawlower.inject_return_exit(get(ordinary, 3)), rawlower.inject_return_exit(get(ordinary, 4)), rawlower.inject_return_exit(get(ordinary, 5))], target)
    return rawlower.ok([rawlower.enter_statement(attributes), guarded, rawlower.exit_statement()])
  }

  return rawlower.fail("memory attributes require a function or while", target)
}

func rawlower.lower_block(statements, includes) {
  VTime result = []

  pick(statements): statement {
    VTime lowered = NONE
    if parser.node_kind(statement) == parser.NODE_ATTRIBUTED {
      lowered = rawlower.lower_attributed(get(statement, 1), get(statement, 2), includes)
    } else {
      lowered = rawlower.lower_ordinary(statement, includes)
    }
    if get(lowered, 0) != true {
      return lowered
    }
    result = rawlower.append_all(result, get(lowered, 1))
  }

  return rawlower.ok(result)
}

func rawlower.lower(statements, includes) {
  rawlower.next_id = 0
  VTime validation = rawlower.validate_block(statements, false)
  if get(validation, 0) != true {
    return validation
  }
  if rawlower.block_has_attribute(statements) != true {
    return rawlower.ok(statements)
  }
  return rawlower.lower_block(statements, includes)
}
