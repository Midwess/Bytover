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
        print("Update view model \(model.transfer?.selected_resources.count)")
        self.transfer = model.transfer
        
        if self.authentication?.user != nil {
            self.is_signed_in = true
        }
    }
    
    func update(_ event: AppEvent) {
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))
        
        var requests: [Request] = try! .bincodeDeserialize(input: effects)
        
        while let request = requests.first {
            requests.removeFirst()
            let data = [UInt8](processEffect(request))
            
            if let newRequests: [Request] = try? .bincodeDeserialize(input: data) {
                requests.append(contentsOf: newRequests)
            }
        }
    }
    
    func processEffect(_ request: Request) -> Data {
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
    
    func onMediasChanged() {
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
        self.update(.transfer(.addResourceSelections(selections)))
    }
    
    func getFileSize(item_identifier: String) -> UInt64 {
        let fetchResult = PHAsset.fetchAssets(withLocalIdentifiers: [item_identifier], options: nil)
        guard let asset = fetchResult.firstObject,
              let resource = PHAssetResource.assetResources(for: asset).first else {
            return 0
        }
        
        // Get file size from resource
        let size = resource.value(forKey: "fileSize") as? Int ?? 0
        return UInt64(size)
    }
    
    func getDocumentsDirectory() -> URL {
        let paths = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)
        
        return paths[0]
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

