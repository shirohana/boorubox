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
default: print("?")
}
