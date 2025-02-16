//
//  Core.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SharedTypes
import SwiftUICore
import UIKit
import Serde

@MainActor
class Core: ObservableObject {
    @Published var app_view_model: AppViewModel
    @Environment(\.openURL) private var openURL
    
    init() {
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        self.app_view_model = app;
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
            print("Open url \(url)")
            openURL(URL(string: url)!)
            return handleResponse(request.id, Data(try! CoreOperationOutput.webView( WebViewOperationOutput.openUrl).bincodeSerialize()))
        case .appCapabilities(.localStorage(.getWorkDirPath)):
            let documentDirectory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask).first!
            return handleResponse(request.id, Data(try! CoreOperationOutput.localStorage( LocalStorageOperationOutput.workDirPath(documentDirectory.path)).bincodeSerialize()))
        case .appCapabilities(.device(.getDeviceInfo)):
            let device = UIDevice.current
            let deviceId = UIDevice.current.identifierForVendor?.uuidString ?? ""
            let deviceName = device.name // or device.model for generic name like "iPhone"
            
            return handleResponse(request.id, Data(try! CoreOperationOutput.device(
                .deviceInfo(DeviceInfo(
                    platform: Platform.iOs,
                    name: deviceName,
                    unique_id: deviceId
                ))
            ).bincodeSerialize()))
        case .appCapabilities(.rpc(.getSignInUrl(let deviceInfo))):
            return nativeHandle(request.id, Data (try! CoreOperation.rpc(.getSignInUrl(deviceInfo)).bincodeSerialize()))
        case .appCapabilities(.void):
            print("Received void");
            return nativeHandle(request.id, Data(try! CoreOperation.void.bincodeSerialize()))
        }
    }
}
