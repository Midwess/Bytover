//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SharedTypes
import SwiftUI
import Serde
import PhotosUI
import Photos
import AVFoundation
import CoreLocation
import Combine
import QuickLookThumbnailing

enum MyError: Error {
    case invalidInput(reason: String)
}

final class SingleWaiter<T> {
    private var continuation: CheckedContinuation<T, Never>?

    func wait() async -> T {
        await withCheckedContinuation { continuation in
            self.continuation = continuation
        }
    }

    func resolve(_ value: T) {
        continuation?.resume(returning: value)
        continuation = nil
    }
}

@MainActor
class Core: NSObject, ObservableObject, ShellRuntime, @preconcurrency CLLocationManagerDelegate {
    @Published var isSignedIn = false
    @Published var selectedMediaItems: [PhotosPickerItem] = []

    var environment: CurrentValueSubject<EnvironmentViewModel?, Never> = .init(nil)
    var authentication: CurrentValueSubject<AuthenticationViewModel?, Never> = .init(nil)
    var transfer: CurrentValueSubject<TransferViewModel?, Never> = .init(nil)
    var nearby: CurrentValueSubject<NearbyViewModel?, Never> = .init(nil)
    var quicklook_path: CurrentValueSubject<LocalResourcePath?, Never> = .init(nil)
    var cloudSession: CurrentValueSubject<CloudSession?, Never> = .init(nil)
    var selectedTransfer: CurrentValueSubject<TransferMethodSelection, Never> = .init(.internet)
    var privatePath: URL?
    var publicPath: URL?

    var alert: CurrentValueSubject<(AlertDialog, SingleWaiter<Bool>)?, Never> = .init(nil)
    var toastMessage: CurrentValueSubject<String?, Never> = .init(nil)

    @Environment(\.openURL) private var openURL

    var lastKnownLocation: CLLocationCoordinate2D?
    var manager = CLLocationManager()
    var nativeProcessor: NativeProcessor?

    override init() {
        super.init()
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        manager.delegate = self

        update_view(app)
    }

    func update_view(_ model: AppViewModel) {
        self.authentication.send(model.authentication)
        self.environment.send(model.environment)
        self.transfer.send(model.transfer)
        self.nearby.send(model.nearby)
        self.cloudSession.send(model.transfer?.cloud_session)

        if self.authentication.value?.user != nil {
            self.isSignedIn = true
        }
    }

    func update(_ event: AppEvent) async {
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))

        var requests: [Request] = try! .bincodeDeserialize(input: effects)

        while let request = requests.first {
            requests.removeFirst()
            let data = [UInt8](await processEffect(request))

            if let newRequests: [Request] = try? .bincodeDeserialize(input: data) {
                requests.append(contentsOf: newRequests)
            }
        }
    }

    func _handleResponse(_ id: UInt32, _ response: CoreOperationOutput) async {
        let effects = [UInt8](handleResponse(id, Data(try! response.bincodeSerialize())))
        if effects.isEmpty {
            return
        }

        var requests: [Request] = try! .bincodeDeserialize(input: effects)
        while let request = requests.first {
            requests.removeFirst()
            let data = [UInt8](await processEffect(request))

            if let newRequests: [Request] = try? .bincodeDeserialize(input: data) {
                requests.append(contentsOf: newRequests)
            }
        }
    }

    func nativeProcessor() async -> NativeProcessor {
        while self.nativeProcessor == nil {
            try? await Task.sleep(nanoseconds: 100000000) // 100ms
        }

        return self.nativeProcessor!
    }

    func processEffect(_ request: Request) async -> Data {
        switch request.effect {
        case .appCapabilities(.initNativeExecutor):
            self.nativeProcessor = await NativeProcessor.init(self)
            self.checkLocationAuthorization()
            return handleResponse(request.id, Data(try! CoreOperationOutput.initNativeExecutor.bincodeSerialize()))
        case .appCapabilities(.webView(.openUrl(let url))):
            openURL(URL(string: url)!)
            return handleResponse(request.id, Data(try! CoreOperationOutput.webView(WebViewOperationOutput.openUrl).bincodeSerialize()))
        case .appCapabilities(.device(.loadThumbnailPng(let localStoragePath))):
            switch localStoragePath {
            case .platformIdentifier(let identifier):
                let thumbnail = await self.getThumbnailData(for: identifier)
                return handleResponse(request.id, Data(try! CoreOperationOutput.device(.loadThumbnailPng(thumbnail?.bytes)).bincodeSerialize()))
            default:
                return handleResponse(request.id, Data(try! CoreOperationOutput.device(.loadThumbnailPng(nil)).bincodeSerialize()))
            }
        case .appCapabilities(.device(.open(.open(let path)))):
            await self.open(path: path)
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.device(.open(.openSession(let path)))):
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.device(.getGeoLocation)):
            if let lastKnownLocation {
                let longitude = lastKnownLocation.longitude.magnitude
                let latitude = lastKnownLocation.latitude.magnitude
                return handleResponse(request.id, Data(try! CoreOperationOutput.device(.getGeoLocation(GeoLocation(latitude: latitude, longitude: longitude))).bincodeSerialize()))
            }

            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.persistent(let ops)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.persistent(ops).bincodeSerialize()))
        case .appCapabilities(.device(.getDeviceInfo)):
            let device = UIDevice.current
            let deviceId = UIDevice.current.identifierForVendor?.uuidString ?? ""
            let deviceName = device.name

            return handleResponse(request.id, Data(try! CoreOperationOutput.device(
                .deviceInfo(DeviceInfo(
                    platform: Platform.ios,
                    name: deviceName,
                    unique_id: deviceId,
                    device_type: .applePhone
                ))
            ).bincodeSerialize()))
        case .appCapabilities(.device(.getGeoLocation)):
            let geoLocation = GeoLocation(latitude: self.lastKnownLocation?.latitude ?? 0.0, longitude: self.lastKnownLocation?.longitude ?? 0.0)
            return handleResponse(request.id, Data(try! CoreOperationOutput.device(.getGeoLocation(geoLocation)).bincodeSerialize()))
        case .appCapabilities(.rpc(let rpc)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.rpc(rpc).bincodeSerialize()))
        case .appCapabilities(.render):
            self.update_view(try! .bincodeDeserialize(input: [UInt8](BitBridge.view())))
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.transfer(let trans)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.transfer(trans).bincodeSerialize()))
        case .appCapabilities(.internet(let internetOps)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.internet(internetOps).bincodeSerialize()))
        case .appCapabilities(.p2P(let p2p)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.p2P(p2p).bincodeSerialize()))
        case .appCapabilities(.notified(let event)):
            Task {
                await self.update(event)
            }

            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.dialog(.alert(let alert))):
            self.alert.send((alert, SingleWaiter()))
            let response = await self.alert.value!.1.wait()
            self.alert.send(nil)
            return handleResponse(request.id, Data(try! CoreOperationOutput.dialog(.alert(is_confirmed: response)).bincodeSerialize()))
        case .appCapabilities(.dialog(.toast(let message))):
            self.toastMessage.send(message)
            return handleResponse(request.id, Data(try! CoreOperationOutput.dialog(.toast).bincodeSerialize()))
        case .appCapabilities(.delay(let duration)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.delay(duration).bincodeSerialize()))
        case .appCapabilities(.dialog(.message)):
            return Data(try! CoreOperationOutput.void.bincodeSerialize())
        }
    }

    func msgFromNative(_ event: Data) async -> Data {
        let event: MessageToShell = try! .bincodeDeserialize(input: event.bytes)
        switch event {
        case .handleResponse(let id, let response):
            await self._handleResponse(id, response)
            return Data(try! MessageToShellResponse.voidResponse.bincodeSerialize())
        case .pathResolver(.getAbsolutePath(let path)):
            let result = await self.resolveAbsolutePath(path: path) ?? ""
            return Data(try! MessageToShellResponse.pathResolverResponse(.getAbsolutePath(absolute_path: result)).bincodeSerialize())
        case .pathResolver(.getLocalResourcePath(let absolute_path)):
            let result = try! await self.getRelativePath(absolutePath: absolute_path)
            return Data(try! MessageToShellResponse.pathResolverResponse(.getLocalResourcePath(path: result)).bincodeSerialize())
        case .pathResolver(.getSessionDirPath(let session_id)):
            let public_dir = self.getDocumentsDirectory(isPrivate: false).path
            let session_dir = "\(public_dir)/session-\(session_id)"
            return Data(try! MessageToShellResponse.pathResolverResponse(.getSessionDirPath(path: session_dir)).bincodeSerialize())
        case .pathResolver(.getSystemDirPath):
            let private_dir = self.getDocumentsDirectory(isPrivate: true).path
            return Data(try! MessageToShellResponse.pathResolverResponse(.getSystemDirPath(path: private_dir)).bincodeSerialize())
        case .pathResolver(.getThumbnailDirPath):
            let private_dir = self.getDocumentsDirectory(isPrivate: true).path
            let thumbnail_dir = "\(private_dir)/thumbnails"
            return Data(try! MessageToShellResponse.pathResolverResponse(.getThumbnailDirPath(path: thumbnail_dir)).bincodeSerialize())
        case .notify(let event):
            await self.update(event)
            return Data(try! MessageToShellResponse.voidResponse.bincodeSerialize())
        }
    }

    func getAbsoluteUrl(from platformIdentifier: String) async -> String? {
        let type = platformIdentifier.split(separator: "://").first?.description ?? ""
        switch type {
        case "bookmark":
            let bookmarkString = String(platformIdentifier.dropFirst("bookmark://".count))
            guard let bookmarkData = Data(base64Encoded: bookmarkString) else {
                return nil
            }

            var isStale = false
            do {
                let url = try URL(resolvingBookmarkData: bookmarkData, relativeTo: nil, bookmarkDataIsStale: &isStale)
                if isStale {
                    return nil
                }

                return url.path
            } catch {
                return nil
            }
        case "phasset":
            let identifier = platformIdentifier.dropFirst("phasset://".count)

            return await PHAsset.getCachedAsset(identifier: String(identifier))?.fileUrl?.path ?? ""
        default:
            return nil
        }
    }

    func onFileSelected(urls: [URL]) async {
        await self.update(.transfer(.beginLoadingResources))
        for url in urls {
            _ = url.startAccessingSecurityScopedResource()

            guard let bookmarkData = try? url.bookmarkData(options: .minimalBookmark, includingResourceValuesForKeys: nil, relativeTo: nil) else {
                continue
            }

            let bookmarkString = bookmarkData.base64EncodedString()
            let bookmarkUrl = "bookmark://" + bookmarkString

            let resourceSelection = ResourceSelection(
                path: .platformIdentifier(bookmarkUrl),
                type: nil
            )

            url.stopAccessingSecurityScopedResource()
            await self.update(.transfer(.addResources([resourceSelection])))
        }

        await self.update(.transfer(.endLoadingResources))
    }

    func onMediasChanged() async {
        await self.update(.transfer(.beginLoadingResources))
        defer { Task { await self.update(.transfer(.endLoadingResources)) } }

        let items = self.selectedMediaItems
        guard !items.isEmpty else {
            self.selectedMediaItems.removeAll()
            return
        }

        let chunkSize = 5
        for i in stride(from: 0, to: items.count, by: chunkSize) {
            let upperBound = min(i + chunkSize, items.count)
            let chunk = Array(items[i..<upperBound])

            await withThrowingTaskGroup(of: Void.self) { group in
                for item in chunk {
                    group.addTask { [weak self] in
                        guard let self else { return }
                        await self.processPickedMediaItem(item)
                    }
                }

                try? await group.waitForAll()
            }
        }

        self.selectedMediaItems.removeAll()
    }

    private func processPickedMediaItem(_ item: PhotosPickerItem) async {
        guard let identifier = item.itemIdentifier else { return }
        guard let assetCached = await PHAsset.getCachedAsset(identifier: identifier) else { return }

        let assetType = assetCached.asset.mediaType
        let resourceType: ResourceType = {
            switch assetType {
            case .image: return .image
            case .video: return .video
            default: return .file
            }
        }()

        let phassetUrl = "phasset://" + identifier
        let resourceSelection = ResourceSelection(
            path: .platformIdentifier(phassetUrl),
            type: resourceType
        )

        await self.update(.transfer(.addResources([resourceSelection])))
    }

    func open(path: LocalResourcePath) async {
        quicklook_path.value = path
    }

    func getFileSize(item_identifier: String) async -> UInt64 {
        return await PHAsset.getCachedAsset(identifier: item_identifier)?.fileSize ?? 0
    }

    func getFileName(item_identifier: String) async -> String {
        return await PHAsset.getCachedAsset(identifier: item_identifier)?.fileName ?? ""
    }

    func getDocumentsDirectory(isPrivate: Bool) -> URL {
        if privatePath != nil && isPrivate {
            return privatePath!
        }

        if publicPath != nil && !isPrivate {
            return publicPath!
        }

        privatePath = try! FileManager.default.url(for: .applicationSupportDirectory, in: .userDomainMask, appropriateFor: nil, create: true)

        publicPath = try! FileManager.default.url(for: .documentDirectory, in: .userDomainMask, appropriateFor: nil, create: true)

        return isPrivate ? privatePath! : publicPath!
    }

    func getThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 256, height: 256)) async -> Data? {
        if itemIdentifier.starts(with: "phasset://") {
            let assetId = itemIdentifier.split(separator: "://").last!.description
            guard let asset_cached = await PHAsset.getCachedAsset(identifier: assetId) else {
                return nil
            }

            if asset_cached.asset.mediaType == .video {
                return await getVideoThumbnailData(for: assetId, size: size)
            }

            let manager = PHImageManager.default()
            let options = PHImageRequestOptions()
            options.resizeMode = .fast
            options.isNetworkAccessAllowed = true
            options.deliveryMode = .opportunistic
            options.isSynchronous = false

            return await withCheckedContinuation { continuation in
                manager.requestImage(
                    for: asset_cached.asset,
                    targetSize: size,
                    contentMode: .aspectFit,
                    options: options
                ) { image, info in
                    let isDegraded = (info?[PHImageResultIsDegradedKey] as? Bool) ?? false
                    let error = info?[PHImageErrorKey] as? Error

                    if let image = image, !isDegraded, error == nil, let pngData = image.pngData() {
                        continuation.resume(returning: pngData)
                    } else if error != nil {
                        continuation.resume(returning: nil)
                    }
                }
            }
        } else if itemIdentifier.starts(with: "bookmark://") {
            guard let absolutePath = await getAbsoluteUrl(from: itemIdentifier) else {
                print("Cannot generate an absolute url from bookmark: \(itemIdentifier)")
                return nil
            }

            let url = URL(fileURLWithPath: absolutePath)

            let thumbnailSize = CGSize(width: size.width, height: size.height)
            let scale = UIScreen.main.scale
            let request = QLThumbnailGenerator.Request(
                fileAt: url,
                size: thumbnailSize,
                scale: scale,
                representationTypes: .all
            )

            return await withCheckedContinuation { continuation in
                QLThumbnailGenerator.shared.generateBestRepresentation(for: request) { thumbnail, error in
                    if let thumbnail = thumbnail,
                       let pngData = thumbnail.uiImage.pngData() {
                        continuation.resume(returning: pngData)
                    } else {
                        print("Failed to generate thumbnail: \(error?.localizedDescription ?? "unknown error")")
                        continuation.resume(returning: nil)
                    }
                }
            }
        }

        return nil
    }

    func checkLocationAuthorization() {
        manager.delegate = self
        manager.startUpdatingLocation()

        switch manager.authorizationStatus {
        case .notDetermined:
            manager.requestWhenInUseAuthorization()
        case .authorizedWhenInUse:
            print("Location authorized when in use")
            lastKnownLocation = manager.location?.coordinate
        default:
            print("Location service disabled")
        }
    }

    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        if let location = locations.first?.coordinate {
            lastKnownLocation = locations.first?.coordinate
            Task {
                manager.stopUpdatingLocation()
            }
        }
    }

    func getVideoThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 128, height: 128)) async -> Data? {
        let fetchResult: PHAssetCached? = await PHAsset.getCachedAsset(identifier: itemIdentifier)
        guard let asset_cached = fetchResult, asset_cached.asset.mediaType == .video else {
            return nil
        }

        let manager = PHImageManager.default()
        let options = PHVideoRequestOptions()
        options.isNetworkAccessAllowed = true

        let avAsset = await withCheckedContinuation { continuation in
            manager.requestAVAsset(forVideo: asset_cached.asset, options: options) { avAsset, _, _ in
                continuation.resume(returning: avAsset)
            }
        }

        guard let avAsset = avAsset else {
            return nil
        }

        let imageGenerator = AVAssetImageGenerator(asset: avAsset)
        imageGenerator.appliesPreferredTrackTransform = true
        imageGenerator.maximumSize = size

        let cgImage: CGImage? = await withCheckedContinuation { continuation in
            imageGenerator.generateCGImageAsynchronously(for: CMTime(seconds: 1, preferredTimescale: 60)) { cgImage, _, error in
                if let cgImage = cgImage {
                    continuation.resume(returning: cgImage)
                } else {
                    print("Error or no image generated: \(error?.localizedDescription ?? "Unknown error")")
                    continuation.resume(returning: nil)
                }
            }
        }

        if let cgImage = cgImage {
            return UIImage(cgImage: cgImage).pngData()
        } else {
            return nil
        }
    }

    func resolveAbsolutePath(path: LocalResourcePath) async -> String? {
        switch path {
        case .absolutePath(let path):
            return path
        case .relativePath(let path, let isPrivate):
            let workdir = self.getDocumentsDirectory(isPrivate: isPrivate)
            let path = workdir.appendingPathComponent(path).path
            return path
        case .platformIdentifier(let identifier):
            return await self.getAbsoluteUrl(from: identifier)
        }
    }

    func getRelativePath(absolutePath: String) async throws -> LocalResourcePath {
        let private_path = self.getDocumentsDirectory(isPrivate: true).path
        let public_path = self.getDocumentsDirectory(isPrivate: false).path
        if absolutePath.hasPrefix(private_path) {
            var path = String(absolutePath.dropFirst(private_path.count))
            if path.hasPrefix("/") {
                path = String(path.dropFirst(1))
            }

            print("Resolving from \(absolutePath) to private \(path)")
            return LocalResourcePath.relativePath(path: path, is_private: true)
        }

        if absolutePath.hasPrefix(public_path) {
            var path = String(absolutePath.dropFirst(public_path.count))
            if path.hasPrefix("/") {
                path = String(path.dropFirst(1))
            }

            print("Resolving from \(absolutePath) to private \(path)")
            return LocalResourcePath.relativePath(path: path, is_private: false)
        }

        throw MyError.invalidInput(reason: "The absolutePath is not in the sandboxed directory")
    }
}

extension Data {
    var bytes: [UInt8] {
        return [UInt8](self)
    }
}

extension UIImage {
    static func fromAbsolutePath(_ path: String) -> UIImage? {
        return UIImage(contentsOfFile: path)
    }

    static func fromURL(_ url: URL) -> UIImage? {
        guard url.isFileURL else { return nil }
        return UIImage(contentsOfFile: url.path)
    }
}

extension Image {
    static func fromPath(_ path: LocalResourcePath, core: Core) async -> Image? {
        switch path {
        case .absolutePath(let path):
            guard let uiImage = UIImage.fromAbsolutePath(path) else {
                return nil
            }
            return Image(uiImage: uiImage)
        case .relativePath(let path, let isPrivate):
            let workdir = await core.getDocumentsDirectory(isPrivate: isPrivate)
            let fullPath = workdir.appendingPathComponent(path).path
            guard let uiImage = UIImage.fromAbsolutePath(fullPath) else {
                return nil
            }

            return Image(uiImage: uiImage)
        case .platformIdentifier(let identifier):
            guard let cachedAsset = await PHAsset.getCachedAsset(identifier: identifier),
                  let fileUrl = cachedAsset.fileUrl,
                  let uiImage = UIImage.fromURL(fileUrl) else {
                return nil
            }
            return Image(uiImage: uiImage)
        }
    }
}

class PHAssetCached {
    var fileSize: UInt64
    var fileName: String
    var fileUrl: URL?
    var asset: PHAsset

    init(fileName: String, fileSize: UInt64, fileUrl: URL?, asset: PHAsset) {
        self.fileName = fileName
        self.fileSize = fileSize
        self.fileUrl = fileUrl
        self.asset = asset
    }
}

class AssetCache {
    static var shared = AssetCache()
    private var cache: [String: PHAssetCached] = [:]
    private let queue = DispatchQueue(label: "com.bitbridge.assetcache")

    func get(identifier: String) -> PHAssetCached? {
        queue.sync {
            return cache[identifier]
        }
    }

    func set(identifier: String, asset: PHAssetCached) {
        queue.sync {
            cache[identifier] = asset
        }
    }

    func clear() {
        queue.sync {
            cache.removeAll()
        }
    }
}

extension PHAsset {
    static func getCachedAsset(identifier: String, _ includeUrl: Bool = true) async -> PHAssetCached? {
        let identifier = identifier.components(separatedBy: "://").last!

        if let cached = AssetCache.shared.get(identifier: identifier) {
            return cached
        }

        let options = PHFetchOptions()
        options.fetchLimit = 1
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: options)
        guard let asset = fetchResult.firstObject,
              let resource = PHAssetResource.assetResources(for: asset).first else {
                return nil
        }

        let fileSize = resource.value(forKey: "fileSize") as? Int ?? 0

        let url = includeUrl ? await asset.getAbsoluteURL() : nil
        let cached = PHAssetCached(
            fileName: resource.originalFilename,
            fileSize: UInt64(fileSize),
            fileUrl: url,
            asset: asset
        )

        AssetCache.shared.set(identifier: identifier, asset: cached)
        return cached
    }

    func getAbsoluteURL(completionHandler: @escaping (URL?) -> Void) {
        switch self.mediaType {
        case .image:
            let options = PHContentEditingInputRequestOptions()
            options.isNetworkAccessAllowed = true
            options.canHandleAdjustmentData = { _ in return false }

            self.requestContentEditingInput(with: options) { (contentEditingInput, _) in
                completionHandler(contentEditingInput?.fullSizeImageURL)
            }

        case .video:
            let options = PHVideoRequestOptions()
            options.version = .original
            options.isNetworkAccessAllowed = true
            options.deliveryMode = .highQualityFormat

            PHImageManager.default().requestAVAsset(forVideo: self, options: options) { (asset, _, _) in
                if let urlAsset = asset as? AVURLAsset {
                    completionHandler(urlAsset.url)
                } else {
                    completionHandler(nil)
                }
            }

        default:
            let resources = PHAssetResource.assetResources(for: self)
            if let resource = resources.first {
                let tempDirURL = FileManager.default.temporaryDirectory
                let fileName = resource.originalFilename
                let localURL = tempDirURL.appendingPathComponent(fileName)

                try? FileManager.default.removeItem(at: localURL)

                PHAssetResourceManager.default().writeData(for: resource, toFile: localURL, options: nil) { (error) in
                    if error == nil {
                        completionHandler(localURL)
                    } else {
                        completionHandler(nil)
                    }
                }
            } else {
                completionHandler(nil)
            }
        }
    }

    func getAbsoluteURL() async -> URL? {
        return await withCheckedContinuation { continuation in
            getAbsoluteURL { url in
                continuation.resume(returning: url)
            }
        }
    }
}
extension View {
    func onAppearAndReceive<P: Publisher>(
        _ publisher: P,
        defaultValue: P.Output? = nil,
        perform action: @escaping (P.Output) -> Void
    ) -> some View where P.Failure == Never {
        self
            .onAppear {
                if let defaultValue = defaultValue {
                    action(defaultValue)
                } else if let currentValue = publisher as? CurrentValueSubject<P.Output, Never> {
                    currentValue.send(currentValue.value)
                }
            }
            .onReceive(publisher, perform: action)
    }

    func onAppearOrChange<V: Equatable>(
        of value: V,
        perform action: @escaping (V?, V) -> Void
    ) -> some View {
        self
            .onAppear {
                action(nil, value)
            }
            .onChange(of: value) { oldValue, newValue in
                action(oldValue, newValue)
            }
    }
}
