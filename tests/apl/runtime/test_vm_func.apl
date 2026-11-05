AVStr source = "func sum_two(left, right) { \nreturn left + right \n} \nAVInt x = sum_two(20, 22) \nout x"
VTime vm_output = vm.run_source(source)

out "VM function output:"
pick(vm_output): line {
  out line
}
out "Done!"
