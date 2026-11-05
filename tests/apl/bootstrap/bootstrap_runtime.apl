AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVInt age = input \nList values = [age, 2, [3, 4]] \nfunc inc(x) { \nreturn x + 1 \n} \nVTime next = inc(age) \nVTime first = values[0] \nout public \nout \"\\n\" \nout next \nout \"\\n\" \nout first \nout \"\\n\" \nAVStr missing = input \nout missing \nout \"\\n\""
VTime output = bootstrap.run_with_input(source, ["hello", "token", "41"])

out "Bootstrap runtime output:\n"

pick(output): line {
  out line
}

out "Done!\n"
