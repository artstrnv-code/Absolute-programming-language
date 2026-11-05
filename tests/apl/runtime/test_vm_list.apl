AVStr source = "List items = [1, 2] \nadd(items, 3) \nVTime first = get(items, 0) \nVTime last = pop(items) \nout first \nout last \nout len(items)"
VTime vm_output = vm.run_source(source)

out "VM list output:"
pick(vm_output): line {
  out line
}
out "Done!"
