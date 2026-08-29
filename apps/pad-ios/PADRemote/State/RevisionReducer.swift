import Foundation

enum RevisionDecision: Equatable, Sendable {
    case accepted
    case duplicate
    case gap(expected: Int64, received: Int64)
    case newEpoch(previous: String?)
}

struct RevisionCursor: Codable, Equatable, Sendable {
    private(set) var serverEpoch: String?
    private(set) var revision: Int64 = 0

    mutating func welcome(epoch: String, revision: Int64) {
        serverEpoch = epoch
        self.revision = revision
    }

    mutating func resync(epoch: String, latestRevision: Int64) {
        serverEpoch = epoch
        revision = max(0, latestRevision)
    }

    mutating func apply(epoch: String, revision incoming: Int64) -> RevisionDecision {
        guard serverEpoch == epoch else {
            let previous = serverEpoch
            serverEpoch = epoch
            revision = incoming
            return .newEpoch(previous: previous)
        }
        guard incoming > revision else { return .duplicate }
        guard incoming == revision + 1 else {
            return .gap(expected: revision + 1, received: incoming)
        }
        revision = incoming
        return .accepted
    }
}

struct ResyncCheckpointBuffer: Equatable, Sendable {
    let epoch: String
    let revision: Int64
    private(set) var events: [RemoteFrame] = []

    @discardableResult
    mutating func capture(_ frame: RemoteFrame) -> Bool {
        guard frame.serverEpoch == epoch,
              let incoming = frame.revision,
              incoming > revision else { return false }
        guard events.count < 1_000 else { return true }
        events.append(frame)
        return false
    }

    mutating func drainInRevisionOrder() -> [RemoteFrame] {
        defer { events.removeAll() }
        return events.sorted { ($0.revision ?? 0) < ($1.revision ?? 0) }
    }
}

struct ReconnectSchedule: Equatable, Sendable {
    static let caps: [TimeInterval] = [0.25, 0.5, 1, 2, 4, 8]

    /// Full jitter: a uniformly distributed delay in [0, cap].
    func delay(attempt: Int, randomUnit: Double = Double.random(in: 0 ... 1)) -> TimeInterval {
        let normalizedAttempt = max(attempt, 0)
        let unit = min(max(randomUnit, 0), 1)
        if normalizedAttempt < Self.caps.count {
            return Self.caps[normalizedAttempt] * unit
        }
        // After the six rapid recovery attempts, cool down to 30...60s so a
        // disabled Mac gateway cannot cause an 8-second foreground loop.
        return 30 + (30 * unit)
    }
}
