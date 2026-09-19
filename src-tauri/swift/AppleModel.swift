// The model Apple ships with the system, reached through the framework meant
// for applications rather than through the command line tool. Nothing to
// download, nothing to start, and no terms for the user to accept by hand.
//
// Exposed to Rust as plain C. Text crosses as UTF-8, the caller owns nothing
// it did not allocate, and every string handed back is freed with
// langolier_apple_free.
import Foundation
import FoundationModels

/// Why the model cannot answer. Kept in step with apple.rs.
private enum Status: Int32 {
    case available = 0
    case deviceNotEligible = 1
    case intelligenceOff = 2
    case modelNotReady = 3
    case systemTooOld = 4
    case unknownReason = 5
}

@_cdecl("langolier_apple_availability")
public func langolier_apple_availability() -> Int32 {
    guard #available(macOS 26.0, *) else { return Status.systemTooOld.rawValue }
    switch SystemLanguageModel.default.availability {
    case .available:
        return Status.available.rawValue
    case .unavailable(let why):
        switch why {
        case .deviceNotEligible: return Status.deviceNotEligible.rawValue
        case .appleIntelligenceNotEnabled: return Status.intelligenceOff.rawValue
        case .modelNotReady: return Status.modelNotReady.rawValue
        @unknown default: return Status.unknownReason.rawValue
        }
    @unknown default:
        return Status.unknownReason.rawValue
    }
}

@_cdecl("langolier_apple_free")
public func langolier_apple_free(_ p: UnsafeMutablePointer<CChar>?) {
    free(p)
}

/// Generates an answer, handing each new piece of text to `onDelta`.
///
/// The stream carries the whole answer so far rather than the last piece, so
/// the difference is what gets passed on. Returning zero from `onDelta` stops
/// the generation, which is how a cancelled conversation ends.
///
/// Returns 0 on success. On failure the reason is written to `error` as a C
/// string the caller frees.
@_cdecl("langolier_apple_generate")
public func langolier_apple_generate(
    _ instructions: UnsafePointer<CChar>?,
    _ prompt: UnsafePointer<CChar>?,
    _ temperature: Double,
    _ maxTokens: Int32,
    _ onDelta: @convention(c) @escaping (UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Int32,
    _ context: UnsafeMutableRawPointer?,
    _ error: UnsafeMutablePointer<UnsafeMutablePointer<CChar>?>?
) -> Int32 {
    guard #available(macOS 26.0, *) else {
        error?.pointee = strdup("Apple's on-device model needs macOS 26 or newer.")
        return 1
    }
    let ask = prompt.map { String(cString: $0) } ?? ""
    let brief = instructions.map { String(cString: $0) }
    nonisolated(unsafe) let ctx = context
    nonisolated(unsafe) var failure: String?
    let done = DispatchSemaphore(value: 0)

    Task {
        defer { done.signal() }
        let session = LanguageModelSession(instructions: brief)
        let options = GenerationOptions(
            temperature: temperature > 0 ? temperature : nil,
            maximumResponseTokens: maxTokens > 0 ? Int(maxTokens) : nil
        )
        var sent = ""
        do {
            for try await snapshot in session.streamResponse(to: ask, options: options) {
                let whole = snapshot.content
                guard whole.count > sent.count else { continue }
                let piece = String(whole.dropFirst(sent.count))
                sent = whole
                let keepGoing = piece.withCString { onDelta($0, ctx) }
                if keepGoing == 0 { return }
            }
        } catch {
            failure = String(describing: error)
        }
    }
    done.wait()

    if let failure {
        error?.pointee = strdup(failure)
        return 1
    }
    return 0
}
