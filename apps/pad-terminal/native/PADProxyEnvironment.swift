#if os(macOS)
import Foundation
import SystemConfiguration

/// Host-app-only adaptation, not a PAC evaluator or credential/keychain reader.
/// Explicit environment presence (even empty) suppresses system adaptation.
enum PADProxyEnvironment {
    static func applying(to environment: [String: String], source: [String: String]) -> [String: String] {
        var result = environment
        let pairs = [("http_proxy", "HTTP_PROXY"), ("https_proxy", "HTTPS_PROXY"),
                     ("no_proxy", "NO_PROXY"), ("all_proxy", "ALL_PROXY")]
        for (lower, upper) in pairs {
            if let value = source[lower] ?? source[upper] { result[lower] = value }
        }
        let explicitProxy = ["http_proxy", "https_proxy", "all_proxy"].contains { result[$0] != nil }
        guard !explicitProxy else { return result }
        guard let settings = SCDynamicStoreCopyProxies(nil) as? [String: Any] else { return result }
        func enabled(_ key: CFString) -> Bool {
            (settings[key as String] as? NSNumber)?.boolValue == true
        }
        let automatic = enabled(kSCPropNetProxiesProxyAutoConfigEnable)
            || enabled(kSCPropNetProxiesProxyAutoDiscoveryEnable)
        let hasStatic = enabled(kSCPropNetProxiesHTTPEnable) || enabled(kSCPropNetProxiesHTTPSEnable)
            || enabled(kSCPropNetProxiesSOCKSEnable)
        // macOS exact exceptions, glob/CIDR and simple-host exclusions do not
        // have equivalent Undici NO_PROXY semantics. Do not guess or drop them.
        let exceptions = settings[kSCPropNetProxiesExceptionsList as String] as? [String] ?? []
        let unsupportedBypass = result["no_proxy"] == nil
            && (!exceptions.isEmpty || enabled(kSCPropNetProxiesExcludeSimpleHostnames))
        if automatic || (hasStatic && unsupportedBypass) { result["PAD_PROXY_WARNING"] = "unsupported-system" }
        guard hasStatic, !automatic, !unsupportedBypass else { return result }
        func endpoint(_ flag: CFString, _ hostKey: CFString, _ portKey: CFString, scheme: String) -> String? {
            guard enabled(flag), let host = settings[hostKey as String] as? String,
                  let port = settings[portKey as String] as? NSNumber,
                  (1...65535).contains(port.intValue), port.doubleValue == Double(port.intValue),
                  !host.isEmpty, !host.contains(where: { $0.isWhitespace }),
                  !host.contains(where: { "/@?#%".contains($0) }) else { return nil }
            var url = URLComponents()
            url.scheme = scheme
            url.host = host
            url.port = port.intValue
            return url.string
        }
        let http = endpoint(kSCPropNetProxiesHTTPEnable, kSCPropNetProxiesHTTPProxy,
                            kSCPropNetProxiesHTTPPort, scheme: "http")
        // macOS HTTPSProxy is an HTTP CONNECT endpoint, not TLS to the proxy.
        let https = endpoint(kSCPropNetProxiesHTTPSEnable, kSCPropNetProxiesHTTPSProxy,
                             kSCPropNetProxiesHTTPSPort, scheme: "http")
        if (enabled(kSCPropNetProxiesHTTPEnable) && http == nil)
            || (enabled(kSCPropNetProxiesHTTPSEnable) && https == nil) {
            result["PAD_PROXY_WARNING"] = "unsupported-system"
            return result
        }
        if http != nil || https != nil {
            result["http_proxy"] = http ?? ""
            result["https_proxy"] = https ?? ""
        } else if let socks = endpoint(kSCPropNetProxiesSOCKSEnable, kSCPropNetProxiesSOCKSProxy,
                                       kSCPropNetProxiesSOCKSPort, scheme: "socks5") {
            result["all_proxy"] = socks
        } else {
            result["PAD_PROXY_WARNING"] = "unsupported-system"
            return result
        }
        // Only system-derived routes get a default loopback bypass. Explicit
        // NO_PROXY (including empty) is never amended, nor are explicit routes.
        if result["no_proxy"] == nil { result["no_proxy"] = "localhost,127.0.0.1,::1" }
        return result
    }
}
#endif
