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
import Combine

@MainActor
class Core: ObservableObject, ShellRuntime {
    @Published var environment: EnvironmentViewModel?
    @Published var authentication: AuthenticationViewModel?
    @Published var transfer: TransferViewModel?
    
    @Published var is_signed_in = true
    
    @Published var selectedMediaItems: [PhotosPickerItem] = []
    
    @Environment(\.openURL) private var openURL
    
    lazy var nativeProcessor: NativeProcessor = NativeProcessor(self)

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
            return self.nativeProcessor.handle(request.id, Data (try! CoreOperation.localStorage(ops).bincodeSerialize()))
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
        case .appCapabilities(.device(.getGeoLocation)):
            let geoLocation = await self.getGeoLocation();
            return handleResponse(request.id, Data(try! CoreOperationOutput.device(.getGeoLocation(geoLocation)).bincodeSerialize()))
        case .appCapabilities(.rpc(let rpc)):
            return self.nativeProcessor.handle(request.id, Data (try! CoreOperation.rpc(rpc).bincodeSerialize()))
        case .appCapabilities(.void):
            return self.nativeProcessor.handle(request.id, Data(try! CoreOperation.void.bincodeSerialize()))
        case .appCapabilities(.database(let database)):
            return self.nativeProcessor.handle(request.id, Data(try! CoreOperation.database(database).bincodeSerialize()))
        case .appCapabilities(.render):
            self.update_view(try! .bincodeDeserialize(input: [UInt8](BitBridge.view())))
            return handleResponse(request.id, Data(try! CoreOperationOutput.void.bincodeSerialize()))
        case .appCapabilities(.transfer(let trans)):
            return self.nativeProcessor.handle(request.id, Data (try! CoreOperation.transfer(trans).bincodeSerialize()))
        case .appCapabilities(.internet(let internetOps)):
            return self.nativeProcessor.handle(request.id, Data (try! CoreOperation.internet(internetOps).bincodeSerialize()))
        }
    }
    
    func msgFromNative(_ event: Data) {
        
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
                    return .file
                }
            }()
            
            // Create resource selection
            let resourceSelection = ResourceSelection(
                path: .platformIdentifier(identifier),
                type: resourceType
            )
            
            selections.append(resourceSelection)
        }
        
        self.selectedMediaItems.removeAll()
        await self.update(.transfer(.addResources(selections)))
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

    func getGeoLocation() async -> GeoLocation? {
        let locationManager = CLLocationManager()
        
        // Request permission if not already granted
        switch locationManager.authorizationStatus {
        case .notDetermined:
            locationManager.requestWhenInUseAuthorization()
        case .restricted, .denied:
            // Request again even if previously denied
            locationManager.requestWhenInUseAuthorization()
        case .authorizedWhenInUse, .authorizedAlways:
            locationManager.startUpdatingLocation()
        @unknown default:
            locationManager.requestWhenInUseAuthorization()
        }
        
        // Wait for location update with timeout
        for _ in 0..<10 { // Try for about 10 seconds
            if let location = locationManager.location {
                print("Found geolocation \(location)")
                return GeoLocation(latitude: location.coordinate.latitude, longitude: location.coordinate.longitude)
            }
            
            try? await Task.sleep(nanoseconds: 1_000_000_000) // Wait 1 second
        }
        
        return nil
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
        x.transfer = TransferViewModel(selected_resources: [], transfer_method_selection: .device);
        x.transfer?.selected_resources.append(SelectedResourceViewModel(order_id: 10, name: "Screenshot", size_gb: 0.02, size_mb: 20, display_path: "xyz", thumbnail_path: nil, type: .image));
        x.transfer?.selected_resources.append(SelectedResourceViewModel(order_id: 11, name: "Folder 102384921", size_gb: 1.2, size_mb: 1200, display_path: "xyz", thumbnail_path: nil, type: .folder));
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

extension LocalResourcePath {
    func asString() -> String {
        switch self {
        case .localPath(let path): return path
        case .platformIdentifier(let identifier): return identifier
        }
    }
}

extension Image {
    static func fromRelativePath(_ path: LocalResourcePath) -> Image? {
        if let uiImage = UIImage.fromRelativePath(path.asString()) {
            return Image(uiImage: uiImage)
        }
        
        return nil
    }
}

extension Double {
    func display() -> String {
        String(self)
    }
}

extension UIImage {
    var averageColor: UIColor? {
        guard let inputImage = CIImage(image: self) else { return nil }
        let extentVector = CIVector(x: inputImage.extent.origin.x, y: inputImage.extent.origin.y, z: inputImage.extent.size.width, w: inputImage.extent.size.height)
        
        guard let filter = CIFilter(name: "CIAreaAverage", parameters: [kCIInputImageKey: inputImage, kCIInputExtentKey: extentVector]) else { return nil }
        guard let outputImage = filter.outputImage else { return nil }
        
        var bitmap = [UInt8](repeating: 0, count: 4)
        let context = CIContext(options: [.workingColorSpace: kCFNull!])
        context.render(outputImage, toBitmap: &bitmap, rowBytes: 4, bounds: CGRect(x: 0, y: 0, width: 1, height: 1), format: .RGBA8, colorSpace: nil)
        
        return UIColor(red: CGFloat(bitmap[0]) / 255, green: CGFloat(bitmap[1]) / 255, blue: CGFloat(bitmap[2]) / 255, alpha: 1)
    }
    
    var backgroundColor: UIColor {
        var hue: CGFloat = 0
        var saturation: CGFloat = 0
        var brightness: CGFloat = 0
        var alpha: CGFloat = 0
        self.averageColor?.getHue(&hue, saturation: &saturation, brightness: &brightness, alpha: &alpha)
        return UIColor(hue: hue, saturation: 0.6, brightness: 0.3, alpha: alpha)
    }
}

class LocationManager: NSObject, ObservableObject, CLLocationManagerDelegate {
    private let locationManager = CLLocationManager()
    @Published var location: CLLocation?
    @Published var authorizationStatus: CLAuthorizationStatus
    
    override init() {
        authorizationStatus = locationManager.authorizationStatus
        
        super.init()
        locationManager.delegate = self
        locationManager.desiredAccuracy = kCLLocationAccuracyBest
    }
    
    func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
        authorizationStatus = manager.authorizationStatus
        
        if manager.authorizationStatus == .authorizedWhenInUse || 
           manager.authorizationStatus == .authorizedAlways {
            locationManager.startUpdatingLocation()
        }
    }
    
    func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
        guard let location = locations.last else { return }
        self.location = location
    }
}
