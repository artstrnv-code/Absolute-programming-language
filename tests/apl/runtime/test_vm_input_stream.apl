AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVStr missing = input \nout public \nout hidden \nout missing"
VTime output = vm.run_source_with_input(source, ["hello", "token"])

out "VM input stream output:"

pick(output): line {
  out line
}

out "Done!"
