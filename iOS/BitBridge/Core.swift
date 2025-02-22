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
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var environment: EnvironmentViewModel?
    @Published var authentication: AuthenticationViewModel?
    
    @Published var is_signed_in = false

    @Environment(\.openURL) private var openURL
    
    init() {
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        update_view(app)
    }
    
    func update_view(_ model: AppViewModel) {
        self.authentication = model.authentication
        self.environment = model.environment
        
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
}
