import Foundation
import Vision
import AppKit
import SwiftRs

@_cdecl("recognize_text")
public func recognizeText(filePath: SRString) -> SRString {
    let path = filePath.toString()

    guard let image = NSImage(byReferencingFile: path),
          let cgImage = image.cgImage(forProposedRect: nil, context: nil, hints: nil)
    else {
        return SRString("")
    }

    var resultText = ""

    let request = VNRecognizeTextRequest { request, error in
        guard error == nil,
              let observations = request.results as? [VNRecognizedTextObservation]
        else { return }

        for observation in observations {
            if let candidate = observation.topCandidates(1).first {
                resultText += candidate.string + "\n"
            }
        }
    }

    request.recognitionLevel = .accurate
    request.usesLanguageCorrection = true
    request.recognitionLanguages = ["en-US"]

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])

    do {
        try handler.perform([request])
    } catch {
        return SRString("")
    }

    return SRString(resultText)
}
