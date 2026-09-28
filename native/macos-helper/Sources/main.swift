// organize-helper: non-privileged native operations for OrganizeMyMac that are cleaner with Apple
// frameworks than from Rust. Packaged as a Tauri sidecar and started by the Rust core only.
//
//   organize-helper similar          stdin: {"paths": [...], "threshold": 0.5, "hashDistance": 12}
//                                    stdout: one JSON object with the groups; stderr: JSON progress lines
//   organize-helper photos-status    stdout: {"status": "authorized" | "denied" | ...}
//   organize-helper photos-similar   stdin: {"threshold": 0.5, "hashDistance": 12, "limit": 20000}
//
// The helper never deletes files. Photos assets are only deleted through PhotoKit, which shows
// the system confirmation and moves them to Recently Deleted (`photos-delete`).

import AppKit
import CoreGraphics
import Foundation
import ImageIO
import Photos
import Vision

// MARK: - I/O

struct SimilarInput: Decodable {
    var paths: [String] = []
    var threshold: Float = 0.5
    var hashDistance: Int = 12
    var limit: Int = 20000

    enum CodingKeys: String, CodingKey { case paths, threshold, hashDistance, limit }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        paths = try c.decodeIfPresent([String].self, forKey: .paths) ?? []
        threshold = try c.decodeIfPresent(Float.self, forKey: .threshold) ?? 0.5
        hashDistance = try c.decodeIfPresent(Int.self, forKey: .hashDistance) ?? 12
        limit = try c.decodeIfPresent(Int.self, forKey: .limit) ?? 20000
    }
}

struct Member: Encodable {
    let index: Int
    let id: String
    let width: Int
    let height: Int
    let distance: Float
}

struct Output: Encodable {
    let groups: [[Member]]
    let analyzed: Int
    let candidates: Int
    let featurePrints: Int
    let failed: Int
    let elapsedMs: Int
}

let stderr = FileHandle.standardError
func progress(_ stage: String, _ done: Int, _ total: Int) {
    let line = "{\"stage\":\"\(stage)\",\"done\":\(done),\"total\":\(total)}\n"
    stderr.write(line.data(using: .utf8)!)
}

func readInput<T: Decodable>(_ type: T.Type) -> T {
    let data = FileHandle.standardInput.readDataToEndOfFile()
    do { return try JSONDecoder().decode(T.self, from: data) } catch {
        FileHandle.standardError.write("invalid input: \(error)\n".data(using: .utf8)!)
        exit(2)
    }
}

func write<T: Encodable>(_ value: T) {
    let data = try! JSONEncoder().encode(value)
    FileHandle.standardOutput.write(data)
    FileHandle.standardOutput.write("\n".data(using: .utf8)!)
}

// MARK: - Image analysis

struct Analyzed {
    let id: String
    let image: CGImage
    let width: Int
    let height: Int
    let hash: UInt64
    var aspect: Double { Double(width) / Double(max(height, 1)) }
}

/// Thumbnail (orientation applied) and the original pixel size, without decoding the full image.
func loadThumbnail(_ url: URL, maxPixel: Int = 512) -> (CGImage, Int, Int)? {
    guard let src = CGImageSourceCreateWithURL(url as CFURL, [kCGImageSourceShouldCache: false] as CFDictionary) else { return nil }
    let props = CGImageSourceCopyPropertiesAtIndex(src, 0, nil) as? [CFString: Any]
    var w = props?[kCGImagePropertyPixelWidth] as? Int ?? 0
    var h = props?[kCGImagePropertyPixelHeight] as? Int ?? 0
    if let o = props?[kCGImagePropertyOrientation] as? Int, o >= 5 { swap(&w, &h) }
    let opts: [CFString: Any] = [
        kCGImageSourceCreateThumbnailFromImageAlways: true,
        kCGImageSourceCreateThumbnailWithTransform: true,
        kCGImageSourceThumbnailMaxPixelSize: maxPixel,
        kCGImageSourceShouldCacheImmediately: true,
    ]
    guard let thumb = CGImageSourceCreateThumbnailAtIndex(src, 0, opts as CFDictionary) else { return nil }
    return (thumb, w > 0 ? w : thumb.width, h > 0 ? h : thumb.height)
}

/// Difference hash: 9×8 grayscale, one bit per horizontal neighbour comparison.
func dHash(_ image: CGImage) -> UInt64 {
    let w = 9, h = 8
    var pixels = [UInt8](repeating: 0, count: w * h)
    let ctx = CGContext(data: &pixels, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w, space: CGColorSpaceCreateDeviceGray(), bitmapInfo: CGImageAlphaInfo.none.rawValue)!
    ctx.interpolationQuality = .medium
    ctx.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
    var hash: UInt64 = 0
    for y in 0..<h {
        for x in 0..<(w - 1) {
            hash <<= 1
            if pixels[y * w + x] > pixels[y * w + x + 1] { hash |= 1 }
        }
    }
    return hash
}

func featurePrint(_ image: CGImage) -> VNFeaturePrintObservation? {
    let request = VNGenerateImageFeaturePrintRequest()
    let handler = VNImageRequestHandler(cgImage: image, options: [:])
    do {
        try handler.perform([request])
        return request.results?.first
    } catch {
        return nil
    }
}

final class UnionFind {
    var parent: [Int]
    init(_ n: Int) { parent = Array(0..<n) }
    func find(_ x: Int) -> Int {
        var x = x
        while parent[x] != x { parent[x] = parent[parent[x]]; x = parent[x] }
        return x
    }
    func union(_ a: Int, _ b: Int) { let (ra, rb) = (find(a), find(b)); if ra != rb { parent[rb] = ra } }
}

/// Pre-filter by aspect ratio and perceptual hash (cheap, all pairs), then Vision feature prints
/// only for images that have at least one candidate pair, and cluster pairs under the threshold.
func cluster(_ items: [Analyzed], indices: [Int], threshold: Float, hashDistance: Int, started: Date, failed: Int) -> Output {
    let n = items.count
    var pairs: [(Int, Int)] = []
    let order = (0..<n).sorted { items[$0].aspect < items[$1].aspect }
    for a in 0..<n {
        let i = order[a]
        var b = a + 1
        // Sorted by aspect ratio: stop as soon as ratios differ by more than 8%.
        while b < n, items[order[b]].aspect <= items[i].aspect * 1.08 {
            let j = order[b]
            if (items[i].hash ^ items[j].hash).nonzeroBitCount <= hashDistance { pairs.append((i, j)) }
            b += 1
        }
    }
    let involved = Array(Set(pairs.flatMap { [$0.0, $0.1] })).sorted()
    var prints = [Int: VNFeaturePrintObservation]()
    let lock = NSLock()
    var done = 0
    DispatchQueue.concurrentPerform(iterations: involved.count) { k in
        let i = involved[k]
        let fp = featurePrint(items[i].image)
        lock.lock()
        if let fp { prints[i] = fp }
        done += 1
        if done % 8 == 0 || done == involved.count { progress("features", done, involved.count) }
        lock.unlock()
    }
    let uf = UnionFind(n)
    var best = [Int: Float]()
    for (i, j) in pairs {
        guard let a = prints[i], let b = prints[j] else { continue }
        var d: Float = 0
        guard (try? a.computeDistance(&d, to: b)) != nil else { continue }
        if d <= threshold {
            uf.union(i, j)
            best[i] = min(best[i] ?? .infinity, d)
            best[j] = min(best[j] ?? .infinity, d)
        }
    }
    var groups = [Int: [Int]]()
    for i in 0..<n where best[i] != nil { groups[uf.find(i), default: []].append(i) }
    let out = groups.values.filter { $0.count > 1 }.map { g in
        g.map { i in Member(index: indices[i], id: items[i].id, width: items[i].width, height: items[i].height, distance: best[i] ?? 0) }
    }
    return Output(groups: out, analyzed: n, candidates: involved.count, featurePrints: prints.count, failed: failed, elapsedMs: Int(Date().timeIntervalSince(started) * 1000))
}

// MARK: - Commands

func similarFiles() {
    let input = readInput(SimilarInput.self)
    let started = Date()
    var items = [Analyzed?](repeating: nil, count: input.paths.count)
    let lock = NSLock()
    var done = 0
    DispatchQueue.concurrentPerform(iterations: input.paths.count) { i in
        autoreleasepool {
            let url = URL(fileURLWithPath: input.paths[i])
            if let (thumb, w, h) = loadThumbnail(url) {
                let a = Analyzed(id: input.paths[i], image: thumb, width: w, height: h, hash: dHash(thumb))
                lock.lock(); items[i] = a; lock.unlock()
            }
            lock.lock()
            done += 1
            if done % 16 == 0 || done == input.paths.count { progress("thumbnails", done, input.paths.count) }
            lock.unlock()
        }
    }
    let valid = items.enumerated().compactMap { i, a in a.map { (i, $0) } }
    let result = cluster(valid.map { $0.1 }, indices: valid.map { $0.0 }, threshold: input.threshold, hashDistance: input.hashDistance, started: started, failed: input.paths.count - valid.count)
    write(result)
}

func photosStatusString(_ s: PHAuthorizationStatus) -> String {
    switch s {
    case .authorized: return "authorized"
    case .limited: return "limited"
    case .denied: return "denied"
    case .restricted: return "restricted"
    case .notDetermined: return "notDetermined"
    @unknown default: return "unknown"
    }
}

func photosStatus(request: Bool) {
    var status = PHPhotoLibrary.authorizationStatus(for: .readWrite)
    if request && status == .notDetermined {
        let sem = DispatchSemaphore(value: 0)
        PHPhotoLibrary.requestAuthorization(for: .readWrite) { s in status = s; sem.signal() }
        sem.wait()
    }
    write(["status": photosStatusString(status)])
}

/// Similar photos in the Photos library, through PhotoKit thumbnails (never the library files).
func photosSimilar() {
    let input = readInput(SimilarInput.self)
    let started = Date()
    guard [.authorized, .limited].contains(PHPhotoLibrary.authorizationStatus(for: .readWrite)) else {
        write(["error": "not authorized"]); exit(3)
    }
    let options = PHFetchOptions()
    options.sortDescriptors = [NSSortDescriptor(key: "creationDate", ascending: false)]
    options.fetchLimit = input.limit
    let assets = PHAsset.fetchAssets(with: .image, options: options)
    let manager = PHImageManager.default()
    let req = PHImageRequestOptions()
    req.isSynchronous = true
    req.deliveryMode = .fastFormat
    req.resizeMode = .fast
    req.isNetworkAccessAllowed = false // never download originals from iCloud
    var items: [Analyzed] = []
    var indices: [Int] = []
    var failed = 0
    assets.enumerateObjects { asset, i, _ in
        autoreleasepool {
            var cg: CGImage?
            manager.requestImage(for: asset, targetSize: CGSize(width: 512, height: 512), contentMode: .aspectFit, options: req) { image, _ in
                cg = image?.cgImage(forProposedRect: nil, context: nil, hints: nil)
            }
            if let cg {
                items.append(Analyzed(id: asset.localIdentifier, image: cg, width: asset.pixelWidth, height: asset.pixelHeight, hash: dHash(cg)))
                indices.append(i)
            } else {
                failed += 1
            }
            if i % 32 == 0 { progress("thumbnails", i + 1, assets.count) }
        }
    }
    write(cluster(items, indices: indices, threshold: input.threshold, hashDistance: input.hashDistance, started: started, failed: failed))
}

struct DeleteInput: Decodable { let ids: [String] }

/// Deletes Photos assets through PhotoKit: macOS asks the user to confirm, and the photos go to
/// Recently Deleted in Photos.
func photosDelete() {
    let input = readInput(DeleteInput.self)
    let assets = PHAsset.fetchAssets(withLocalIdentifiers: input.ids, options: nil)
    let sem = DispatchSemaphore(value: 0)
    var ok = false
    var message: String?
    PHPhotoLibrary.shared().performChanges({ PHAssetChangeRequest.deleteAssets(assets) }) { success, error in
        ok = success; message = error?.localizedDescription; sem.signal()
    }
    sem.wait()
    write(["ok": ok ? "true" : "false", "error": message ?? ""])
}

struct ThumbInput: Decodable { let id: String; let out: String }

/// Writes a 320 px JPEG preview of one Photos asset (from PhotoKit, never downloading originals).
func photosThumbnail() {
    let input = readInput(ThumbInput.self)
    guard let asset = PHAsset.fetchAssets(withLocalIdentifiers: [input.id], options: nil).firstObject else { write(["ok": "false"]); return }
    let req = PHImageRequestOptions()
    req.isSynchronous = true
    req.deliveryMode = .highQualityFormat
    req.isNetworkAccessAllowed = false
    var data: Data?
    PHImageManager.default().requestImage(for: asset, targetSize: CGSize(width: 320, height: 320), contentMode: .aspectFit, options: req) { image, _ in
        guard let image, let tiff = image.tiffRepresentation, let rep = NSBitmapImageRep(data: tiff) else { return }
        data = rep.representation(using: .jpeg, properties: [.compressionFactor: 0.7])
    }
    if let data, (try? data.write(to: URL(fileURLWithPath: input.out))) != nil { write(["ok": "true"]) } else { write(["ok": "false"]) }
}

struct EvictInput: Decodable { let paths: [String] }
struct EvictOutcome: Encodable { let path: String; let ok: Bool; let error: String? }

/// Removes local copies of iCloud Drive files (they stay in iCloud and download again on open).
/// Public API; fails for files that are not uploaded yet.
func evict() {
    let input = readInput(EvictInput.self)
    let out = input.paths.map { p -> EvictOutcome in
        do {
            try FileManager.default.evictUbiquitousItem(at: URL(fileURLWithPath: p))
            return EvictOutcome(path: p, ok: true, error: nil)
        } catch {
            return EvictOutcome(path: p, ok: false, error: error.localizedDescription)
        }
    }
    write(out)
}

let args = CommandLine.arguments
switch args.count > 1 ? args[1] : "" {
case "similar": similarFiles()
case "photos-status": photosStatus(request: args.contains("--request"))
case "photos-similar": photosSimilar()
case "photos-delete": photosDelete()
case "photos-thumbnail": photosThumbnail()
case "evict": evict()
case "--version": print("organize-helper 1")
default:
    FileHandle.standardError.write("usage: organize-helper similar|photos-status [--request]|photos-similar|photos-delete\n".data(using: .utf8)!)
    exit(64)
}
