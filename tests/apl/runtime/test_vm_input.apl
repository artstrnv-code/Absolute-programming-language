AVStr source = "AVStr public = input \nASVStr hidden = secret input \nAVStr typ = \"\" \nAVStr level = \"\" \ntyp, level = info(public) \nout typ \nout level \nout public \ntyp, level = info(hidden) \nout typ \nout level \nout hidden"
VTime output = vm.run_source(source)

out "VM input output:"

pick(output): line {
  out line
}

out "Done!"
