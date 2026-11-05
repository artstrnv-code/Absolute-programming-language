AVStr source = "AVStr typ = \"\" \nAVStr level = \"\" \nSASVStr master = \"root\" \ntyp, level = info(master) \nout typ \nout level \nout master"
VTime vm_output = vm.run_source(source)

out "VM security output:"
pick(vm_output): line {
  out line
}
out "Done!"
