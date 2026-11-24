import AppKit
import ImageIO
import SwiftUI

/// Downsampled thumbnails via ImageIO, cached in memory.
enum Thumbnails {
    private static let cache: NSCache<NSString, NSImage> = {
        let c = NSCache<NSString, NSImage>()
        c.countLimit = 600
        return c
    }()

    static func load(path: String, maxPixels: Int) async -> NSImage? {
        let key = "\(maxPixels):\(path)" as NSString
        if let hit = cache.object(forKey: key) { return hit }
        let image = await Task.detached(priority: .utility) { () -> NSImage? in
            let url = URL(fileURLWithPath: path) as CFURL
            guard let src = CGImageSourceCreateWithURL(url, nil) else { return nil }
            let opts: [CFString: Any] = [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceThumbnailMaxPixelSize: maxPixels,
            ]
            guard let cg = CGImageSourceCreateThumbnailAtIndex(src, 0, opts as CFDictionary) else { return nil }
            return NSImage(cgImage: cg, size: .zero)
        }.value
        if let image { cache.setObject(image, forKey: key) }
        return image
    }
}

struct Thumbnail: View {
    let path: String
    var maxPixels = 480

    @State private var image: NSImage?

    var body: some View {
        // The image lives in an overlay so it fills the frame the caller gives
        // without affecting layout (scaledToFill would otherwise grow the cell).
        Rectangle()
            .fill(.quaternary)
            .overlay {
                if let image {
                    Image(nsImage: image)
                        .resizable()
                        .scaledToFill()
                        .transition(.opacity)
                        // scaledToFill spills past the frame; clipping hides that but doesn't stop it
                        // taking clicks meant for the neighbouring cell.
                        .allowsHitTesting(false)
                } else {
                    ProgressView().controlSize(.small)
                }
            }
            .clipped()
            .contentShape(Rectangle())
        .task(id: path) {
            image = await Thumbnails.load(path: path, maxPixels: maxPixels)
        }
    }
}
