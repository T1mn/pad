/// Pure composer-only policy; marked-text events always stay with the input method.
enum PADComposerKeyDecision: Equatable {
    case system
    case newline
    case send
    case consume

    static func decide(
        isReturn: Bool, hasMarkedText: Bool, shift: Bool,
        controlOrOption: Bool, isRepeat: Bool, canSend: Bool
    ) -> Self {
        guard isReturn, !hasMarkedText, !controlOrOption else { return .system }
        if shift { return .newline }
        return canSend && !isRepeat ? .send : .consume
    }
}
