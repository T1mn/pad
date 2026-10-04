#if os(macOS)
import AppKit
import SwiftUI

/// Independently implemented visual tokens, based on public cmux screenshots.
/// No cmux source, artwork, runtime, or credentials are included.
enum PADWorkbenchStyle {
    static let barHeight: CGFloat = 34
    static let sidebarWidth: CGFloat = 280
    static let sidebar = color("sidebar", dark: 0x20211f, light: 0xecece9)
    static let canvas = color("canvas", dark: 0x262722, light: 0xfafaf7)
    static let chrome = color("chrome", dark: 0x232420, light: 0xf0f0ed)
    static let input = color("input", dark: 0x30312b, light: 0xecece7)
    static let border = color("border", dark: 0x3a3b36, light: 0xd4d5cf)
    static let text = color("text", dark: 0xd8d8d4, light: 0x292b26)
    static let muted = color("muted", dark: 0xa4a79f, light: 0x62665e)
    static let accent = Color(red: 52 / 255, green: 120 / 255, blue: 246 / 255)

    private static func color(_ name: String, dark: UInt32, light: UInt32) -> Color {
        Color(nsColor: NSColor(name: NSColor.Name("PAD.\(name)")) { appearance in
            let rgb = appearance.bestMatch(from: [.darkAqua, .aqua]) == .darkAqua ? dark : light
            return NSColor(srgbRed: CGFloat((rgb >> 16) & 255) / 255,
                           green: CGFloat((rgb >> 8) & 255) / 255,
                           blue: CGFloat(rgb & 255) / 255, alpha: 1)
        })
    }
}

/// Point-sized text, not a bitmap scale: updates live without recreating PTYs.
private struct PADReadableFont: ViewModifier {
    @AppStorage("pad.terminal.textSizeOffset") private var offset = 0
    let size: CGFloat
    let weight: Font.Weight
    let design: Font.Design

    func body(content: Content) -> some View {
        content.font(.system(size: max(12, size + 2) + CGFloat(min(4, max(0, offset))),
                             weight: weight, design: design))
    }
}

extension View {
    func padFont(size: CGFloat = 12, weight: Font.Weight = .regular,
                 design: Font.Design = .default) -> some View {
        modifier(PADReadableFont(size: size, weight: weight, design: design))
    }
}

struct PADTextSizeMenu: View {
    @AppStorage("pad.terminal.textSizeOffset") private var offset = 0

    var body: some View {
        Menu {
            Picker("界面与对话字号", selection: $offset) {
                Text("标准（正文 14pt）").tag(0)
                Text("大（正文 16pt）").tag(2)
                Text("特大（正文 18pt）").tag(4)
            }
            Text("终端字号独立使用 ⌘+ / ⌘− 调整")
        } label: {
            Image(systemName: "textformat.size").frame(width: 28, height: 26)
        }
        .menuStyle(.borderlessButton).menuIndicator(.hidden).fixedSize()
        .help("调整界面与对话字号（保留终端会话）")
        .accessibilityLabel("界面字号")
    }
}

struct PADChromeButton: View {
    let symbol: String
    let help: String
    var active = false
    var enabled = true
    let action: () -> Void
    @State private var hovered = false

    var body: some View {
        Button(action: action) {
            Image(systemName: symbol)
                .font(.system(size: 12, weight: .regular))
                .foregroundStyle(active ? PADWorkbenchStyle.text : PADWorkbenchStyle.muted)
                .frame(width: 24, height: 24)
                .background(hovered || active ? PADWorkbenchStyle.input : Color.clear)
                .clipShape(RoundedRectangle(cornerRadius: 3))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!enabled)
        .opacity(enabled ? 1 : 0.35)
        .onHover { hovered = $0 }
        .help(help)
        .accessibilityLabel(help)
    }
}
#endif
