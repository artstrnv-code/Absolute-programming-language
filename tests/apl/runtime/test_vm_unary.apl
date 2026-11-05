AVStr source = "AVInt x = 4 \nAVInt y = -x \nAVFloat z = -1.5 \nAVBool flag = false \nout y \nout z \nout not flag \nout -(x + 1)"
VTime output = vm.run_source(source)

out "VM unary output:"

pick(output): line {
  out line
}

out "Done!"
