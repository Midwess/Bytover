//
//  Core.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SharedTypes
import SwiftUICore
import SwiftUI
import Serde
import SharedTypes
import PhotosUI
import Photos
import PhotosUI
import Photos
import AVFoundation
import CoreLocation
import Combine;

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
    var environment: CurrentValueSubject<EnvironmentViewModel?, Never> = .init(nil)
    var authentication: CurrentValueSubject<AuthenticationViewModel?, Never> = .init(nil)
    var transfer: CurrentValueSubject<TransferViewModel?, Never> = .init(nil)
    var nearby: CurrentValueSubject<NearbyViewModel?, Never> = .init(nil)
    @Published var isSignedIn = true
    @Published var selectedMediaItems: [PhotosPickerItem] = []
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
            self.nativeProcessor = NativeProcessor(self)
            self.checkLocationAuthorization()
            return handleResponse(request.id, Data(try! CoreOperationOutput.initNativeExecutor.bincodeSerialize()))
        case .appCapabilities(.webView(.openUrl(let url))):
            openURL(URL(string: url)!)
            return handleResponse(request.id, Data(try! CoreOperationOutput.webView(WebViewOperationOutput.openUrl).bincodeSerialize()))
        case .appCapabilities(.localStorage(.loadFileThumbnailPng(let localStoragePath))):
            switch localStoragePath {
            case .platformIdentifier(let phAssetIdentifier):
                let thumbnail = await self.getThumbnailData(for: phAssetIdentifier)
                return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(.loadFileThumbnailPng(thumbnail?.bytes)).bincodeSerialize()))
            default:
                let errorMessage = "Loading thumbnail for non-platform identifier paths is unsupported"
                return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(.loadFileThumbnailPng(nil)).bincodeSerialize()))
            }
        case .appCapabilities(.localStorage(.getWorkDirPath)):
            let documentDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage( LocalStorageOperationOutput.workDirPath(documentDirectory.path)).bincodeSerialize()))
        case .appCapabilities(.localStorage(.getAbsolutePath(let path))):
            let absolutePath = switch path {
            case .absolutePath(let absolute):
                absolute
            case .platformIdentifier(let identifier):
                await PHAsset.getCachedAsset(identifier: identifier)?.fileUrl?.path() ?? ""
            case .relativePath(let relative):
                getDocumentsDirectory().appendingPathComponent(relative).path()
            };
            
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(.getAbsolutePath(absolutePath)).bincodeSerialize()))
        case .appCapabilities(.localStorage(let ops)):
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.localStorage(ops).bincodeSerialize()))
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
            let geoLocation = GeoLocation(latitude: self.lastKnownLocation?.latitude ?? 0.0, longitude: self.lastKnownLocation?.longitude ?? 0.0);
            return handleResponse(request.id, Data(try! CoreOperationOutput.device(.getGeoLocation(geoLocation)).bincodeSerialize()))
        case .appCapabilities(.rpc(let rpc)):
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.rpc(rpc).bincodeSerialize()))
        case .appCapabilities(.void):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.void.bincodeSerialize()))
        case .appCapabilities(.database(let database)):
            return await self.nativeProcessor().handle(request.id, Data(try! CoreOperation.database(database).bincodeSerialize()))
        case .appCapabilities(.render):
            self.update_view(try! .bincodeDeserialize(input: [UInt8](BitBridge.view())))
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.transfer(let trans)):
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.transfer(trans).bincodeSerialize()))
        case .appCapabilities(.internet(let internetOps)):
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.internet(internetOps).bincodeSerialize()))
        case .appCapabilities(.p2P(let p2p)):
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.p2P(p2p).bincodeSerialize()))
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
            return await self.nativeProcessor().handle(request.id, Data (try! CoreOperation.delay(duration).bincodeSerialize()))
        }
    }
    
    func msgFromNative(_ event: Data) async {
        let event: MessageToShell = try! .bincodeDeserialize(input: event.bytes)
        switch event {
        case .handleResponse(let id, let response):
            await self._handleResponse(id, response)
        }
    }
    
    func onMediasChanged() async {
        await self.update(.transfer(.beginLoadingResources))
        for item in self.selectedMediaItems {
            guard let identifier = item.itemIdentifier else { continue }
            
            guard let asset = await PHAsset.getCachedAsset(identifier: identifier)?.asset else {
                continue;
            }
            
            let asset_type = asset.mediaType
            
            let resourceType: ResourceType = {
                switch asset_type {
                case .image:
                    return .image
                case .video:
                    return .video
                default:
                    return .file
                }
            }()
            
            let resourceSelection = ResourceSelection(
                path: .platformIdentifier(identifier),
                type: resourceType
            )
            
            await self.update(.transfer(.addResources([resourceSelection])))
        }
        
        await self.update(.transfer(.endLoadingResources))
        self.selectedMediaItems.removeAll()
    }
    
    func getFileSize(item_identifier: String) async -> UInt64 {
        return await PHAsset.getCachedAsset(identifier: item_identifier)?.fileSize ?? 0
    }

    func getFileName(item_identifier: String) async -> String {
        return await PHAsset.getCachedAsset(identifier: item_identifier)?.fileName ?? ""
    }
    
    func getDocumentsDirectory() -> URL {
        let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        
        return paths[0]
    }
    
    func getThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 128, height: 128)) async -> Data? {
        guard let asset_cached = await PHAsset.getCachedAsset(identifier: itemIdentifier) else {
            return nil
        }
        
        if asset_cached.asset.mediaType == .video {
            return await getVideoThumbnailData(for: itemIdentifier, size: size)
        }
        
        let manager = PHImageManager.default()
        let options = PHImageRequestOptions()
        options.resizeMode = .exact
        options.isNetworkAccessAllowed = true
        options.deliveryMode = .fastFormat
        options.isSynchronous = false
        
        return await withCheckedContinuation { continuation in
            manager.requestImage(
                for: asset_cached.asset,
                targetSize: size,
                contentMode: .aspectFit,
                options: options
            ) { image, info in
                if let image = image, let pngData = image.pngData() {
                    continuation.resume(returning: pngData)
                } else {
                    continuation.resume(returning: nil)
                }
            }
        }
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
                print("on location updated")
                await self.update(.nearby(.onLocationUpdated(GeoLocation(latitude: lastKnownLocation!.latitude, longitude: lastKnownLocation!.longitude))))
                
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
            imageGenerator.generateCGImageAsynchronously(for: CMTime(seconds: 1, preferredTimescale: 60)) { cgImage, time, error in
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
}

@MainActor
class CoreMock: Core {
    static func empty() -> Core {
        CoreMock() as Core
    }
    
    static func withSelectedFileTransfers() -> Core {
        let x = CoreMock() as Core;
        let avatarViewModel = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Bear.png?r=146&g=108&b=85", dominant_color_r: 146, dominant_color_g: 108, dominant_color_b: 85)
        let receive_session = ReceiveSessionViewModel(id: 1, peer_avatar: avatarViewModel, peer_name: "Tien Dang", peer_description: "nearby", resources: [
            ReceiveResourceViewModel(id: 1, name: "ScreenShot.png", display_size: "2.0MB", thumbnail: .none, is_completed: false)
        ], is_completed: false, is_in_progress: true, display_download_speed: "2.0 MB/s", progress: 0.8)
        
        x.transfer = .init(TransferViewModel(selected_resources: [], is_loading_selected_resources: false, transfer_method_selection: .device, nearby_peers: [], received_sessions: [receive_session]));
        x.transfer.value?.selected_resources.append(SelectedResourceViewModel(order_id: 10, name: "Screenshot", size_gb: 0.02, size_mb: 20, display_path: "xyz", thumbnail_path: nil, type: .image, is_valid: false));
        x.transfer.value?.selected_resources.append(SelectedResourceViewModel(order_id: 11, name: "Folder 102384921", size_gb: 1.2, size_mb: 1200, display_path: "xyz", thumbnail_path: nil, type: .folder, is_valid: true));
        return x
    }
    
    override func update(_ event: AppEvent) async {}
    
    override func update_view(_ model: AppViewModel) {}
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
    static func fromPath(_ path: LocalResourcePath) async -> Image? {
        switch path {
        case .absolutePath(let path):
            guard let uiImage = UIImage.fromAbsolutePath(path) else { return nil }
            return Image(uiImage: uiImage)
        case .relativePath(let path):
            let documentsDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            let fullPath = documentsDirectory.appendingPathComponent(path).path
            guard let uiImage = UIImage.fromAbsolutePath(fullPath) else { return nil }
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
    static func getCachedAsset(identifier: String) async -> PHAssetCached? {
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
        
        let cached = PHAssetCached(
            fileName: resource.originalFilename,
            fileSize: UInt64(fileSize),
            fileUrl: await asset.getAbsoluteURL(),
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
