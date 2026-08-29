import Foundation
import Security

protocol SecureTokenStoring: Sendable {
    func save(_ value: Data, account: String) throws
    func read(account: String) throws -> Data?
    func delete(account: String) throws
}

enum KeychainStoreError: LocalizedError {
    case status(OSStatus)

    var errorDescription: String? {
        guard case let .status(status) = self else { return nil }
        return SecCopyErrorMessageString(status, nil) as String? ?? "钥匙串错误 \(status)"
    }
}

final class KeychainTokenStore: SecureTokenStoring, @unchecked Sendable {
    private let service: String

    init(service: String = "cn.ghostcloud.pad.remote.device-token") {
        self.service = service
    }

    func save(_ value: Data, account: String) throws {
        let query = baseQuery(account: account)
        let status = SecItemCopyMatching(query as CFDictionary, nil)
        if status == errSecSuccess {
            let update: [String: Any] = [
                kSecValueData as String: value,
                kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
            ]
            let updateStatus = SecItemUpdate(query as CFDictionary, update as CFDictionary)
            guard updateStatus == errSecSuccess else { throw KeychainStoreError.status(updateStatus) }
            return
        }
        guard status == errSecItemNotFound else { throw KeychainStoreError.status(status) }
        var add = query
        add[kSecValueData as String] = value
        // PAD Remote connects only in the foreground, so the token does not
        // need to be readable while the device is locked.
        add[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        let addStatus = SecItemAdd(add as CFDictionary, nil)
        guard addStatus == errSecSuccess else { throw KeychainStoreError.status(addStatus) }
    }

    func read(account: String) throws -> Data? {
        var query = baseQuery(account: account)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        var result: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &result)
        if status == errSecItemNotFound { return nil }
        guard status == errSecSuccess else { throw KeychainStoreError.status(status) }
        return result as? Data
    }

    func delete(account: String) throws {
        let status = SecItemDelete(baseQuery(account: account) as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw KeychainStoreError.status(status)
        }
    }

    private func baseQuery(account: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: account,
            kSecAttrSynchronizable as String: false,
        ]
    }
}

final class InMemoryTokenStore: SecureTokenStoring, @unchecked Sendable {
    private var values: [String: Data] = [:]
    private let lock = NSLock()

    func save(_ value: Data, account: String) throws {
        lock.lock(); defer { lock.unlock() }
        values[account] = value
    }

    func read(account: String) throws -> Data? {
        lock.lock(); defer { lock.unlock() }
        return values[account]
    }

    func delete(account: String) throws {
        lock.lock(); defer { lock.unlock() }
        values.removeValue(forKey: account)
    }
}
