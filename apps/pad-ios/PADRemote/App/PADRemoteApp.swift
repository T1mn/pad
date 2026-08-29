import SwiftUI

@main
struct PADRemoteApp: App {
    @StateObject private var model = RemoteAppModel()
    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
                .environmentObject(model)
                .onChange(of: scenePhase) { _, phase in
                    switch phase {
                    case .active: model.sceneBecameActive()
                    case .background: model.sceneEnteredBackground()
                    case .inactive: break
                    @unknown default: break
                    }
                }
        }
    }
}
