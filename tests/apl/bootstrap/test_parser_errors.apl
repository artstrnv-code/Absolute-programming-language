# Test structural syntax failures in the APL-written parser.

VTime quote = char(34)
List invalid_sources = [
  "if true { \nout 1",
  "AVInt value = (1 + 2",
  "VTime value = len([1]",
  "List values = [1, 2",
  "VTime value = pow(2 3)",
  "List values = [1 2]",
  "VTime value = pow(2,)",
  "List values = [1] \nVTime value = values[0",
  "out 1 \n} out 2",
  "if true { \nout 1 \n\"}\""
]

out "Parser error reports:"

pick(invalid_sources): source {
  VTime report = bootstrap.compile_report(source)
  out get(report, 0)
  out get(report, 1)
}

out "Done!"
