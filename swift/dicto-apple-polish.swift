// dicto-apple-polish
//
// Long-lived sidecar binary that polishes transcripts using Apple's
// Foundation Models framework (the on-device LLM powering Apple
// Intelligence on macOS 26+).
//
// Protocol: line-delimited JSON over stdin / stdout.
//
//   On startup the sidecar emits one line:
//     {"ready": true, "availability": "available" | "<reason>"}
//
//   Then for each request line on stdin:
//     {"id": "<opaque>", "system": "<instructions>", "user": "<prompt>"}
//   the sidecar replies with:
//     {"id": "<opaque>", "ok": true,  "text": "..."}
//   or:
//     {"id": "<opaque>", "ok": false, "error": "..."}
//
// Each polish call gets a fresh LanguageModelSession, so previous
// transcripts can't contaminate the next request — important for
// privacy and consistency. Sessions are cheap to create.
//
// Build:  swiftc -O dicto-apple-polish.swift -o dicto-apple-polish
// (Requires macOS 26 SDK; runs only on macOS 26+ where Apple
// Intelligence is enabled.)

import Foundation
@preconcurrency import FoundationModels

struct PolishRequest: Decodable {
    let id: String
    let system: String
    let user: String
}

struct PolishOk: Encodable {
    let id: String
    let ok: Bool = true
    let text: String
}

struct PolishErr: Encodable {
    let id: String
    let ok: Bool = false
    let error: String
}

struct ReadyMessage: Encodable {
    let ready: Bool = true
    let availability: String
}

// MARK: - Availability

@available(macOS 26.0, *)
func availabilityString() -> String {
    switch SystemLanguageModel.default.availability {
    case .available:
        return "available"
    case .unavailable(let reason):
        switch reason {
        case .appleIntelligenceNotEnabled:
            return "appleIntelligenceNotEnabled"
        case .modelNotReady:
            return "modelNotReady"
        case .deviceNotEligible:
            return "deviceNotEligible"
        @unknown default:
            return "unavailable"
        }
    @unknown default:
        return "unknown"
    }
}

// MARK: - JSON I/O

let stdoutHandle = FileHandle.standardOutput
let stderrHandle = FileHandle.standardError

func emitLine<T: Encodable>(_ value: T) {
    let encoder = JSONEncoder()
    encoder.outputFormatting = []
    do {
        var data = try encoder.encode(value)
        data.append(0x0A)
        stdoutHandle.write(data)
    } catch {
        // If encoding fails, write a best-effort fallback so the host
        // doesn't deadlock waiting on a reply.
        let fallback = "{\"ok\":false,\"error\":\"encode failure\"}\n"
        if let bytes = fallback.data(using: .utf8) {
            stdoutHandle.write(bytes)
        }
    }
}

func logErr(_ msg: String) {
    if let bytes = (msg + "\n").data(using: .utf8) {
        stderrHandle.write(bytes)
    }
}

// MARK: - Polish

@available(macOS 26.0, *)
func polish(_ req: PolishRequest) async -> Encodable {
    let tStart = Date()

    // Fresh session per call — instructions become part of system context,
    // and we don't want carry-over between unrelated transcripts. The
    // model itself is shared across sessions, so subsequent calls only
    // pay for session setup (~tens of ms) not model load.
    let instructions = Instructions(req.system)
    let session = LanguageModelSession(instructions: instructions)
    let tAfterInit = Date()

    do {
        // GenerationOptions: low temperature for consistent polish output.
        // The model is small and conservative by default; bumping
        // sampling temperature risks reformatting that breaks
        // sanity_check on the Rust side. Cap at 384 tokens — long enough
        // for any reasonable bullet list, short enough that a runaway
        // generation can't hold the pipeline hostage.
        let options = GenerationOptions(temperature: 0.2, maximumResponseTokens: 384)
        let response = try await session.respond(to: req.user, options: options)
        let tDone = Date()
        let initMs = Int(tAfterInit.timeIntervalSince(tStart) * 1000)
        let genMs = Int(tDone.timeIntervalSince(tAfterInit) * 1000)
        logErr("dicto-apple-polish: id=\(req.id) init=\(initMs)ms gen=\(genMs)ms total=\(initMs+genMs)ms")
        return PolishOk(id: req.id, text: response.content)
    } catch let err as LanguageModelSession.GenerationError {
        return PolishErr(id: req.id, error: "generation: \(err)")
    } catch {
        return PolishErr(id: req.id, error: "\(error)")
    }
}

/// Prewarm the Foundation Models framework so the first real polish call
/// doesn't pay the cold-load cost (model load + framework init ~ 1–2 s on
/// first use after device boot). We deliberately don't `respond` — just
/// instantiating a session is enough to trigger the load.
@available(macOS 26.0, *)
func prewarm() {
    let session = LanguageModelSession(instructions: Instructions("warmup"))
    session.prewarm()
    logErr("dicto-apple-polish: prewarm requested")
}

// MARK: - Main loop

@available(macOS 26.0, *)
func mainLoop() async {
    emitLine(ReadyMessage(availability: availabilityString()))

    // Kick off the model load in the background so the first real polish
    // call doesn't pay a ~1-2 s cold start.
    if availabilityString() == "available" {
        prewarm()
    }

    // If unavailable, stay alive so the host can read the ready message,
    // then close stdin and we exit naturally on EOF.
    let stdin = FileHandle.standardInput

    // Read line-by-line. We use a small accumulator because FileHandle
    // doesn't give a line-oriented API directly on this OS version.
    var buffer = Data()
    while true {
        let chunk = stdin.availableData
        if chunk.isEmpty {
            break // EOF — host closed the pipe.
        }
        buffer.append(chunk)

        // Split on newline.
        while let nl = buffer.firstIndex(of: 0x0A) {
            let line = buffer.subdata(in: 0..<nl)
            buffer.removeSubrange(0...nl)
            if line.isEmpty { continue }

            await handleLine(line)
        }
    }
}

@available(macOS 26.0, *)
func handleLine(_ line: Data) async {
    let decoder = JSONDecoder()
    do {
        let req = try decoder.decode(PolishRequest.self, from: line)
        let response: Encodable = await polish(req)
        if let ok = response as? PolishOk {
            emitLine(ok)
        } else if let err = response as? PolishErr {
            emitLine(err)
        }
    } catch {
        logErr("dicto-apple-polish: bad request — \(error)")
        // Emit a generic error so the host doesn't hang waiting.
        emitLine(PolishErr(id: "unknown", error: "bad request: \(error)"))
    }
}

// MARK: - Entry

if #available(macOS 26.0, *) {
    let task = Task { await mainLoop() }
    // RunLoop-style wait: block main thread until the async main loop exits.
    let semaphore = DispatchSemaphore(value: 0)
    Task {
        _ = await task.value
        semaphore.signal()
    }
    semaphore.wait()
} else {
    // Pre-26 path: announce unavailable, then exit. The Rust side reads
    // the ready message and skips this provider.
    print("{\"ready\":true,\"availability\":\"deviceNotEligible\"}")
}
