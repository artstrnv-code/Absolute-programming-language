AVStr public_stop = "stop \"done\" \nout \"after\""
AVStr secret_stop = "ASVStr token = \"root\" \nstop token \nout \"after\""
AVStr public_fail = "fail \"bad\" \nout \"after\""

VTime stop_output = vm.run_source(public_stop)
VTime secret_output = vm.run_source(secret_stop)
VTime fail_output = vm.run_source(public_fail)

out "VM stop/fail output:"

pick(stop_output): line {
  out line
}

pick(secret_output): line {
  out line
}

pick(fail_output): line {
  out line
}

out "Done!"
