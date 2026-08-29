import SwiftUI

struct PairingView: View {
    @EnvironmentObject private var model: RemoteAppModel
    @State private var pairingURI = ""
    @State private var showsScanner = false

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(spacing: 28) {
                    Image(systemName: "iphone.and.arrow.forward")
                        .font(.system(size: 56, weight: .medium))
                        .foregroundStyle(.tint)
                        .accessibilityHidden(true)

                    VStack(spacing: 8) {
                        Text("连接你的 Mac")
                            .font(.largeTitle.bold())
                        Text("在 PAD Desktop 打开“远程连接”，然后扫描一次性二维码。")
                            .font(.body)
                            .foregroundStyle(.secondary)
                            .multilineTextAlignment(.center)
                    }

                    VStack(spacing: 12) {
                        Button {
                            showsScanner = true
                        } label: {
                            Label("扫描二维码", systemImage: "qrcode.viewfinder")
                                .frame(maxWidth: .infinity)
                                .padding(.vertical, 5)
                        }
                        .buttonStyle(.borderedProminent)
                        .controlSize(.large)

                        HStack {
                            Rectangle().frame(height: 1).foregroundStyle(Color(.separator))
                            Text("或手工粘贴")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Rectangle().frame(height: 1).foregroundStyle(Color(.separator))
                        }

                        TextField("pad://remote/pair?…", text: $pairingURI, axis: .vertical)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                            .font(.system(.footnote, design: .monospaced))
                            .padding(12)
                            .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 12))
                            .accessibilityLabel("配对链接")

                        HStack {
                            Button("从剪贴板粘贴") {
                                pairingURI = UIPasteboard.general.string ?? ""
                            }
                            .buttonStyle(.bordered)
                            Spacer()
                            Button("连接") { model.pair(uri: pairingURI) }
                                .buttonStyle(.borderedProminent)
                                .disabled(pairingURI.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                        }
                    }
                    .frame(maxWidth: 520)

                    if case .pairing = model.connectionState {
                        Label(model.connectionState.title, systemImage: model.connectionState.symbol)
                            .foregroundStyle(.secondary)
                    }
                    if let error = model.lastError {
                        VStack(alignment: .leading, spacing: 8) {
                            Label(error, systemImage: "exclamationmark.triangle.fill")
                                .font(.callout)
                                .foregroundStyle(.red)
                            if model.settingsRecoveryAvailable {
                                Button("打开系统设置") { model.openSystemSettings() }
                                    .buttonStyle(.bordered)
                            }
                        }
                        .frame(maxWidth: 520, alignment: .leading)
                    }

                    VStack(alignment: .leading, spacing: 8) {
                        Label("端到端 WSS，加密连接", systemImage: "lock.shield")
                        Label("二维码证书指纹严格校验", systemImage: "checkmark.seal")
                        Label("当前版本仅支持同一局域网直连", systemImage: "network")
                    }
                    .font(.footnote)
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: 520, alignment: .leading)
                }
                .padding(.horizontal, 24)
                .padding(.vertical, 48)
            }
            .background(Color(.systemBackground))
            .navigationTitle("PAD Remote")
            .sheet(isPresented: $showsScanner) {
                NavigationStack {
                    QRScannerView { value in
                        pairingURI = value
                        showsScanner = false
                        model.pair(uri: value)
                    }
                    .ignoresSafeArea(edges: .bottom)
                    .navigationTitle("扫描 Mac 二维码")
                    .navigationBarTitleDisplayMode(.inline)
                    .toolbar {
                        ToolbarItem(placement: .cancellationAction) {
                            Button("取消") { showsScanner = false }
                        }
                    }
                }
            }
        }
    }
}
