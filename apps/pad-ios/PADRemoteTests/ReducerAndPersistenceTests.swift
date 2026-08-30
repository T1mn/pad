import Foundation
import XCTest
@testable import PADRemote

private actor PairingMailboxProbe {
    private(set) var trace: [String] = []
    private(set) var cachedRevisions: [Int64] = [-1]
    private var checkpointInstalled = false

    func consume(_ delivery: String) async {
        switch delivery {
        case "paired":
            checkpointInstalled = true
            trace.append("checkpoint-installed")
            // Model the actor-backed cache/outbox/host commits performed by
            // completePairing. A following event must not overtake this await.
            try? await Task.sleep(for: .milliseconds(30))
            cachedRevisions.removeAll()
            trace.append("pairing-commit-finished")
        case "event-13":
            guard checkpointInstalled else {
                trace.append("event-without-checkpoint")
                return
            }
            cachedRevisions.append(13)
            trace.append("event-applied")
        default:
            trace.append("unexpected")
        }
    }

    func snapshot() -> (trace: [String], cachedRevisions: [Int64]) {
        (trace, cachedRevisions)
    }
}

final class ReducerAndPersistenceTests: XCTestCase {
    func testOrderedTransportMailboxAwaitsPairingCommitBeforeFollowingEvent() async {
        let probe = PairingMailboxProbe()
        let mailbox = OrderedAsyncMailbox<String> { delivery in
            await probe.consume(delivery)
        }

        mailbox.yield("paired")
        mailbox.yield("event-13")
        await mailbox.finishAndWait()

        let snapshot = await probe.snapshot()
        XCTAssertEqual(snapshot.trace, [
            "checkpoint-installed",
            "pairing-commit-finished",
            "event-applied",
        ])
        XCTAssertEqual(snapshot.cachedRevisions, [13])
    }

    func testRevisionReducerHandlesDuplicateGapAndEpoch() {
        var cursor = RevisionCursor()
        XCTAssertEqual(cursor.apply(epoch: "epoch-a", revision: 1), .newEpoch(previous: nil))
        XCTAssertEqual(cursor.apply(epoch: "epoch-a", revision: 1), .duplicate)
        XCTAssertEqual(cursor.apply(epoch: "epoch-a", revision: 3), .gap(expected: 2, received: 3))
        XCTAssertEqual(cursor.revision, 1)
        XCTAssertEqual(cursor.apply(epoch: "epoch-a", revision: 2), .accepted)
        XCTAssertEqual(cursor.apply(epoch: "epoch-b", revision: 8), .newEpoch(previous: "epoch-a"))
        XCTAssertEqual(cursor.revision, 8)
    }

    func testPairAndResyncCheckpointsAcceptTheNextRevision() {
        var paired = RevisionCursor()
        paired.resync(epoch: "paired-epoch", latestRevision: 12)
        XCTAssertEqual(paired.apply(epoch: "paired-epoch", revision: 13), .accepted)

        var restarted = RevisionCursor()
        restarted.resync(epoch: "old", latestRevision: 90)
        restarted.resync(epoch: "new", latestRevision: 7)
        XCTAssertEqual(restarted.apply(epoch: "new", revision: 8), .accepted)

        var gapRecovered = RevisionCursor()
        gapRecovered.resync(epoch: "stable", latestRevision: 2)
        XCTAssertEqual(gapRecovered.apply(epoch: "stable", revision: 5), .gap(expected: 3, received: 5))
        gapRecovered.resync(epoch: "stable", latestRevision: 5)
        XCTAssertEqual(gapRecovered.apply(epoch: "stable", revision: 6), .accepted)
    }

    func testPairedCheckpointBuffersEventAcrossIdentityResetAndDetectsOverflow() {
        var checkpoint = ResyncCheckpointBuffer(epoch: "paired", revision: 12)
        let first = RemoteFrame(
            type: .event,
            serverEpoch: "paired",
            revision: 13,
            kind: "task_output",
            payload: .object(["task_id": .string("task-1")])
        )
        XCTAssertFalse(checkpoint.capture(first))

        var resetCursor = RevisionCursor()
        resetCursor.resync(epoch: "paired", latestRevision: 12)
        let buffered = checkpoint.drainInRevisionOrder()
        XCTAssertEqual(buffered.map(\.revision), [13])
        XCTAssertEqual(resetCursor.apply(epoch: "paired", revision: 13), .accepted)

        var overflowing = ResyncCheckpointBuffer(epoch: "paired", revision: 12)
        for offset in 1 ... 1_000 {
            XCTAssertFalse(overflowing.capture(RemoteFrame(
                type: .event,
                serverEpoch: "paired",
                revision: Int64(12 + offset),
                kind: "noop"
            )))
        }
        XCTAssertTrue(overflowing.capture(RemoteFrame(
            type: .event,
            serverEpoch: "paired",
            revision: 1_013,
            kind: "task_output",
            payload: .object(["task_id": .string("attention")])
        )))
    }

    func testReconnectCapsAndFullJitter() {
        let schedule = ReconnectSchedule()
        XCTAssertEqual(schedule.delay(attempt: 0, randomUnit: 1), 0.25)
        XCTAssertEqual(schedule.delay(attempt: 1, randomUnit: 0.5), 0.25)
        XCTAssertEqual(schedule.delay(attempt: 5, randomUnit: 1), 8)
        XCTAssertEqual(schedule.delay(attempt: 6, randomUnit: 0), 30)
        XCTAssertEqual(schedule.delay(attempt: 99, randomUnit: 1), 60)
        XCTAssertEqual(schedule.delay(attempt: -2, randomUnit: 0), 0)
    }

    func testPermanentTransportFailuresStopWhileTimeoutRetries() {
        XCTAssertEqual(
            RemoteTransportFailurePolicy.decision(for: RemoteTransportError.subprotocolMismatch),
            .stop(
                message: "Mac 未确认 pad.remote.v1 安全子协议。",
                offersSettings: false
            )
        )
        XCTAssertEqual(
            RemoteTransportFailurePolicy.decision(for: URLError(.cannotFindHost)),
            .stop(
                message: "无法解析二维码中的 Mac 地址。请确认处于同一局域网，或重新生成二维码。",
                offersSettings: false
            )
        )
        XCTAssertEqual(RemoteTransportFailurePolicy.decision(for: URLError(.timedOut)), .retry)
        XCTAssertEqual(
            RemoteCommandTerminationPolicy.disposition(for: "remote_disabled"),
            .preservePairingAndStop
        )
        XCTAssertEqual(
            RemoteCommandTerminationPolicy.disposition(for: "device_revoked"),
            .forgetPairing
        )
    }

    func testHealthyOrInFlightConnectionIsNotReopened() {
        XCTAssertFalse(RemoteConnectionLifecyclePolicy.shouldOpen(state: .online, hasTransport: true))
        XCTAssertFalse(RemoteConnectionLifecyclePolicy.shouldOpen(state: .connecting, hasTransport: true))
        XCTAssertFalse(RemoteConnectionLifecyclePolicy.shouldOpen(state: .pairing, hasTransport: true))
        XCTAssertFalse(RemoteConnectionLifecyclePolicy.shouldOpen(state: .reconnecting(attempt: 1), hasTransport: true))
        XCTAssertTrue(RemoteConnectionLifecyclePolicy.shouldOpen(state: .offline, hasTransport: true))
        XCTAssertTrue(RemoteConnectionLifecyclePolicy.shouldOpen(state: .suspended, hasTransport: false))
        XCTAssertTrue(RemoteConnectionLifecyclePolicy.shouldOpen(state: .reconnecting(attempt: 1), hasTransport: false))
        XCTAssertFalse(RemoteStorageReadinessPolicy.canOpen(
            hasPendingInvitation: false,
            hasPairedHost: true,
            storageReady: false
        ))
        XCTAssertTrue(RemoteStorageReadinessPolicy.canOpen(
            hasPendingInvitation: true,
            hasPairedHost: false,
            storageReady: false
        ))
        XCTAssertNotEqual(
            MessageScrollSignal(id: "stable", textLength: 1),
            MessageScrollSignal(id: "stable", textLength: 2)
        )
    }

    func testKeychainInterfaceCanBeSubstitutedWithoutPersistence() throws {
        let store: SecureTokenStoring = InMemoryTokenStore()
        let token = Data("private-device-token".utf8)
        try store.save(token, account: "device:test")
        XCTAssertEqual(try store.read(account: "device:test"), token)
        try store.delete(account: "device:test")
        XCTAssertNil(try store.read(account: "device:test"))
    }

    func testOutboxEnqueueIsIdempotentAndKeepsStableUUID() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteTests-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("outbox.json")
        let store = OutboxStore(fileURL: file)
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "a", count: 64),
            deviceID: "device-a"
        )
        try await store.reset(for: identity)
        let id = UUID()
        let command = PendingCommand(id: id, action: .prompt, payload: .object(["prompt": .string("hello")]))

        _ = try await store.enqueue(command, for: identity)
        _ = try await store.enqueue(command, for: identity)
        let persisted = try await store.all(for: identity)
        XCTAssertEqual(persisted.count, 1)
        XCTAssertEqual(persisted.first?.id, id)

        try await store.removeSucceeded(id: id, for: identity)
        let afterRemoval = try await store.all(for: identity)
        XCTAssertTrue(afterRemoval.isEmpty)
    }

    func testOutboxRejectsOversizedHeadAndContinuesWithNextCommand() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteOversize-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = OutboxStore(fileURL: directory.appendingPathComponent("outbox.json"))
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "a", count: 64),
            deviceID: "device-a"
        )
        try await store.reset(for: identity)
        let oversized = PendingCommand(
            action: .prompt,
            payload: .object(["prompt": .string(String(repeating: "x", count: FrameCodec.maximumFrameBytes))])
        )
        do {
            _ = try await store.enqueue(oversized, for: identity)
            XCTFail("oversized command must be rejected")
        } catch let error as FrameCodecError {
            XCTAssertEqual(error, .tooLarge)
        }
        let valid = PendingCommand(action: .listSidebar, payload: .object([:]))
        _ = try await store.enqueue(valid, for: identity)
        let remainingAfterOversize = try await store.all(for: identity)
        XCTAssertEqual(remainingAfterOversize.map(\.id), [valid.id])
    }

    func testOutboxWritesAreTransactionalWhenDiskWriteFails() async throws {
        enum TestWriteError: Error { case denied }
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteAtomic-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("outbox.json")
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "b", count: 64),
            deviceID: "device-b"
        )
        let healthy = OutboxStore(fileURL: file)
        try await healthy.reset(for: identity)

        let failingInsert = OutboxStore(fileURL: file, writer: { _, _ in throw TestWriteError.denied })
        try await failingInsert.activate(for: identity)
        do {
            _ = try await failingInsert.enqueue(
                PendingCommand(action: .listSidebar, payload: .object([:])),
                for: identity
            )
            XCTFail("write failure must be surfaced")
        } catch TestWriteError.denied {}
        let afterFailedInsert = try await failingInsert.all(for: identity)
        XCTAssertTrue(afterFailedInsert.isEmpty)

        let command = PendingCommand(action: .bootstrap, payload: .object([:]))
        _ = try await healthy.enqueue(command, for: identity)
        let failingRemoval = OutboxStore(fileURL: file, writer: { _, _ in throw TestWriteError.denied })
        try await failingRemoval.activate(for: identity)
        do {
            try await failingRemoval.removeSucceeded(id: command.id, for: identity)
            XCTFail("remove failure must be surfaced")
        } catch TestWriteError.denied {}
        let afterFailedRemoval = try await failingRemoval.all(for: identity)
        XCTAssertEqual(afterFailedRemoval.map(\.id), [command.id])
    }

    func testOutboxPreservesTwoPromptTransactionsInAdmissionOrder() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteOrder-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = OutboxStore(fileURL: directory.appendingPathComponent("outbox.json"))
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "c", count: 64),
            deviceID: "device-c"
        )
        try await store.reset(for: identity)
        let first = PendingCommand.promptTransaction(taskID: "task", prompt: "第一条")
        let second = PendingCommand.promptTransaction(taskID: "task", prompt: "第二条")
        _ = try await store.enqueue(first, for: identity)
        _ = try await store.enqueue(second, for: identity)
        let prompts = try await store.all(for: identity).filter { $0.action == .prompt }
        XCTAssertEqual(prompts.compactMap { $0.payload.objectValue?["prompt"]?.stringValue }, ["第一条", "第二条"])
    }

    func testOutboxCountBudgetRejectsTheWholeBatch() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteOutboxBudget-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let store = OutboxStore(fileURL: directory.appendingPathComponent("outbox.json"))
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "e", count: 64),
            deviceID: "device-e"
        )
        try await store.reset(for: identity)
        let commands = (0 ..< 257).map { index in
            PendingCommand(
                id: UUID(),
                action: .listSidebar,
                payload: .object(["index": .number(Double(index))])
            )
        }
        do {
            _ = try await store.enqueue(commands, for: identity)
            XCTFail("the complete oversized batch must be rejected")
        } catch let error as OutboxCapacityError {
            XCTAssertEqual(error, .tooManyCommands)
        }
        let remaining = try await store.all(for: identity)
        XCTAssertTrue(remaining.isEmpty)
    }

    func testSwitchingFromHostAToHostBClearsOldCommandsBeforeFlush() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteHostIsolation-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("outbox.json")
        let hostA = RemoteHostIdentity(
            endpointAuthority: "a.local:47321",
            fingerprint: String(repeating: "a", count: 64),
            deviceID: "device-a"
        )
        let hostB = RemoteHostIdentity(
            endpointAuthority: "b.local:47322",
            fingerprint: String(repeating: "b", count: 64),
            deviceID: "device-b"
        )
        let store = OutboxStore(fileURL: file)
        try await store.reset(for: hostA)
        _ = try await store.enqueue(PendingCommand(
            action: .prompt,
            payload: .object(["task_id": .string("task-a"), "prompt": .string("只属于 A")])
        ), for: hostA)
        let hostACommands = try await store.all(for: hostA)
        XCTAssertEqual(hostACommands.count, 1)

        try await store.reset(for: hostB)
        let hostBCommands = try await store.all(for: hostB)
        XCTAssertTrue(hostBCommands.isEmpty)

        let afterRestart = OutboxStore(fileURL: file)
        try await afterRestart.activate(for: hostB)
        let reloadedHostBCommands = try await afterRestart.all(for: hostB)
        XCTAssertTrue(reloadedHostBCommands.isEmpty)
    }

    func testLateHostAEnqueueCannotEnterHostBOutbox() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteLateHostWrite-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let hostA = RemoteHostIdentity(
            endpointAuthority: "a.local:47321",
            fingerprint: String(repeating: "a", count: 64),
            deviceID: "device-a"
        )
        let hostB = RemoteHostIdentity(
            endpointAuthority: "b.local:47322",
            fingerprint: String(repeating: "b", count: 64),
            deviceID: "device-b"
        )
        let store = OutboxStore(fileURL: directory.appendingPathComponent("outbox.json"))
        try await store.reset(for: hostA)
        try await store.reset(for: hostB)

        do {
            _ = try await store.enqueue(PendingCommand(
                action: .prompt,
                payload: .object(["task_id": .string("task-a"), "prompt": .string("不得泄漏给 B")])
            ), for: hostA)
            XCTFail("an old identity must never mutate the active outbox")
        } catch let error as BoundStoreError {
            guard case .staleIdentity = error else {
                XCTFail("expected staleIdentity, got \(error)")
                return
            }
        }
        let hostBCommands = try await store.all(for: hostB)
        XCTAssertTrue(hostBCommands.isEmpty)
    }

    func testCacheFromHostAIsNeverShownAfterBindingHostB() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteCacheIsolation-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("cache.json")
        let hostA = RemoteHostIdentity(
            endpointAuthority: "a.local:47321",
            fingerprint: String(repeating: "a", count: 64),
            deviceID: "device-a"
        )
        let hostB = RemoteHostIdentity(
            endpointAuthority: "b.local:47322",
            fingerprint: String(repeating: "b", count: 64),
            deviceID: "device-b"
        )
        let store = CacheStore(fileURL: file)
        _ = try await store.reset(for: hostA)
        var hostAState = CachedRemoteState()
        hostAState.tasks = [RemoteTaskSummary(id: "task-a", title: "A", status: .idle, updatedAt: Date())]
        try await store.save(hostAState, for: hostA)

        let hostBState = try await store.reset(for: hostB)
        XCTAssertTrue(hostBState.tasks.isEmpty)
        let afterRestart = CacheStore(fileURL: file)
        let reloaded = try await afterRestart.activate(for: hostB)
        XCTAssertTrue(reloaded.tasks.isEmpty)
    }

    func testRapidRevisionPersistenceCannotRegressTheCachedCursor() async throws {
        let directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("PADRemoteRevisionOrder-\(UUID().uuidString)", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let file = directory.appendingPathComponent("cache.json")
        let identity = RemoteHostIdentity(
            endpointAuthority: "mac.local:47321",
            fingerprint: String(repeating: "d", count: 64),
            deviceID: "device-d"
        )
        let store = CacheStore(fileURL: file)
        _ = try await store.reset(for: identity)
        var newer = CachedRemoteState()
        newer.cursor.resync(epoch: "epoch", latestRevision: 12)
        var older = CachedRemoteState()
        older.cursor.resync(epoch: "epoch", latestRevision: 11)

        let wroteNewer = try await store.save(newer, for: identity, sequence: 2)
        let wroteOlder = try await store.save(older, for: identity, sequence: 1)
        XCTAssertTrue(wroteNewer)
        XCTAssertFalse(wroteOlder)
        let reloaded = try await CacheStore(fileURL: file).activate(for: identity)
        XCTAssertEqual(reloaded.cursor.revision, 12)
    }

    func testCacheBudgetKeepsSelectedTaskAndBoundsMessages() throws {
        let tasks = (0 ..< 300).map {
            RemoteTaskSummary(id: "task-\($0)", title: "任务 \($0)", status: .idle, updatedAt: Date())
        }
        let message = RemoteMessage(
            id: "seed",
            role: .assistant,
            text: String(repeating: "中", count: 2_000),
            createdAt: Date(),
            isStreaming: true
        )
        var messages: [String: [RemoteMessage]] = [:]
        for task in tasks {
            let count = task.id == "task-0" ? 320 : 4
            messages[task.id] = (0 ..< count).map { index in
                var copy = message
                copy.text += "-\(index)"
                return RemoteMessage(
                    id: "\(task.id)-\(index)",
                    role: copy.role,
                    text: copy.text,
                    createdAt: copy.createdAt,
                    isStreaming: copy.isStreaming
                )
            }
        }
        let trimmed = CachedStateBudget.trim(CachedRemoteState(
            tasks: tasks,
            selectedTaskID: "task-299",
            messagesByTask: messages,
            liveStreamsByTask: [
                "task-299": RemoteLiveStreamState(messageID: "live", textBlocks: [0: "partial"], startedAt: Date()),
            ]
        ))
        XCTAssertLessThanOrEqual(trimmed.tasks.count, CachedStateBudget.maximumTasks)
        XCTAssertTrue(trimmed.tasks.contains(where: { $0.id == "task-299" }))
        XCTAssertTrue(trimmed.messagesByTask.values.allSatisfy { $0.count <= CachedStateBudget.maximumMessagesPerTask })
        XCTAssertTrue(trimmed.liveStreamsByTask.isEmpty)
        let totalCharacters = trimmed.messagesByTask.values.flatMap { $0 }.reduce(0) { $0 + $1.text.count }
        XCTAssertLessThanOrEqual(totalCharacters, CachedStateBudget.maximumTextCharacters)
        XCTAssertTrue(trimmed.messagesByTask.values.flatMap { $0 }.allSatisfy { !$0.isStreaming })
        XCTAssertLessThan(try JSONEncoder().encode(trimmed).count, CachedStateBudget.maximumEnvelopeBytes)
    }

    func testTransientDesktopErrorsRetainTheOutboxCommandForReconnect() {
        XCTAssertTrue(RemoteCommandErrorPolicy.shouldRetainOutbox(code: "server_busy"))
        XCTAssertTrue(RemoteCommandErrorPolicy.shouldRetainOutbox(code: "server_unavailable"))
        XCTAssertTrue(RemoteCommandErrorPolicy.shouldRetainOutbox(code: "command_timeout"))
        XCTAssertFalse(RemoteCommandErrorPolicy.shouldRetainOutbox(code: "command_outcome_unknown"))
        XCTAssertEqual(
            RemoteCommandErrorPolicy.disposition(for: "command_outcome_unknown"),
            .resyncWithoutRetry
        )
        XCTAssertFalse(RemoteCommandErrorPolicy.shouldRetainOutbox(code: "task_not_found"))
        XCTAssertFalse(RemoteCommandErrorPolicy.shouldRetainOutbox(code: nil))
    }

    func testHistoryReducerReadsNestedTextBlocksAndMillisecondDates() {
        let timestamp = 1_700_000_000_000.0
        let result: JSONValue = .object([
            "task_id": .string("task-1"),
            "messages": .array([
                .object([
                    "id": .string("m1"),
                    "kind": .string("user_message"),
                    "content": .array([
                        .object(["type": .string("input_text"), "text": .string("第一段")]),
                        .object(["type": .string("input_text"), "text": .string("第二段")]),
                    ]),
                    "created_at": .number(timestamp),
                ]),
            ]),
        ])
        var state = CachedRemoteState()
        RemoteContentReducer.apply(result: result, action: .history, to: &state)
        XCTAssertEqual(state.messagesByTask["task-1"]?.first?.text, "第一段\n第二段")
        XCTAssertEqual(state.messagesByTask["task-1"]?.first?.role, .user)
        let createdAt = try? XCTUnwrap(state.messagesByTask["task-1"]?.first?.createdAt.timeIntervalSince1970)
        XCTAssertEqual(createdAt ?? 0, 1_700_000_000, accuracy: 0.1)
    }

    func testPendingEmptyHistoryPreservesCacheAndOptimisticBubbleUntilAcknowledged() {
        var state = CachedRemoteState(
            tasks: [RemoteTaskSummary(id: "task", title: "任务", status: .running, updatedAt: Date())],
            selectedTaskID: "task",
            messagesByTask: [
                "task": [RemoteMessage(
                    id: "server-old",
                    role: .assistant,
                    text: "已有内容",
                    createdAt: Date(),
                    isStreaming: false
                )],
            ]
        )
        let optimistic = RemoteMessage(
            id: "local-command",
            role: .user,
            text: "离线问题",
            createdAt: Date(),
            isStreaming: false
        )
        RemoteContentReducer.addOptimisticMessage(optimistic, taskID: "task", to: &state)

        RemoteContentReducer.apply(
            result: .object([
                "task_id": .string("task"),
                "pending": .bool(true),
                "messages": .array([]),
            ]),
            action: .history,
            to: &state
        )
        XCTAssertEqual(state.messagesByTask["task"]?.map(\.id), ["server-old", "local-command"])

        RemoteContentReducer.apply(
            result: .object([
                "task_id": .string("task"),
                "pending": .bool(false),
                "messages": .array([
                    .object([
                        "id": .string("server-user"),
                        "role": .string("user"),
                        "content": .string("离线问题"),
                    ]),
                ]),
            ]),
            action: .history,
            to: &state
        )
        XCTAssertEqual(state.messagesByTask["task"]?.map(\.id), ["server-user"])
        XCTAssertNil(state.pendingLocalMessageIDsByTask["task"])
    }

    func testPermanentPromptFailureRemovesOnlyItsOptimisticBubble() {
        let keep = RemoteMessage(
            id: "local-keep",
            role: .user,
            text: "保留",
            createdAt: Date(),
            isStreaming: false
        )
        let remove = RemoteMessage(
            id: "local-remove",
            role: .user,
            text: "失败",
            createdAt: Date(),
            isStreaming: false
        )
        var state = CachedRemoteState()
        RemoteContentReducer.addOptimisticMessage(keep, taskID: "task", to: &state)
        RemoteContentReducer.addOptimisticMessage(remove, taskID: "task", to: &state)
        RemoteContentReducer.removeOptimisticMessage(id: remove.id, taskID: "task", from: &state)
        XCTAssertEqual(state.messagesByTask["task"]?.map(\.id), [keep.id])
        XCTAssertEqual(state.pendingLocalMessageIDsByTask["task"], [keep.id])
    }

    func testAuthoritativeRecoveryDiscardsAStaleLiveBubble() {
        var state = CachedRemoteState()
        let output: JSONValue = .object([
            "task_id": .string("task"),
            "poll": .object(["events": .array([
                .object([
                    "type": .string("message_update"),
                    "assistantMessageEvent": .object([
                        "type": .string("text_delta"),
                        "contentIndex": .number(0),
                        "delta": .string("未结束"),
                    ]),
                ]),
            ])]),
        ])
        _ = RemoteContentReducer.applyTaskOutput(output, to: &state)
        XCTAssertEqual(state.messagesByTask["task"]?.last?.isStreaming, true)
        RemoteContentReducer.discardLiveStreamsForAuthoritativeRecovery(&state)
        RemoteContentReducer.apply(
            result: .object([
                "task_id": .string("task"),
                "messages": .array([
                    .object(["id": .string("final"), "role": .string("assistant"), "content": .string("权威")]),
                ]),
            ]),
            action: .history,
            to: &state
        )
        XCTAssertNil(state.liveStreamsByTask["task"])
        XCTAssertEqual(state.messagesByTask["task"]?.map(\.id), ["final"])
    }

    func testBootstrapSnapshotReplacesGhostTasksAndRestoresPendingInteraction() {
        let oldMessage = RemoteMessage(
            id: "old-message",
            role: .assistant,
            text: "旧内容",
            createdAt: Date(),
            isStreaming: false
        )
        var state = CachedRemoteState(
            tasks: [
                RemoteTaskSummary(id: "task-a", title: "A", status: .idle, updatedAt: Date()),
                RemoteTaskSummary(id: "task-b", title: "B", status: .idle, updatedAt: Date()),
            ],
            selectedTaskID: "task-a",
            messagesByTask: ["task-a": [oldMessage]],
            pendingUIRequestsByTask: [
                "task-a": [RemoteUIRequest(
                    id: "old-request",
                    kind: .confirm,
                    title: "旧请求",
                    message: nil,
                    options: [],
                    defaultIndex: nil,
                    defaultValue: nil,
                    requiresResponse: true
                )],
            ]
        )
        let snapshot: JSONValue = .object([
            "records": .object([
                "profiles": .array([
                    .object(["id": .string("profile-1"), "name": .string("配置")]),
                ]),
                "projects": .array([
                    .object(["id": .string("project-1"), "name": .string("项目")]),
                ]),
                "tasks": .array([
                    .object([
                        "id": .string("task-b"),
                        "title": .string("B 最新"),
                        "status": .string("needs_input"),
                        "pending_ui_requests": .array([
                            .object([
                                "id": .string("input-1"),
                                "kind": .string("input"),
                                "response_action": .string("respond_ui"),
                                "requires_response": .bool(true),
                                "title": .string("请输入"),
                            ]),
                        ]),
                    ]),
                ]),
            ]),
        ])
        RemoteContentReducer.apply(result: snapshot, action: .bootstrap, to: &state)
        XCTAssertEqual(state.tasks.map(\.id), ["task-b"])
        XCTAssertEqual(state.selectedTaskID, "task-b")
        XCTAssertNil(state.messagesByTask["task-a"])
        XCTAssertNil(state.pendingUIRequestsByTask["task-a"])
        XCTAssertEqual(state.pendingUIRequestsByTask["task-b"]?.first?.id, "input-1")
        XCTAssertEqual(
            BootstrapFollowupPolicy.historyTaskID(previous: nil, current: state.selectedTaskID),
            "task-b"
        )
    }

    func testTaskOutputStreamsStartTwoDeltasAndAuthoritativeEndWithStableID() throws {
        var state = CachedRemoteState()
        let base: [String: JSONValue] = [
            "task_id": .string("task-live"),
            "task": .object([
                "id": .string("task-live"),
                "title": .string("实时任务"),
                "status": .string("running"),
            ]),
        ]
        let pending: JSONValue = .array([
            .object([
                "id": .string("confirm-1"),
                "kind": .string("confirm"),
                "response_action": .string("respond_ui"),
                "requires_response": .bool(true),
                "title": .string("继续？"),
            ]),
        ])

        func payload(event: JSONValue, includePending: Bool = false) -> JSONValue {
            var value = base
            var poll: [String: JSONValue] = ["events": .array([event])]
            if includePending { poll["pending_ui_requests"] = pending }
            value["poll"] = .object(poll)
            return .object(value)
        }

        let started = RemoteContentReducer.applyTaskOutput(payload(event: .object([
            "type": .string("message_start"),
            "message": .object([
                "role": .string("assistant"),
                "content": .array([]),
                "timestamp": .number(1_700_000_000_000),
            ]),
        ]), includePending: true), to: &state)
        XCTAssertEqual(started.taskID, "task-live")
        XCTAssertFalse(started.shouldRefreshHistory)
        let streamingID = try XCTUnwrap(state.messagesByTask["task-live"]?.first?.id)
        XCTAssertTrue(state.messagesByTask["task-live"]?.first?.isStreaming == true)

        _ = RemoteContentReducer.applyTaskOutput(payload(event: .object([
            "type": .string("message_update"),
            "assistantMessageEvent": .object([
                "type": .string("text_delta"),
                "contentIndex": .number(0),
                "delta": .string("实时"),
            ]),
        ])), to: &state)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.id, streamingID)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.text, "实时")

        _ = RemoteContentReducer.applyTaskOutput(payload(event: .object([
            "type": .string("message_update"),
            "assistantMessageEvent": .object([
                "type": .string("text_delta"),
                "contentIndex": .number(0),
                "delta": .string("输出"),
            ]),
        ])), to: &state)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.id, streamingID)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.text, "实时输出")

        let ended = RemoteContentReducer.applyTaskOutput(payload(event: .object([
            "type": .string("message_end"),
            "message": .object([
                "role": .string("assistant"),
                "content": .array([
                    .object(["type": .string("text"), "text": .string("最终权威文本")]),
                ]),
                "timestamp": .number(1_700_000_001_000),
            ]),
        ])), to: &state)
        XCTAssertTrue(ended.shouldRefreshHistory)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.id, streamingID)
        XCTAssertEqual(state.messagesByTask["task-live"]?.first?.text, "最终权威文本")
        XCTAssertFalse(state.messagesByTask["task-live"]?.first?.isStreaming == true)
        XCTAssertNil(state.liveStreamsByTask["task-live"])
        XCTAssertEqual(state.pendingUIRequestsByTask["task-live"]?.first?.kind, .confirm)
        XCTAssertEqual(state.tasks.first?.status, .running)
    }

    func testTaskOutputLegacyEmptyPendingDoesNotClearButAuthoritativeEmptyDoes() {
        let request = RemoteUIRequest(
            id: "input-1",
            kind: .input,
            title: "输入",
            message: nil,
            options: [],
            defaultIndex: nil,
            defaultValue: nil,
            requiresResponse: true,
            placeholder: "只作为提示"
        )
        var state = CachedRemoteState(pendingUIRequestsByTask: ["task": [request]])
        _ = RemoteContentReducer.applyTaskOutput(.object([
            "task_id": .string("task"),
            "poll": .object(["pending_ui_requests": .array([]), "events": .array([])]),
        ]), to: &state)
        XCTAssertEqual(state.pendingUIRequestsByTask["task"]?.map(\.id), [request.id])

        _ = RemoteContentReducer.applyTaskOutput(.object([
            "task_id": .string("task"),
            "pending_ui_requests": .array([]),
            "poll": .object(["pending_ui_requests": .array([]), "events": .array([])]),
        ]), to: &state)
        XCTAssertEqual(state.pendingUIRequestsByTask["task"], [])
    }

    func testPendingInputSeparatesPlaceholderAndEditorPrefill() {
        let requests = RemoteContentReducer.uiRequests(in: .array([
            .object([
                "id": .string("input"),
                "kind": .string("input"),
                "response_action": .string("respond_ui"),
                "requires_response": .bool(true),
                "placeholder": .string("例如：说明原因"),
            ]),
            .object([
                "id": .string("editor"),
                "kind": .string("editor"),
                "response_action": .string("respond_ui"),
                "requires_response": .bool(true),
                "prefill": .string("已有文本"),
            ]),
        ]))
        XCTAssertEqual(requests[0].placeholder, "例如：说明原因")
        XCTAssertNil(requests[0].defaultValue)
        XCTAssertEqual(requests[1].defaultValue, "已有文本")
    }

    func testLateHistoryCannotEraseOrDuplicateAnActiveDeltaStream() throws {
        var state = CachedRemoteState()
        func output(_ event: JSONValue) -> JSONValue {
            .object([
                "task_id": .string("task-stream"),
                "poll": .object(["events": .array([event])]),
            ])
        }
        _ = RemoteContentReducer.applyTaskOutput(output(.object([
            "type": .string("message_start"),
            "message": .object([
                "role": .string("assistant"),
                "content": .array([]),
                "timestamp": .number(1_700_000_000_000),
            ]),
        ])), to: &state)
        for delta in ["A", "B"] {
            _ = RemoteContentReducer.applyTaskOutput(output(.object([
                "type": .string("message_update"),
                "assistantMessageEvent": .object([
                    "type": .string("text_delta"),
                    "contentIndex": .number(0),
                    "delta": .string(delta),
                ]),
            ])), to: &state)
        }
        let stableID = try XCTUnwrap(state.messagesByTask["task-stream"]?.last?.id)

        RemoteContentReducer.apply(
            result: .object([
                "task_id": .string("task-stream"),
                "messages": .array([
                    .object([
                        "id": .string("stale-partial"),
                        "role": .string("assistant"),
                        "content": .string("A"),
                    ]),
                ]),
            ]),
            action: .history,
            to: &state
        )
        XCTAssertEqual(state.messagesByTask["task-stream"]?.last?.id, stableID)
        XCTAssertEqual(state.messagesByTask["task-stream"]?.last?.text, "AB")

        _ = RemoteContentReducer.applyTaskOutput(output(.object([
            "type": .string("message_update"),
            "assistantMessageEvent": .object([
                "type": .string("text_delta"),
                "contentIndex": .number(0),
                "delta": .string("C"),
            ]),
        ])), to: &state)
        _ = RemoteContentReducer.applyTaskOutput(output(.object([
            "type": .string("message_end"),
            "message": .object([
                "role": .string("assistant"),
                "content": .array([.object(["type": .string("text"), "text": .string("ABC")])]),
            ]),
        ])), to: &state)

        let finalMatches = state.messagesByTask["task-stream"]?.filter { $0.text == "ABC" } ?? []
        XCTAssertEqual(finalMatches.count, 1)
        XCTAssertEqual(finalMatches.first?.id, stableID)
        XCTAssertFalse(finalMatches.first?.isStreaming == true)
    }

    func testLateGetMessagesControlResponseBecomesAuthoritativeHistory() {
        let payload: JSONValue = .object([
            "task_id": .string("task-late"),
            "poll": .object([
                "messages": .array([
                    .object([
                        "type": .string("response"),
                        "id": .string("rpc-1"),
                        "value": .object([
                            "command": .string("get_messages"),
                            "success": .bool(true),
                            "data": .object([
                                "messages": .array([
                                    .object([
                                        "id": .string("m-final"),
                                        "role": .string("assistant"),
                                        "content": .array([
                                            .object(["type": .string("text"), "text": .string("迟到的权威历史")]),
                                        ]),
                                    ]),
                                ]),
                            ]),
                        ]),
                    ]),
                ]),
            ]),
        ])
        var state = CachedRemoteState()
        let reduction = RemoteContentReducer.applyTaskOutput(payload, to: &state)
        XCTAssertEqual(reduction.taskID, "task-late")
        XCTAssertEqual(state.messagesByTask["task-late"]?.first?.id, "m-final")
        XCTAssertEqual(state.messagesByTask["task-late"]?.first?.text, "迟到的权威历史")
    }

    func testBackgroundTaskRefreshNeverChangesCurrentSelection() {
        var state = CachedRemoteState(
            tasks: [
                RemoteTaskSummary(id: "task-a", title: "A", status: .running, updatedAt: Date()),
                RemoteTaskSummary(id: "task-b", title: "B", status: .running, updatedAt: Date()),
            ],
            selectedTaskID: "task-b"
        )
        let output: JSONValue = .object([
            "task_id": .string("task-a"),
            "task": .object(["id": .string("task-a"), "title": .string("A updated"), "status": .string("streaming")]),
            "poll": .object(["messages": .array([])]),
        ])
        _ = RemoteContentReducer.applyTaskOutput(output, to: &state)
        XCTAssertEqual(state.selectedTaskID, "task-b")

        RemoteContentReducer.apply(
            result: .object(["task_id": .string("task-a"), "messages": .array([])]),
            action: .history,
            to: &state
        )
        XCTAssertEqual(state.selectedTaskID, "task-b")
    }

    func testInteractionCommandKeepsOwningTaskAndWireFields() {
        let promptTransaction = PendingCommand.promptTransaction(
            taskID: "cached-running",
            prompt: "继续",
            localMessageID: "local-prompt"
        )
        XCTAssertEqual(promptTransaction.map(\.action), [.startTask, .prompt])
        XCTAssertEqual(promptTransaction[0].payload.objectValue?["task_id"]?.stringValue, "cached-running")
        XCTAssertEqual(promptTransaction[1].payload.objectValue?["prompt"]?.stringValue, "继续")
        XCTAssertEqual(promptTransaction[1].localMessageID, "local-prompt")

        let request = RemoteUIRequest(
            id: "confirm-1",
            kind: .confirm,
            title: "继续？",
            message: nil,
            options: [],
            defaultIndex: nil,
            defaultValue: nil,
            requiresResponse: true
        )
        let command = PendingCommand.respondUI(taskID: "original-task", request: request, value: .bool(true))
        XCTAssertEqual(command.action, .respondUI)
        XCTAssertEqual(command.payload.objectValue?["task_id"]?.stringValue, "original-task")
        XCTAssertEqual(command.payload.objectValue?["request_id"]?.stringValue, "confirm-1")
        XCTAssertEqual(command.payload.objectValue?["response_kind"]?.stringValue, "confirm")
        XCTAssertEqual(command.payload.objectValue?["cancelled"], .bool(false))
        XCTAssertEqual(command.payload.objectValue?["value"], .bool(true))

        let cancelled = PendingCommand.cancelUI(taskID: "original-task", request: request)
        XCTAssertEqual(cancelled.payload.objectValue?["task_id"]?.stringValue, "original-task")
        XCTAssertEqual(cancelled.payload.objectValue?["request_id"]?.stringValue, "confirm-1")
        XCTAssertEqual(cancelled.payload.objectValue?["cancelled"], .bool(true))
        XCTAssertNil(cancelled.payload.objectValue?["value"])

        let select = RemoteUIRequest(
            id: "select-1",
            kind: .select,
            title: "选择",
            message: nil,
            options: ["第一项", "第二项"],
            defaultIndex: 1,
            defaultValue: nil,
            requiresResponse: true
        )
        let selectCommand = PendingCommand.respondUI(
            taskID: "original-task",
            request: select,
            value: .string(select.options[1])
        )
        XCTAssertEqual(selectCommand.payload.objectValue?["value"], .string("第二项"))

        let create = PendingCommand.createTask(taskID: "stable-task-id")
        XCTAssertEqual(create.payload.objectValue?["task_id"]?.stringValue, "stable-task-id")
    }
}
