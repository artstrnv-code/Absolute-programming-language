# APL parser bootstrap helpers.
# This module consumes tokens from std/lexer.apl and builds a small AST shape
# in APL lists. It is not the full parser yet; it is the first self-hosted
# parser layer.

AVStr parser.NODE_DECL = "Decl"
AVStr parser.NODE_LIST_DECL = "ListDecl"
AVStr parser.NODE_VTIME_DECL = "VTimeDecl"
AVStr parser.NODE_INFO_ASSIGN = "InfoAssign"
AVStr parser.NODE_ASSIGN = "Assign"
AVStr parser.NODE_EXPR = "Expr"
AVStr parser.NODE_OUT = "Out"
AVStr parser.NODE_STOP = "Stop"
AVStr parser.NODE_FAIL = "Fail"
AVStr parser.NODE_IF = "If"
AVStr parser.NODE_WHILE = "While"
AVStr parser.NODE_MATCH = "Match"
AVStr parser.NODE_ERR = "ErrorReturn"
AVStr parser.NODE_PICK = "Pick"
AVStr parser.NODE_BREAK = "Break"
AVStr parser.NODE_CONTINUE = "Continue"
AVStr parser.NODE_FUNC = "Func"
AVStr parser.NODE_RETURN = "Return"
AVStr parser.NODE_SECRETUP = "SecretUp"
AVStr parser.NODE_INCLUDE = "Include"
AVStr parser.NODE_EXCLUDE = "Exclude"
AVStr parser.NODE_USING = "UsingNamespace"
AVStr parser.NODE_NC = "NoConflicts"
AVStr parser.NODE_CLASS = "Class"
AVStr parser.NODE_STRUCT = "Struct"
AVStr parser.NODE_CLASS_INSTANCE = "ClassInstance"
AVStr parser.NODE_ATTRIBUTED = "Attributed"
AVStr parser.NODE_ERROR = "Error"
AVStr parser.MATCH_VALUE = "Value"
AVStr parser.MATCH_RANGE = "InclusiveRange"
AVStr parser.SOURCE_AT = "SourceAt"
AVBool parser.RECORD_SOURCE_POSITIONS = false

AVStr parser.EXPR_INT = "Int"
AVStr parser.EXPR_FLOAT = "Float"
AVStr parser.EXPR_STR = "Str"
AVStr parser.EXPR_BOOL = "Bool"
AVStr parser.EXPR_NONE = "None"
AVStr parser.EXPR_VAR = "Var"
AVStr parser.EXPR_INPUT = "Input"
AVStr parser.EXPR_SECRET_INPUT = "SecretInput"
AVStr parser.EXPR_UNARY = "Unary"
AVStr parser.EXPR_BINARY = "Binary"
AVStr parser.EXPR_CALL = "Call"
AVStr parser.EXPR_LIST = "List"
AVStr parser.EXPR_INDEX = "Index"
AVStr parser.EXPR_SLICE = "Slice"
AVStr parser.EXPR_SELF = "Self"
AVStr parser.EXPR_TAG = "Tag"
AVStr parser.EXPR_ERROR = "Error"

List parser.DECL_KEYWORDS = ["AVInt", "AVFloat", "AVBool", "AVStr", "AVBytes", "AVJson",
    "ASVInt", "ASVFloat", "ASVBool", "ASVStr", "ASVBytes", "ASVJson",
    "SASVInt", "SASVFloat", "SASVBool", "SASVStr", "SASVBytes", "SASVJson"]

List parser.ASSIGN_OPS = ["=", "+=", "-=", "*=", "/=", "%="]
List parser.BINARY_OPS = ["==", "!=", ">", "<", ">=", "<=", "and", "or", "+", "-", "*", "/", "**", "%"]
List parser.STATEMENT_KEYWORDS = ["include", "exclude", "using", "nc", "class", "struct", "if", "func", "while", "match", "pick", "break", "continue", "return", "secretup", "stop", "fail", "out", "VTime", "List"]

func parser.node_kind(node) {
  return get(node, 0)
}

func parser.node_token_index(node) {
  if len(node) >= 3 {
    if get(node, len(node) - 2) == parser.SOURCE_AT {
      return get(node, len(node) - 1)
    }
  }

  return NONE
}

func parser.node_at(node, token_index) {
  if parser.RECORD_SOURCE_POSITIONS {
    add(node, parser.SOURCE_AT)
    add(node, token_index)
  }

  return node
}

func parser.expr_kind(expression) {
  return get(expression, 0)
}

func parser.expr_value(expression) {
  return get(expression, 1)
}

func parser.includes_value(values, requested) {
  pick(values): value {
    if value == requested {
      return true
    }
  }
  return false
}

func parser.is_symbol_token(token, value) {
  if lexer.token_kind(token) != lexer.TOKEN_SYM {
    return false
  }

  return lexer.token_value(token) == value
}

func parser.is_keyword_token(token, value) {
  if lexer.token_kind(token) != lexer.TOKEN_KW {
    return false
  }

  return lexer.token_value(token) == value
}

func parser.is_identifier_token(token) {
  return lexer.token_kind(token) == lexer.TOKEN_IDENT
}

func parser.is_newline_token(token) {
  return lexer.token_kind(token) == lexer.TOKEN_NEWLINE
}

func parser.skip_newlines(tokens, index) {
  while (index < len(tokens)) (-1) {
    if parser.is_newline_token(get(tokens, index)) == false {
      break
    }
    index += 1
  }
  return index
}

func parser.line_end_index(tokens, index) {
  if parser.is_newline_token(get(tokens, index)) == false {
    return -1
  }
  return parser.skip_newlines(tokens, index)
}

func parser.continuation_index(tokens, index, word) {
  VTime candidate = parser.skip_newlines(tokens, index)
  if parser.is_word_token(get(tokens, candidate), word) {
    return candidate
  }
  return index
}

func parser.is_word_token(token, value) {
  if (lexer.token_kind(token) != lexer.TOKEN_IDENT) and (lexer.token_kind(token) != lexer.TOKEN_KW) {
    return false
  }

  return lexer.token_value(token) == value
}

func parser.is_decl_keyword(value) {
  pick(parser.DECL_KEYWORDS): keyword {
    if keyword == value {
      return true
    }
  }

  return false
}

func parser.next_starts_statement_or_block_end(tokens, index) {
  if index >= len(tokens) {
    return true
  }

  VTime token = get(tokens, index)

  if lexer.token_kind(token) == lexer.TOKEN_EOF {
    return true
  }

  if parser.is_newline_token(token) {
    return true
  }

  if parser.is_symbol_token(token, "}") {
    return true
  }

  if lexer.token_kind(token) != lexer.TOKEN_KW {
    return false
  }

  VTime value = lexer.token_value(token)

  if parser.is_decl_keyword(value) {
    return true
  }

  pick(parser.STATEMENT_KEYWORDS): keyword {
    if value == keyword {
      return true
    }
  }

  return false
}

func parser.is_assign_op(value) {
  pick(parser.ASSIGN_OPS): op {
    if op == value {
      return true
    }
  }

  return false
}

func parser.is_binary_op(value) {
  pick(parser.BINARY_OPS): op {
    if op == value {
      return true
    }
  }

  return false
}

func parser.is_factor_op(value) {
  if value == "*" {
    return true
  }

  if value == "/" {
    return true
  }

  return value == "%"
}

func parser.is_term_op(value) {
  if value == "+" {
    return true
  }

  if value == "-" {
    return true
  }

  return false
}

func parser.is_comparison_op(value) {
  if value == "==" {
    return true
  }

  if value == "!=" {
    return true
  }

  if value == ">" {
    return true
  }

  if value == "<" {
    return true
  }

  if value == ">=" {
    return true
  }

  if value == "<=" {
    return true
  }

  return false
}

func parser.is_factor_token(token) {
  if lexer.token_kind(token) != lexer.TOKEN_SYM {
    return false
  }

  return parser.is_factor_op(lexer.token_value(token))
}

func parser.is_term_token(token) {
  if lexer.token_kind(token) != lexer.TOKEN_SYM {
    return false
  }

  return parser.is_term_op(lexer.token_value(token))
}

func parser.is_comparison_token(token) {
  if lexer.token_kind(token) != lexer.TOKEN_SYM {
    return false
  }

  return parser.is_comparison_op(lexer.token_value(token))
}

func parser.parse_expr_token(token) {
  VTime kind = lexer.token_kind(token)
  VTime value = lexer.token_value(token)

  if kind == lexer.TOKEN_INT {
    return [parser.EXPR_INT, int(value)]
  }

  if kind == lexer.TOKEN_FLOAT {
    return [parser.EXPR_FLOAT, float(value)]
  }

  if kind == lexer.TOKEN_STR {
    return [parser.EXPR_STR, value]
  }

  if kind == lexer.TOKEN_IDENT {
    if (lexer.is_digit(value[0])) and (contains(value, ".")) {
      return [parser.EXPR_FLOAT, NONE]
    }
  }

  if value == "true" {
    return [parser.EXPR_BOOL, true]
  }

  if value == "false" {
    return [parser.EXPR_BOOL, false]
  }

  if value == "NONE" {
    return [parser.EXPR_NONE, NONE]
  }

  return [parser.EXPR_VAR, value]
}

func parser.parse_call_args(tokens, index) {
  VTime args = []

  while (index < len(tokens)) (-1) {
    VTime token = get(tokens, index)

    if lexer.token_kind(token) == lexer.TOKEN_EOF {
      return [args, index, "call expects )"]
    }

    if parser.is_symbol_token(token, ")") {
      return [args, index + 1, NONE]
    }

    VTime parsed_arg = parser.parse_expression(tokens, index)
    VTime arg = get(parsed_arg, 0)

    if parser.expr_kind(arg) == parser.EXPR_ERROR {
      return [args, get(parsed_arg, 1), parser.expr_value(arg)]
    }

    add(args, arg)
    index = get(parsed_arg, 1)

    if index >= len(tokens) {
      return [args, index, "call expects )"]
    }

    VTime separator = get(tokens, index)

    if lexer.token_kind(separator) == lexer.TOKEN_EOF {
      return [args, index, "call expects )"]
    }

    if parser.is_symbol_token(separator, ")") {
      return [args, index + 1, NONE]
    }

    if parser.is_symbol_token(separator, ",") {
      index += 1
      VTime next_token = get(tokens, index)

      if (index >= len(tokens)) or (parser.is_symbol_token(next_token, ")")) {
        return [args, index, "expected expression after ,"]
      }

      continue
    }

    return [args, index, "call expects , or )"]
  }

  return [args, index, "call expects )"]
}

func parser.parse_qualified_name(tokens, index) {
  VTime token = get(tokens, index)

  if parser.is_identifier_token(token) != true {
    return [NONE, index, "expected identifier"]
  }

  VTime name = lexer.token_value(token)
  index += 1

  VTime namespace_width = parser.namespace_width(tokens, index)

  while (namespace_width > 0) (-1) {
    VTime segment = get(tokens, index + namespace_width)
    VTime segment_kind = lexer.token_kind(segment)

    if (segment_kind != lexer.TOKEN_IDENT) and (segment_kind != lexer.TOKEN_KW) {
      return [NONE, index + namespace_width, "expected name after ::"]
    }

    name = join([name, "::", lexer.token_value(segment)], "")
    index += namespace_width + 1
    namespace_width = parser.namespace_width(tokens, index)
  }

  return [name, index, NONE]
}

func parser.namespace_width(tokens, index) {
  if parser.is_symbol_token(get(tokens, index), "::") {
    return 1
  }

  if parser.is_symbol_token(get(tokens, index), ":") {
    if parser.is_symbol_token(get(tokens, index + 1), ":") {
      return 2
    }
  }

  return 0
}

func parser.inside_index_expression(tokens, index) {
  VTime depth = 0
  VTime cursor = index - 1
  while (cursor >= 0) (-1) {
    VTime token = get(tokens, cursor)
    if parser.is_symbol_token(token, "]") {
      depth += 1
    } else if parser.is_symbol_token(token, "[") {
      if depth > 0 {
        depth -= 1
      } else {
        if cursor == 0 {
          return false
        }
        VTime previous = get(tokens, cursor - 1)
        VTime previous_kind = lexer.token_kind(previous)
        if (previous_kind == lexer.TOKEN_IDENT) or (previous_kind == lexer.TOKEN_STR) {
          return true
        }
        if (parser.is_symbol_token(previous, ")")) or (parser.is_symbol_token(previous, "]")) {
          return true
        }
        return false
      }
    }
    cursor -= 1
  }
  return false
}

func parser.tokens_open_std_namespace(tokens) {
  VTime index = 0

  while (index < len(tokens)) (-1) {
    if (parser.is_keyword_token(get(tokens, index), "using")) and (parser.is_keyword_token(get(tokens, index + 1), "namespace")) {
      if (parser.is_identifier_token(get(tokens, index + 2))) and (lexer.token_value(get(tokens, index + 2)) == "std") {
        return true
      }
    }

    if parser.is_keyword_token(get(tokens, index), "nc") {
      VTime namespace_index = index + 1

      while (parser.is_identifier_token(get(tokens, namespace_index))) (-1) {
        if lexer.token_value(get(tokens, namespace_index)) == "std" {
          return true
        }

        namespace_index += 1
        if parser.is_symbol_token(get(tokens, namespace_index), ",") != true {
          break
        }
        namespace_index += 1
      }
    }

    index += 1
  }

  return false
}

func parser.parse_list_items(tokens, index) {
  VTime items = []

  while (index < len(tokens)) (-1) {
    VTime token = get(tokens, index)

    if lexer.token_kind(token) == lexer.TOKEN_EOF {
      return [items, index, "list expects ]"]
    }

    if parser.is_symbol_token(token, "]") {
      return [items, index + 1, NONE]
    }

    VTime parsed_item = parser.parse_expression(tokens, index)
    VTime item = get(parsed_item, 0)

    if parser.expr_kind(item) == parser.EXPR_ERROR {
      return [items, get(parsed_item, 1), parser.expr_value(item)]
    }

    add(items, item)
    index = get(parsed_item, 1)

    if index >= len(tokens) {
      return [items, index, "list expects ]"]
    }

    VTime separator = get(tokens, index)

    if lexer.token_kind(separator) == lexer.TOKEN_EOF {
      return [items, index, "list expects ]"]
    }

    if parser.is_symbol_token(separator, "]") {
      return [items, index + 1, NONE]
    }

    if parser.is_symbol_token(separator, ",") {
      index += 1
      VTime next_token = get(tokens, index)

      if (index >= len(tokens)) or (parser.is_symbol_token(next_token, "]")) {
        return [items, index, "expected expression after ,"]
      }

      continue
    }

    return [items, index, "list expects , or ]"]
  }

  return [items, index, "list expects ]"]
}

func parser.parse_expr_primary(tokens, index) {
  if index >= len(tokens) {
    return [parser.expr_error(tokens, "expected expression", index), index]
  }

  VTime token = get(tokens, index)
  VTime kind = lexer.token_kind(token)
  VTime value = lexer.token_value(token)
  VTime next_token = get(tokens, index + 1)

  if kind == lexer.TOKEN_EOF {
    return [parser.expr_error(tokens, "expected expression", index), index]
  }

  if kind == lexer.TOKEN_ERROR {
    return [parser.expr_error(tokens, value, index), index + 1]
  }

  if (kind == lexer.TOKEN_SYM) and (value == "[") {
    VTime parsed_items = parser.parse_list_items(tokens, index + 1)

    if get(parsed_items, 2) != NONE {
      return [parser.expr_error(tokens, get(parsed_items, 2), get(parsed_items, 1)), get(parsed_items, 1)]
    }

    return [[parser.EXPR_LIST, get(parsed_items, 0)], get(parsed_items, 1)]
  }

  if (kind == lexer.TOKEN_KW) and (value == "secret") {
    if parser.is_keyword_token(next_token, "input") {
      return [[parser.EXPR_SECRET_INPUT, NONE], index + 2]
    }
  }

  if (kind == lexer.TOKEN_KW) and (value == "input") {
    return [[parser.EXPR_INPUT, NONE], index + 1]
  }

  if (kind == lexer.TOKEN_SYM) and (value == "(") {
    VTime parsed_group = parser.parse_expression(tokens, index + 1)
    VTime close_index = get(parsed_group, 1)
    VTime close_token = get(tokens, close_index)

    if parser.is_symbol_token(close_token, ")") {
      return [get(parsed_group, 0), close_index + 1]
    }

    return [parser.expr_error(tokens, "group expects )", close_index), close_index]
  }

  if kind == lexer.TOKEN_IDENT {
    VTime qualified = parser.parse_qualified_name(tokens, index)
    VTime name_end = get(qualified, 1)

    if get(qualified, 2) != NONE {
      return [parser.expr_error(tokens, get(qualified, 2), name_end), name_end]
    }

    if parser.is_symbol_token(get(tokens, name_end), "(") {
      VTime parsed_args = parser.parse_call_args(tokens, name_end + 1)

      if get(parsed_args, 2) != NONE {
        return [parser.expr_error(tokens, get(parsed_args, 2), get(parsed_args, 1)), get(parsed_args, 1)]
      }

      return [[parser.EXPR_CALL, get(qualified, 0), get(parsed_args, 0)], get(parsed_args, 1)]
    }

    if name_end > index + 1 {
      if parser.inside_index_expression(tokens, index) != true {
        return [[parser.EXPR_VAR, join(split(get(qualified, 0), "::"), ".")], name_end]
      }
    }
  }

  if kind == lexer.TOKEN_SYM {
    return [parser.expr_error(tokens, "expected expression", index), index + 1]
  }

  return [parser.parse_expr_token(token), index + 1]
}

func parser.parse_index_or_slice(tokens, target, index) {
  VTime token = get(tokens, index)

  if parser.is_symbol_token(token, ":") {
    VTime end = NONE
    VTime step = NONE
    index += 1
    token = get(tokens, index)

    if (parser.is_symbol_token(token, ":") == false) and (parser.is_symbol_token(token, "]") == false) {
      VTime parsed_end = parser.parse_expression(tokens, index)
      end = get(parsed_end, 0)
      index = get(parsed_end, 1)
      token = get(tokens, index)
    }

    if parser.is_symbol_token(token, ":") {
      index += 1
      token = get(tokens, index)

      if parser.is_symbol_token(token, "]") == false {
        VTime parsed_step = parser.parse_expression(tokens, index)
        step = get(parsed_step, 0)
        index = get(parsed_step, 1)
      }
    }

    token = get(tokens, index)
    if parser.is_symbol_token(token, "]") == false {
      return [parser.expr_error(tokens, "slice expects ]", index), index]
    }

    return [[parser.EXPR_SLICE, target, NONE, end, step], index + 1]
  }

  VTime parsed_first = parser.parse_expression(tokens, index)
  VTime first = get(parsed_first, 0)

  if parser.expr_kind(first) == parser.EXPR_ERROR {
    return parsed_first
  }

  index = get(parsed_first, 1)
  token = get(tokens, index)

  if parser.is_symbol_token(token, ":") == false {
    if parser.is_symbol_token(token, "]") == false {
      return [parser.expr_error(tokens, "index expects ]", index), index]
    }

    return [[parser.EXPR_INDEX, target, first], index + 1]
  }

  VTime end = NONE
  VTime step = NONE
  index += 1
  token = get(tokens, index)

  if (parser.is_symbol_token(token, ":") == false) and (parser.is_symbol_token(token, "]") == false) {
    VTime parsed_end = parser.parse_expression(tokens, index)
    end = get(parsed_end, 0)
    index = get(parsed_end, 1)
    token = get(tokens, index)
  }

  if parser.is_symbol_token(token, ":") {
    index += 1
    token = get(tokens, index)

    if parser.is_symbol_token(token, "]") == false {
      VTime parsed_step = parser.parse_expression(tokens, index)
      step = get(parsed_step, 0)
      index = get(parsed_step, 1)
    }
  }

  token = get(tokens, index)
  if parser.is_symbol_token(token, "]") == false {
    return [parser.expr_error(tokens, "slice expects ]", index), index]
  }

  return [[parser.EXPR_SLICE, target, first, end, step], index + 1]
}

func parser.parse_expr_postfix(tokens, index) {
  VTime parsed = parser.parse_expr_primary(tokens, index)
  VTime expression = get(parsed, 0)
  index = get(parsed, 1)

  while (index < len(tokens)) (-1) {
    VTime token = get(tokens, index)

    if lexer.token_kind(token) != lexer.TOKEN_SYM {
      return [expression, index]
    }

    if lexer.token_value(token) != "[" {
      return [expression, index]
    }

    parsed = parser.parse_index_or_slice(tokens, expression, index + 1)
    expression = get(parsed, 0)
    index = get(parsed, 1)
  }

  return [expression, index]
}

func parser.parse_expr_unary(tokens, index) {
  VTime token = get(tokens, index)
  VTime kind = lexer.token_kind(token)
  VTime op = lexer.token_value(token)

  if (kind == lexer.TOKEN_SYM) and (op == "-") {
    VTime next_token = get(tokens, index + 1)
    VTime next_kind = lexer.token_kind(next_token)

    if next_kind == lexer.TOKEN_INT {
      VTime signed_value = join(["-", lexer.token_value(next_token)], "")
      return [[parser.EXPR_INT, int(signed_value)], index + 2]
    }

    if next_kind == lexer.TOKEN_FLOAT {
      VTime signed_value = join(["-", lexer.token_value(next_token)], "")
      return [[parser.EXPR_FLOAT, float(signed_value)], index + 2]
    }

    VTime parsed_value = parser.parse_expr_unary(tokens, index + 1)
    return [[parser.EXPR_UNARY, op, get(parsed_value, 0)], get(parsed_value, 1)]
  }

  if (kind == lexer.TOKEN_KW) and (op == "not") {
    VTime parsed_value = parser.parse_expr_unary(tokens, index + 1)
    return [[parser.EXPR_UNARY, op, get(parsed_value, 0)], get(parsed_value, 1)]
  }

  return parser.parse_expr_postfix(tokens, index)
}

func parser.parse_expr_self(tokens, index) {
  VTime parsed_left = parser.parse_expr_unary(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime op_token = get(tokens, next_index)
  VTime op = lexer.token_value(op_token)

  if parser.is_symbol_token(op_token, "=self=") {
    if parser.expr_kind(left) == parser.EXPR_VAR {
      return [[parser.EXPR_SELF, parser.expr_value(left)], next_index + 1]
    }
  }

  return [left, next_index]
}

func parser.parse_expr_factor(tokens, index) {
  VTime parsed_left = parser.parse_expr_power(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)
  VTime op = lexer.token_value(token)

  while (parser.is_factor_token(token)) (-1) {
    VTime parsed_right = parser.parse_expr_power(tokens, next_index + 1)
    left = [parser.EXPR_BINARY, op, left, get(parsed_right, 0)]
    next_index = get(parsed_right, 1)
    token = get(tokens, next_index)
    op = lexer.token_value(token)
  }

  return [left, next_index]
}

func parser.parse_expr_power(tokens, index) {
  VTime parsed_left = parser.parse_expr_self(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)

  if parser.is_symbol_token(token, "**") {
    VTime parsed_right = parser.parse_expr_power(tokens, next_index + 1)
    return [[parser.EXPR_BINARY, "**", left, get(parsed_right, 0)], get(parsed_right, 1)]
  }

  return [left, next_index]
}

func parser.parse_expr_term(tokens, index) {
  VTime parsed_left = parser.parse_expr_factor(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)
  VTime op = lexer.token_value(token)

  while (parser.is_term_token(token)) (-1) {
    VTime parsed_right = parser.parse_expr_factor(tokens, next_index + 1)
    left = [parser.EXPR_BINARY, op, left, get(parsed_right, 0)]
    next_index = get(parsed_right, 1)
    token = get(tokens, next_index)
    op = lexer.token_value(token)
  }

  return [left, next_index]
}

func parser.parse_expr_comparison(tokens, index) {
  VTime parsed_left = parser.parse_expr_term(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)
  VTime op = lexer.token_value(token)

  if parser.is_comparison_token(token) {
    VTime parsed_right = parser.parse_expr_term(tokens, next_index + 1)
    return [[parser.EXPR_BINARY, op, left, get(parsed_right, 0)], get(parsed_right, 1)]
  }

  return [left, next_index]
}

func parser.parse_expr_and(tokens, index) {
  VTime parsed_left = parser.parse_expr_comparison(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)
  VTime op = lexer.token_value(token)

  while (parser.is_keyword_token(token, "and")) (-1) {
    VTime parsed_right = parser.parse_expr_comparison(tokens, next_index + 1)
    left = [parser.EXPR_BINARY, op, left, get(parsed_right, 0)]
    next_index = get(parsed_right, 1)
    token = get(tokens, next_index)
    op = lexer.token_value(token)
  }

  return [left, next_index]
}

func parser.parse_expr_or(tokens, index) {
  VTime parsed_left = parser.parse_expr_and(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime token = get(tokens, next_index)
  VTime op = lexer.token_value(token)

  while (parser.is_keyword_token(token, "or")) (-1) {
    VTime parsed_right = parser.parse_expr_and(tokens, next_index + 1)
    left = [parser.EXPR_BINARY, op, left, get(parsed_right, 0)]
    next_index = get(parsed_right, 1)
    token = get(tokens, next_index)
    op = lexer.token_value(token)
  }

  return [left, next_index]
}

func parser.parse_expression(tokens, index) {
  VTime parsed_left = parser.parse_expr_or(tokens, index)
  VTime left = get(parsed_left, 0)
  VTime next_index = get(parsed_left, 1)
  VTime op_token = get(tokens, next_index)
  VTime op = lexer.token_value(op_token)

  if parser.is_symbol_token(op_token, ":") {
    VTime tag_token = get(tokens, next_index + 1)
    VTime tag = lexer.token_value(tag_token)

    if parser.is_keyword_token(tag_token, "AV") {
      return [[parser.EXPR_TAG, left, tag], next_index + 2]
    }

    if parser.is_keyword_token(tag_token, "ASV") {
      return [[parser.EXPR_TAG, left, tag], next_index + 2]
    }

    if parser.is_keyword_token(tag_token, "SASV") {
      return [[parser.EXPR_TAG, left, tag], next_index + 2]
    }
  }

  return [left, next_index]
}

func parser.source_diagnostic(source, token_index, category, message) {
  VTime offsets = get(lexer.tokenize_with_offsets(source), 1)
  VTime offset = len(source)

  if token_index < len(offsets) {
    offset = get(offsets, token_index)
  }

  VTime position = lexer.position_at(source, offset)
  return join([category, " at ", str(get(position, 0)), ":", str(get(position, 1)), ": ", message], "")
}

func parser.diagnostic(tokens, index, message) {
  if len(tokens) > 0 {
    VTime context = get(tokens, len(tokens) - 1)

    if lexer.token_kind(context) == lexer.TOKEN_EOF {
      return parser.source_diagnostic(lexer.token_value(context), index, "parse error", message)
    }
  }

  return message
}

func parser.expr_error(tokens, message, index) {
  return [parser.EXPR_ERROR, parser.diagnostic(tokens, index, message)]
}

func parser.error(tokens, message, index) {
  return [[parser.NODE_ERROR, parser.diagnostic(tokens, index, message)], index]
}

func parser.error_at(tokens, message, error_index, next_index) {
  return [[parser.NODE_ERROR, parser.diagnostic(tokens, error_index, message)], next_index]
}

func parser.parse_statement(tokens, index) {
  if index >= len(tokens) {
    return parser.error(tokens, "unexpected end", index)
  }

  VTime token = get(tokens, index)
  VTime kind = lexer.token_kind(token)
  VTime value = lexer.token_value(token)

  if kind == lexer.TOKEN_ERROR {
    return parser.error_at(tokens, value, index, index + 1)
  }

  if parser.is_symbol_token(token, "@") {
    VTime attributes = []
    VTime attribute_index = index

    while (parser.is_symbol_token(get(tokens, attribute_index), "@")) (-1) {
      VTime name_token = get(tokens, attribute_index + 1)
      if parser.is_identifier_token(name_token) != true {
        return parser.error(tokens, "memory attribute expects identifier", attribute_index + 1)
      }
      VTime name = lexer.token_value(name_token)
      if ((name != "marked") and (name != "collector")) and (name != "flee") {
        return parser.error(tokens, join(["unknown memory attribute `", name, "`"], ""), attribute_index + 1)
      }
      if parser.includes_value(attributes, name) {
        return parser.error(tokens, join(["duplicate memory attribute `", name, "`"], ""), attribute_index + 1)
      }
      add(attributes, name)
      attribute_index += 2
      attribute_index = parser.skip_newlines(tokens, attribute_index)
    }

    VTime parsed_target = parser.parse_statement(tokens, attribute_index)
    VTime target = get(parsed_target, 0)
    VTime target_kind = parser.node_kind(target)
    if (target_kind != parser.NODE_FUNC) and (target_kind != parser.NODE_WHILE) {
      return parser.error(tokens, "memory attributes require a function or while", attribute_index)
    }
    return [parser.node_at([parser.NODE_ATTRIBUTED, attributes, target], index), get(parsed_target, 1)]
  }

  if parser.is_keyword_token(token, "include") {
    VTime arrow_token = get(tokens, index + 1)
    VTime library_token = get(tokens, index + 2)

    if parser.is_symbol_token(arrow_token, ">>") != true {
      return parser.error(tokens, "include expects >>", index + 1)
    }

    if parser.is_identifier_token(library_token) != true {
      return parser.error(tokens, "include expects library name", index + 2)
    }

    return [parser.node_at([parser.NODE_INCLUDE, lexer.token_value(library_token)], index), index + 3]
  }

  if parser.is_keyword_token(token, "exclude") {
    VTime symbol_token = get(tokens, index + 1)
    VTime arrow_token = get(tokens, index + 2)
    VTime library_token = get(tokens, index + 3)

    if lexer.token_kind(symbol_token) != lexer.TOKEN_STR {
      return parser.error(tokens, "exclude expects string literal", index + 1)
    }

    if parser.is_symbol_token(arrow_token, "<<") != true {
      return parser.error(tokens, "exclude expects <<", index + 2)
    }

    if parser.is_identifier_token(library_token) != true {
      return parser.error(tokens, "exclude expects library name", index + 3)
    }

    return [parser.node_at([parser.NODE_EXCLUDE, lexer.token_value(symbol_token), lexer.token_value(library_token)], index), index + 4]
  }

  if parser.is_keyword_token(token, "using") {
    VTime namespace_token = get(tokens, index + 1)
    VTime library_token = get(tokens, index + 2)

    if parser.is_keyword_token(namespace_token, "namespace") != true {
      return parser.error(tokens, "using expects namespace", index + 1)
    }

    if parser.is_identifier_token(library_token) != true {
      return parser.error(tokens, "using namespace expects library name", index + 2)
    }

    return [parser.node_at([parser.NODE_USING, lexer.token_value(library_token)], index), index + 3]
  }

  if parser.is_keyword_token(token, "nc") {
    VTime namespaces = []
    VTime namespace_index = index + 1

    while (namespace_index < len(tokens)) (-1) {
      VTime namespace_token = get(tokens, namespace_index)

      if parser.is_identifier_token(namespace_token) != true {
        if len(namespaces) == 0 {
          return parser.error(tokens, "nc expects library name", namespace_index)
        }

        break
      }

      add(namespaces, lexer.token_value(namespace_token))
      namespace_index += 1

      if parser.is_symbol_token(get(tokens, namespace_index), ",") != true {
        break
      }

      namespace_index += 1
    }

    return [parser.node_at([parser.NODE_NC, namespaces], index), namespace_index]
  }

  if parser.is_keyword_token(token, "struct") {
    VTime name_token = get(tokens, index + 1)
    VTime open_token = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) != true {
      return parser.error(tokens, "struct expects identifier", index + 1)
    }
    if parser.is_symbol_token(open_token, "{") != true {
      return parser.error(tokens, "struct expects {", index + 2)
    }

    VTime fields = []
    VTime struct_index = index + 3
    if parser.is_newline_token(get(tokens, struct_index)) == false {
      return parser.error(tokens, "struct body must start on a new line", struct_index)
    }
    struct_index = parser.skip_newlines(tokens, struct_index)

    while (struct_index < len(tokens)) (-1) {
      if parser.is_symbol_token(get(tokens, struct_index), "}") {
        return [parser.node_at([parser.NODE_STRUCT, lexer.token_value(name_token), fields], index), struct_index + 1]
      }

      VTime parsed_field = parser.parse_statement(tokens, struct_index)
      VTime field = get(parsed_field, 0)
      VTime field_kind = parser.node_kind(field)
      if field_kind == parser.NODE_ERROR {
        return parsed_field
      }
      if ((field_kind != parser.NODE_DECL) and (field_kind != parser.NODE_LIST_DECL)) and (field_kind != parser.NODE_CLASS_INSTANCE) {
        return parser.error(tokens, "struct body expects a typed field", struct_index)
      }
      add(fields, field)
      struct_index = get(parsed_field, 1)
      VTime field_end = parser.line_end_index(tokens, struct_index)
      if field_end < 0 {
        return parser.error(tokens, "expected end of line after struct field", struct_index)
      }
      struct_index = field_end
    }

    return parser.error(tokens, "struct expects }", struct_index)
  }

  if parser.is_keyword_token(token, "class") {
    VTime name_token = get(tokens, index + 1)
    VTime open_token = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) != true {
      return parser.error(tokens, "class expects identifier", index + 1)
    }

    if parser.is_symbol_token(open_token, "{") != true {
      return parser.error(tokens, "class expects {", index + 2)
    }

    VTime parent = NONE
    VTime exclusions = []
    VTime fields = []
    VTime methods = []
    VTime class_index = index + 3

    if parser.is_newline_token(get(tokens, class_index)) == false {
      return parser.error(tokens, "class body must start on a new line", class_index)
    }
    class_index = parser.skip_newlines(tokens, class_index)

    while (class_index < len(tokens)) (-1) {
      VTime class_token = get(tokens, class_index)

      if parser.is_symbol_token(class_token, "}") {
        return [parser.node_at([parser.NODE_CLASS, lexer.token_value(name_token), parent, exclusions, fields, methods], index), class_index + 1]
      }

      if parser.is_keyword_token(class_token, "class") {
        if (parser.is_symbol_token(get(tokens, class_index + 1), ":") != true) or (parser.is_symbol_token(get(tokens, class_index + 2), ":") != true) {
          return parser.error(tokens, "class directive expects ::", class_index + 1)
        }

        VTime directive_token = get(tokens, class_index + 3)

        if parser.is_word_token(directive_token, "inheritance") {
          VTime parent_token = get(tokens, class_index + 4)
          if lexer.token_kind(parent_token) != lexer.TOKEN_STR {
            return parser.error(tokens, "class inheritance expects string literal", class_index + 4)
          }
          if parent != NONE {
            return parser.error(tokens, "class allows only one inheritance directive", class_index)
          }
          parent = lexer.token_value(parent_token)
          class_index += 5
          VTime inheritance_end = parser.line_end_index(tokens, class_index)
          if inheritance_end < 0 {
            return parser.error(tokens, "expected end of line after class directive", class_index)
          }
          class_index = inheritance_end
          continue
        }

        if parser.is_keyword_token(directive_token, "exclude") {
          VTime symbol_token = get(tokens, class_index + 4)
          VTime method_token = get(tokens, class_index + 5)
          VTime arrow_token = get(tokens, class_index + 6)
          VTime source_token = get(tokens, class_index + 7)

          if lexer.token_kind(symbol_token) != lexer.TOKEN_STR {
            return parser.error(tokens, "class exclusion expects string literal", class_index + 4)
          }
          if parser.is_word_token(method_token, "method") != true {
            return parser.error(tokens, "class exclusion expects method", class_index + 5)
          }
          if parser.is_symbol_token(arrow_token, ">>") != true {
            return parser.error(tokens, "class exclusion expects >>", class_index + 6)
          }
          if parser.is_identifier_token(source_token) != true {
            return parser.error(tokens, "class exclusion expects parent class", class_index + 7)
          }

          add(exclusions, [lexer.token_value(symbol_token), lexer.token_value(source_token)])
          class_index += 8
          VTime exclusion_end = parser.line_end_index(tokens, class_index)
          if exclusion_end < 0 {
            return parser.error(tokens, "expected end of line after class directive", class_index)
          }
          class_index = exclusion_end
          continue
        }

        return parser.error(tokens, "unknown class directive", class_index + 3)
      }

      VTime is_override = false
      VTime member_index = class_index
      if parser.is_keyword_token(class_token, "override") {
        is_override = true
        member_index += 1
        class_token = get(tokens, member_index)
      }

      VTime parsed_member = parser.parse_statement(tokens, member_index)
      VTime member = get(parsed_member, 0)
      VTime member_kind = parser.node_kind(member)

      if member_kind == parser.NODE_ERROR {
        return parsed_member
      }

      if member_kind == parser.NODE_FUNC {
        add(methods, [is_override, member])
      } else {
        if is_override {
          return parser.error(tokens, "override expects func", member_index)
        }
        if ((member_kind != parser.NODE_DECL) and (member_kind != parser.NODE_LIST_DECL)) and (member_kind != parser.NODE_CLASS_INSTANCE) {
          return parser.error(tokens, "class body expects a typed field, method, or class directive", member_index)
        }
        add(fields, member)
      }

      class_index = get(parsed_member, 1)
      VTime member_end = parser.line_end_index(tokens, class_index)
      if member_end < 0 {
        return parser.error(tokens, "expected end of line after class member", class_index)
      }
      class_index = member_end
    }

    return parser.error(tokens, "class expects }", class_index)
  }

  if (parser.is_keyword_token(token, "AVInt")) and (parser.is_identifier_token(get(tokens, index + 1))) {
    VTime qualified_main = parser.parse_qualified_name(tokens, index + 1)
    VTime main_name = get(qualified_main, 0)

    if (main_name == "main") and (parser.tokens_open_std_namespace(tokens)) {
      main_name = "std::main"
    }

    if main_name == "std::main" {
      VTime params_open_index = get(qualified_main, 1)

      if parser.is_symbol_token(get(tokens, params_open_index), "(") != true {
        return parser.error(tokens, "std::main expects (", params_open_index)
      }

      if parser.is_symbol_token(get(tokens, params_open_index + 1), ")") != true {
        return parser.error(tokens, "std::main does not accept parameters", params_open_index + 1)
      }

      if parser.is_symbol_token(get(tokens, params_open_index + 2), "{") != true {
        return parser.error(tokens, "std::main expects {", params_open_index + 2)
      }

      VTime parsed_main_body = parser.parse_block(tokens, params_open_index + 3, true)
      return [parser.node_at([parser.NODE_FUNC, "std::main", [], get(parsed_main_body, 0)], index), get(parsed_main_body, 1)]
    }
  }

  if parser.is_keyword_token(token, "func") {
    VTime name_token = get(tokens, index + 1)
    VTime params_open = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) == false {
      return parser.error(tokens, "func expects identifier", index + 1)
    }

    if parser.is_symbol_token(params_open, "(") == false {
      return parser.error(tokens, "func expects (", index + 2)
    }

    VTime params = []
    VTime param_index = index + 3

    while (param_index < len(tokens)) (-1) {
      VTime param_token = get(tokens, param_index)

      if parser.is_symbol_token(param_token, ")") {
        break
      }

      if parser.is_identifier_token(param_token) == false {
        return parser.error(tokens, "func expects parameter identifier", param_index)
      }

      add(params, lexer.token_value(param_token))
      param_index += 1

      if param_index >= len(tokens) {
        return parser.error(tokens, "func expects )", param_index)
      }

      VTime separator = get(tokens, param_index)

      if parser.is_symbol_token(separator, ")") {
        break
      }

      if parser.is_symbol_token(separator, ",") == false {
        return parser.error(tokens, "func expects , or )", param_index)
      }

      param_index += 1

      if param_index >= len(tokens) {
        return parser.error(tokens, "func expects parameter identifier", param_index)
      }

      if parser.is_symbol_token(get(tokens, param_index), ")") {
        return parser.error(tokens, "func expects parameter identifier", param_index)
      }
    }

    if param_index >= len(tokens) {
      return parser.error(tokens, "func expects )", param_index)
    }

    VTime body_open_index = param_index + 1
    VTime body_open = get(tokens, body_open_index)

    if parser.is_symbol_token(body_open, "{") == false {
      return parser.error(tokens, "func expects {", body_open_index)
    }

    VTime parsed_body = parser.parse_block(tokens, body_open_index + 1, true)
    return [parser.node_at([parser.NODE_FUNC, lexer.token_value(name_token), params, get(parsed_body, 0)], index), get(parsed_body, 1)]
  }

  if parser.is_keyword_token(token, "List") {
    VTime name_token = get(tokens, index + 1)
    VTime assign_token = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) == false {
      return parser.error(tokens, "list declaration expects identifier", index + 1)
    }

    if parser.is_symbol_token(assign_token, "=") == false {
      return parser.error(tokens, "list declaration expects =", index + 2)
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 3)
    return [parser.node_at([parser.NODE_LIST_DECL, lexer.token_value(name_token), get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "VTime") {
    VTime name_token = get(tokens, index + 1)
    VTime assign_token = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) == false {
      return parser.error(tokens, "vtime declaration expects identifier", index + 1)
    }

    if parser.is_symbol_token(assign_token, "=") == false {
      return parser.error(tokens, "vtime declaration expects =", index + 2)
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 3)
    return [parser.node_at([parser.NODE_VTIME_DECL, lexer.token_value(name_token), get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "if") {
    VTime parsed_condition = parser.parse_expression(tokens, index + 1)
    VTime open_index = get(parsed_condition, 1)
    VTime open_token = get(tokens, open_index)

    if parser.is_symbol_token(open_token, "{") == false {
      return parser.error(tokens, "if expects {", open_index)
    }

    VTime parsed_body = parser.parse_block(tokens, open_index + 1, true)
    VTime next_index = get(parsed_body, 1)
    VTime else_body = []
    next_index = parser.continuation_index(tokens, next_index, "else")
    VTime next_token = get(tokens, next_index)

    if parser.is_keyword_token(next_token, "else") {
      VTime else_open = get(tokens, next_index + 1)

      if parser.is_keyword_token(else_open, "if") {
        VTime parsed_else_if = parser.parse_statement(tokens, next_index + 1)
        else_body = [get(parsed_else_if, 0)]
        next_index = get(parsed_else_if, 1)
      } else {

        if parser.is_symbol_token(else_open, "{") == false {
          return parser.error(tokens, "else expects {", next_index + 1)
        }

        VTime parsed_else = parser.parse_block(tokens, next_index + 2, true)
        else_body = get(parsed_else, 0)
        next_index = get(parsed_else, 1)
      }
    }

    return [parser.node_at([parser.NODE_IF, get(parsed_condition, 0), get(parsed_body, 0), else_body], index), next_index]
  }

  if parser.is_keyword_token(token, "while") {
    VTime condition_open = get(tokens, index + 1)

    if parser.is_symbol_token(condition_open, "(") == false {
      return parser.error(tokens, "while expects condition (", index + 1)
    }

    VTime parsed_condition = parser.parse_expression(tokens, index + 2)
    VTime condition_close_index = get(parsed_condition, 1)
    VTime condition_close = get(tokens, condition_close_index)

    if parser.is_symbol_token(condition_close, ")") == false {
      return parser.error(tokens, "while expects condition )", condition_close_index)
    }

    VTime limit_open_index = condition_close_index + 1
    VTime limit_open = get(tokens, limit_open_index)

    if parser.is_symbol_token(limit_open, "(") == false {
      return parser.error(tokens, "while expects limit (", limit_open_index)
    }

    VTime parsed_limit = parser.parse_expression(tokens, limit_open_index + 1)
    VTime limit_close_index = get(parsed_limit, 1)

    VTime limit_close = get(tokens, limit_close_index)

    if parser.is_symbol_token(limit_close, ")") == false {
      return parser.error(tokens, "while expects limit )", limit_close_index)
    }

    VTime body_open_index = limit_close_index + 1
    VTime body_open = get(tokens, body_open_index)

    if parser.is_symbol_token(body_open, "{") == false {
      return parser.error(tokens, "while expects {", body_open_index)
    }

    VTime parsed_body = parser.parse_block(tokens, body_open_index + 1, true)
    VTime next_index = get(parsed_body, 1)
    VTime limit_body = []
    VTime else_body = []

    next_index = parser.continuation_index(tokens, next_index, "limit")

    if parser.is_word_token(get(tokens, next_index), "limit") {
      if parser.is_symbol_token(get(tokens, next_index + 1), "{") == false {
        return parser.error(tokens, "while limit expects {", next_index + 1)
      }

      VTime parsed_limit_body = parser.parse_block(tokens, next_index + 2, true)
      limit_body = get(parsed_limit_body, 0)
      next_index = get(parsed_limit_body, 1)
    }

    next_index = parser.continuation_index(tokens, next_index, "else")

    if parser.is_keyword_token(get(tokens, next_index), "else") {
      if parser.is_symbol_token(get(tokens, next_index + 1), "{") == false {
        return parser.error(tokens, "while else expects {", next_index + 1)
      }

      VTime parsed_else_body = parser.parse_block(tokens, next_index + 2, true)
      else_body = get(parsed_else_body, 0)
      next_index = get(parsed_else_body, 1)
    }

    return [parser.node_at([parser.NODE_WHILE, get(parsed_condition, 0), get(parsed_limit, 0), get(parsed_body, 0), limit_body, else_body], index), next_index]
  }

  if parser.is_keyword_token(token, "match") {
    VTime parsed_value = parser.parse_expression(tokens, index + 1)
    VTime case_index = get(parsed_value, 1)

    if parser.is_symbol_token(get(tokens, case_index), "{") == false {
      return parser.error(tokens, "match expects {", case_index)
    }

    case_index += 1
    if parser.is_newline_token(get(tokens, case_index)) == false {
      return parser.error(tokens, "match body must start on a new line", case_index)
    }
    case_index = parser.skip_newlines(tokens, case_index)
    VTime cases = []
    VTime default_body = []
    VTime has_default = false

    while (parser.is_symbol_token(get(tokens, case_index), "}") == false) (-1) {
      if parser.is_keyword_token(get(tokens, case_index), "case") == false {
        return parser.error(tokens, "match expects case", case_index)
      }

      case_index += 1
      if parser.is_keyword_token(get(tokens, case_index), "default") {
        if has_default {
          return parser.error(tokens, "duplicate case default", case_index)
        }

        if parser.is_symbol_token(get(tokens, case_index + 1), "{") == false {
          return parser.error(tokens, "case default expects {", case_index + 1)
        }

        VTime parsed_default = parser.parse_block(tokens, case_index + 2, true)
        default_body = get(parsed_default, 0)
        has_default = true
        case_index = get(parsed_default, 1)

        VTime default_end = parser.line_end_index(tokens, case_index)
        if default_end < 0 {
          return parser.error(tokens, "expected end of line after case", case_index)
        }
        case_index = default_end

        if parser.is_symbol_token(get(tokens, case_index), "}") == false {
          return parser.error(tokens, "case default must be last", case_index)
        }

        continue
      }

      VTime patterns = []
      while (true) (-1) {
        VTime parsed_start = parser.parse_expression(tokens, case_index)
        VTime pattern_end = get(parsed_start, 1)

        if parser.is_symbol_token(get(tokens, pattern_end), "..") {
          VTime parsed_end = parser.parse_expression(tokens, pattern_end + 1)
          add(patterns, [parser.MATCH_RANGE, get(parsed_start, 0), get(parsed_end, 0)])
          case_index = get(parsed_end, 1)
        } else {
          add(patterns, [parser.MATCH_VALUE, get(parsed_start, 0)])
          case_index = pattern_end
        }

        if parser.is_symbol_token(get(tokens, case_index), ",") == false {
          break
        }

        case_index += 1
      }

      if parser.is_symbol_token(get(tokens, case_index), "{") == false {
        return parser.error(tokens, "case expects {", case_index)
      }

      VTime parsed_case_body = parser.parse_block(tokens, case_index + 1, true)
      add(cases, [patterns, get(parsed_case_body, 0)])
      case_index = get(parsed_case_body, 1)
      VTime case_end = parser.line_end_index(tokens, case_index)
      if case_end < 0 {
        return parser.error(tokens, "expected end of line after case", case_index)
      }
      case_index = case_end
    }

    if (len(cases) == 0) and (has_default == false) {
      return parser.error(tokens, "match requires at least one case", case_index)
    }

    return [parser.node_at([parser.NODE_MATCH, get(parsed_value, 0), cases, default_body], index), case_index + 1]
  }

  if parser.is_keyword_token(token, "pick") {
    VTime value_open = get(tokens, index + 1)

    if parser.is_symbol_token(value_open, "(") == false {
      return parser.error(tokens, "pick expects (", index + 1)
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 2)
    VTime value_close_index = get(parsed_value, 1)
    VTime value_close = get(tokens, value_close_index)

    if parser.is_symbol_token(value_close, ")") == false {
      return parser.error(tokens, "pick expects )", value_close_index)
    }

    VTime colon_index = value_close_index + 1
    VTime colon = get(tokens, colon_index)

    if parser.is_symbol_token(colon, ":") == false {
      return parser.error(tokens, "pick expects :", colon_index)
    }

    VTime item_token = get(tokens, colon_index + 1)
    VTime body_open_index = colon_index + 2
    VTime body_open = get(tokens, body_open_index)

    if parser.is_identifier_token(item_token) == false {
      return parser.error(tokens, "pick expects item identifier", colon_index + 1)
    }

    if parser.is_symbol_token(body_open, "{") == false {
      return parser.error(tokens, "pick expects {", body_open_index)
    }

    VTime parsed_body = parser.parse_block(tokens, body_open_index + 1, true)
    return [parser.node_at([parser.NODE_PICK, get(parsed_value, 0), lexer.token_value(item_token), get(parsed_body, 0)], index), get(parsed_body, 1)]
  }

  if (kind == lexer.TOKEN_KW) and (parser.is_decl_keyword(value)) {
    VTime name_token = get(tokens, index + 1)
    VTime assign_token = get(tokens, index + 2)

    if parser.is_identifier_token(name_token) == false {
      return parser.error(tokens, "declaration expects identifier", index + 1)
    }

    if parser.is_symbol_token(assign_token, "=") == false {
      return parser.error(tokens, "declaration expects =", index + 2)
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 3)
    return [parser.node_at([parser.NODE_DECL, value, lexer.token_value(name_token), get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "out") {
    VTime parsed_value = parser.parse_expression(tokens, index + 1)
    return [parser.node_at([parser.NODE_OUT, get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "stop") {
    if parser.next_starts_statement_or_block_end(tokens, index + 1) {
      return [parser.node_at([parser.NODE_STOP, NONE], index), index + 1]
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 1)
    return [parser.node_at([parser.NODE_STOP, get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "fail") {
    VTime parsed_value = parser.parse_expression(tokens, index + 1)
    return [parser.node_at([parser.NODE_FAIL, get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "return") {
    if (parser.is_symbol_token(get(tokens, index + 1), "(")) and (parser.is_keyword_token(get(tokens, index + 2), "ERR")) {
      if parser.is_symbol_token(get(tokens, index + 3), "=") == false {
        return parser.error(tokens, "ERR expects =", index + 3)
      }

      if parser.is_symbol_token(get(tokens, index + 4), "[") == false {
        return parser.error(tokens, "ERR message expects [", index + 4)
      }

      VTime parsed_message = parser.parse_expression(tokens, index + 5)
      VTime message_close = get(parsed_message, 1)
      if parser.is_symbol_token(get(tokens, message_close), "]") == false {
        return parser.error(tokens, "ERR message expects ]", message_close)
      }

      if parser.is_symbol_token(get(tokens, message_close + 1), "[") == false {
        return parser.error(tokens, "ERR code expects [", message_close + 1)
      }

      VTime parsed_code = parser.parse_expression(tokens, message_close + 2)
      VTime code_close = get(parsed_code, 1)
      if parser.is_symbol_token(get(tokens, code_close), "]") == false {
        return parser.error(tokens, "ERR code expects ]", code_close)
      }

      if parser.is_symbol_token(get(tokens, code_close + 1), ")") == false {
        return parser.error(tokens, "ERR expects )", code_close + 1)
      }

      return [parser.node_at([parser.NODE_ERR, get(parsed_message, 0), get(parsed_code, 0)], index), code_close + 2]
    }

    VTime parsed_value = parser.parse_expression(tokens, index + 1)
    return [parser.node_at([parser.NODE_RETURN, get(parsed_value, 0)], index), get(parsed_value, 1)]
  }

  if parser.is_keyword_token(token, "secretup") {
    VTime open_token = get(tokens, index + 1)
    VTime name_token = get(tokens, index + 2)
    VTime close_token = get(tokens, index + 3)

    if parser.is_symbol_token(open_token, "(") == false {
      return parser.error(tokens, "secretup expects (", index + 1)
    }

    if parser.is_identifier_token(name_token) == false {
      return parser.error(tokens, "secretup expects identifier", index + 2)
    }

    if parser.is_symbol_token(close_token, ")") == false {
      return parser.error(tokens, "secretup expects )", index + 3)
    }

    return [parser.node_at([parser.NODE_SECRETUP, lexer.token_value(name_token)], index), index + 4]
  }

  if parser.is_keyword_token(token, "break") {
    return [parser.node_at([parser.NODE_BREAK], index), index + 1]
  }

  if parser.is_keyword_token(token, "continue") {
    return [parser.node_at([parser.NODE_CONTINUE], index), index + 1]
  }

  if kind == lexer.TOKEN_IDENT {
    if (parser.is_identifier_token(get(tokens, index + 1))) and (parser.is_symbol_token(get(tokens, index + 2), "=")) {
      VTime parsed_value = parser.parse_expression(tokens, index + 3)
      return [parser.node_at([parser.NODE_CLASS_INSTANCE, value, lexer.token_value(get(tokens, index + 1)), get(parsed_value, 0)], index), get(parsed_value, 1)]
    }

    VTime qualified_statement_name = parser.parse_qualified_name(tokens, index)
    VTime qualified_statement_end = get(qualified_statement_name, 1)
    VTime op_token = get(tokens, index + 1)
    VTime op = lexer.token_value(op_token)

    if parser.is_symbol_token(op_token, ",") {
      VTime level_token = get(tokens, index + 2)
      VTime assign_token = get(tokens, index + 3)
      VTime info_token = get(tokens, index + 4)
      VTime open_token = get(tokens, index + 5)
      VTime source_token = get(tokens, index + 6)
      VTime close_token = get(tokens, index + 7)

      if parser.is_identifier_token(level_token) == false {
        return parser.error(tokens, "info assignment expects identifier", index + 2)
      }

      if parser.is_symbol_token(assign_token, "=") == false {
        return parser.error(tokens, "info assignment expects =", index + 3)
      }

      if parser.is_keyword_token(info_token, "info") == false {
        return parser.error(tokens, "info assignment expects info", index + 4)
      }

      if parser.is_symbol_token(open_token, "(") == false {
        return parser.error(tokens, "info assignment expects (", index + 5)
      }

      if parser.is_identifier_token(source_token) == false {
        return parser.error(tokens, "info assignment expects source identifier", index + 6)
      }

      if parser.is_symbol_token(close_token, ")") == false {
        return parser.error(tokens, "info assignment expects )", index + 7)
      }

      return [parser.node_at([parser.NODE_INFO_ASSIGN, value, lexer.token_value(level_token), lexer.token_value(source_token)], index), index + 8]
    }

    if parser.is_symbol_token(get(tokens, qualified_statement_end), "(") {
      VTime parsed_expr = parser.parse_expression(tokens, index)
      return [parser.node_at([parser.NODE_EXPR, get(parsed_expr, 0)], index), get(parsed_expr, 1)]
    }

    if parser.is_symbol_token(op_token, "++") {
      return [parser.node_at([parser.NODE_ASSIGN, value, "+=", [parser.EXPR_INT, 1]], index), index + 2]
    }

    if parser.is_symbol_token(op_token, "--") {
      return [parser.node_at([parser.NODE_ASSIGN, value, "-=", [parser.EXPR_INT, 1]], index), index + 2]
    }

    if (lexer.token_kind(op_token) == lexer.TOKEN_SYM) and (parser.is_assign_op(op)) {
      VTime parsed_value = parser.parse_expression(tokens, index + 2)
      return [parser.node_at([parser.NODE_ASSIGN, value, op, get(parsed_value, 0)], index), get(parsed_value, 1)]
    }
  }

  return parser.error_at(tokens, "unknown statement", index, index + 1)
}

func parser.parse_block(tokens, index, expect_close) {
  VTime statements = []

  if expect_close {
    if parser.is_newline_token(get(tokens, index)) == false {
      add(statements, [parser.NODE_ERROR, parser.diagnostic(tokens, index, "block body must start on a new line")])
      return [statements, index, false]
    }
  }

  index = parser.skip_newlines(tokens, index)

  while (index < len(tokens)) (-1) {
    VTime token = get(tokens, index)

    if lexer.token_kind(token) == lexer.TOKEN_EOF {
      if expect_close {
        add(statements, [parser.NODE_ERROR, parser.diagnostic(tokens, index, "expected }")])
      }

      return [statements, index, false]
    }

    if parser.is_symbol_token(token, "}") {
      if expect_close {
        return [statements, index + 1, true]
      }

      add(statements, [parser.NODE_ERROR, parser.diagnostic(tokens, index, "unexpected }")])
      return [statements, index + 1, false]
    }

    VTime parsed = parser.parse_statement(tokens, index)
    VTime statement = get(parsed, 0)
    add(statements, statement)
    index = get(parsed, 1)

    if parser.node_kind(statement) == parser.NODE_ERROR {
      return [statements, index]
    }

    VTime next_index = parser.line_end_index(tokens, index)
    if next_index >= 0 {
      index = next_index
    } else {
      VTime next_token = get(tokens, index)
      if (expect_close == false) and (lexer.token_kind(next_token) == lexer.TOKEN_EOF) {
        continue
      }
      add(statements, [parser.NODE_ERROR, parser.diagnostic(tokens, index, "expected end of line after statement")])
      return [statements, index, false]
    }
  }

  if expect_close {
    add(statements, [parser.NODE_ERROR, "expected }"])
  }

  return [statements, index, false]
}

func parser.parse_tokens(tokens) {
  VTime parsed = parser.parse_block(tokens, 0, false)
  VTime statements = get(parsed, 0)

  return statements
}

func parser.parse_source_with_mode(source, record_source_positions) {
  parser.RECORD_SOURCE_POSITIONS = record_source_positions
  VTime tokens = lexer.tokenize(source)
  add(tokens, [lexer.TOKEN_EOF, source])
  VTime statements = parser.parse_tokens(tokens)
  parser.RECORD_SOURCE_POSITIONS = false
  return statements
}

func parser.parse_source(source) {
  return parser.parse_source_with_mode(source, false)
}

func parser.parse_source_with_positions(source) {
  return parser.parse_source_with_mode(source, true)
}
