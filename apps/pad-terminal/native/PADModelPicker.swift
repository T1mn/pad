#if os(macOS)
import SwiftUI

/// Search is entirely local. Return in this field never submits the composer.
struct PADModelPicker: View {
    let catalog: PADCatalog
    let selectedProvider: String?
    let selectedModelId: String?
    let canSelect: Bool
    let loading: Bool
    let directoryStatus: String
    let showsOpenAISync: Bool
    let canSyncOpenAI: Bool
    let onSyncOpenAI: () -> Void
    let onSelect: (PADModelInfo) -> Void

    @State private var query = ""
    @FocusState private var searchFocused: Bool

    private var groups: [PADModelGroup] {
        let matches = PADModelSearch.filter(catalog.models, query: query) { info in
            [info.name, info.id, info.provider,
             catalog.providers.first { $0.id == info.provider }?.name ?? info.provider]
        }
        return PADWorkbenchUI.groupedModels(matches, providers: catalog.providers)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("选择模型").padFont(size: 12, weight: .semibold)
            TextField("搜索模型名称、ID 或 Provider", text: $query)
                .textFieldStyle(.roundedBorder)
                .padFont(size: 12)
                .focused($searchFocused)
                .onSubmit { /* Search only; selection requires a click. */ }
            if showsOpenAISync {
                Text(directoryStatus)
                    .padFont(size: 12)
                    .foregroundStyle(PADWorkbenchStyle.muted)
                    .fixedSize(horizontal: false, vertical: true)
                Button("同步 OpenAI 模型", action: onSyncOpenAI)
                    .padFont(size: 12)
                    .disabled(!canSyncOpenAI)
            }
            Divider()
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 8) {
                    if groups.isEmpty {
                        Text(loading ? "正在加载模型…" : (query.isEmpty ? "没有可用模型" : "没有匹配的模型"))
                            .padFont(size: 12)
                            .foregroundStyle(PADWorkbenchStyle.muted)
                            .padding(.vertical, 16)
                    }
                    ForEach(groups) { group in
                        Text(group.name)
                            .padFont(size: 12, weight: .semibold)
                            .foregroundStyle(PADWorkbenchStyle.muted)
                        ForEach(group.models, id: \.selectionKey) { info in
                            modelRow(info)
                        }
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.trailing, 6)
            }
            .frame(height: 300)
            if !canSelect {
                Text("仅浏览目录；选择任务并等待账号或任务操作完成后可切换模型。")
                    .padFont(size: 12)
                    .foregroundStyle(PADWorkbenchStyle.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
        .padding(14)
        .frame(width: 420)
        .foregroundStyle(PADWorkbenchStyle.text)
        .background(PADWorkbenchStyle.canvas)
        .onAppear { searchFocused = true }
    }

    private func modelRow(_ info: PADModelInfo) -> some View {
        Button { onSelect(info) } label: {
            HStack(alignment: .top, spacing: 8) {
                Image(systemName: selectedProvider == info.provider && selectedModelId == info.id
                      ? "checkmark" : "circle")
                    .font(.system(size: 12))
                    .frame(width: 16)
                    .padding(.top, 3)
                VStack(alignment: .leading, spacing: 3) {
                    Text(info.name).padFont(size: 12, weight: .medium)
                    Text(info.id).padFont(size: 12, design: .monospaced)
                    Text(info.source == "openai_account"
                         ? "官方账号目录" : "内置模型目录 · 账号权限未验证")
                        .padFont(size: 12)
                    if !info.isSelectable {
                        Text("当前版本暂不支持").padFont(size: 12)
                    }
                }
                .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 0)
            }
            .foregroundStyle(info.isSelectable && canSelect ? PADWorkbenchStyle.text : PADWorkbenchStyle.muted)
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(RoundedRectangle(cornerRadius: 4).fill(PADWorkbenchStyle.input))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .disabled(!canSelect || !info.isSelectable)
    }
}
#endif
