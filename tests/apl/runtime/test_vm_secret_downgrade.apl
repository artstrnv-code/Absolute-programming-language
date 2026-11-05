AVStr source = "out \"before\" \nASVStr token = \"root\" \nVTime tmp = token \nAVStr public = tmp \nout public \nout \"after\""
VTime output = vm.run_source(source)

out "VM secret downgrade output:"

pick(output): line {
  out line
}

out "Done!"
