include >> classes

class Message {
  AVStr text = ""

  func __init__(text) {
    self.text = text
  }

  func value() {
    return self.text
  }

  func __out__() {
    out self.value()
  }
}

class LoudMessage {
  class::inheritance "Message"

  func __init__(text) {
    self.text = text
  }

  override func value() {
    return self.text + "!"
  }
}

AVInt std::main() {
  Message message = LoudMessage("APL")
  VTime alias = message
  out message
  out "\nidentity: "
  out classes::id(message) == classes::id(alias)
  return 0
}
