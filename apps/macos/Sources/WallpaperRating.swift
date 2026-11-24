import SwiftUI

/// A friendly face for the 0–1 wallpaper score: five tiers, each with stars and a name.
enum WallpaperRating: Int, CaseIterable, Comparable {
    case notAWallpaper = 1, notIdeal, okay, good, great

    init(_ score: Double) {
        switch score {
        case 0.85...: self = .great
        case 0.65..<0.85: self = .good
        case 0.45..<0.65: self = .okay
        case 0.2..<0.45: self = .notIdeal
        default: self = .notAWallpaper
        }
    }

    var stars: Int { rawValue }

    /// The score stored when the user picks this many stars themselves.
    var userScore: Double {
        switch self {
        case .great: return 0.95
        case .good: return 0.75
        case .okay: return 0.55
        case .notIdeal: return 0.3
        case .notAWallpaper: return 0.1
        }
    }

    /// Lowest score that earns this tier; used for the "minimum rating" filter.
    var threshold: Double {
        switch self {
        case .great: return 0.85
        case .good: return 0.65
        case .okay: return 0.45
        case .notIdeal: return 0.2
        case .notAWallpaper: return 0
        }
    }

    var title: String {
        switch self {
        case .great: return "Great wallpaper"
        case .good: return "Good wallpaper"
        case .okay: return "Okay wallpaper"
        case .notIdeal: return "Not ideal"
        case .notAWallpaper: return "Not a wallpaper"
        }
    }

    var color: Color {
        switch self {
        case .great, .good: return .yellow
        case .okay: return .secondary
        case .notIdeal, .notAWallpaper: return .orange
        }
    }

    var starString: String { String(repeating: "★", count: stars) + String(repeating: "☆", count: 5 - stars) }

    static func < (a: Self, b: Self) -> Bool { a.rawValue < b.rawValue }
}

/// Choices for the "minimum rating" filter, in plain words.
enum MinimumRating: CaseIterable, Identifiable {
    case any, okay, good, great
    var id: Self { self }

    var title: String {
        switch self {
        case .any: return "Show all photos"
        case .okay: return "Okay or better"
        case .good: return "Good or better"
        case .great: return "Great only"
        }
    }

    var threshold: Double {
        switch self {
        case .any: return 0
        case .okay: return WallpaperRating.okay.threshold
        case .good: return WallpaperRating.good.threshold
        case .great: return WallpaperRating.great.threshold
        }
    }

    init(threshold: Double) {
        self = Self.allCases.last { $0.threshold <= threshold + 1e-6 } ?? .any
    }
}

/// Five small stars, filled to the rating.
struct StarsView: View {
    let rating: WallpaperRating

    var body: some View {
        HStack(spacing: 1) {
            ForEach(1...5, id: \.self) { i in
                Image(systemName: i <= rating.stars ? "star.fill" : "star")
                    .foregroundStyle(i <= rating.stars ? AnyShapeStyle(rating.color) : AnyShapeStyle(.tertiary))
            }
        }
        .font(.system(size: 8, weight: .semibold))
        .accessibilityElement()
        .accessibilityLabel("\(rating.title), \(rating.stars) of 5 stars")
    }
}

/// Clickable stars for rating a photo yourself.
struct StarPicker: View {
    let rating: WallpaperRating?
    let isCustom: Bool
    let onSet: (WallpaperRating) -> Void
    @State private var hover: Int?

    var body: some View {
        HStack(spacing: 2) {
            ForEach(1...5, id: \.self) { i in
                let lit = i <= (hover ?? rating?.stars ?? 0)
                Image(systemName: lit ? "star.fill" : "star")
                    .foregroundStyle(lit ? AnyShapeStyle(hover != nil ? Color.accentColor : (rating?.color ?? .secondary)) : AnyShapeStyle(.tertiary))
                    .contentShape(Rectangle())
                    .onHover { hover = $0 ? i : (hover == i ? nil : hover) }
                    .onTapGesture { if let r = WallpaperRating(rawValue: i) { onSet(r) } }
            }
        }
        .font(.system(size: 13))
        .help(isCustom ? "Your rating. Click a star to change it." : "Click a star to rate this photo yourself.")
        .accessibilityElement()
        .accessibilityLabel(rating.map { "\($0.title), \($0.stars) of 5 stars" } ?? "Not rated")
        .accessibilityAdjustableAction { dir in
            let now = rating?.stars ?? 3
            if let r = WallpaperRating(rawValue: dir == .increment ? min(5, now + 1) : max(1, now - 1)) { onSet(r) }
        }
    }
}

extension PhotoEntry {
    var rating: WallpaperRating? { score.map(WallpaperRating.init) }

    /// "Landscape · Rocks and water on an island"
    var ratingDetail: String? {
        guard let label = scoreLabel, !label.isEmpty else { return nil }
        return label.prefix(1).uppercased() + label.dropFirst()
    }
}
