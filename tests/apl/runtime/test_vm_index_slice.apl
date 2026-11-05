AVStr source = "List items = [1, 2, 3, 4] \nVTime first = items[0] \nVTime tail = items[1:] \nAVStr text = \"APL\" \nout first \nout get(tail, 0) \nout get(tail, 2) \nout text[::-1]"
VTime vm_output = vm.run_source(source)

out "VM index/slice output:"
pick(vm_output): line {
  out line
}
out "Done!"
