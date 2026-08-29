import Foundation

enum RemoteProtocolConstants {
    static let version = 1
    static let webSocketSubprotocol = "pad.remote.v1"
    static let maximumFrameBytes = 1_048_576
}

enum PairingURIError: LocalizedError, Equatable {
    case invalidRoute
    case unsupportedVersion
    case missingField(String)
    case duplicateField(String)
    case unknownField(String)
    case tooLong
    case invalidEndpoint
    case invalidFingerprint
    case invalidSecret

    var errorDescription: String? {
        switch self {
        case .invalidRoute: return "这不是 PAD Remote 配对链接。"
        case .unsupportedVersion: return "配对链接版本不受支持。"
        case let .missingField(field): return "配对链接缺少字段：\(field)。"
        case let .duplicateField(field): return "配对链接包含重复字段：\(field)。"
        case let .unknownField(field): return "配对链接包含未知字段：\(field)。"
        case .tooLong: return "配对链接过长。请在 Mac 上重新生成二维码。"
        case .invalidEndpoint: return "Mac 的安全连接地址无效。"
        case .invalidFingerprint: return "证书指纹无效。"
        case .invalidSecret: return "配对密钥无效。请在 Mac 上重新生成二维码。"
        }
    }
}

/// A one-use pairing invitation. Never persist or log this value: it contains `secret`.
struct PairingInvitation: Equatable, Sendable {
    let endpoint: URL
    let fingerprint: String
    let pairingID: String
    let secret: String

    init(uri: String) throws {
        let trimmed = uri.trimmingCharacters(in: .whitespacesAndNewlines)
        guard trimmed.utf8.count <= 8_192 else { throw PairingURIError.tooLong }
        guard let components = URLComponents(string: trimmed),
              components.scheme?.lowercased() == "pad",
              components.host?.lowercased() == "remote",
              components.path == "/pair" else {
            throw PairingURIError.invalidRoute
        }
        var query: [String: String] = [:]
        for item in components.queryItems ?? [] {
            let key = item.name.lowercased()
            guard ["v", "endpoint", "fingerprint", "pairing_id", "secret"].contains(key) else {
                throw PairingURIError.unknownField(key)
            }
            guard query[key] == nil else { throw PairingURIError.duplicateField(key) }
            guard let value = item.value else { throw PairingURIError.missingField(key) }
            query[key] = value
        }
        func required(_ key: String) throws -> String {
            guard let value = query[key], !value.isEmpty else {
                throw PairingURIError.missingField(key)
            }
            return value
        }
        guard try required("v") == String(RemoteProtocolConstants.version) else {
            throw PairingURIError.unsupportedVersion
        }
        let endpointString = try required("endpoint")
        guard let endpoint = URL(string: endpointString),
              endpointString.utf8.count <= 2_048,
              endpoint.scheme?.lowercased() == "wss",
              endpoint.host != nil,
              endpoint.port.map({ (1 ... 65_535).contains($0) }) == true,
              endpoint.user == nil,
              endpoint.password == nil,
              endpoint.path.isEmpty || endpoint.path == "/",
              endpoint.query == nil,
              endpoint.fragment == nil else {
            throw PairingURIError.invalidEndpoint
        }

        let fingerprintRaw = try required("fingerprint")
        let lowercaseHex = CharacterSet(charactersIn: "0123456789abcdef")
        guard fingerprintRaw.utf8.count == 64,
              fingerprintRaw.unicodeScalars.allSatisfy(lowercaseHex.contains) else {
            throw PairingURIError.invalidFingerprint
        }
        let secret = try required("secret")
        guard secret.count == 43,
              let decodedSecret = Self.decodeBase64URLNoPadding(secret), decodedSecret.count == 32 else {
            throw PairingURIError.invalidSecret
        }

        self.endpoint = endpoint
        self.fingerprint = fingerprintRaw
        let pairingID = try required("pairing_id")
        guard pairingID.utf8.count <= 256 else { throw PairingURIError.tooLong }
        self.pairingID = pairingID
        self.secret = secret
    }

    private static func decodeBase64URLNoPadding(_ value: String) -> Data? {
        let allowed = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")
        guard !value.contains("="),
              value.unicodeScalars.allSatisfy(allowed.contains) else {
            return nil
        }
        let normalized = value.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/")
        let padding = String(repeating: "=", count: (4 - normalized.count % 4) % 4)
        return Data(base64Encoded: normalized + padding)
    }

}

/// Safe-to-persist host metadata. Secrets and device tokens are deliberately absent.
struct PairedHost: Codable, Equatable, Sendable {
    let endpoint: URL
    let fingerprint: String
    let deviceID: String
    var displayName: String

    var identity: RemoteHostIdentity {
        RemoteHostIdentity(
            endpointAuthority: "\(endpoint.host?.lowercased() ?? ""):\(endpoint.port ?? 0)",
            fingerprint: fingerprint,
            deviceID: deviceID
        )
    }
}

struct RemoteHostIdentity: Codable, Equatable, Hashable, Sendable {
    let endpointAuthority: String
    let fingerprint: String
    let deviceID: String
}
