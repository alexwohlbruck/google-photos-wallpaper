import Foundation
import FoundationModels
import ImageIO
import Vision

/// Rates photos 0...1 for wallpaper suitability, entirely on-device.
///
/// 1. Vision (Neural Engine): Apple's aesthetics model (quality, plus a "utility"
///    flag for practical, non-memorable shots), scene classification labels, faces, people.
/// 2. Apple Intelligence's on-device language model classifies the photo into a
///    wallpaper-relevant kind from those signals. (It can't see pixels yet, and
///    it's much more reliable at picking a category than inventing a number.)
///    Without Apple Intelligence, a keyword mapping of the Vision labels is used.
/// 3. The score blends how wallpaper-friendly that kind is with image quality.
enum PhotoRater {
    /// Bump when the algorithm changes so existing ratings are recomputed.
    static let version: UInt32 = 11

    struct Signals {
        var aesthetic: Double      // -1...1
        var utility: Bool
        var labels: [(String, Float)]
        var faceCount: Int
        var faceArea: Double       // fraction of the frame
        var humanCount: Int
        var humanArea: Double      // fraction of the frame covered by people's bodies
        var colors: [String] = []  // dominant color names, most common first
        var brightness: Double = 0.5

        func label(_ id: String) -> Float { labels.first { $0.0 == id }?.1 ?? 0 }

        /// People are the subject, not part of the scenery. Detectors miss people who are
        /// small or partly hidden, so the scene classifier's "people"/"crowd" labels count too.
        var peopleProminent: Bool {
            (faceArea > 0.04 && (humanCount > 0 || animalConfidence < 0.5)) || humanArea > 0.15 || (humanCount >= 3 && humanArea > 0.08)
                || label("crowd") > 0.5 || label("people") > 0.85
        }

        /// Any label (at `minimum` confidence or more) containing one of these whole words.
        /// Whole words, so "text" doesn't match "textile".
        func hasLabel(_ words: [String], minimum: Float) -> Bool {
            labels.contains { l in l.1 >= minimum && l.0.split(separator: "_").contains { words.contains(String($0)) } }
        }

        /// "structure" is left out on purpose: Vision applies it to shelves, furniture and the like.
        var hasSceneryEvidence: Bool {
            hasLabel(["outdoor", "sky", "land", "mountain", "hill", "water", "ocean", "sea", "beach", "lake",
                      "river", "forest", "field", "cityscape", "skyline", "building", "bridge", "sunset", "cloud",
                      "desert", "canyon", "snow", "shore", "coast", "tree", "grass", "aurora"], minimum: 0.3)
        }

        /// Text on paper or a screen. Apple's "utility" flag alone doesn't mean this.
        var hasDocumentEvidence: Bool {
            hasLabel(["document", "handwriting", "printed", "screenshot", "receipt", "chalkboard", "whiteboard"], minimum: 0.3)
        }

        /// Colorful art, decorations and crafts, which often make great wallpapers.
        var hasArtEvidence: Bool {
            hasLabel(["art", "decoration", "mask", "celebration", "stained", "graffiti", "illustrations", "painting",
                      "mural", "sculpture", "pattern", "mosaic", "origami", "lantern", "fireworks"], minimum: 0.25)
        }

        /// Labels too broad to name a photo by.
        static let genericLabels: Set<String> = [
            "outdoor", "indoor", "structure", "animal", "mammal", "feline", "canine", "plant", "land", "adult",
            "material", "wood_processed", "wood_natural", "sky", "blue_sky", "cloudy", "night_sky", "liquid",
            "water_body", "foliage", "people", "art", "decoration", "textile", "conveyance", "vehicle",
        ]

        /// A Vision label in plain words: "adult_cat" → "cat", "wood_processed" → "wood".
        static func readable(_ id: String) -> String {
            var w = id
            for prefix in ["adult_", "baby_"] where w.hasPrefix(prefix) { w.removeFirst(prefix.count) }
            if w.hasPrefix("wood_") { w = "wood" }
            return w.replacingOccurrences(of: "_", with: " ")
        }

        /// The most specific thing in the photo: "cat", not "mammal".
        var subject: String? {
            if peopleProminent { return label("crowd") > 0.4 || humanCount >= 3 ? "crowd" : (humanCount > 1 ? "people" : "person") }
            return labels.first { $0.1 >= 0.3 && !Self.genericLabels.contains($0.0) }.map { Self.readable($0.0) }
        }

        /// Everything else worth mentioning, in plain words, without repeats.
        var otherContents: [String] {
            var seen = Set([subject ?? ""])
            return labels.filter { $0.1 > 0.3 && !["outdoor", "indoor", "structure", "animal", "mammal", "feline", "canine", "adult"].contains($0.0) }
                .map { Self.readable($0.0) }
                .filter { seen.insert($0).inserted }
        }

        /// People labels are whole Vision identifiers: "adult" alone, never "adult_cat".
        var hasPeopleLabel: Bool {
            labels.contains { ["people", "adult", "child", "baby", "crowd", "selfie", "wedding"].contains($0.0) && $0.1 >= 0.35 }
        }

        /// Strongest animal label, if the photo is clearly about an animal.
        var animalConfidence: Float {
            ["animal", "dog", "cat", "bird", "mammal", "canine", "feline", "horse"].map(label).max() ?? 0
        }
    }

    static var languageModelAvailable: Bool {
        if #available(macOS 26, *) {
            if case .available = SystemLanguageModel.default.availability { return true }
        }
        return false
    }

    /// `avoid`: names already given to other photos, so the model doesn't repeat itself.
    static func rate(_ photo: Photo, avoid: [String] = []) async -> PhotoScore? {
        guard let signals = try? analyze(path: photo.path) else { return nil }
        var kind: Kind
        var people: People
        var summary = ""
        if #available(macOS 26, *), languageModelAvailable,
           let verdict = try? await classify(signals) {
            kind = Kind(rawValue: verdict.kind.rawValue) ?? .other
            people = People(rawValue: verdict.people.rawValue) ?? .none
            summary = verdict.summary
        } else {
            (kind, people) = fallbackClassify(signals)
        }
        let modelKind = kind
        // Documents and screenshots need text-like labels; otherwise reclassify.
        if kind == .document_or_screenshot, !signals.hasDocumentEvidence {
            kind = fallbackClassify(signals).0
        }
        if [.other, .indoor_casual, .document_or_screenshot].contains(kind), signals.hasArtEvidence, !signals.hasDocumentEvidence {
            kind = .abstract_or_pattern
        }
        // The language model only sees labels, and sometimes invents scenery that isn't there.
        // Only accept a scenic kind when the labels back it up.
        let scenic: [Kind] = [.landscape, .seascape, .sky, .cityscape, .architecture]
        if scenic.contains(kind), !signals.hasSceneryEvidence {
            kind = signals.labels.isEmpty ? .abstract_or_pattern : fallbackClassify(signals).0
        }
        if signals.animalConfidence > 0.7, !signals.hasDocumentEvidence { kind = .animal }
        // Trust the body/face detectors over the language model about people.
        if signals.peopleProminent, ![.portrait_or_selfie, .group_photo].contains(kind) {
            kind = max(signals.faceCount, signals.humanCount) > 2 ? .group_photo : .people_in_scenery
            people = .prominent
        }
        if kind != modelKind { summary = "" }  // The model's description no longer matches.
        let value = score(kind: kind, people: people, signals: signals)
        let label = [kind.title, summary].filter { !$0.isEmpty }.joined(separator: " · ")
        let tags = PhotoTag.tags(for: signals, kind: kind).map(\.rawValue)
        // Naming is its own creative step, after the facts above are settled.
        var title: String?
        if #available(macOS 26, *), languageModelAvailable {
            title = (try? await name(signals, kind: kind, summary: summary, avoid: avoid)) ?? nil
        }
        return PhotoScore(photoId: photo.id, value: value, label: label, version: version, tags: tags,
                          title: title ?? fallbackTitle(kind: kind, signals: signals))
    }

    // MARK: Vision

    static func analyze(path: String) throws -> Signals {
        let url = URL(fileURLWithPath: path) as CFURL
        guard let src = CGImageSourceCreateWithURL(url, nil),
              let image = CGImageSourceCreateThumbnailAtIndex(src, 0, [
                  kCGImageSourceCreateThumbnailFromImageAlways: true,
                  kCGImageSourceThumbnailMaxPixelSize: 1024,
                  kCGImageSourceCreateThumbnailWithTransform: true,
              ] as CFDictionary)
        else { throw CocoaError(.fileReadCorruptFile) }

        let aesthetics = VNCalculateImageAestheticsScoresRequest()
        let classify = VNClassifyImageRequest()
        let faces = VNDetectFaceRectanglesRequest()
        let humans = VNDetectHumanRectanglesRequest()
        humans.upperBodyOnly = false
        try VNImageRequestHandler(cgImage: image).perform([aesthetics, classify, faces, humans])
        let bodies = humans.results ?? []

        let a = aesthetics.results?.first
        let labels = (classify.results ?? []).filter { $0.confidence > 0.15 }.prefix(16).map { ($0.identifier, $0.confidence) }
        let faceBoxes = faces.results ?? []
        let (colors, brightness) = palette(image)
        return Signals(
            aesthetic: Double(a?.overallScore ?? 0),
            utility: a?.isUtility ?? false,
            labels: Array(labels),
            faceCount: faceBoxes.count,
            faceArea: faceBoxes.reduce(0) { $0 + $1.boundingBox.width * $1.boundingBox.height },
            humanCount: bodies.count,
            humanArea: min(1.0, bodies.reduce(0.0) { $0 + Double($1.boundingBox.width * $1.boundingBox.height) }),
            colors: colors,
            brightness: brightness
        )
    }

    /// Dominant color names and average brightness, from a 16×16 downsample.
    static func palette(_ image: CGImage) -> ([String], Double) {
        let n = 16
        var px = [UInt8](repeating: 0, count: n * n * 4)
        guard let ctx = CGContext(data: &px, width: n, height: n, bitsPerComponent: 8, bytesPerRow: n * 4,
                                  space: CGColorSpaceCreateDeviceRGB(),
                                  bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { return ([], 0.5) }
        ctx.interpolationQuality = .medium
        ctx.draw(image, in: CGRect(x: 0, y: 0, width: n, height: n))
        var counts: [String: Int] = [:]
        var total = 0.0
        for i in stride(from: 0, to: px.count, by: 4) {
            let (r, g, b) = (Double(px[i]) / 255, Double(px[i + 1]) / 255, Double(px[i + 2]) / 255)
            let mx = max(r, g, b), mn = min(r, g, b), v = mx, s = mx == 0 ? 0 : (mx - mn) / mx
            total += v
            let name: String
            if v < 0.18 { name = "black" }
            else if s < 0.18 { name = v > 0.85 ? "white" : "gray" }
            else {
                var h = 0.0
                if mx == r { h = (g - b) / (mx - mn) } else if mx == g { h = 2 + (b - r) / (mx - mn) } else { h = 4 + (r - g) / (mx - mn) }
                h = (h * 60).truncatingRemainder(dividingBy: 360); if h < 0 { h += 360 }
                switch h {
                case ..<15, 345...: name = "red"
                case ..<40: name = v < 0.55 ? "brown" : "orange"
                case ..<65: name = "yellow"
                case ..<160: name = "green"
                case ..<200: name = "teal"
                case ..<255: name = "blue"
                case ..<290: name = "purple"
                default: name = "pink"
                }
            }
            counts[name, default: 0] += 1
        }
        let pixels = n * n
        let colors = counts.filter { Double($0.value) / Double(pixels) >= 0.12 }
            .sorted { $0.value > $1.value }.prefix(3).map(\.key)
        return (colors, total / Double(pixels))
    }

    // MARK: Scoring

    enum Kind: String, CaseIterable {
        case landscape, seascape, sky, cityscape, architecture, nature_closeup, abstract_or_pattern, animal
        case people_in_scenery, portrait_or_selfie, group_photo, food, indoor_casual, document_or_screenshot, vehicle, other

        var title: String {
            self == .abstract_or_pattern ? "art or pattern" : rawValue.replacingOccurrences(of: "_", with: " ")
        }

        /// How wallpaper-friendly this kind of photo is, before quality.
        var suitability: Double {
            switch self {
            case .landscape, .seascape: return 1
            case .sky, .cityscape: return 0.95
            case .architecture, .abstract_or_pattern: return 0.85
            case .nature_closeup: return 0.8
            case .people_in_scenery: return 0.7
            case .animal: return 0.6
            case .vehicle: return 0.55
            case .other: return 0.45
            case .indoor_casual: return 0.3
            case .group_photo: return 0.2
            case .portrait_or_selfie, .food: return 0.15
            case .document_or_screenshot: return 0
            }
        }
    }

    enum People: String { case none, small_in_frame, prominent }

    static func score(kind: Kind, people: People, signals: Signals) -> Double {
        if kind == .document_or_screenshot { return 0.02 }
        let quality = (signals.aesthetic + 1) / 2
        var value = 0.6 * kind.suitability + 0.4 * quality
        // Apple flags practical, non-memorable shots as "utility". A nudge, not a verdict.
        if signals.utility { value -= 0.1 }
        // Photos about people are personal, not wallpapers, however well shot.
        if people == .prominent || signals.peopleProminent { value = min(value, 0.35) }
        return max(0, min(1, value))
    }

    // MARK: Titles

    /// Trim quotes and punctuation the model sometimes adds, fix ALL CAPS / lowercase,
    /// and reject empty or rambling titles.
    static func cleanTitle(_ raw: String?) -> String? {
        guard var t = raw?.trimmingCharacters(in: .whitespacesAndNewlines) else { return nil }
        t = t.trimmingCharacters(in: CharacterSet(charactersIn: "\"'“”‘’.!*"))
        let words = t.split(separator: " ")
        guard (1...7).contains(words.count), t.count <= 48 else { return nil }
        return titleCase(t)
    }

    static func titleCase(_ s: String) -> String {
        let small: Set<String> = ["a", "an", "and", "at", "by", "for", "in", "of", "on", "or", "over", "the", "to", "under", "with", "beneath", "above"]
        return s.lowercased().split(separator: " ").enumerated().map { i, w in
            let word = String(w)
            return i > 0 && small.contains(word) ? word : word.prefix(1).uppercased() + word.dropFirst()
        }.joined(separator: " ")
    }

    /// Without Apple Intelligence: main color plus the most specific label ("Blue Waterfall").
    static func fallbackTitle(kind: Kind, signals: Signals) -> String {
        let subject = fallbackSubject(kind: kind, signals: signals)
        guard let color = signals.colors.first, !["gray", "black", "white"].contains(color) else { return subject }
        return titleCase("\(color) \(subject)")
    }

    static func fallbackSubject(kind: Kind, signals: Signals) -> String {
        let generic: Set<String> = ["outdoor", "structure", "material", "land", "liquid", "water", "people", "adult",
                                    "machine", "conveyance", "wood_processed", "textile", "cloudy", "sky", "blue_sky"]
        if let label = signals.labels.first(where: { !generic.contains($0.0) && $0.1 > 0.3 })?.0 {
            return label.replacingOccurrences(of: "sunset_sunrise", with: "sunset")
                .split(separator: "_").map { $0.capitalized }.joined(separator: " ")
        }
        return kind.title.capitalized
    }

    // MARK: Fallback without Apple Intelligence

    static func fallbackClassify(_ s: Signals) -> (Kind, People) {
        let people: People = s.peopleProminent ? .prominent : (s.faceCount + s.humanCount > 0 ? .small_in_frame : .none)
        // Whole-word matches on label identifiers ("water_body" has the words water and body),
        // so "text" doesn't match "textile".
        let words = Set(s.labels.filter { $0.1 > 0.2 }.flatMap { $0.0.split(separator: "_").map(String.init) })
        func any(_ list: [String]) -> Bool { list.contains { words.contains($0) } }

        let kind: Kind
        if s.hasDocumentEvidence { kind = .document_or_screenshot }
        else if s.peopleProminent { kind = max(s.faceCount, s.humanCount) > 2 ? .group_photo : .portrait_or_selfie }
        else if any(["food", "dish", "meal", "dessert", "drink", "baked"]) { kind = .food }
        else if any(["ocean", "sea", "beach", "shore", "coast", "lake", "water_body"]) { kind = .seascape }
        else if any(["mountain", "hill", "landscape", "canyon", "desert", "forest", "field", "valley"]) { kind = .landscape }
        else if any(["cityscape", "skyline", "skyscraper"]) { kind = .cityscape }
        else if s.hasArtEvidence { kind = .abstract_or_pattern }
        else if any(["building", "bridge", "architecture", "tower", "church", "castle"]) { kind = .architecture }
        else if any(["sky", "cloud", "sunset", "aurora", "night_sky"]) { kind = .sky }
        else if any(["flower", "plant", "leaf", "tree"]) { kind = .nature_closeup }
        else if any(["dog", "cat", "animal", "bird", "mammal"]) { kind = .animal }
        else if any(["car", "vehicle", "train", "airplane", "boat"]) { kind = .vehicle }
        else if any(["indoor", "room", "interior", "restaurant"]) { kind = .indoor_casual }
        else if s.faceCount > 0 { kind = .people_in_scenery }
        else { kind = .other }
        return (kind, people)
    }
}

// MARK: Apple Intelligence classification

@available(macOS 26, *)
@Generable
enum GenKind: String {
    case landscape, seascape, sky, cityscape, architecture, nature_closeup, abstract_or_pattern, animal
    case people_in_scenery, portrait_or_selfie, group_photo, food, indoor_casual, document_or_screenshot, vehicle, other
}

@available(macOS 26, *)
@Generable
enum GenPeople: String { case none, small_in_frame, prominent }

@available(macOS 26, *)
@Generable
struct Verdict {
    @Guide(description: "What kind of photo this is")
    var kind: GenKind
    @Guide(description: "How prominent people are in the photo")
    var people: GenPeople
    @Guide(description: "At most 8 words describing the photo")
    var summary: String
}

@available(macOS 26, *)
@Generable
struct WallpaperNames {
    @Guide(description: "Four different candidate names, each 2 to 5 words", .count(4))
    var names: [String]
}

@available(macOS 26, *)
extension PhotoRater {
    static let instructions = """
    You classify photos for use as a computer desktop wallpaper, based on an automatic analysis of the image. \
    Good wallpapers: scenery, nature, cityscapes, architecture, skies, calm compositions with negative space, \
    and colorful art, decorations and patterns (use abstract_or_pattern for those). \
    Poor wallpapers: screenshots, documents, receipts, food close-ups, and photos dominated by people's faces \
    (selfies and group shots are personal, not wallpapers). A person small in a scenic landscape is fine.
    """

    static let namingInstructions = """
    You title desktop wallpapers. A title says plainly what the photo shows, then adds one vivid detail: \
    the setting, a color, the light or the time of day. 2 to 5 words, in title case. \
    Examples: "Cat on the Fence", "Tabby Under Blue Skies", "Harbor at Dusk", "Red Stadium Lights", \
    "Fog Over the Ridge", "Golden Gate at Sunset", "Skull Masks in Blue". \
    Every title must name the subject. Use only the details given; never invent places or objects. \
    Avoid vague mood-only titles like "Morning Breeze" or "Quiet Moment".
    """

    /// The model suggests four titles; we pick one that names the subject and isn't taken.
    /// (Listing taken names in the prompt doesn't work: the small model copies them.)
    static func name(_ s: Signals, kind: Kind, summary: String, avoid: [String]) async throws -> String? {
        let subject = s.subject
        let others = s.otherContents
        var prompt = "Title this photo.\nsubject: \(subject ?? (kind == .abstract_or_pattern ? "abstract shapes" : kind.title))"
        prompt += "\ntype: \(kind.title)"
        if !others.isEmpty { prompt += "\nalso in the photo: \(others.prefix(6).joined(separator: ", "))" }
        prompt += "\nmain colors: \(s.colors.isEmpty ? "muted" : s.colors.joined(separator: ", "))"
        // Only mention light when it's notable; "medium" ends up in names otherwise.
        if s.brightness < 0.3 { prompt += "\nlight: dark, moody" } else if s.brightness > 0.7 { prompt += "\nlight: bright, airy" }
        if !summary.isEmpty { prompt += "\ndescription: \(summary)" }
        let session = LanguageModelSession(instructions: namingInstructions)
        let candidates = try await session.respond(
            to: prompt, generating: WallpaperNames.self,
            options: GenerationOptions(sampling: .random(probabilityThreshold: 0.9), temperature: 0.8)
        ).content.names.compactMap(cleanTitle)

        let taken = Set(avoid.map { $0.lowercased() })
        let banned = ["landscape", "photo", "image", "abstract", "structure", "untitled", "other", "processed"]
        let colorWords = Set(["black", "white", "gray", "grey", "red", "orange", "yellow", "green", "teal", "blue", "purple", "pink", "brown"])
        func words(_ c: String) -> [String] { c.lowercased().split { !$0.isLetter }.map(String.init) }
        let usable = candidates.filter { c in !taken.contains(c.lowercased()) && !words(c).contains { banned.contains($0) } }
        // Best: names the subject without piling on colors ("Black Crowd" describes people by color).
        let subjectWords = Set(subject.map(words) ?? [])
        let best = usable.filter { c in
            let w = words(c)
            let colorful = w.filter(colorWords.contains).count
            return (subjectWords.isEmpty || w.contains { subjectWords.contains($0) || subjectWords.contains($0 + "s") || subjectWords.contains(String($0.dropLast())) })
                && (colorful == 0 || (colorful == 1 && !s.peopleProminent))
        }
        return best.randomElement() ?? usable.randomElement() ?? candidates.first { !taken.contains($0.lowercased()) }
    }

    static func classify(_ s: Signals) async throws -> Verdict {
        let contents = s.labels.filter { $0.1 > 0.3 }.map { "\($0.0) (\(Int($0.1 * 100))%)" }.joined(separator: ", ")
        let prompt = """
        Classify this photo:
        contents: \(contents.isEmpty ? "unknown" : contents)
        faces: \(s.faceCount), covering \(Int(s.faceArea * 100))% of the frame
        people: \(s.humanCount), covering \(Int(s.humanArea * 100))% of the frame
        """
        let session = LanguageModelSession(instructions: instructions)
        return try await session.respond(to: prompt, generating: Verdict.self, options: GenerationOptions(temperature: 0)).content
    }
}
