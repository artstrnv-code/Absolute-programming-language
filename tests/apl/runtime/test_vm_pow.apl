AVStr source = "AVInt x = pow(2, 3) \nAVInt nested = 2 ** 3 ** 2 \nAVInt signed = -2 ** 2 \nAVFloat y = 2.0 ** 3 \nAVInt bad = 2 ** -1 \nASVInt hidden = 2 \nout x \nout nested \nout signed \nout y == 8.0 \nout bad \nout hidden ** 2"
VTime output = vm.run_source(source)

out "VM pow output:"

pick(output): line {
  out line
}

out "Done!"
