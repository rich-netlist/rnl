// Observe the WindowServer screenshot of the shipped app. No script is injected into its WebView.
import AppKit
import Vision

let out = CommandLine.arguments[1]
let apps = NSRunningApplication.runningApplications(withBundleIdentifier: "io.github.rich-netlist.viewer")
guard let app = apps.first,
      let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]],
      let window = windows.first(where: {
          ($0[kCGWindowOwnerPID as String] as? Int32) == app.processIdentifier &&
          ($0[kCGWindowLayer as String] as? Int) == 0 &&
          (($0[kCGWindowBounds as String] as? [String:Double])?["Width"] ?? 0) > 400
      }), let id = window[kCGWindowNumber as String] as? Int else {
    fputs("No visible viewer window\n", stderr); exit(2)
}
let capture = Process()
capture.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
capture.arguments = ["-x", "-o", "-l", String(id), out]
try capture.run(); capture.waitUntilExit()
guard capture.terminationStatus == 0,
      let image = NSImage(contentsOfFile:out)?.cgImage(forProposedRect:nil,context:nil,hints:nil) else {
    fputs("Window screenshot unavailable\n", stderr); exit(3)
}
let request = VNRecognizeTextRequest()
request.recognitionLevel = .accurate
request.usesLanguageCorrection = false
try VNImageRequestHandler(cgImage:image).perform([request])
let words = (request.results ?? []).compactMap { observation -> [String:Any]? in
    guard let text=observation.topCandidates(1).first?.string else { return nil }
    let r=observation.boundingBox
    return ["text":text,"x":r.minX,"y":r.minY,"width":r.width,"height":r.height]
}
let width=image.width, height=image.height
var bytes = [UInt8](repeating:0,count:width*height*4)
let color = CGColorSpaceCreateDeviceRGB()
let ctx = CGContext(data:&bytes,width:width,height:height,bitsPerComponent:8,bytesPerRow:width*4,
                    space:color,bitmapInfo:CGImageAlphaInfo.premultipliedLast.rawValue)!
ctx.draw(image,in:CGRect(x:0,y:0,width:width,height:height))
// Schematic region excludes the left source pane, toolbar, title and window border.
var ink=0, total=0
for y in Int(Double(height)*0.12)..<Int(Double(height)*0.88) {
    for x in Int(Double(width)*0.40)..<Int(Double(width)*0.96) {
        let i=(y*width+x)*4; total += 1
        if bytes[i]<100 && bytes[i+1]<100 && bytes[i+2]<100 { ink += 1 }
    }
}
let result: [String:Any] = ["pid":app.processIdentifier,"window":id,
    "title":window[kCGWindowName as String] as? String ?? "", "width":width,"height":height,
    "circuitInk":ink,"circuitDensity":Double(ink)/Double(total),"words":words]
let json=try JSONSerialization.data(withJSONObject:result,options:[.sortedKeys])
print(String(data:json,encoding:.utf8)!)
