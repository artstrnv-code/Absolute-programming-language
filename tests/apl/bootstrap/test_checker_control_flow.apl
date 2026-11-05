# Test APL checker control-flow context validation.

List control_flow_sources = [
  "func choose(value) { \nif true { \nreturn value \n} \nreturn NONE \n} \nVTime result = choose(1)",
  "func work() { \nwhile (true) (1) { \ncontinue \n} \npick([1]): item { \nbreak \n} \nreturn NONE \n}",
  "while (true) (-1) { \nbreak \n}",
  "return NONE",
  "if true { \nreturn NONE \n}",
  "break",
  "continue",
  "if true { \nbreak \n}",
  "func work() { \nbreak \n}",
  "while (true) (-2) { \nbreak \n}",
  "while (true) (1) { \nfunc nested() { \nbreak \n} \n}"
]

out "Checker control-flow reports:"

pick(control_flow_sources): control_flow_source {
  VTime control_flow_report = bootstrap.compile_report(control_flow_source)
  out get(control_flow_report, 0)

  if get(control_flow_report, 0) == checker.STATUS_FAIL {
    out get(control_flow_report, 1)
  }
}

out "Done!"
