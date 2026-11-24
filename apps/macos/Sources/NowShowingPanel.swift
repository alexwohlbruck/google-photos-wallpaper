import SwiftUI

/// Right-hand panel of the main window: the current wallpaper and how it changes.
struct NowShowingPanel: View {
    @Environment(AppModel.self) private var model

    var body: some View {
        Form {
            Section {
                current
            }

            Section {
                HStack(spacing: 8) {
                    control("Previous", "backward.fill") { model.previous() }
                    control("Random", "shuffle") { model.shuffle() }
                    control("Next", "forward.fill") { model.next() }
                }
                .disabled(model.status.eligibleCount == 0)
                .listRowSeparator(.hidden)

                Toggle("Change automatically", isOn: Binding(get: { !model.settings.paused }, set: { _ in model.togglePaused() }))
                Picker("Every", selection: Binding(
                    get: { model.settings.intervalMinutes },
                    set: { v in model.update { $0.intervalMinutes = v } }
                )) {
                    ForEach(changeIntervals.filter { $0.1 > 0 }, id: \.1) { Text($0.0.replacingOccurrences(of: "Every ", with: "")).tag($0.1) }
                }
                .disabled(model.settings.paused)
                Picker("Order", selection: Binding(get: { model.settings.order }, set: { v in model.update { $0.order = v } })) {
                    Text("Random").tag(Order.random)
                    Text("In order").tag(Order.sequential)
                }
                TimelineView(.periodic(from: .now, by: 30)) { _ in
                    LabeledContent("Next change", value: nextChange)
                }
            } header: {
                Text("Rotation")
            }

            if model.isRating || model.status.activity != nil {
                Section("Working") {
                    if model.isRating {
                        HStack {
                            ProgressView().controlSize(.small)
                            Text("Rating and tagging photos… \(model.status.unratedCount) left")
                                .foregroundStyle(.secondary)
                        }
                    }
                    if let activity = model.status.activity {
                        HStack {
                            ProgressView().controlSize(.small)
                            Text(activity).foregroundStyle(.secondary).lineLimit(2)
                            Spacer()
                            Button("Cancel") { model.cancel() }
                        }
                    }
                }
            }
        }
        .formStyle(.grouped)
    }

    @ViewBuilder private var current: some View {
        if let photo = model.status.current {
            let entry = model.currentEntry
            VStack(alignment: .leading, spacing: 8) {
                Thumbnail(path: photo.path, maxPixels: 900)
                    .aspectRatio(16 / 10, contentMode: .fit)
                    .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                    .onTapGesture(count: 2) { NSWorkspace.shared.activateFileViewerSelecting([photo.url]) }
                    .help("Double-click to show in Finder")
                HStack(alignment: .firstTextBaseline) {
                    Text(entry?.title ?? photo.filename)
                        .font(.title3.weight(.semibold))
                        .lineLimit(2)
                    Spacer()
                    if let entry {
                        Button { model.rename(entry) } label: { Image(systemName: "pencil") }
                            .buttonStyle(.borderless)
                            .help("Rename")
                    }
                }
                if let entry {
                    HStack(spacing: 8) {
                        StarPicker(rating: entry.rating, isCustom: entry.customScore) { model.setRating(entry, $0) }
                        Text(entry.rating?.title ?? "Not rated yet").font(.callout.weight(.medium))
                        if entry.customScore {
                            Button("Auto") { model.setRating(entry, nil) }
                                .buttonStyle(.link)
                                .font(.caption)
                                .help("Go back to the automatic rating")
                        }
                    }
                }
                if let detail = entry?.ratingDetail {
                    Text(detail).font(.callout).foregroundStyle(.secondary)
                }
                if let tags = entry?.tags, !tags.isEmpty {
                    Text(tags.compactMap { PhotoTag(rawValue: $0)?.title }.joined(separator: " · "))
                        .font(.caption).foregroundStyle(.secondary)
                }
                Text(sourceName(photo)).font(.caption).foregroundStyle(.tertiary)
            }
            .padding(.vertical, 4)
        } else {
            VStack(spacing: 6) {
                Image(systemName: "photo.on.rectangle.angled").font(.system(size: 28)).foregroundStyle(.secondary)
                Text(model.sources.isEmpty ? "Add photos to get started" : "No wallpaper set yet").font(.headline)
                Text("Press Next, or double-click a photo.").font(.caption).foregroundStyle(.secondary)
            }
            .frame(maxWidth: .infinity)
            .padding(.vertical, 20)
        }
    }

    private func control(_ title: String, _ symbol: String, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Label(title, systemImage: symbol)
                .labelStyle(.iconOnly)
                .frame(maxWidth: .infinity)
        }
        .controlSize(.large)
        .help("\(title) wallpaper")
    }

    private var nextChange: String {
        if model.settings.paused { return "Paused" }
        guard let next = model.status.nextChangeUnix else { return "—" }
        return RelativeDateTimeFormatter().localizedString(for: Date(timeIntervalSince1970: TimeInterval(next)), relativeTo: .now)
    }

    private func sourceName(_ photo: Photo) -> String {
        model.sources.first { $0.id == photo.sourceId }.map { "From \($0.name)" } ?? ""
    }
}
