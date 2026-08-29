import SwiftUI

struct RootView: View {
    @EnvironmentObject private var model: RemoteAppModel
    @Environment(\.horizontalSizeClass) private var horizontalSizeClass

    var body: some View {
        Group {
            if model.pairedHost == nil {
                PairingView()
            } else if horizontalSizeClass == .regular {
                NavigationSplitView {
                    TaskSidebarView()
                        .navigationSplitViewColumnWidth(min: 260, ideal: 310, max: 380)
                } detail: {
                    ConversationView()
                }
                .navigationSplitViewStyle(.balanced)
            } else {
                CompactTaskNavigationView()
            }
        }
        .tint(.accentColor)
    }
}

private struct CompactTaskNavigationView: View {
    @EnvironmentObject private var model: RemoteAppModel

    var body: some View {
        NavigationStack {
            TaskSidebarView(compact: true)
                .navigationDestination(for: String.self) { taskID in
                    ConversationView()
                        .onAppear { model.selectTask(taskID) }
                }
        }
    }
}
