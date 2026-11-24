import SwiftUI

@main
struct PinwheelApp: App {
    @State private var model = AppModel()

    var body: some Scene {
        // One main window: library on the left and middle, Now Showing on the right.
        Window("Pinwheel", id: WindowID.main) {
            MainView()
                .environment(model)
                .frame(minWidth: 900, minHeight: 520)
                .errorAlert(model)
        }
        .defaultSize(width: 1200, height: 720)
        // Open on first run (nothing to rotate yet) or when asked; otherwise start quietly in the menu bar.
        .defaultLaunchBehavior(model.sources.isEmpty || CommandLine.arguments.contains("--show-library") ? .presented : .suppressed)

        // Quick controls from the menu bar.
        MenuBarExtra {
            MenuBarMenu().environment(model)
        } label: {
            Image(systemName: "fan")
                .symbolVariant(model.settings.paused ? .slash : .none)
        }
        .menuBarExtraStyle(.menu)

        // The standard Settings window: resizes to each tab and opens with ⌘, .
        SwiftUI.Settings {
            SettingsView()
                .environment(model)
                .errorAlert(model)
        }
    }
}

enum WindowID {
    static let main = "main"
}

/// A plain menu, like other menu bar utilities.
struct MenuBarMenu: View {
    @Environment(AppModel.self) private var model
    @Environment(\.openWindow) private var openWindow

    var body: some View {
        if let photo = model.status.current {
            Text(model.currentEntry?.title ?? photo.filename)
        }
        Button("Next Wallpaper") { model.next() }
            .disabled(model.status.eligibleCount == 0)
        Button("Previous Wallpaper") { model.previous() }
            .disabled(model.status.eligibleCount == 0)
        Button("Random Wallpaper") { model.shuffle() }
            .disabled(model.status.eligibleCount == 0)
        Toggle("Change Automatically", isOn: Binding(get: { !model.settings.paused }, set: { _ in model.togglePaused() }))
        Divider()
        Button("Open Pinwheel") {
            openWindow(id: WindowID.main)
            NSApp.activate(ignoringOtherApps: true)
        }
        Toggle("Open at Login", isOn: Binding(get: { _ = model.revision; return model.launchAtLogin },
                                              set: { model.launchAtLogin = $0 }))
        SettingsLink { Text("Settings…") }
        Divider()
        Button("Quit Pinwheel") { NSApp.terminate(nil) }
    }
}

extension View {
    func errorAlert(_ model: AppModel) -> some View {
        alert(
            "Something went wrong",
            isPresented: Binding(get: { model.errorMessage != nil }, set: { if !$0 { model.errorMessage = nil } }),
            actions: { Button("OK") { model.errorMessage = nil } },
            message: { Text(model.errorMessage ?? "") }
        )
    }

    /// The app runs as a menu bar agent, so bring it forward when opening a window.
    func activatesApp() -> some View {
        onAppear { NSApp.activate(ignoringOtherApps: true) }
    }
}
