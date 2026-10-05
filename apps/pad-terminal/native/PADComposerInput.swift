#if os(macOS)
import AppKit
import SwiftUI

/// Local NSTextView handling, never a window/global Return shortcut.
struct PADComposerInput: NSViewRepresentable {
    @Binding var text: String
    var isEditable: Bool
    var canSend: () -> Bool
    var onSend: () -> Void
    @AppStorage("pad.terminal.textSizeOffset") private var textSizeOffset = 0

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeNSView(context: Context) -> NSScrollView {
        let scroll = NSScrollView()
        scroll.drawsBackground = false
        scroll.hasVerticalScroller = true
        scroll.autohidesScrollers = true

        let input = ComposerTextView()
        input.isRichText = false
        input.allowsUndo = true
        input.drawsBackground = false
        input.isVerticallyResizable = true
        input.isHorizontallyResizable = false
        input.minSize = .zero
        input.maxSize = NSSize(width: CGFloat.greatestFiniteMagnitude, height: CGFloat.greatestFiniteMagnitude)
        input.autoresizingMask = [.width]
        input.textContainerInset = NSSize(width: 5, height: 4)
        input.textContainer?.lineFragmentPadding = 0
        input.textContainer?.widthTracksTextView = true
        input.textContainer?.containerSize = NSSize(width: 0, height: CGFloat.greatestFiniteMagnitude)
        input.delegate = context.coordinator
        // Read the latest binding/guards synchronously, not a render-time canSend snapshot.
        input.canSend = { [weak coordinator = context.coordinator] in
            coordinator?.parent.canSend() ?? false
        }
        input.onSend = { [weak coordinator = context.coordinator] in
            coordinator?.parent.onSend()
        }
        scroll.documentView = input
        updateNSView(scroll, context: context)
        return scroll
    }

    func updateNSView(_ scroll: NSScrollView, context: Context) {
        context.coordinator.parent = self
        guard let input = scroll.documentView as? ComposerTextView else { return }
        input.isEditable = isEditable
        input.font = .monospacedSystemFont(ofSize: 14 + CGFloat(min(4, max(0, textSizeOffset))), weight: .regular)
        input.textColor = NSColor(PADWorkbenchStyle.text)
        input.insertionPointColor = NSColor(PADWorkbenchStyle.text)
        // SwiftUI redraws must not reset selection or replace an active IME marked range.
        if !input.hasMarkedText(), input.string != text {
            input.string = text
        }
    }

    final class Coordinator: NSObject, NSTextViewDelegate {
        var parent: PADComposerInput
        init(_ parent: PADComposerInput) { self.parent = parent }

        func textDidChange(_ notification: Notification) {
            guard let input = notification.object as? NSTextView else { return }
            parent.text = input.string
        }
    }
}

private final class ComposerTextView: NSTextView {
    var canSend: () -> Bool = { false }
    var onSend: () -> Void = {}

    private func decision(for event: NSEvent) -> PADComposerKeyDecision {
        PADComposerKeyDecision.decide(
            isReturn: event.keyCode == 36 || event.keyCode == 76,
            hasMarkedText: hasMarkedText(),
            shift: event.modifierFlags.contains(.shift),
            controlOrOption: !event.modifierFlags.intersection([.control, .option]).isEmpty,
            isRepeat: event.isARepeat,
            canSend: isEditable && canSend()
        )
    }

    override func keyDown(with event: NSEvent) {
        switch decision(for: event) {
        case .system:
            // Check marked text BEFORE AppKit commits it; this event cannot also send.
            super.keyDown(with: event)
        case .newline:
            if isEditable { insertNewlineIgnoringFieldEditor(self) }
        case .send:
            onSend()
        case .consume:
            break // Disabled/pending sends and held Return never mutate the draft.
        }
    }

    override func performKeyEquivalent(with event: NSEvent) -> Bool {
        // Keep Cmd+Return local to the focused composer; consume once, not again in keyDown.
        guard window?.firstResponder === self,
              event.modifierFlags.contains(.command) else {
            return super.performKeyEquivalent(with: event)
        }
        switch decision(for: event) {
        case .system: return super.performKeyEquivalent(with: event)
        case .newline:
            if isEditable { insertNewlineIgnoringFieldEditor(self) }
        case .send: onSend()
        case .consume: break
        }
        return true
    }
}
#endif
