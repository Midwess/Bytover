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

@MainActor
class Core: ObservableObject {
    @Published var environment: EnvironmentViewModel?
    @Published var authentication: AuthenticationViewModel?
    @Published var transfer: TransferViewModel?
    
    @Published var is_signed_in = false
    
    @Published var selectedMediaItems: [PhotosPickerItem] = []
    
    @Environment(\.openURL) private var openURL
    
    init() {
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        update_view(app)
    }
    
    func update_view(_ model: AppViewModel) {
        self.authentication = model.authentication
        self.environment = model.environment
        self.transfer = model.transfer
        
        if self.authentication?.user != nil {
            self.is_signed_in = true
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
    
    func processEffect(_ request: Request) async -> Data {
        switch request.effect {
        case .appCapabilities(.webView(.openUrl(let url))):
            openURL(URL(string: url)!)
            return handleResponse(request.id, Data(try! CoreOperationOutput.webView( WebViewOperationOutput.openUrl).bincodeSerialize()))
        case .appCapabilities(.localStorage(.getWorkDirPath)):
            let documentDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage( LocalStorageOperationOutput.workDirPath(documentDirectory.path)).bincodeSerialize()))
        case .appCapabilities(.localStorage(.loadFileSizeFromPlatformIdentifier(let identifier))):
            let fileSize = self.getFileSize(item_identifier: identifier)
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(LocalStorageOperationOutput.loadFileSizeFromPlatformIdentifier(fileSize)).bincodeSerialize()))
        case .appCapabilities(.localStorage(.loadFileNameFromPlatformIdentifier(let identifier))):
            let fileName = self.getFileName(item_identifier: identifier)
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage(LocalStorageOperationOutput.loadFileNameFromPlatformIdentifier(fileName)).bincodeSerialize()))
        case .appCapabilities(.localStorage(.loadFileThumbnailPngFromPlatformIdentifier(let identifier))):
            let fileThumbnailData = await self.getThumbnailData(for: identifier)
            let response = CoreOperationOutput.localStorage(LocalStorageOperationOutput.loadFileThumbnailPngFromPlatformIdentifier(fileThumbnailData?.bytes))
            return handleResponse(request.id, try! Data(response.bincodeSerialize()))
        case .appCapabilities(.localStorage(let ops)):
            return nativeHandle(request.id, Data (try! CoreOperation.localStorage(ops).bincodeSerialize()))
        case .appCapabilities(.device(.getDeviceInfo)):
            let device = UIDevice.current
            let deviceId = UIDevice.current.identifierForVendor?.uuidString ?? ""
            let deviceName = device.name
            
            return handleResponse(request.id, Data(try! CoreOperationOutput.device(
                .deviceInfo(DeviceInfo(
                    platform: Platform.ios,
                    name: deviceName,
                    unique_id: deviceId
                ))
            ).bincodeSerialize()))
        case .appCapabilities(.rpc(let rpc)):
            return nativeHandle(request.id, Data (try! CoreOperation.rpc(rpc).bincodeSerialize()))
        case .appCapabilities(.void):
            return nativeHandle(request.id, Data(try! CoreOperation.void.bincodeSerialize()))
        case .appCapabilities(.database(let database)):
            return nativeHandle(request.id, Data(try! CoreOperation.database(database).bincodeSerialize()))
        case .appCapabilities(.render):
            self.update_view(try! .bincodeDeserialize(input: [UInt8](BitBridge.view())))
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.transfer(let trans)):
            return nativeHandle(request.id, Data (try! CoreOperation.transfer(trans).bincodeSerialize()))
        }
    }
    
    func onMediasChanged() async {
        var selections: [ResourceSelection] = []
        for item in self.selectedMediaItems {
            guard let identifier = item.itemIdentifier else { continue }
            
            let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [identifier], options: nil)
            guard let asset = fetchResult.firstObject else { continue }
            
            let resources = PHAssetResource.assetResources(for: asset)
            guard let resource = resources.first else { continue }
            
            let resourceType: ResourceType = {
                switch asset.mediaType {
                case .image:
                    return .image
                case .video:
                    return .video
                default:
                    return .other
                }
            }()
            
            // Create resource selection
            let resourceSelection = ResourceSelection(
                data: .platformIdentifier(identifier),
                type: resourceType,
                name: resource.originalFilename
            )
            
            selections.append(resourceSelection)
        }
        
        self.selectedMediaItems.removeAll()
        await self.update(.transfer(.addResourceSelections(selections)))
    }
    
    func getFileSize(item_identifier: String) -> UInt64 {
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [item_identifier], options: nil)
        guard let asset = fetchResult.firstObject,
              let resource = PHAssetResource.assetResources(for: asset).first else {
            return 0
        }
        
        let size = resource.value(forKey: "fileSize") as? Int ?? 0
        return UInt64(size)
    }

    func getFileName(item_identifier: String) -> String {
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [item_identifier], options: nil)
        guard let asset = fetchResult.firstObject,
              let resource = PHAssetResource.assetResources(for: asset).first else {
            return "Unknown"
        }
        
        return resource.originalFilename
    }
    
    func getDocumentsDirectory() -> URL {
        let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        
        return paths[0]
    }
    
    func displayResourcePath(path: LocalResourcePath) -> String {
        switch path {
        case .localPath(let localPath): return localPath
        case .platformIdentifier(let platformPath): return platformPath
        }
    }
    
    public func bytesToMB(bytesLength: Float) -> Float {
        let result = bytesLength / 1024 / 1024
        return roundf(result * 10) / 10
    }
    
    public func bytesToGB(bytesLength: Float) -> Float {
        let result = bytesToMB(bytesLength: bytesLength) / 1024
        return roundf(result * 10) / 10
    }

    func getThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 96, height: 96)) async -> Data? {
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [itemIdentifier], options: nil)
        guard let asset = fetchResult.firstObject else {
            return nil
        }
        
        if asset.mediaType == .video {
            return await getVideoThumbnailData(for: itemIdentifier, size: size)
        }
        
        let manager = PHImageManager.default()
        let options = PHImageRequestOptions()
        options.resizeMode = .fast
        options.isNetworkAccessAllowed = true
        options.isSynchronous = false
        
        return await withCheckedContinuation { continuation in
            manager.requestImage(
                for: asset,
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

    func getVideoThumbnailData(for itemIdentifier: String, size: CGSize = CGSize(width: 96, height: 96)) async -> Data? {
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [itemIdentifier], options: nil)
        guard let asset = fetchResult.firstObject, asset.mediaType == .video else {
            return nil
        }
        
        let manager = PHImageManager.default()
        let options = PHVideoRequestOptions()
        options.isNetworkAccessAllowed = true
        
        // Request the AVAsset
        let avAsset = await withCheckedContinuation { continuation in
            manager.requestAVAsset(forVideo: asset, options: options) { avAsset, _, _ in
                continuation.resume(returning: avAsset)
            }
        }
        
        guard let avAsset = avAsset else {
            return nil
        }
        
        // Generate the thumbnail
        let imageGenerator = AVAssetImageGenerator(asset: avAsset)
        imageGenerator.appliesPreferredTrackTransform = true
        imageGenerator.maximumSize = size
        
        let cgImage: CGImage? = await withCheckedContinuation { continuation in
            imageGenerator.generateCGImageAsynchronously(for: CMTime(seconds: 1, preferredTimescale: 60)) { cgImage, time, error in
                if let cgImage = cgImage {
                    continuation.resume(returning: cgImage)
                } else {
                    // If there's an error or no image, just return nil
                    print("Error or no image generated: \(error?.localizedDescription ?? "Unknown error")")
                    continuation.resume(returning: nil)
                }
            }
        }
        
        // If we got a valid CGImage, convert it to UIImage
        if let cgImage = cgImage {
            return UIImage(cgImage: cgImage).pngData()
        } else {
            return nil
        }
    }
}

struct DataUrl: Transferable {
    let url: URL
    
    static var transferRepresentation: some TransferRepresentation {
        FileRepresentation(contentType: .data) { data in
            SentTransferredFile(data.url)
        } importing: { received in
            Self(url: received.file)
        }
    }
}

extension Data {
    /// Converts Data to an array of UInt8 bytes
    var bytes: [UInt8] {
        return [UInt8](self)
    }
    
    /// Initializes Data from an array of UInt8 bytes
    init(bytes: [UInt8]) {
        self.init(bytes)
    }
}

@MainActor
class CoreMock: Core {
    static func empty() -> Core {
        CoreMock() as Core
    }
    
    static func withSelectedFileTransfers() -> Core {
        let x = CoreMock() as Core;
        x.transfer = TransferViewModel(session: TransferSession(order_id: 1, resources: [], processes: []), selected_resources: []);
        x.transfer?.selected_resources.append(LocalResource(name: "Screenshot", size: 200000000, path: .localPath("xyz"), thumbnail_path: "/local/thumbnail", type: .image));
        x.transfer?.selected_resources.append(LocalResource(name: "Folder 102384921", size: 1238310, path: .localPath("xyz"), thumbnail_path: nil, type: .folder));
        x.transfer?.selected_resources.append(LocalResource(name: "Video 29323", size: 500000, path: .localPath("/local"), thumbnail_path: nil, type: .video));
        x.transfer?.selected_resources.append(LocalResource(name: "File 1", size: 100000, path: .localPath("ocal"), thumbnail_path: nil, type: .file));
        return x
    }
    
    override func update(_ event: AppEvent) async {}
    
    override func update_view(_ model: AppViewModel) {}
}

// Extension for UIImage to load from local path
extension UIImage {
    /// Initializes a UIImage from a local file path
    /// - Parameter path: The file path as a String
    /// - Returns: An optional UIImage, nil if loading fails
    static func fromRelativePath(_ path: String) -> UIImage? {
        let documentDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
        return UIImage(contentsOfFile: documentDirectory.path.appending("/").appending(path))
    }
    
    /// Initializes a UIImage from a URL (must be a file URL)
    /// - Parameter url: The file URL
    /// - Returns: An optional UIImage, nil if loading fails
    static func fromURL(_ url: URL) -> UIImage? {
        guard url.isFileURL else { return nil }
        return UIImage(contentsOfFile: url.path)
    }
}

extension Image {
    static func fromRelativePath(_ path: String) -> Image? {
        if let uiImage = UIImage.fromRelativePath(path) {
            return Image(uiImage: uiImage)
        }
        return nil
    }
}
