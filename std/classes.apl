# Class surface lowering for the self-hosted frontend.
# It lowers identity-bearing class syntax to ordinary functions plus the
# table operations exported by libraries/classes.apl.

func classlower.ok(value) {
  return [true, value]
}

func classlower.fail(message, node) {
  return [false, message, node]
}

func classlower.class_name(node) {
  return get(node, 1)
}

func classlower.class_parent(node) {
  return get(node, 2)
}

func classlower.class_exclusions(node) {
  return get(node, 3)
}

func classlower.class_fields(node) {
  return get(node, 4)
}

func classlower.class_methods(node) {
  return get(node, 5)
}

func classlower.find(classes, name) {
  pick(classes): class_node {
    if classlower.class_name(class_node) == name {
      return class_node
    }
  }
  return NONE
}

func classlower.includes_value(values, requested) {
  pick(values): value {
    if value == requested {
      return true
    }
  }
  return false
}

func classlower.own_method(class_node, name) {
  pick(classlower.class_methods(class_node)): entry {
    VTime method = get(entry, 1)
    if get(method, 1) == name {
      return entry
    }
  }
  return NONE
}

func classlower.excludes_method(class_node, name) {
  pick(classlower.class_exclusions(class_node)): exclusion {
    if get(exclusion, 0) == name {
      return true
    }
  }
  return false
}

func classlower.effective_method(classes, class_node, name) {
  VTime own = classlower.own_method(class_node, name)
  if own != NONE {
    return [classlower.class_name(class_node), get(own, 1)]
  }
  if classlower.excludes_method(class_node, name) {
    return NONE
  }
  VTime parent = classlower.class_parent(class_node)
  if parent == NONE {
    return NONE
  }
  VTime parent_node = classlower.find(classes, parent)
  if parent_node == NONE {
    return NONE
  }
  return classlower.effective_method(classes, parent_node, name)
}

func classlower.field_name(field) {
  if parser.node_kind(field) == parser.NODE_DECL {
    return get(field, 2)
  }
  if parser.node_kind(field) == parser.NODE_CLASS_INSTANCE {
    return get(field, 2)
  }
  return get(field, 1)
}

func classlower.field_value(field) {
  if parser.node_kind(field) == parser.NODE_DECL {
    return get(field, 3)
  }
  if parser.node_kind(field) == parser.NODE_CLASS_INSTANCE {
    return get(field, 3)
  }
  return get(field, 2)
}

func classlower.effective_fields(classes, class_node) {
  VTime fields = []
  VTime parent = classlower.class_parent(class_node)
  if parent != NONE {
    VTime parent_node = classlower.find(classes, parent)
    if parent_node != NONE {
      fields = classlower.effective_fields(classes, parent_node)
    }
  }
  pick(classlower.class_fields(class_node)): field {
    add(fields, field)
  }
  return fields
}

func classlower.effective_field(classes, class_node, requested) {
  pick(classlower.effective_fields(classes, class_node)): field {
    if classlower.field_name(field) == requested {
      return field
    }
  }
  return NONE
}

func classlower.is_descendant(classes, child_name, parent_name) {
  VTime current = classlower.find(classes, child_name)
  while (current != NONE) (-1) {
    if classlower.class_name(current) == parent_name {
      return true
    }
    VTime parent = classlower.class_parent(current)
    if parent == NONE {
      return false
    }
    current = classlower.find(classes, parent)
  }
  return false
}

func classlower.ancestry(classes, class_node) {
  VTime result = []
  VTime parent = classlower.class_parent(class_node)
  if parent != NONE {
    VTime parent_node = classlower.find(classes, parent)
    if parent_node != NONE {
      result = classlower.ancestry(classes, parent_node)
    }
  }
  add(result, classlower.class_name(class_node))
  return result
}

func classlower.method_impl_name(class_name, method_name) {
  return join(["aplclass.impl.", class_name, ".", method_name], "")
}

func classlower.constructor_name(class_name) {
  return join(["aplclass.new.", class_name], "")
}

func classlower.dispatch_name(class_name, method_name) {
  return join(["aplclass.dispatch.", class_name, ".", method_name], "")
}

func classlower.setter_name(class_name, field_name) {
  return join(["aplclass.set.", class_name, ".", field_name], "")
}

func classlower.surface_field(field) {
  VTime kind = parser.node_kind(field)
  if kind == parser.NODE_DECL {
    return [kind, get(field, 1), get(field, 2), get(field, 3)]
  }
  if kind == parser.NODE_CLASS_INSTANCE {
    return [kind, get(field, 1), get(field, 2), get(field, 3)]
  }
  return [kind, get(field, 1), get(field, 2)]
}

func classlower.surface(class_node) {
  VTime fields = []
  pick(classlower.class_fields(class_node)): field {
    add(fields, classlower.surface_field(field))
  }

  VTime methods = []
  pick(classlower.class_methods(class_node)): entry {
    VTime method = get(entry, 1)
    add(methods, [get(entry, 0), [parser.NODE_FUNC, get(method, 1), get(method, 2), []]])
  }

  return [
    parser.NODE_CLASS,
    classlower.class_name(class_node),
    classlower.class_parent(class_node),
    classlower.class_exclusions(class_node),
    fields,
    methods
  ]
}

func classlower.surface_is_valid(surface) {
  if len(surface) != 6 {
    return false
  }
  if parser.node_kind(surface) != parser.NODE_CLASS {
    return false
  }
  if verifier.is_str(classlower.class_name(surface)) != true {
    return false
  }
  if contains(classlower.class_name(surface), ".") {
    return false
  }

  VTime parent = classlower.class_parent(surface)
  if parent != NONE {
    if verifier.is_str(parent) != true {
      return false
    }
    if contains(parent, ".") {
      return false
    }
  }

  pick(classlower.class_exclusions(surface)): exclusion {
    if len(exclusion) != 2 {
      return false
    }
    if (verifier.is_str(get(exclusion, 0)) != true) or (verifier.is_str(get(exclusion, 1)) != true) {
      return false
    }
  }

  pick(classlower.class_fields(surface)): field {
    VTime kind = parser.node_kind(field)
    if kind == parser.NODE_DECL {
      if len(field) != 4 {
        return false
      }
      if parser.is_decl_keyword(get(field, 1)) != true {
        return false
      }
    } else if kind == parser.NODE_CLASS_INSTANCE {
      if len(field) != 4 {
        return false
      }
      if verifier.is_str(get(field, 1)) != true {
        return false
      }
    } else if kind == parser.NODE_LIST_DECL {
      if len(field) != 3 {
        return false
      }
    } else {
      return false
    }
    if verifier.is_str(classlower.field_name(field)) != true {
      return false
    }
  }

  pick(classlower.class_methods(surface)): entry {
    if len(entry) != 2 {
      return false
    }
    if verifier.is_bool(get(entry, 0)) != true {
      return false
    }
    VTime method = get(entry, 1)
    if len(method) != 4 {
      return false
    }
    if parser.node_kind(method) != parser.NODE_FUNC {
      return false
    }
    if verifier.is_str(get(method, 1)) != true {
      return false
    }
    pick(get(method, 2)): param {
      if verifier.is_str(param) != true {
        return false
      }
    }
    if len(get(method, 3)) != 0 {
      return false
    }
  }
  return true
}

func classlower.env_get(env, name) {
  VTime index = len(env) - 1
  while (index >= 0) (-1) {
    VTime entry = get(env, index)
    if get(entry, 0) == name {
      return get(entry, 1)
    }
    index -= 1
  }
  return NONE
}

func classlower.env_put(env, name, class_name) {
  VTime result = []
  pick(env): entry {
    if get(entry, 0) != name {
      add(result, entry)
    }
  }
  add(result, [name, class_name])
  return result
}

func classlower.expr_class(expression, env, classes) {
  VTime kind = parser.expr_kind(expression)
  if kind == parser.EXPR_VAR {
    return classlower.env_get(env, get(expression, 1))
  }
  if kind == parser.EXPR_CALL {
    VTime called = get(expression, 1)
    if classlower.find(classes, called) != NONE {
      return called
    }
  }
  return NONE
}

func classlower.rewrite_exprs(expressions, env, classes) {
  VTime result = []
  pick(expressions): expression {
    add(result, classlower.rewrite_expr(expression, env, classes))
  }
  return result
}

func classlower.rewrite_optional_expr(expression, env, classes) {
  if expression == NONE {
    return NONE
  }
  return classlower.rewrite_expr(expression, env, classes)
}

func classlower.rewrite_expr(expression, env, classes) {
  VTime kind = parser.expr_kind(expression)

  if kind == parser.EXPR_VAR {
    VTime name = get(expression, 1)
    VTime parts = split(name, ".")
    if len(parts) == 2 {
      VTime target = get(parts, 0)
      VTime static_class = classlower.env_get(env, target)
      if static_class != NONE {
        if classlower.effective_field(classes, classlower.find(classes, static_class), get(parts, 1)) != NONE {
          return [parser.EXPR_CALL, "classes._get", [[parser.EXPR_VAR, target], [parser.EXPR_STR, get(parts, 1)]]]
        }
      }
    }
    return expression
  }

  if kind == parser.EXPR_CALL {
    VTime name = get(expression, 1)
    VTime args = classlower.rewrite_exprs(get(expression, 2), env, classes)
    if classlower.find(classes, name) != NONE {
      return [parser.EXPR_CALL, classlower.constructor_name(name), args]
    }
    VTime parts = split(name, ".")
    if len(parts) == 2 {
      VTime target = get(parts, 0)
      VTime static_class = classlower.env_get(env, target)
      if static_class != NONE {
        if classlower.effective_method(classes, classlower.find(classes, static_class), get(parts, 1)) != NONE {
          VTime dispatched_args = [[parser.EXPR_VAR, target]]
          pick(args): arg {
            add(dispatched_args, arg)
          }
          return [parser.EXPR_CALL, classlower.dispatch_name(static_class, get(parts, 1)), dispatched_args]
        }
      }
    }
    return [kind, name, args]
  }

  if kind == parser.EXPR_UNARY {
    return [kind, get(expression, 1), classlower.rewrite_expr(get(expression, 2), env, classes)]
  }
  if kind == parser.EXPR_BINARY {
    return [kind, get(expression, 1), classlower.rewrite_expr(get(expression, 2), env, classes), classlower.rewrite_expr(get(expression, 3), env, classes)]
  }
  if kind == parser.EXPR_LIST {
    return [kind, classlower.rewrite_exprs(get(expression, 1), env, classes)]
  }
  if kind == parser.EXPR_INDEX {
    return [kind, classlower.rewrite_expr(get(expression, 1), env, classes), classlower.rewrite_expr(get(expression, 2), env, classes)]
  }
  if kind == parser.EXPR_SLICE {
    return [kind, classlower.rewrite_expr(get(expression, 1), env, classes), classlower.rewrite_optional_expr(get(expression, 2), env, classes), classlower.rewrite_optional_expr(get(expression, 3), env, classes), classlower.rewrite_optional_expr(get(expression, 4), env, classes)]
  }
  if kind == parser.EXPR_TAG {
    return [kind, classlower.rewrite_expr(get(expression, 1), env, classes), get(expression, 2)]
  }
  return expression
}

func classlower.rewrite_block(statements, env, classes) {
  VTime result = []

  pick(statements): statement {
    VTime kind = parser.node_kind(statement)

    if kind == parser.NODE_CLASS_INSTANCE {
      VTime declared_class = get(statement, 1)
      VTime name = get(statement, 2)
      VTime initial = get(statement, 3)
      VTime actual_class = classlower.expr_class(initial, env, classes)
      if actual_class == NONE {
        return [false, join(["class instance `", name, "` must be initialized by a class constructor"], ""), statement]
      }
      if classlower.is_descendant(classes, actual_class, declared_class) != true {
        return [false, join(["class `", actual_class, "` is not compatible with `", declared_class, "`"], ""), statement]
      }
      add(result, [parser.NODE_VTIME_DECL, name, classlower.rewrite_expr(initial, env, classes)])
      env = classlower.env_put(env, name, declared_class)
      continue
    }

    if kind == parser.NODE_VTIME_DECL {
      VTime initial_class = classlower.expr_class(get(statement, 2), env, classes)
      add(result, [kind, get(statement, 1), classlower.rewrite_expr(get(statement, 2), env, classes)])
      if initial_class != NONE {
        env = classlower.env_put(env, get(statement, 1), initial_class)
      }
      continue
    }

    if kind == parser.NODE_ASSIGN {
      VTime name = get(statement, 1)
      VTime parts = split(name, ".")
      if len(parts) == 2 {
        VTime target = get(parts, 0)
        VTime static_class = classlower.env_get(env, target)
        if static_class != NONE {
          VTime field_name = get(parts, 1)
          if classlower.effective_field(classes, classlower.find(classes, static_class), field_name) == NONE {
            return [false, join(["unknown field `", field_name, "` in class `", static_class, "`"], ""), statement]
          }
          VTime replacement = classlower.rewrite_expr(get(statement, 3), env, classes)
          VTime op = get(statement, 2)
          if op != "=" {
            VTime binary_op = op[:len(op) - 1]
            replacement = [parser.EXPR_BINARY, binary_op, [parser.EXPR_CALL, "classes._get", [[parser.EXPR_VAR, target], [parser.EXPR_STR, field_name]]], replacement]
          }
          add(result, [parser.NODE_EXPR, [parser.EXPR_CALL, classlower.setter_name(static_class, field_name), [[parser.EXPR_VAR, target], replacement]]])
          continue
        }
      }
      VTime assigned_class = classlower.expr_class(get(statement, 3), env, classes)
      VTime existing_class = classlower.env_get(env, name)
      if (assigned_class != NONE) and (existing_class != NONE) {
        if classlower.is_descendant(classes, assigned_class, existing_class) != true {
          return [false, join(["class `", assigned_class, "` is not compatible with `", existing_class, "`"], ""), statement]
        }
      }
      add(result, [kind, name, get(statement, 2), classlower.rewrite_expr(get(statement, 3), env, classes)])
      if (assigned_class != NONE) and (existing_class == NONE) {
        env = classlower.env_put(env, name, assigned_class)
      }
      continue
    }

    if kind == parser.NODE_OUT {
      VTime output_class = classlower.expr_class(get(statement, 1), env, classes)
      if output_class != NONE {
        if classlower.effective_method(classes, classlower.find(classes, output_class), "__out__") == NONE {
          return [false, join(["class `", output_class, "` requires __out__ for out"], ""), statement]
        }
        add(result, [parser.NODE_EXPR, [parser.EXPR_CALL, classlower.dispatch_name(output_class, "__out__"), [classlower.rewrite_expr(get(statement, 1), env, classes)]]])
      } else {
        add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes)])
      }
      continue
    }

    if (kind == parser.NODE_DECL) or (kind == parser.NODE_LIST_DECL) {
      VTime value_index = 2
      if kind == parser.NODE_DECL {
        value_index = 3
        add(result, [kind, get(statement, 1), get(statement, 2), classlower.rewrite_expr(get(statement, 3), env, classes)])
      } else {
        add(result, [kind, get(statement, 1), classlower.rewrite_expr(get(statement, 2), env, classes)])
      }
      continue
    }

    if kind == parser.NODE_EXPR {
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes)])
      continue
    }
    if kind == parser.NODE_RETURN {
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes)])
      continue
    }
    if kind == parser.NODE_FAIL {
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes)])
      continue
    }
    if kind == parser.NODE_STOP {
      add(result, [kind, classlower.rewrite_optional_expr(get(statement, 1), env, classes)])
      continue
    }
    if kind == parser.NODE_ERR {
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes), classlower.rewrite_expr(get(statement, 2), env, classes)])
      continue
    }

    if kind == parser.NODE_FUNC {
      VTime body_report = classlower.rewrite_block(get(statement, 3), [], classes)
      if get(body_report, 0) != true {
        return body_report
      }
      add(result, [kind, get(statement, 1), get(statement, 2), get(body_report, 1)])
      continue
    }

    if kind == parser.NODE_IF {
      VTime body_report = classlower.rewrite_block(get(statement, 2), env[:], classes)
      if get(body_report, 0) != true {
        return body_report
      }
      VTime else_report = classlower.rewrite_block(get(statement, 3), env[:], classes)
      if get(else_report, 0) != true {
        return else_report
      }
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes), get(body_report, 1), get(else_report, 1)])
      continue
    }

    if kind == parser.NODE_WHILE {
      VTime body_report = classlower.rewrite_block(get(statement, 3), env[:], classes)
      if get(body_report, 0) != true {
        return body_report
      }
      VTime limit_report = classlower.rewrite_block(get(statement, 4), env[:], classes)
      if get(limit_report, 0) != true {
        return limit_report
      }
      VTime else_report = classlower.rewrite_block(get(statement, 5), env[:], classes)
      if get(else_report, 0) != true {
        return else_report
      }
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes), classlower.rewrite_expr(get(statement, 2), env, classes), get(body_report, 1), get(limit_report, 1), get(else_report, 1)])
      continue
    }

    if kind == parser.NODE_MATCH {
      VTime rewritten_cases = []
      pick(get(statement, 2)): case_entry {
        VTime patterns = []
        pick(get(case_entry, 0)): pattern {
          if get(pattern, 0) == parser.MATCH_RANGE {
            add(patterns, [parser.MATCH_RANGE, classlower.rewrite_expr(get(pattern, 1), env, classes), classlower.rewrite_expr(get(pattern, 2), env, classes)])
          } else {
            add(patterns, [parser.MATCH_VALUE, classlower.rewrite_expr(get(pattern, 1), env, classes)])
          }
        }
        VTime case_report = classlower.rewrite_block(get(case_entry, 1), env[:], classes)
        if get(case_report, 0) != true {
          return case_report
        }
        add(rewritten_cases, [patterns, get(case_report, 1)])
      }
      VTime default_report = classlower.rewrite_block(get(statement, 3), env[:], classes)
      if get(default_report, 0) != true {
        return default_report
      }
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes), rewritten_cases, get(default_report, 1)])
      continue
    }

    if kind == parser.NODE_PICK {
      VTime pick_report = classlower.rewrite_block(get(statement, 3), env[:], classes)
      if get(pick_report, 0) != true {
        return pick_report
      }
      add(result, [kind, classlower.rewrite_expr(get(statement, 1), env, classes), get(statement, 2), get(pick_report, 1)])
      continue
    }

    if kind == parser.NODE_CLASS {
      return [false, "class declarations are only allowed at top level", statement]
    }

    add(result, statement)
  }

  return [true, result, env]
}

func classlower.validate_class(classes, class_node) {
  VTime name = classlower.class_name(class_node)
  VTime parent = classlower.class_parent(class_node)

  if contains(name, ".") {
    return classlower.fail("class name must be a simple identifier", class_node)
  }

  if parent != NONE {
    if parent == name {
      return classlower.fail("class cannot inherit itself", class_node)
    }
    if classlower.find(classes, parent) == NONE {
      return classlower.fail(join(["unknown parent class `", parent, "`"], ""), class_node)
    }
    VTime seen = [name]
    VTime current = parent
    while (current != NONE) (-1) {
      if classlower.includes_value(seen, current) {
        return classlower.fail("class inheritance cycle", class_node)
      }
      add(seen, current)
      VTime current_node = classlower.find(classes, current)
      if current_node == NONE {
        break
      }
      current = classlower.class_parent(current_node)
    }
  }

  VTime init_count = 0
  VTime method_names = []
  pick(classlower.class_methods(class_node)): entry {
    VTime method = get(entry, 1)
    VTime method_name = get(method, 1)
    if contains(method_name, ".") {
      return classlower.fail("method name must be a simple identifier", method)
    }
    if classlower.includes_value(method_names, method_name) {
      return classlower.fail(join(["duplicate method `", method_name, "` in class `", name, "`"], ""), method)
    }
    add(method_names, method_name)
    if method_name == "__init__" {
      init_count += 1
      if get(entry, 0) {
        return classlower.fail("__init__ is class-local and cannot use override", method)
      }
    } else {
      if method_name == "__out__" {
        if len(get(method, 2)) != 0 {
          return classlower.fail("__out__ does not accept parameters", method)
        }
      }
      VTime inherited = NONE
      if parent != NONE {
        inherited = classlower.effective_method(classes, classlower.find(classes, parent), method_name)
      }
      if get(entry, 0) {
        if inherited == NONE {
          return classlower.fail(join(["override method `", method_name, "` has no inherited target"], ""), method)
        }
        if len(get(method, 2)) != len(get(get(inherited, 1), 2)) {
          return classlower.fail(join(["override method `", method_name, "` changes parameter count"], ""), method)
        }
      } else {
        if inherited != NONE {
          return classlower.fail(join(["method `", method_name, "` requires override"], ""), method)
        }
      }
    }
  }

  if init_count != 1 {
    return classlower.fail(join(["class `", name, "` requires exactly one __init__"], ""), class_node)
  }

  VTime field_names = []
  if parent != NONE {
    pick(classlower.effective_fields(classes, classlower.find(classes, parent))): parent_field {
      add(field_names, classlower.field_name(parent_field))
    }
  }
  pick(classlower.class_fields(class_node)): field {
    VTime field_name = classlower.field_name(field)
    if contains(field_name, ".") {
      return classlower.fail("field name must be a simple identifier", field)
    }
    if classlower.includes_value(field_names, field_name) {
      return classlower.fail(join(["duplicate inherited field `", field_name, "` in class `", name, "`"], ""), field)
    }
    add(field_names, field_name)
  }

  VTime excluded_names = []
  pick(classlower.class_exclusions(class_node)): exclusion {
    VTime method_name = get(exclusion, 0)
    VTime source_name = get(exclusion, 1)
    if parent == NONE {
      return classlower.fail("class exclusion requires inheritance", class_node)
    }
    if source_name != parent {
      return classlower.fail("class exclusion source must be the direct parent", class_node)
    }
    if classlower.includes_value(excluded_names, method_name) {
      return classlower.fail(join(["duplicate class exclusion `", method_name, "`"], ""), class_node)
    }
    if classlower.effective_method(classes, classlower.find(classes, parent), method_name) == NONE {
      return classlower.fail(join(["unknown inherited method `", method_name, "`"], ""), class_node)
    }
    if classlower.own_method(class_node, method_name) != NONE {
      return classlower.fail(join(["excluded method `", method_name, "` cannot also be declared"], ""), class_node)
    }
    add(excluded_names, method_name)
  }

  return classlower.ok(class_node)
}

func classlower.method_function(classes, class_node, entry) {
  VTime method = get(entry, 1)
  VTime params = ["self"]
  pick(get(method, 2)): param {
    add(params, param)
  }
  VTime env = [["self", classlower.class_name(class_node)]]
  VTime body_report = classlower.rewrite_block(get(method, 3), env, classes)
  if get(body_report, 0) != true {
    return body_report
  }
  return [true, [parser.NODE_FUNC, classlower.method_impl_name(classlower.class_name(class_node), get(method, 1)), params, get(body_report, 1)]]
}

func classlower.constructor_function(classes, class_node) {
  VTime init_entry = classlower.own_method(class_node, "__init__")
  VTime init_method = get(init_entry, 1)
  VTime params = get(init_method, 2)
  VTime fields = classlower.effective_fields(classes, class_node)
  VTime field_names = []
  VTime field_values = []
  VTime body = []
  VTime field_index = 0
  pick(fields): field {
    add(field_names, [parser.EXPR_STR, classlower.field_name(field)])
    VTime local_name = join(["__apl_field_", classlower.class_name(class_node), "_", str(field_index)], "")
    VTime default_value = classlower.rewrite_expr(classlower.field_value(field), [], classes)
    VTime field_kind = parser.node_kind(field)
    if field_kind == parser.NODE_DECL {
      add(body, [parser.NODE_DECL, get(field, 1), local_name, default_value])
    } else if field_kind == parser.NODE_LIST_DECL {
      add(body, [parser.NODE_LIST_DECL, local_name, default_value])
    } else {
      add(body, [parser.NODE_VTIME_DECL, local_name, default_value])
    }
    add(field_values, [parser.EXPR_VAR, local_name])
    field_index += 1
  }
  VTime ancestry_values = []
  pick(classlower.ancestry(classes, class_node)): ancestor {
    add(ancestry_values, [parser.EXPR_STR, ancestor])
  }
  add(body, [parser.NODE_VTIME_DECL, "self", [parser.EXPR_CALL, "classes._new", [[parser.EXPR_STR, classlower.class_name(class_node)], [parser.EXPR_LIST, ancestry_values], [parser.EXPR_LIST, field_names], [parser.EXPR_LIST, field_values]]]])
  VTime init_args = [[parser.EXPR_VAR, "self"]]
  pick(params): param {
    add(init_args, [parser.EXPR_VAR, param])
  }
  add(body, [parser.NODE_EXPR, [parser.EXPR_CALL, classlower.method_impl_name(classlower.class_name(class_node), "__init__"), init_args]])
  add(body, [parser.NODE_RETURN, [parser.EXPR_VAR, "self"]])
  return [parser.NODE_FUNC, classlower.constructor_name(classlower.class_name(class_node)), params, body]
}

func classlower.setter_function(class_node, field) {
  VTime class_name = classlower.class_name(class_node)
  VTime field_name = classlower.field_name(field)
  VTime checked_name = join(["__apl_checked_", class_name, "_", field_name], "")
  VTime body = []
  VTime field_kind = parser.node_kind(field)
  if field_kind == parser.NODE_DECL {
    add(body, [parser.NODE_DECL, get(field, 1), checked_name, [parser.EXPR_VAR, "value"]])
  } else if field_kind == parser.NODE_LIST_DECL {
    add(body, [parser.NODE_LIST_DECL, checked_name, [parser.EXPR_VAR, "value"]])
  } else {
    add(body, [parser.NODE_VTIME_DECL, checked_name, [parser.EXPR_VAR, "value"]])
  }
  add(body, [parser.NODE_EXPR, [parser.EXPR_CALL, "classes._set", [[parser.EXPR_VAR, "self"], [parser.EXPR_STR, field_name], [parser.EXPR_VAR, checked_name]]]])
  add(body, [parser.NODE_RETURN, [parser.EXPR_VAR, checked_name]])
  return [parser.NODE_FUNC, classlower.setter_name(class_name, field_name), ["self", "value"], body]
}

func classlower.dispatch_body(classes, static_class, method_name, params, index) {
  if index >= len(classes) {
    return [[parser.NODE_FAIL, [parser.EXPR_STR, join(["invalid dynamic class for method `", method_name, "`"], "")]]]
  }

  VTime candidate = get(classes, index)
  VTime rest = classlower.dispatch_body(classes, static_class, method_name, params, index + 1)
  if classlower.is_descendant(classes, classlower.class_name(candidate), static_class) != true {
    return rest
  }
  VTime implementation = classlower.effective_method(classes, candidate, method_name)
  if implementation == NONE {
    return rest
  }

  VTime call_args = [[parser.EXPR_VAR, "self"]]
  pick(params): param {
    add(call_args, [parser.EXPR_VAR, param])
  }
  VTime condition = [parser.EXPR_BINARY, "==", [parser.EXPR_CALL, "classes.type", [[parser.EXPR_VAR, "self"]]], [parser.EXPR_STR, classlower.class_name(candidate)]]
  VTime call = [parser.NODE_RETURN, [parser.EXPR_CALL, classlower.method_impl_name(get(implementation, 0), method_name), call_args]]
  return [[parser.NODE_IF, condition, [call], rest]]
}

func classlower.dispatch_functions(classes, class_node) {
  VTime result = []
  VTime method_names = []
  VTime current = class_node
  while (current != NONE) (-1) {
    pick(classlower.class_methods(current)): entry {
      VTime method = get(entry, 1)
      VTime method_name = get(method, 1)
      if method_name != "__init__" {
        if classlower.excludes_method(class_node, method_name) != true {
          if classlower.includes_value(method_names, method_name) != true {
            VTime effective = classlower.effective_method(classes, class_node, method_name)
            if effective != NONE {
              VTime effective_method = get(effective, 1)
              VTime params = get(effective_method, 2)
              VTime dispatch_params = ["self"]
              pick(params): param {
                add(dispatch_params, param)
              }
              add(result, [parser.NODE_FUNC, classlower.dispatch_name(classlower.class_name(class_node), method_name), dispatch_params, classlower.dispatch_body(classes, classlower.class_name(class_node), method_name, params, 0)])
              add(method_names, method_name)
            }
          }
        }
      }
    }
    VTime parent = classlower.class_parent(current)
    if parent == NONE {
      current = NONE
    } else {
      current = classlower.find(classes, parent)
    }
  }
  return result
}

func classlower.has_local_descendant(classes, local_classes, class_node) {
  pick(local_classes): local_class {
    if classlower.is_descendant(classes, classlower.class_name(local_class), classlower.class_name(class_node)) {
      return true
    }
  }
  return false
}

func classlower.lower(statements, includes, imported_classes) {
  VTime local_classes = []
  VTime ordinary = []

  pick(statements): statement {
    if parser.node_kind(statement) == parser.NODE_CLASS {
      if classlower.find(local_classes, classlower.class_name(statement)) != NONE {
        return classlower.fail(join(["duplicate class `", classlower.class_name(statement), "`"], ""), statement)
      }
      add(local_classes, statement)
    } else {
      add(ordinary, statement)
    }
  }

  VTime classes = []
  pick(imported_classes): imported_class {
    if classlower.surface_is_valid(imported_class) != true {
      return classlower.fail("invalid imported class surface", imported_class)
    }
    if classlower.find(classes, classlower.class_name(imported_class)) != NONE {
      return classlower.fail(join(["duplicate imported class `", classlower.class_name(imported_class), "`"], ""), imported_class)
    }
    add(classes, imported_class)
  }
  pick(local_classes): class_node {
    if classlower.find(classes, classlower.class_name(class_node)) != NONE {
      return classlower.fail(join(["duplicate class `", classlower.class_name(class_node), "`"], ""), class_node)
    }
    add(classes, class_node)
  }

  if len(classes) == 0 {
    return [true, statements, [], []]
  }

  if (len(local_classes) > 0) and (classlower.includes_value(includes, "classes") != true) {
    return classlower.fail("class declarations require include >> classes", get(local_classes, 0))
  }

  pick(local_classes): class_node {
    VTime validation = classlower.validate_class(classes, class_node)
    if get(validation, 0) != true {
      return validation
    }
  }

  VTime generated = []
  VTime dispatch_replacements = []
  pick(imported_classes): imported_class {
    if classlower.has_local_descendant(classes, local_classes, imported_class) {
      pick(classlower.dispatch_functions(classes, imported_class)): dispatch {
        add(generated, dispatch)
        add(dispatch_replacements, get(dispatch, 1))
      }
    }
  }
  pick(local_classes): class_node {
    pick(classlower.class_methods(class_node)): entry {
      VTime method_report = classlower.method_function(classes, class_node, entry)
      if get(method_report, 0) != true {
        return method_report
      }
      add(generated, get(method_report, 1))
    }
    add(generated, classlower.constructor_function(classes, class_node))
    pick(classlower.effective_fields(classes, class_node)): field {
      add(generated, classlower.setter_function(class_node, field))
    }
    pick(classlower.dispatch_functions(classes, class_node)): dispatch {
      add(generated, dispatch)
    }
  }

  VTime rewritten = classlower.rewrite_block(ordinary, [], classes)
  if get(rewritten, 0) != true {
    return rewritten
  }
  pick(get(rewritten, 1)): statement {
    add(generated, statement)
  }

  VTime surfaces = []
  pick(local_classes): class_node {
    add(surfaces, classlower.surface(class_node))
  }
  return [true, generated, surfaces, dispatch_replacements]
}
