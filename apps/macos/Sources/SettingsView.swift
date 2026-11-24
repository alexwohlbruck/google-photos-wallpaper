import SwiftUI

struct SettingsView: View {
    var body: some View {
        TabView {
            // Grouped forms scroll and have no natural height, so each tab gets one that fits it;
            // the Settings window animates between them.
            GeneralSettings().frame(height: 330)
                .tabItem { Label("General", systemImage: "gearshape") }
            FilterSettings().frame(height: 580)
                .tabItem { Label("Filters", systemImage: "line.3.horizontal.decrease.circle") }
            GoogleSettings().frame(height: 440)
                .tabItem { Label("Google", systemImage: "person.crop.circle") }
        }
        .frame(width: 500)
        .activatesApp()
    }
}

/// Shared with the Now Showing panel.
let changeIntervals: [(String, UInt32)] = [
    ("Never", 0),
    ("Every 5 minutes", 5),
    ("Every 15 minutes", 15),
    ("Every 30 minutes", 30),
    ("Every hour", 60),
    ("Every 3 hours", 180),
    ("Every 6 hours", 360),
    ("Every 12 hours", 720),
    ("Every day", 1440),
    ("Every week", 10080),
]

struct GeneralSettings: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        Form {
            Picker("Change picture", selection: binding(\.intervalMinutes)) {
                ForEach(options, id: \.1) { Text($0.0).tag($0.1) }
            }
            Picker("Order", selection: binding(\.order)) {
                Text("Random").tag(Order.random)
                Text("In order").tag(Order.sequential)
            }
            Toggle("Change picture when the app starts", isOn: binding(\.changeOnLaunch))
            Toggle("Pause automatic changes", isOn: binding(\.paused))
            Section {
                Toggle("Open at login", isOn: Binding(get: { _ = model.revision; return model.launchAtLogin },
                                                       set: { model.launchAtLogin = $0 }))
            }
            Section {
                LabeledContent("Data folder") {
                    Button("Show in Finder") {
                        NSWorkspace.shared.open(URL(fileURLWithPath: model.engine.dataDir()))
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    /// Keep a custom interval visible if settings.json has one that isn't in the list.
    private var options: [(String, UInt32)] {
        let current = model.settings.intervalMinutes
        if changeIntervals.contains(where: { $0.1 == current }) { return changeIntervals }
        return changeIntervals + [("Every \(current) minutes", current)]
    }

    private func binding<T>(_ path: WritableKeyPath<Settings, T>) -> Binding<T> {
        Binding(get: { model.settings[keyPath: path] }, set: { v in model.update { $0[keyPath: path] = v } })
    }
}

enum Orientation: String, CaseIterable, Identifiable {
    case any = "Any shape"
    case landscape = "Landscape and square"
    case wide = "Wide only (3:2 or wider)"
    case portrait = "Portrait only"
    var id: String { rawValue }

    var aspect: (min: Double, max: Double) {
        switch self {
        case .any: return (0, 0)
        case .landscape: return (1.0, 0)
        case .wide: return (1.5, 0)
        case .portrait: return (0, 1.0)
        }
    }

    init(_ f: Filters) {
        self = Orientation.allCases.first { $0.aspect.min == f.minAspect && $0.aspect.max == f.maxAspect } ?? .any
    }
}

struct FilterSettings: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        Form {
            Section {
                Picker("Photo shape", selection: Binding(
                    get: { Orientation(model.settings.filters) },
                    set: { o in model.update { $0.filters.minAspect = o.aspect.min; $0.filters.maxAspect = o.aspect.max } }
                )) {
                    ForEach(Orientation.allCases) { Text($0.rawValue).tag($0) }
                }
            }
            Section {
                Toggle("Rate photos on this Mac", isOn: Binding(
                    get: { model.settings.autoRate }, set: { v in model.update { $0.autoRate = v } }))
                Picker("Use photos rated", selection: Binding(
                    get: { MinimumRating(threshold: model.settings.filters.minScore) },
                    set: { m in model.update { $0.filters.minScore = m.threshold } }
                )) {
                    ForEach(MinimumRating.allCases) { m in
                        if m == .any { Text(m.title).tag(m) } else {
                            Text("\(m.title)  \(WallpaperRating(m.threshold).starString)").tag(m)
                        }
                    }
                }
                .disabled(!model.settings.autoRate)
                Toggle("Show better wallpapers more often", isOn: Binding(
                    get: { model.settings.preferHighScores }, set: { v in model.update { $0.preferHighScores = v } }))
                    .disabled(!model.settings.autoRate)
                HStack {
                    if model.isRating {
                        ProgressView().controlSize(.small)
                        Text("\(model.status.unratedCount) left").foregroundStyle(.secondary)
                    }
                    Spacer()
                    Button("Re-rate All Photos") { model.rerateAll() }.disabled(!model.settings.autoRate)
                }
            } header: {
                Text("Wallpaper rating")
            } footer: {
                Text(PhotoRater.languageModelAvailable
                     ? "Each photo gets 1–5 stars for how well it works as a wallpaper, using Apple's on-device image analysis and Apple Intelligence. Nothing leaves your Mac."
                     : "Each photo gets 1–5 stars for how well it works as a wallpaper, using Apple's on-device image analysis. Turn on Apple Intelligence for better ratings. Nothing leaves your Mac.")
                    .foregroundStyle(.secondary)
            }
            Section {
                LabeledContent("Minimum width") { pixels(\.minWidth) }
                LabeledContent("Minimum height") { pixels(\.minHeight) }
                Button("Match this display") {
                    guard let screen = NSScreen.main else { return }
                    let size = screen.frame.size
                    let scale = screen.backingScaleFactor
                    // Half the native resolution still looks sharp for photos.
                    model.update {
                        $0.filters.minWidth = UInt32(size.width * scale / 2)
                        $0.filters.minHeight = UInt32(size.height * scale / 2)
                    }
                }
            } header: {
                Text("Size")
            } footer: {
                Text("\(model.status.eligibleCount) photos pass these filters. Use 0 for no limit.")
                    .foregroundStyle(.secondary)
            }
        }
        .formStyle(.grouped)
    }

    private func pixels(_ path: WritableKeyPath<Filters, UInt32>) -> some View {
        HStack(spacing: 4) {
            TextField("", value: Binding(
                get: { Int(model.settings.filters[keyPath: path]) },
                set: { v in model.update { $0.filters[keyPath: path] = UInt32(max(0, v)) } }
            ), format: .number.grouping(.never))
            .multilineTextAlignment(.trailing)
            .frame(width: 80)
            .labelsHidden()
            Text("px").foregroundStyle(.secondary)
        }
    }
}

struct GoogleSettings: View {
    @Environment(AppModel.self) private var model
    @State private var clientId = ""
    @State private var clientSecret = ""
    @State private var showClient = false

    var body: some View {
        Form {
            Section {
                if model.status.googleSignedIn {
                    LabeledContent("Google Photos") {
                        Label("Signed in", systemImage: "checkmark.circle.fill").foregroundStyle(.green)
                    }
                    HStack {
                        Button("Pick Photos…") { model.importFromGoogle() }.disabled(model.isBusy)
                        Spacer()
                        Button("Sign Out", role: .destructive) { model.signOut() }
                    }
                } else {
                    LabeledContent("Google Photos") { Text("Not signed in").foregroundStyle(.secondary) }
                    HStack {
                        if model.isBusy {
                            ProgressView().controlSize(.small)
                            Text(model.status.activity ?? "").foregroundStyle(.secondary)
                            Spacer()
                            Button("Cancel") { model.cancel() }
                        } else {
                            Button("Sign In with Google…") { model.signIn() }
                                .disabled(!model.status.googleConfigured)
                            if !model.status.googleConfigured {
                                Text("Add an OAuth client below first.").font(.caption).foregroundStyle(.secondary)
                            }
                        }
                    }
                }
            } footer: {
                Text("Google only lets apps see photos you pick. Use **Pick Photos…** to choose photos (up to 2,000 at a time; tick a date header to select a whole day). They're downloaded to this Mac and used offline. Albums can't be picked or followed, so new photos only appear when you pick them.")
                    .foregroundStyle(.secondary)
            }

            Section {
                DisclosureGroup("Use your own OAuth client", isExpanded: $showClient) {
                    TextField("Client ID", text: $clientId, prompt: Text("…apps.googleusercontent.com"))
                    SecureField("Client secret", text: $clientSecret, prompt: Text("GOCSPX-…"))
                    Button("Save") { model.setGoogleClient(id: clientId, secret: clientSecret) }
                        .disabled(clientId.isEmpty)
                    Text("Create a **Desktop app** OAuth client in Google Cloud Console and enable the Photos Picker API.")
                        .font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .formStyle(.grouped)
        .onAppear { showClient = !model.status.googleConfigured }
    }
}
