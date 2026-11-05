AVStr source = "AVFloat x = 1.5 \nAVFloat y = -2.5 \nAVFloat z = x + y \nout z \nout z < 0.0"
VTime output = vm.run_source(source)

out "VM float output:"

pick(output): line {
  out line
}

out "Done!"
