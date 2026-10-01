import ApplicationServices
import Foundation
let a = CommandLine.arguments
func pt() -> CGPoint { CGPoint(x: Double(a[2])!, y: Double(a[3])!) }
func post(_ t: CGEventType, _ b: CGMouseButton, _ p: CGPoint) {
  let e = CGEvent(mouseEventSource: nil, mouseType: t, mouseCursorPosition: p, mouseButton: b)!
  e.post(tap: .cghidEventTap); usleep(40000)
}
switch a[1] {
case "move": post(.mouseMoved, .left, pt())
case "click": let p = pt(); post(.mouseMoved, .left, p); post(.leftMouseDown, .left, p); post(.leftMouseUp, .left, p)
case "rclick": let p = pt(); post(.mouseMoved, .left, p); post(.rightMouseDown, .right, p); post(.rightMouseUp, .right, p)
case "dclick": let p = pt(); post(.mouseMoved, .left, p)
  for i in 1...2 { let d = CGEvent(mouseEventSource: nil, mouseType: .leftMouseDown, mouseCursorPosition: p, mouseButton: .left)!; d.setIntegerValueField(.mouseEventClickState, value: Int64(i)); d.post(tap: .cghidEventTap)
    let u = CGEvent(mouseEventSource: nil, mouseType: .leftMouseUp, mouseCursorPosition: p, mouseButton: .left)!; u.setIntegerValueField(.mouseEventClickState, value: Int64(i)); u.post(tap: .cghidEventTap); usleep(50000) }
case "scroll": let p = pt(); post(.mouseMoved, .left, p)
  let n = Int32(a[4])!; let e = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: n, wheel2: 0, wheel3: 0)!; e.location = p; e.post(tap: .cghidEventTap)
case "drag": let p = pt(); let q = CGPoint(x: Double(a[4])!, y: Double(a[5])!)
  post(.mouseMoved, .left, p); post(.leftMouseDown, .left, p)
  for i in 1...12 { let t = Double(i) / 12; post(.leftMouseDragged, .left, CGPoint(x: p.x + (q.x - p.x) * t, y: p.y + (q.y - p.y) * t)) }
  let hold = a.count > 6 ? UInt32(a[6])! * 1000 : 150000
  var waited: UInt32 = 0
  while waited < hold { post(.leftMouseDragged, .left, q); usleep(100000); waited += 100000 }
  post(.leftMouseUp, .left, q)
case "dragvia": let p = pt(); let q = CGPoint(x: Double(a[4])!, y: Double(a[5])!); let hold = UInt32(a[6])! * 1000; let r = CGPoint(x: Double(a[7])!, y: Double(a[8])!)
  post(.mouseMoved, .left, p); post(.leftMouseDown, .left, p)
  for i in 1...12 { let t = Double(i) / 12; post(.leftMouseDragged, .left, CGPoint(x: p.x + (q.x - p.x) * t, y: p.y + (q.y - p.y) * t)) }
  var waited: UInt32 = 0
  while waited < hold { post(.leftMouseDragged, .left, q); usleep(100000); waited += 100000 }
  for i in 1...8 { let t = Double(i) / 8; post(.leftMouseDragged, .left, CGPoint(x: q.x + (r.x - q.x) * t, y: q.y + (r.y - q.y) * t)) }
  usleep(200000); post(.leftMouseUp, .left, r)
default: print("?")
}
