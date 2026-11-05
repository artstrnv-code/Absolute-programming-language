AVStr source = "AVInt good = input \nAVInt bad = input \nAVFloat ratio = input \nAVBool flag = input \nAVStr text = input \nAVInt assigned = 0 \nassigned = input \nout good \nout bad \nout ratio \nout flag \nout text \nout assigned"
VTime output = vm.run_source_with_input(source, ["42", "oops", "2.5", "true", "abc", "7"])

out "VM typed input output:"

pick(output): line {
  out line
}

out "Done!"
