import Foundation
import XCTest
@testable import PADRemote

final class PairingAndProtocolTests: XCTestCase {
    private let secret = String(repeating: "A", count: 43)
    private let fingerprint = String(repeating: "a", count: 64)

    func testParsesRustV1PairingURI() throws {
        let uri = makeURI(endpoint: "wss://192.168.1.24:47321")
        let invitation = try PairingInvitation(uri: uri)
        XCTAssertEqual(invitation.endpoint.absoluteString, "wss://192.168.1.24:47321")
        XCTAssertEqual(invitation.fingerprint, fingerprint)
        XCTAssertEqual(invitation.pairingID, "pair-123")
        XCTAssertEqual(invitation.secret, secret)
    }

    func testRejectsDuplicateAndNonWSSFields() {
        let duplicate = makeURI(endpoint: "wss://mac.local:47321") + "&secret=\(secret)"
        XCTAssertThrowsError(try PairingInvitation(uri: duplicate)) { error in
            XCTAssertEqual(error as? PairingURIError, .duplicateField("secret"))
        }
        XCTAssertThrowsError(try PairingInvitation(uri: makeURI(endpoint: "ws://mac.local:47321")))
    }

    func testWireCommandUsesParamsAndRoundTrips() throws {
        let id = UUID()
        let frame = RemoteFrame(
            type: .command,
            commandID: id,
            action: .prompt,
            params: .object(["task_id": .string("task-1"), "prompt": .string("你好")])
        )
        let data = try FrameCodec().encode(frame)
        let object = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        XCTAssertNotNil(object["params"])
        XCTAssertNil(object["payload"])
        XCTAssertEqual(try FrameCodec().decode(data), frame)
    }

    func testCodecEnforcesOneMiBForEncodeAndDecode() throws {
        let oversized = RemoteFrame(
            type: .command,
            commandID: UUID(),
            action: .prompt,
            params: .object(["prompt": .string(String(repeating: "x", count: FrameCodec.maximumFrameBytes))])
        )
        XCTAssertThrowsError(try FrameCodec().encode(oversized)) { error in
            XCTAssertEqual(error as? FrameCodecError, .tooLarge)
        }
        XCTAssertThrowsError(try FrameCodec().decode(Data(repeating: 0, count: FrameCodec.maximumFrameBytes + 1)))
    }

    func testStructuredAndLegacyWireErrorsDecode() throws {
        let codec = FrameCodec()
        let object = Data(#"{"type":"command_result","command_id":"00000000-0000-0000-0000-000000000001","ok":false,"error":{"code":"task_not_found","message":"missing"}}"#.utf8)
        XCTAssertEqual(try codec.decode(object).error, RemoteWireError(code: "task_not_found", message: "missing"))
        let legacy = Data(#"{"type":"command_result","command_id":"00000000-0000-0000-0000-000000000001","ok":false,"error":"missing"}"#.utf8)
        XCTAssertEqual(try codec.decode(legacy).error?.message, "missing")
    }

    private func makeURI(endpoint: String) -> String {
        var components = URLComponents()
        components.scheme = "pad"
        components.host = "remote"
        components.path = "/pair"
        components.queryItems = [
            URLQueryItem(name: "v", value: "1"),
            URLQueryItem(name: "endpoint", value: endpoint),
            URLQueryItem(name: "fingerprint", value: fingerprint),
            URLQueryItem(name: "pairing_id", value: "pair-123"),
            URLQueryItem(name: "secret", value: secret),
        ]
        return components.url!.absoluteString
    }
}
