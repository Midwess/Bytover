//
//  Core.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 12/2/25.
//

import Foundation
import SharedTypes

@MainActor
class Core: ObservableObject {
    @Published var app_view_model: AppViewModel
    
    init() {
        let app: AppViewModel = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        self.app_view_model = app;
    }
    
    func update(_ event: AppEvent) {
        let effects = [UInt8](processEvent(Data(try! event.bincodeSerialize())))
        
        let requests: [Request] = try! .bincodeDeserialize(input: effects)
        for request in requests {
            processEffect(request)
        }
    }
    
    func processEffect(_ request: Request) {
        switch request.effect {
        case .render:
            app_view_model = try! .bincodeDeserialize(input: [UInt8](BitBridge.view()))
        }
    }
}
