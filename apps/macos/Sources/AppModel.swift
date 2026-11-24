import AppKit
import Observation
import ServiceManagement

/// Receives callbacks from the Rust core. Called on arbitrary threads.
final class CoreBridge: EngineDelegate, @unchecked Sendable {
    weak var model: AppModel?

    /// PINWHEEL_DRY_RUN=1: go through the motions without touching the real desktop (for UI testing).
    private let dryRun = ProcessInfo.processInfo.environment["PINWHEEL_DRY_RUN"] != nil

    func setWallpaper(path: String) -> Bool {
        if dryRun { return true }
        let url = URL(fileURLWithPath: path)
        let apply: () -> Bool = {
            var ok = !NSScreen.screens.isEmpty
            for screen in NSScreen.screens {
                // Keep each screen's existing scaling/fill options.
                let options = NSWorkspace.shared.desktopImageOptions(for: screen) ?? [:]
                do {
                    try NSWorkspace.shared.setDesktopImageURL(url, for: screen, options: options)
                } catch {
                    NSLog("pinwheel: setDesktopImageURL failed: \(error)")
                    ok = false
                }
            }
            return ok
        }
        return Thread.isMainThread ? apply() : DispatchQueue.main.sync(execute: apply)
    }

    func openUrl(url: String) {
        guard let u = URL(string: url) else { return }
        DispatchQueue.main.async { NSWorkspace.shared.open(u) }
    }

    func stateChanged() {
        DispatchQueue.main.async { [weak self] in self?.model?.refresh() }
    }
}

@MainActor
@Observable
final class AppModel {
    let engine: Engine
    private let bridge: CoreBridge

    private(set) var status: Status
    private(set) var sources: [Source] = []
    private(set) var tagCounts: [TagCount] = []
    /// Rating and tags for the current wallpaper.
    private(set) var currentEntry: PhotoEntry?
    private(set) var settings: Settings
    /// Bumped on every refresh so photo grids know to reload.
    private(set) var revision = 0
    var errorMessage: String?

    init() {
        Self.quitIfAlreadyRunning()
        let bridge = CoreBridge()
        do {
            engine = try Engine(dataDir: nil, delegate: bridge)
        } catch {
            fatalError("Couldn't open the app data folder: \(Self.describe(error))")
        }
        self.bridge = bridge
        status = engine.status()
        settings = engine.settings()
        sources = engine.sources()
        bridge.model = self
        engine.start()
        observeDesktops()
        refresh()  // Loads tags and the current photo's details too; also starts rating.
    }

    /// NSWorkspace only sets the picture for each screen's *current* Space. When the
    /// user switches Spaces or plugs in a display, apply the current photo there too.
    private func observeDesktops() {
        let reapply: @Sendable (Notification) -> Void = { [weak self] _ in
            Task { @MainActor in self?.reapplyCurrent() }
        }
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.activeSpaceDidChangeNotification, object: nil, queue: .main, using: reapply)
        NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification, object: nil, queue: .main, using: reapply)

        // After sleep: catch up on a change that came due, then put the wallpaper back on every
        // screen (displays that reconnect on wake sometimes come back with the old picture).
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.didWakeNotification, object: nil, queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                guard let self else { return }
                self.engine.wake()
                self.refresh()
                try? await Task.sleep(for: .seconds(3))
                self.reapplyCurrent()
            }
        }
        NSWorkspace.shared.notificationCenter.addObserver(
            forName: NSWorkspace.screensDidWakeNotification, object: nil, queue: .main, using: reapply)
    }

    private func reapplyCurrent() {
        // While paused, leave the desktop alone in case the user picked something else themselves.
        guard !settings.paused, ProcessInfo.processInfo.environment["PINWHEEL_DRY_RUN"] == nil, let photo = status.current,
              FileManager.default.fileExists(atPath: photo.path) else { return }
        for screen in NSScreen.screens where NSWorkspace.shared.desktopImageURL(for: screen) != photo.url {
            let options = NSWorkspace.shared.desktopImageOptions(for: screen) ?? [:]
            try? NSWorkspace.shared.setDesktopImageURL(photo.url, for: screen, options: options)
        }
    }

    func refresh() {
        status = engine.status()
        settings = engine.settings()
        sources = engine.sources()
        tagCounts = engine.tagCounts()
        currentEntry = status.current.flatMap { cur in
            engine.photos(sourceId: cur.sourceId).first { $0.photo.id == cur.id }
        }
        revision &+= 1
        scheduleRating()
    }

    // MARK: Ratings

    private var ratingTask: Task<Void, Never>?
    private(set) var isRating = false

    /// Rate unrated photos in the background, a few at a time, with on-device models.
    private func scheduleRating() {
        guard settings.autoRate else {
            ratingTask?.cancel()
            return
        }
        guard ratingTask == nil, !engine.photosToRate(version: PhotoRater.version, limit: 1).isEmpty else { return }
        isRating = true
        let engine = self.engine
        ratingTask = Task.detached(priority: .utility) { [weak self] in
            // Names already given, so no two photos share one.
            var recentTitles = engine.photos(sourceId: nil).compactMap(\.title)
            while !Task.isCancelled {
                let batch = engine.photosToRate(version: PhotoRater.version, limit: 8)
                if batch.isEmpty { break }
                var scores: [PhotoScore] = []
                for photo in batch {
                    if Task.isCancelled { break }
                    // A photo that can't be analyzed gets a neutral score so it isn't retried forever.
                    let score = await PhotoRater.rate(photo, avoid: recentTitles)
                    if let t = score?.title { recentTitles.append(t) }
                    scores.append(score
                        ?? PhotoScore(photoId: photo.id, value: 0.5, label: "couldn't analyze", version: PhotoRater.version, tags: [], title: nil))
                }
                try? engine.setScores(scores: scores)
            }
            await MainActor.run {
                self?.ratingTask = nil
                self?.isRating = false
            }
        }
    }

    // MARK: Tags

    enum TagRule { case normal, only, never }

    func rule(for tag: String) -> TagRule {
        if settings.filters.onlyTags.contains(tag) { return .only }
        if settings.filters.neverTags.contains(tag) { return .never }
        return .normal
    }

    func setRule(_ rule: TagRule, for tag: String) {
        update {
            $0.filters.onlyTags.removeAll { $0 == tag }
            $0.filters.neverTags.removeAll { $0 == tag }
            switch rule {
            case .only: $0.filters.onlyTags.append(tag)
            case .never: $0.filters.neverTags.append(tag)
            case .normal: break
            }
        }
    }

    // MARK: Your names and ratings

    /// Ask for a new name with a native alert. Empty restores the suggested name.
    func rename(_ entry: PhotoEntry) {
        let alert = NSAlert()
        alert.messageText = "Rename Wallpaper"
        alert.informativeText = "Leave it empty to use the suggested name."
        alert.addButton(withTitle: "Rename")
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 280, height: 24))
        field.stringValue = entry.title ?? ""
        field.placeholderString = "Name"
        alert.accessoryView = field
        alert.window.initialFirstResponder = field
        NSApp.activate(ignoringOtherApps: true)
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let name = field.stringValue
        let id = entry.photo.id
        perform { try $0.setPhotoTitle(photoId: id, title: name.isEmpty ? nil : name) }
    }

    func useSuggestedName(_ entry: PhotoEntry) {
        let id = entry.photo.id
        perform { try $0.setPhotoTitle(photoId: id, title: nil) }
    }

    /// Rate a photo yourself; `nil` goes back to the automatic rating.
    func setRating(_ entry: PhotoEntry, _ rating: WallpaperRating?) {
        let id = entry.photo.id
        perform { try $0.setPhotoScore(photoId: id, score: rating?.userScore) }
    }

    func rerateAll() {
        perform { try $0.clearScores() }
    }

    var isBusy: Bool { status.activity != nil }

    /// Run a core call off the main thread. Errors other than cancellation are shown in an alert.
    func perform(_ work: @escaping @Sendable (Engine) throws -> Void) {
        let engine = self.engine
        Task.detached(priority: .userInitiated) {
            do {
                try work(engine)
            } catch PinwheelError.Cancelled {
                // The user asked for it.
            } catch {
                let message = Self.describe(error)
                await MainActor.run { self.errorMessage = message }
            }
            await MainActor.run { self.refresh() }
        }
    }

    // MARK: Actions

    func next() { perform { _ = try $0.next() } }
    func previous() { perform { _ = try $0.previous() } }
    func shuffle() { perform { _ = try $0.shuffle() } }
    func show(_ photo: Photo) { perform { _ = try $0.setPhoto(photoId: photo.id) } }
    func exclude(_ photo: Photo) { perform { try $0.excludePhoto(photoId: photo.id) } }
    func include(_ photo: Photo) { perform { try $0.includePhoto(photoId: photo.id) } }
    func removeGooglePhoto(_ photo: Photo) { perform { try $0.removeGooglePhoto(photoId: photo.id) } }
    func importFromGoogle() { perform { _ = try $0.googleImport() } }
    func signIn() { perform { try $0.googleSignIn() } }
    func signOut() { perform { $0.googleSignOut() } }
    func rescan() { perform { try $0.rescan() } }
    func cancel() { engine.cancel() }
    func removeSource(_ s: Source) { perform { try $0.removeSource(sourceId: s.id) } }
    func setEnabled(_ s: Source, _ on: Bool) { perform { try $0.setSourceEnabled(sourceId: s.id, enabled: on) } }

    func togglePaused() {
        let paused = !settings.paused
        perform { try $0.setPaused(paused: paused) }
    }

    func update(_ change: (inout Settings) -> Void) {
        var s = settings
        change(&s)
        settings = s
        perform { try $0.updateSettings(settings: s) }
    }

    func chooseFolder() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = true
        panel.canChooseFiles = false
        panel.allowsMultipleSelection = true
        panel.prompt = "Add Folder"
        panel.message = "Choose folders of photos to use as wallpapers."
        NSApp.activate(ignoringOtherApps: true)
        guard panel.runModal() == .OK else { return }
        let paths = panel.urls.map(\.path)
        perform { engine in
            for p in paths { _ = try engine.addLocalFolder(path: p, recursive: true) }
        }
    }

    /// Ask for a shared album link with a native alert, prefilled from the clipboard.
    func promptSharedAlbum() {
        let alert = NSAlert()
        alert.messageText = "Add a Shared Album"
        alert.informativeText = "In Google Photos, open an album and choose Share → Create link, then paste the link here. New photos in the album are picked up automatically.\n\nThis uses Google's public album page, not an official API, so it may stop working if Google changes the page."
        alert.addButton(withTitle: "Add Album")
        alert.addButton(withTitle: "Cancel")
        let field = NSTextField(frame: NSRect(x: 0, y: 0, width: 340, height: 24))
        field.placeholderString = "https://photos.app.goo.gl/…"
        if let clip = NSPasteboard.general.string(forType: .string)?.trimmingCharacters(in: .whitespacesAndNewlines),
           clip.contains("photos.app.goo.gl/") || clip.contains("photos.google.com/share/") {
            field.stringValue = clip
        }
        alert.accessoryView = field
        alert.window.initialFirstResponder = field
        NSApp.activate(ignoringOtherApps: true)
        guard alert.runModal() == .alertFirstButtonReturn else { return }
        let url = field.stringValue
        guard !url.isEmpty else { return }
        perform { _ = try $0.addSharedAlbum(url: url) }
    }

    func sync(_ source: Source) { perform { try $0.syncSource(sourceId: source.id) } }

    func setGoogleClient(id: String, secret: String) {
        perform { try $0.setGoogleClient(clientId: id, clientSecret: secret) }
    }

    func photos(in sourceId: String?) -> [PhotoEntry] {
        engine.photos(sourceId: sourceId)
    }

    /// Two copies would fight over the wallpaper and the library file. Hand over to the one
    /// already running (scratch copies with their own PINWHEEL_DATA_DIR may run alongside).
    private static func quitIfAlreadyRunning() {
        guard ProcessInfo.processInfo.environment["PINWHEEL_DATA_DIR"] == nil,
              let id = Bundle.main.bundleIdentifier else { return }
        let me = ProcessInfo.processInfo.processIdentifier
        if let other = NSRunningApplication.runningApplications(withBundleIdentifier: id)
            .first(where: { $0.processIdentifier != me }) {
            other.activate()
            exit(0)
        }
    }

    // MARK: Launch at login

    var launchAtLogin: Bool {
        get { SMAppService.mainApp.status == .enabled }
        set {
            do {
                if newValue {
                    try SMAppService.mainApp.register()
                    // macOS may want the user to switch it on in System Settings first.
                    if SMAppService.mainApp.status == .requiresApproval { SMAppService.openSystemSettingsLoginItems() }
                } else {
                    try SMAppService.mainApp.unregister()
                }
            } catch {
                errorMessage = "Couldn't change the login item: \(error.localizedDescription)"
            }
            revision &+= 1
        }
    }

    nonisolated static func describe(_ error: Error) -> String {
        guard let e = error as? PinwheelError else { return error.localizedDescription }
        switch e {
        case .Io(let m): return m
        case .Network(let m): return "Network error: \(m)"
        case .Auth(let m): return "Google sign-in failed: \(m)"
        case .Wallpaper(let m): return "Couldn't set the wallpaper: \(m)"
        case .GoogleNotConfigured: return "This build has no Google OAuth client. Add one in Settings → Google."
        case .NotSignedIn: return "Sign in to Google first."
        case .Cancelled: return "Cancelled."
        case .NoPhotos: return "No photos match your sources and filters. Add a source or loosen the filters."
        case .NotFound: return "That item no longer exists."
        case .Busy: return "Another task is still running. Wait for it to finish or cancel it."
        case .SharedAlbum(let m): return m
        }
    }
}

extension Photo {
    var url: URL { URL(fileURLWithPath: path) }
}

extension Source {
    var symbol: String {
        switch kind {
        case .googlePhotos: return "photo.on.rectangle.angled"
        case .localFolder: return "folder"
        case .sharedAlbum: return "link"
        }
    }

    var detail: String {
        switch kind {
        case .googlePhotos: return "\(photoCount) picked photos"
        case .localFolder(let path, _): return (path as NSString).abbreviatingWithTildeInPath
        case .sharedAlbum:
            if let err = syncError { return "Sync failed: \(err)" }
            guard let t = lastSyncedUnix else { return "\(photoCount) photos · not synced yet" }
            let rel = RelativeDateTimeFormatter().localizedString(for: Date(timeIntervalSince1970: TimeInterval(t)), relativeTo: .now)
            return "\(photoCount) photos · synced \(rel)"
        }
    }
}
