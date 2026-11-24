import Foundation

/// Plain-language tags for browsing and filtering, derived on-device from the same
/// analysis as the rating: Vision's scene labels and people detectors, plus the
/// photo kind chosen by Apple Intelligence (already checked against the labels).
enum PhotoTag: String, CaseIterable, Identifiable {
    case people, landscape, water, sky, nature, flowers, animals, city, architecture
    case art, food, night, snow, vehicles, indoor, documents

    var id: String { rawValue }

    var title: String {
        switch self {
        case .people: return "People"
        case .landscape: return "Landscapes"
        case .water: return "Water & Beaches"
        case .sky: return "Skies & Sunsets"
        case .nature: return "Nature"
        case .flowers: return "Flowers"
        case .animals: return "Pets & Animals"
        case .city: return "City"
        case .architecture: return "Architecture"
        case .art: return "Art & Color"
        case .food: return "Food"
        case .night: return "Night"
        case .snow: return "Snow"
        case .vehicles: return "Vehicles"
        case .indoor: return "Indoors"
        case .documents: return "Documents"
        }
    }

    var symbol: String {
        switch self {
        case .people: return "person.2"
        case .landscape: return "mountain.2"
        case .water: return "water.waves"
        case .sky: return "cloud.sun"
        case .nature: return "leaf"
        case .flowers: return "camera.macro"
        case .animals: return "pawprint"
        case .city: return "building.2"
        case .architecture: return "building.columns"
        case .art: return "paintpalette"
        case .food: return "fork.knife"
        case .night: return "moon.stars"
        case .snow: return "snowflake"
        case .vehicles: return "car"
        case .indoor: return "sofa"
        case .documents: return "doc.text"
        }
    }

    /// Vision label words (whole words of identifiers like "water_body") that imply this tag.
    fileprivate var labelWords: [String] {
        switch self {
        case .people: return ["people", "adult", "child", "baby", "crowd", "selfie", "wedding"]
        case .landscape: return ["mountain", "hill", "land", "canyon", "desert", "valley", "field", "cliff", "volcano"]
        case .water: return ["ocean", "sea", "beach", "lake", "river", "waterfall", "water", "shore", "coast", "harbour"]
        case .sky: return ["sky", "cloudy", "sunset", "sunrise", "aurora", "rainbow"]
        case .nature: return ["forest", "tree", "plant", "grass", "foliage", "jungle", "garden", "moss", "leaf"]
        case .flowers: return ["flower", "blossom", "rose", "tulip", "daisy", "sunflower", "orchid", "lily", "peony", "lavender", "bouquet"]
        case .animals: return ["animal", "dog", "cat", "bird", "mammal", "canine", "feline", "horse", "fish", "puppy", "kitten", "rabbit"]
        case .city: return ["cityscape", "skyline", "skyscraper", "street", "downtown"]
        case .architecture: return ["building", "church", "castle", "tower", "bridge", "monument", "temple", "cathedral", "palace"]
        case .art: return ["art", "decoration", "mask", "graffiti", "illustrations", "painting", "mural", "sculpture", "mosaic", "stained"]
        case .food: return ["food", "dish", "meal", "dessert", "fruit", "vegetable", "baked", "pizza", "sushi"]
        case .night: return ["night", "fireworks", "nightclub"]
        case .snow: return ["snow", "ice", "glacier", "ski", "skiing", "winter"]
        case .vehicles: return ["car", "vehicle", "automobile", "bicycle", "train", "airplane", "boat", "motorcycle", "bus", "streetcar"]
        case .indoor: return ["indoor", "room", "interior", "kitchen", "bedroom", "restaurant", "living"]
        case .documents: return ["document", "handwriting", "printed", "screenshot", "receipt"]
        }
    }

    /// Photo kinds that imply a tag even if no single label crossed the threshold.
    fileprivate static func implied(by kind: PhotoRater.Kind) -> PhotoTag? {
        switch kind {
        case .landscape: return .landscape
        case .seascape: return .water
        case .sky: return .sky
        case .cityscape: return .city
        case .architecture: return .architecture
        case .nature_closeup: return .nature
        case .abstract_or_pattern: return .art
        case .animal: return .animals
        case .people_in_scenery, .portrait_or_selfie, .group_photo: return .people
        case .food: return .food
        case .document_or_screenshot: return .documents
        case .vehicle: return .vehicles
        case .indoor_casual: return .indoor
        case .other: return nil
        }
    }

    static func tags(for s: PhotoRater.Signals, kind: PhotoRater.Kind) -> [PhotoTag] {
        var tags = Set<PhotoTag>()
        // People are matched on whole labels below, so "adult_cat" isn't a person.
        for tag in allCases where tag != .people && s.hasLabel(tag.labelWords, minimum: 0.35) {
            tags.insert(tag)
        }
        if s.hasPeopleLabel { tags.insert(.people) }
        if let t = implied(by: kind) { tags.insert(t) }
        // Detectors catch people the labels miss (and the other way round).
        if s.peopleProminent || s.humanCount > 0 || (s.faceCount > 0 && s.animalConfidence < 0.5) { tags.insert(.people) }
        // Avoid noise: "documents" only when the rater agreed it's a document.
        if kind != .document_or_screenshot { tags.remove(.documents) }
        return allCases.filter(tags.contains)
    }
}
