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
import Combine
import QuickLookThumbnailing

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
    var quicklook_path: CurrentValueSubject<LocalResourcePath?, Never> = .init(nil)
    
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
            case .platformIdentifier(let identifier):
                let thumbnail = await self.getThumbnailData(for: identifier)
                return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(.loadFileThumbnailPng(thumbnail?.bytes)).bincodeSerialize()))
            default:
                let errorMessage = "Loading thumbnail for non-platform identifier paths is unsupported"
                return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(.loadFileThumbnailPng(nil)).bincodeSerialize()))
            }
        case .appCapabilities(.localStorage(.open(let path))):
            await self.open(path: path)
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.localStorage(.getWorkDirPath)):
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(LocalStorageOperationOutput.workDirPath(WorkDir(private_path: getDocumentsDirectory(isPrivate: true).path, public_path: getDocumentsDirectory(isPrivate: false).path))).bincodeSerialize()))
        case .appCapabilities(.localStorage(.getAbsolutePath(let path))):
            let absolutePath = switch path {
            case .absolutePath(let absolute):
                absolute
            case .platformIdentifier(let identifier):
                await getAbsoluteUrl(from: identifier) ?? ""
            case .relativePath(let relative, let isPrivate):
                getDocumentsDirectory(isPrivate: isPrivate).appendingPathComponent(relative).path
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
            url.startAccessingSecurityScopedResource()

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
        for item in self.selectedMediaItems {
            guard let identifier = item.itemIdentifier else { continue }
            
            let startTime = CFAbsoluteTimeGetCurrent()
            guard let assetCached = await PHAsset.getCachedAsset(identifier: identifier) else {
                continue;
            }
            let executionTime = CFAbsoluteTimeGetCurrent() - startTime
            print("getCachedAsset execution time: \(executionTime) seconds")
            
            let asset = assetCached.asset
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
            
            let phassetUrl = "phasset://" + identifier
            
            let resourceSelection = ResourceSelection(
                path: .platformIdentifier(phassetUrl),
                type: resourceType
            )
            
            await self.update(.transfer(.addResources([resourceSelection])))
        }
        
        await self.update(.transfer(.endLoadingResources))
        self.selectedMediaItems.removeAll()
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
        let privatePath = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first;
        let publicPath = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first;
        
        return isPrivate ? privatePath! : publicPath!
    }
    
    func getThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 1024, height: 1024)) async -> Data? {
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
        else if itemIdentifier.starts(with: "bookmark://") {
            guard let absolutePath = await getAbsoluteUrl(from: itemIdentifier) else {
                print("Cannot generate an absolute url from bookmark: \(itemIdentifier)")
                return nil
            }
            
            let url = URL(fileURLWithPath: absolutePath)

            let scale = 1.0
            let thumbnailSize = CGSize(width: size.width, height: size.height)
            let request = QLThumbnailGenerator.Request(
                fileAt: url,
                size: thumbnailSize,
                scale: scale,
                representationTypes: .thumbnail
            )
            
            return await withCheckedContinuation { continuation in
                QLThumbnailGenerator.shared.generateRepresentations(for: request) { thumbnail, _, error in
                    if let thumbnail = thumbnail, let pngData = thumbnail.uiImage.pngData() {
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
        
        // Create avatar view models for different peers
        let bearAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Bear.png?r=146&g=108&b=85", dominant_color_r: 146, dominant_color_g: 108, dominant_color_b: 85)
        let foxAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Fox.png?r=221&g=155&b=104", dominant_color_r: 221, dominant_color_g: 155, dominant_color_b: 104)
        let wolfAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Wolf.png?r=128&g=128&b=128", dominant_color_r: 128, dominant_color_g: 128, dominant_color_b: 128)
        let lionAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Lion.png?r=193&g=154&b=107", dominant_color_r: 193, dominant_color_g: 154, dominant_color_b: 107)
        let tigerAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Tiger.png?r=212&g=142&b=85", dominant_color_r: 212, dominant_color_g: 142, dominant_color_b: 85)
        let pandaAvatar = AvatarViewModel(url: "https://cdn.devlog.studio/public/animal_avatars/Panda.png?r=40&g=40&b=40", dominant_color_r: 40, dominant_color_g: 40, dominant_color_b: 40)
        
        // Create resource view models
        let path = LocalResourcePath.absolutePath("");
        let resource1 = SelectedResourceViewModel(order_id: 1, name: "ScreenShot.png", size_gb: 0, size_mb: 2.0, display_path: "/Photos/ScreenShot.png", path: path, thumbnail_path: nil, type: .image, is_valid: true)
        let resource2 = SelectedResourceViewModel(order_id: 2, name: "Document.pdf", size_gb: 0, size_mb: 5.3, display_path: "/Documents/Document.pdf", path: path, thumbnail_path: nil, type: .image, is_valid: true)
        let resource3 = SelectedResourceViewModel(order_id: 3, name: "Presentation.pptx", size_gb: 0, size_mb: 12.7, display_path: "/Documents/Presentation.pptx", path: path, thumbnail_path: LocalResourcePath.absolutePath(""), type: .file, is_valid: true)
        let resource4 = SelectedResourceViewModel(order_id: 4, name: "Video.mp4", size_gb: 0.25, size_mb: 256, display_path: "/Videos/Video.mp4", path: path, thumbnail_path: nil, type: .video, is_valid: true)
        let resource5 = SelectedResourceViewModel(order_id: 5, name: "Archive.zip", size_gb: 0, size_mb: 45, display_path: "/Downloads/Archive.zip", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource6 = SelectedResourceViewModel(order_id: 6, name: "Image1.jpg", size_gb: 0, size_mb: 3.2, display_path: "/Photos/Image1.jpg", path: path, thumbnail_path: nil, type: .image, is_valid: true)
        let resource7 = SelectedResourceViewModel(order_id: 7, name: "Image2.jpg", size_gb: 0, size_mb: 2.8, display_path: "/Photos/Image2.jpg", path: path, thumbnail_path: nil, type: .image, is_valid: true)
        let resource8 = SelectedResourceViewModel(order_id: 8, name: "Image3.jpg", size_gb: 0, size_mb: 4.1, display_path: "/Photos/Image3.jpg", path: path, thumbnail_path: nil, type: .image, is_valid: true)
        let resource9 = SelectedResourceViewModel(order_id: 9, name: "Project.zip", size_gb: 0, size_mb: 78, display_path: "/Projects/Project.zip", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource10 = SelectedResourceViewModel(order_id: 10, name: "Music.mp3", size_gb: 0, size_mb: 8.5, display_path: "/Music/Music.mp3", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource11 = SelectedResourceViewModel(order_id: 11, name: "Report.docx", size_gb: 0, size_mb: 1.2, display_path: "/Documents/Report.docx", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource12 = SelectedResourceViewModel(order_id: 12, name: "Spreadsheet.xlsx", size_gb: 0, size_mb: 3.7, display_path: "/Documents/Spreadsheet.xlsx", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource13 = SelectedResourceViewModel(order_id: 13, name: "Backup.tar", size_gb: 0.32, size_mb: 320, display_path: "/Backups/Backup.tar", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource14 = SelectedResourceViewModel(order_id: 14, name: "Notes.txt", size_gb: 0, size_mb: 0.045, display_path: "/Documents/Notes.txt", path: path, thumbnail_path: nil, type: .video, is_valid: true)
        let resource15 = SelectedResourceViewModel(order_id: 15, name: "Config.json", size_gb: 0, size_mb: 0.012, display_path: "/Settings/Config.json", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource16 = SelectedResourceViewModel(order_id: 16, name: "Script.sh", size_gb: 0, size_mb: 0.008, display_path: "/Scripts/Script.sh", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        let resource17 = SelectedResourceViewModel(order_id: 17, name: "Photo_album.zip", size_gb: 0.156, size_mb: 156, display_path: "/Photos/Photo_album.zip", path: path, thumbnail_path: nil, type: .file, is_valid: true)
        // Create 11 receive sessions with different properties
        let receive_session1 = ReceiveSessionViewModel(
            id: 1, 
            peer_avatar: bearAvatar, 
            peer_name: "Tien Dang", 
            peer_description: "nearby", 
            image_resources: [
                ImageReceiveResourceViewModel(model: resource1, is_completed: false)
            ], 
            video_resources: [], 
            file_resources: [],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "2.0 MB/s", 
            progress: 0.8
        )
        
        let receive_session2 = ReceiveSessionViewModel(
            id: 2, 
            peer_avatar: foxAvatar, 
            peer_name: "Alex Smith", 
            peer_description: "nearby", 
            image_resources: [
                ImageReceiveResourceViewModel(model: resource2, is_completed: true)
            ], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource3, is_completed: false)
            ],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "1.5 MB/s", 
            progress: 0.45
        )
        
        let receive_session5 = ReceiveSessionViewModel(
            id: 5, 
            peer_avatar: tigerAvatar, 
            peer_name: "Sarah Johnson", 
            peer_description: "nearby", 
            image_resources: [
                ImageReceiveResourceViewModel(model: resource6, is_completed: true),
                ImageReceiveResourceViewModel(model: resource6, is_completed: true),
                ImageReceiveResourceViewModel(model: resource6, is_completed: true),
                ImageReceiveResourceViewModel(model: resource7, is_completed: true),
                ImageReceiveResourceViewModel(model: resource8, is_completed: true),
                ImageReceiveResourceViewModel(model: resource8, is_completed: true),
            ],
            video_resources: [], 
            file_resources: [],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "950 KB/s", 
            progress: 0.67
        )
        
        let receive_session6 = ReceiveSessionViewModel(
            id: 6, 
            peer_avatar: pandaAvatar, 
            peer_name: "David Wilson", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource9, is_completed: false)
            ],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "2.8 MB/s", 
            progress: 0.35
        )
        
        let receive_session7 = ReceiveSessionViewModel(
            id: 7, 
            peer_avatar: bearAvatar, 
            peer_name: "Emily Brown", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource10, is_completed: true)
            ],
            is_completed: true, 
            is_in_progress: false, 
            display_download_speed: "0 KB/s", 
            progress: 1.0
        )
        
        let receive_session8 = ReceiveSessionViewModel(
            id: 8, 
            peer_avatar: foxAvatar, 
            peer_name: "Michael Taylor", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource11, is_completed: false),
                FileReceiveResourceViewModel(model: resource12, is_completed: false)
            ],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "1.1 MB/s", 
            progress: 0.22
        )
        
        let receive_session9 = ReceiveSessionViewModel(
            id: 9, 
            peer_avatar: wolfAvatar, 
            peer_name: "Jessica Martinez", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource13, is_completed: false)
            ],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "4.5 MB/s", 
            progress: 0.05
        )
        
        let receive_session10 = ReceiveSessionViewModel(
            id: 10, 
            peer_avatar: lionAvatar, 
            peer_name: "Daniel Anderson", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [
                VideoReceiveResourceViewModel(model: resource14, is_completed: true)
            ], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource15, is_completed: true),
                FileReceiveResourceViewModel(model: resource16, is_completed: true)
            ],
            is_completed: true, 
            is_in_progress: false, 
            display_download_speed: "0 KB/s", 
            progress: 1.0
        )
        
        let receive_session11 = ReceiveSessionViewModel(
            id: 11, 
            peer_avatar: tigerAvatar, 
            peer_name: "Olivia Wilson", 
            peer_description: "nearby", 
            image_resources: [], 
            video_resources: [], 
            file_resources: [
                FileReceiveResourceViewModel(model: resource17, is_completed: false)
            ],
            is_completed: false, 
            is_in_progress: true, 
            display_download_speed: "3.7 MB/s", 
            progress: 0.58
        )
        
        // Initialize the transfer view model with all receive sessions
        x.transfer = .init(TransferViewModel(
            selected_resources: [], 
            is_loading_selected_resources: false, 
            transfer_method_selection: .device, 
            nearby_peers: [], 
            received_sessions: [
                receive_session1, receive_session2, receive_session5,
                receive_session6, receive_session7, receive_session8, receive_session9, receive_session10, receive_session11
            ]
        ));
        
        // Add selected resources
        x.transfer.value?.selected_resources.append(SelectedResourceViewModel(order_id: 10, name: "Screenshot", size_gb: 0.02, size_mb: 20, display_path: "xyz", path: path, thumbnail_path: nil, type: .image, is_valid: false));
        x.transfer.value?.selected_resources.append(SelectedResourceViewModel(order_id: 11, name: "Folder 102384921", size_gb: 1.2, size_mb: 1200, display_path: "xyz", path: path, thumbnail_path: nil, type: .file, is_valid: true));
        
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
    static func fromPath(_ path: LocalResourcePath, core: Core) async -> Image? {
        switch path {
        case .absolutePath(let path):
            guard let uiImage = UIImage.fromAbsolutePath(path) else {
                print("There is no image at \(path)")
                return nil
            }
            return Image(uiImage: uiImage)
        case .relativePath(let path, let isPrivate):
            let workdir = await core.getDocumentsDirectory(isPrivate: isPrivate)
            let fullPath = workdir.appendingPathComponent(path).path
            guard let uiImage = UIImage.fromAbsolutePath(fullPath) else {
                print("There is no image at \(path)")
                return nil
            }
            return Image(uiImage: uiImage)
        case .platformIdentifier(let identifier):
            guard let cachedAsset = await PHAsset.getCachedAsset(identifier: identifier),
                  let fileUrl = cachedAsset.fileUrl,
                  let uiImage = UIImage.fromURL(fileUrl) else {
                print("There is no image at \(path)")
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
        
        let startTime = CFAbsoluteTimeGetCurrent()
        let url = includeUrl ? await asset.getAbsoluteURL() : nil
        let executionTime = CFAbsoluteTimeGetCurrent() - startTime
        print("getAbsoluteURL execution time: \(executionTime) seconds")
        
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
