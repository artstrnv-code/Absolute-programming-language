struct Point {
  AVInt x = 1
  AVInt y = 2
}

struct Rectangle {
  Point top_left = Point()
  AVInt width = 10
  AVInt height = 20
  List labels = ["demo"]
}

AVInt std::main() {
  Rectangle first = Rectangle()
  Rectangle second = first

  first.top_left.x += 4
  first.width = 30

  out first.top_left.x
  out ":"
  out second.top_left.x
  out ":"
  out first.width
  out ":"
  out second.width
  return 0
}
