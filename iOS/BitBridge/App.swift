//
//  App.swift
//  BitBridge
//
//  Created by Dang Minh Tien on 28/1/25.
//

import SwiftUI
import SharedTypes

@main
struct Main: App {
    @StateObject private var core = Core()
    
    init() {
        core.update(AppEvent.environment(.appLaunched))
    }

    var body: some Scene {
        WindowGroup {
            ContentView()
                .onOpenURL(perform: {url in
                    core.update(.authentication(.onRedirected(url: url.absoluteString)))
                })
        }
    }
}
