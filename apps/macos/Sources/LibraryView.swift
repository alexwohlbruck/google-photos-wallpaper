import SwiftUI

enum SidebarItem: Hashable {
    case all
    case source(String)
    case tag(String)
}

/// The app's one window: sources and tags, the photo grid, and Now Showing.
struct MainView: View {
    @Environment(AppModel.self) private var model
    @State private var selection: SidebarItem? = .all
    @AppStorage("showOnlyInRotation") private var hideFiltered = true
    @State private var bestFirst = false
    @State private var showNowShowing = true

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                Section("Library") {
                    Label("All Photos", systemImage: "photo.stack").tag(SidebarItem.all)
                }
                Section("Sources") {
                    ForEach(model.sources, id: \.id) { source in
                        SourceRow(source: source).tag(SidebarItem.source(source.id))
                    }
                }
                if !model.tagCounts.isEmpty {
                    Section("Tags") {
                        ForEach(model.tagCounts, id: \.tag) { tc in
                            TagRow(tag: tc.tag, count: tc.count).tag(SidebarItem.tag(tc.tag))
                        }
                    }
                }
            }
            .navigationSplitViewColumnWidth(min: 210, ideal: 240)
        } detail: {
            PhotoGrid(selection: selection ?? .all, hideFiltered: hideFiltered, bestFirst: bestFirst)
                .safeAreaInset(edge: .top, spacing: 0) {
                    LibraryBar(hideFiltered: $hideFiltered, bestFirst: $bestFirst)
                }
                .navigationTitle(title)
                .navigationSubtitle("\(model.status.eligibleCount) photos in rotation")
        }
        .inspector(isPresented: $showNowShowing) {
            NowShowingPanel()
                .inspectorColumnWidth(min: 270, ideal: 310, max: 380)
        }
        .toolbar {
            ToolbarItem(placement: .navigation) {
                Menu {
                    Button("Photos from Google…", systemImage: "photo.on.rectangle.angled") { model.importFromGoogle() }
                        .disabled(!model.status.googleSignedIn)
                    Button("Shared Album Link…", systemImage: "link") { model.promptSharedAlbum() }
                    Button("Folder…", systemImage: "folder") { model.chooseFolder() }
                } label: {
                    Label("Add Photos", systemImage: "plus")
                }
                .help("Add photos from Google, a shared album, or a folder")
                .disabled(model.isBusy)
            }
            ToolbarItem(placement: .primaryAction) {
                Toggle(isOn: $showNowShowing) { Label("Now Showing", systemImage: "sidebar.right") }
                    .help("Show or hide the current wallpaper")
            }
        }
        // A real app while the window is open (Dock icon, ⌘-Tab); back to a menu bar agent when closed.
        .onAppear {
            NSApp.setActivationPolicy(.regular)
            NSApp.activate(ignoringOtherApps: true)
        }
        .onDisappear { NSApp.setActivationPolicy(.accessory) }
    }

    private var title: String {
        switch selection ?? .all {
        case .all: return "All Photos"
        case .source(let id): return model.sources.first { $0.id == id }?.name ?? "Source"
        case .tag(let t): return PhotoTag(rawValue: t)?.title ?? t
        }
    }
}

/// Controls for browsing the library and choosing which photos rotate, above the grid.
struct LibraryBar: View {
    @Environment(AppModel.self) private var model
    @Binding var hideFiltered: Bool
    @Binding var bestFirst: Bool

    var body: some View {
        HStack(spacing: 12) {
            Picker("Show", selection: $hideFiltered) {
                Text("All Photos").tag(false)
                Text("In Rotation").tag(true)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            .fixedSize()
            .help("Show every photo, or only the ones used as wallpapers")

            Picker("Sort", selection: $bestFirst) {
                Text("Library Order").tag(false)
                Text("Best First").tag(true)
            }
            .fixedSize()

            Picker("Use", selection: Binding(
                get: { MinimumRating(threshold: model.settings.filters.minScore) },
                set: { m in model.update { $0.filters.minScore = m.threshold } }
            )) {
                ForEach(MinimumRating.allCases) { Text($0.title).tag($0) }
            }
            .fixedSize()
            .disabled(!model.settings.autoRate)
            .help("Which ratings are used as wallpapers")

            if !rules.isEmpty {
                Label(rules, systemImage: "tag")
                    .font(.callout)
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .help("Tag rules. Right-click a tag in the sidebar to change them.")
            }

            Spacer()

            if model.isRating {
                HStack(spacing: 6) {
                    ProgressView().controlSize(.small)
                    Text("Rating \(model.status.unratedCount)…").foregroundStyle(.secondary).monospacedDigit()
                }
                .help("Rating, tagging and naming photos on this Mac")
            }
            Button { model.rescan() } label: { Label("Refresh", systemImage: "arrow.clockwise") }
                .labelStyle(.iconOnly)
                .buttonStyle(.borderless)
                .help("Rescan folders and sync shared albums")
                .disabled(model.isBusy)
        }
        .controlSize(.small)
        .padding(.horizontal, 12)
        .padding(.vertical, 8)
        .background(.bar)
        .overlay(alignment: .bottom) { Divider() }
    }

    private var rules: String {
        let f = model.settings.filters
        let name = { (t: String) in PhotoTag(rawValue: t)?.title ?? t }
        var parts: [String] = []
        if !f.onlyTags.isEmpty { parts.append("Only " + f.onlyTags.map(name).joined(separator: ", ")) }
        if !f.neverTags.isEmpty { parts.append("Never " + f.neverTags.map(name).joined(separator: ", ")) }
        return parts.joined(separator: " · ")
    }
}

struct TagRow: View {
    @Environment(AppModel.self) private var model
    let tag: String
    let count: UInt32

    var body: some View {
        let rule = model.rule(for: tag)
        HStack {
            Label(PhotoTag(rawValue: tag)?.title ?? tag, systemImage: PhotoTag(rawValue: tag)?.symbol ?? "tag")
            Spacer()
            switch rule {
            case .only: Image(systemName: "checkmark.circle.fill").foregroundStyle(.green).help("Only photos with this tag are used")
            case .never: Image(systemName: "nosign").foregroundStyle(.red).help("Photos with this tag are never used")
            case .normal: EmptyView()
            }
            Text("\(count)").foregroundStyle(.secondary).monospacedDigit()
        }
        .contextMenu {
            Picker("Use as wallpaper", selection: Binding(get: { rule }, set: { model.setRule($0, for: tag) })) {
                Text("Normally").tag(AppModel.TagRule.normal)
                Text("Only photos with this tag").tag(AppModel.TagRule.only)
                Text("Never").tag(AppModel.TagRule.never)
            }
            .pickerStyle(.inline)
        }
    }
}

struct SourceRow: View {
    @Environment(AppModel.self) private var model
    let source: Source

    var body: some View {
        HStack {
            Label {
                VStack(alignment: .leading, spacing: 1) {
                    Text(source.name).lineLimit(1)
                    Text(source.detail).font(.caption)
                        .foregroundStyle(source.syncError == nil ? AnyShapeStyle(.secondary) : AnyShapeStyle(.red))
                        .lineLimit(1).truncationMode(.middle)
                        .help(source.syncError ?? source.detail)
                }
            } icon: {
                Image(systemName: source.symbol)
            }
            Spacer()
            Toggle("", isOn: Binding(get: { source.enabled }, set: { model.setEnabled(source, $0) }))
                .labelsHidden()
                .controlSize(.mini)
                .help(source.enabled ? "Used for wallpapers" : "Not used for wallpapers")
        }
        .opacity(source.enabled ? 1 : 0.55)
        .contextMenu {
            if case .googlePhotos = source.kind {
                Button("Add More Photos…") { model.importFromGoogle() }
            }
            if case .localFolder(let path, _) = source.kind {
                Button("Show in Finder") { NSWorkspace.shared.open(URL(fileURLWithPath: path)) }
            }
            if case .sharedAlbum(let url) = source.kind {
                Button("Sync Now") { model.sync(source) }.disabled(model.isBusy)
                Button("Open in Browser") { if let u = URL(string: url) { NSWorkspace.shared.open(u) } }
                Button("Copy Link") {
                    NSPasteboard.general.clearContents()
                    NSPasteboard.general.setString(url, forType: .string)
                }
            }
            Divider()
            Button("Remove Source", role: .destructive) { model.removeSource(source) }
        }
    }
}

struct PhotoGrid: View {
    @Environment(AppModel.self) private var model
    @Environment(\.openSettings) private var openSettings
    let selection: SidebarItem
    let hideFiltered: Bool
    let bestFirst: Bool

    private var sourceId: String? {
        if case .source(let id) = selection { return id }
        return nil
    }

    @State private var entries: [PhotoEntry] = []

    var body: some View {
        Group {
            if model.sources.isEmpty {
                ContentUnavailableView {
                    Label("No Photos Yet", systemImage: "photo.on.rectangle.angled")
                } description: {
                    Text("Pick photos from Google Photos, follow a shared album link, or add a folder from your Mac.")
                } actions: {
                    VStack(spacing: 8) {
                        HStack {
                            if model.status.googleSignedIn {
                                Button("Pick from Google Photos…") { model.importFromGoogle() }
                                    .disabled(model.isBusy)
                            } else {
                                Button("Sign In to Google…") { openSettings() }
                            }
                            Button("Add Shared Album…") { model.promptSharedAlbum() }
                            Button("Add Folder…") { model.chooseFolder() }
                        }
                        .fixedSize()
                    }
                }
            } else if visible.isEmpty {
                ContentUnavailableView("No Photos", systemImage: "photo", description: Text(hideFiltered && !entries.isEmpty ? "None of these photos are in rotation. Choose All Photos to see them." : "Nothing here matches."))
            } else {
                ScrollView {
                    LazyVGrid(columns: [GridItem(.adaptive(minimum: 170), spacing: 10)], spacing: 10) {
                        ForEach(visible, id: \.photo.id) { entry in
                            PhotoCell(entry: entry, isCurrent: entry.photo.id == model.status.current?.id)
                        }
                    }
                    .padding(12)
                }
            }
        }
        .task(id: "\(selection)#\(model.revision)") {
            let all = model.photos(in: sourceId)
            if case .tag(let t) = selection { entries = all.filter { $0.tags.contains(t) } } else { entries = all }
        }
    }

    private var visible: [PhotoEntry] {
        let shown = hideFiltered ? entries.filter { $0.passesFilters && !$0.excluded } : entries
        return bestFirst ? shown.sorted { ($0.score ?? -1) > ($1.score ?? -1) } : shown
    }
}

struct PhotoCell: View {
    @Environment(AppModel.self) private var model
    let entry: PhotoEntry
    let isCurrent: Bool
    @State private var hovering = false

    private var photo: Photo { entry.photo }

    var body: some View {
        Thumbnail(path: photo.path)
            .aspectRatio(16 / 10, contentMode: .fit)
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay {
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .strokeBorder(isCurrent ? Color.accentColor : .clear, lineWidth: 3)
            }
            .overlay(alignment: .bottomLeading) { badges }
            .overlay(alignment: .topTrailing) { scoreBadge }
            .overlay(alignment: .bottom) { hoverTitle }
            .contentShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            .scaleEffect(hovering ? 1.02 : 1)
            .animation(.easeOut(duration: 0.12), value: hovering)
            .onHover { hovering = $0 }
            .onTapGesture(count: 2) { model.show(photo) }
            .help(helpText)
            .contextMenu {
                if let title = entry.title { Text(title + (entry.customTitle ? " (your name)" : "")) }
                if let rating = entry.rating {
                    Text("\(rating.starString)  \(rating.title)\(entry.customScore ? " (your rating)" : "")")
                    if let detail = entry.ratingDetail { Text(detail) }
                    let tags = entry.tags.compactMap { PhotoTag(rawValue: $0)?.title }
                    if !tags.isEmpty { Text(tags.joined(separator: " · ")) }
                    Divider()
                }
                Button("Set as Wallpaper") { model.show(photo) }
                Button("Rename…") { model.rename(entry) }
                if entry.customTitle { Button("Use Suggested Name") { model.useSuggestedName(entry) } }
                Picker("Rating", selection: Binding<WallpaperRating?>(
                    get: { entry.customScore ? entry.rating : nil },
                    set: { model.setRating(entry, $0) }
                )) {
                    Text("Automatic").tag(WallpaperRating?.none)
                    Divider()
                    ForEach(WallpaperRating.allCases.reversed(), id: \.self) { r in
                        Text("\(r.starString)  \(r.title)").tag(Optional(r))
                    }
                }
                Divider()
                if entry.excluded {
                    Button("Allow in Rotation") { model.include(photo) }
                } else {
                    Button("Never Show") { model.exclude(photo) }
                }
                Button("Show in Finder") { NSWorkspace.shared.activateFileViewerSelecting([photo.url]) }
                if photo.sourceId == "google" {
                    Divider()
                    Button("Remove from Library", role: .destructive) { model.removeGooglePhoto(photo) }
                }
            }
    }

    private var helpText: String {
        var t = "\(entry.title.map { "\($0)\n" } ?? "")\(photo.filename) — \(photo.width)×\(photo.height)"
        if let rating = entry.rating { t += "\n\(rating.title)" }
        if let detail = entry.ratingDetail { t += " · \(detail)" }
        let tags = entry.tags.compactMap { PhotoTag(rawValue: $0)?.title }
        if !tags.isEmpty { t += "\n" + tags.joined(separator: " · ") }
        return t
    }

    /// The photo's name, shown while hovering.
    @ViewBuilder private var hoverTitle: some View {
        if hovering, let title = entry.title {
            Text(title)
                .font(.callout.weight(.semibold))
                .foregroundStyle(.white)
                .lineLimit(1)
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .frame(maxWidth: .infinity)
                .background(LinearGradient(colors: [.clear, .black.opacity(0.6)], startPoint: .top, endPoint: .bottom))
                .clipShape(UnevenRoundedRectangle(bottomLeadingRadius: 6, bottomTrailingRadius: 6, style: .continuous))
                .transition(.opacity)
        }
    }

    @ViewBuilder private var scoreBadge: some View {
        if let rating = entry.rating {
            StarsView(rating: rating)
                .padding(.horizontal, 6)
                .padding(.vertical, 4)
                .background(.regularMaterial, in: Capsule())
                // Your own ratings get an outline, so they're easy to tell from automatic ones.
                .overlay { if entry.customScore { Capsule().strokeBorder(Color.accentColor, lineWidth: 1) } }
                .padding(6)
        }
    }

    @ViewBuilder private var badges: some View {
        // The hover title takes this spot.
        if !hovering { HStack(spacing: 4) {
            if isCurrent { badge("Current", "checkmark.circle.fill") }
            if entry.excluded { badge("Hidden", "eye.slash") }
            else if !entry.passesFilters { badge("Not in Rotation", "line.3.horizontal.decrease") }
        }
        .padding(6) }
    }

    private func badge(_ text: String, _ symbol: String) -> some View {
        Label(text, systemImage: symbol)
            .font(.caption2.weight(.semibold))
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(.regularMaterial, in: Capsule())
    }
}
