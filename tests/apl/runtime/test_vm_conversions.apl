AVStr source = "AVStr raw = \"42\" \nAVStr bad = \"no\" \nASVStr hidden = \"7\" \nout int(raw) \nout int(bad) \nout bool(\"true\") \nout str(42) \nout int(hidden)"
VTime output = vm.run_source(source)

out "VM conversions output:"

pick(output): line {
  out line
}

out "Done!"
