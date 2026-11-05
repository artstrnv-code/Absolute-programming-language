AVStr source = "List items = [1, 2, 3, 4] \nAVInt total = 0 \npick(items): item { \nif item == 2 { \ncontinue \n} \nif item == 4 { \nbreak \n} \ntotal += item \n} \nout total \nout item"
VTime vm_output = vm.run_source(source)

out "VM pick output:"
pick(vm_output): line {
  out line
}
out "Done!"
