AVStr source = "AVStr text = \"A P L\" \nList parts = split(text, \" \") ASVStr hidden = \"secret words\" out len(parts) \nout join(parts, \"-\") out contains(text, \"P\") out ord(\"A\") out char(90) out split(hidden, \" \")"
VTime output = vm.run_source(source)

out "VM string helpers output:"

pick(output): line {
  out line
}

out "Done!"
