// Offline policy only; no UI, host, login, or model requests.
// swiftc apps/pad-terminal/native/PADComposerKeyDecision.swift \
//   apps/pad-terminal/scripts/composer-key-smoke.swift -o /tmp/pad-composer-key-smoke
// /tmp/pad-composer-key-smoke
@main
struct PADComposerKeySmoke {
    static func main() {
        func decision(marked: Bool = false, shift: Bool = false,
                      enabled: Bool = true, repeatKey: Bool = false,
                      modified: Bool = false, isReturn: Bool = true) -> PADComposerKeyDecision {
            PADComposerKeyDecision.decide(
                isReturn: isReturn, hasMarkedText: marked, shift: shift,
                controlOrOption: modified, isRepeat: repeatKey, canSend: enabled
            )
        }
        precondition(decision() == .send, "Enter sends")
        precondition(decision(shift: true) == .newline, "Shift+Enter inserts newline")
        precondition(decision(marked: true) == .system, "IME confirmation belongs to AppKit")
        precondition(decision(marked: true, shift: true) == .system, "IME wins over Shift")
        precondition(decision(enabled: false) == .consume, "Disabled/pending send leaves draft intact")
        precondition(decision(shift: true, enabled: false) == .newline, "Busy tasks still permit draft editing")
        precondition(decision(repeatKey: true) == .consume, "Held Enter cannot resend")
        precondition(decision(modified: true) == .system, "Control/Option retain native editing")
        precondition(decision(isReturn: false) == .system, "Other keys are untouched")
        print("PAD_COMPOSER_KEY_POLICY_OK; modelRequests=0; nativeIMEInteractionTested=false")
    }
}
