# Test APL checker protection-flow validation.

List protection_sources = [
  "ASVInt hidden = 1 \nSASVInt vault = hidden \nVTime temporary = hidden \nASVInt copy = temporary",
  "List items = [1:ASV, 2:SASV] \nSASVInt vault = 1",
  "ASVStr value = secret input",
  "ASVInt hidden = 1 \nAVInt public = hidden",
  "AVStr public = secret input",
  "SASVStr vault = input",
  "ASVInt hidden = 1 \nList items = [hidden:AV]",
  "ASVInt hidden = 1 \nout hidden",
  "ASVInt hidden = 1 \nVTime temporary = hidden \nAVInt public = temporary",
  "List items = [1] \nASVInt hidden = 2 \nadd(items, hidden) \nout items",
  "AVInt value = 1 \nsecretup(value) \nsecretup(value) \nsecretup(value)"
]

out "Checker protection reports:"

pick(protection_sources): protection_source {
  VTime protection_report = bootstrap.compile_report(protection_source)
  out get(protection_report, 0)

  if get(protection_report, 0) == checker.STATUS_FAIL {
    out get(protection_report, 1)
  }
}

out "Done!"
